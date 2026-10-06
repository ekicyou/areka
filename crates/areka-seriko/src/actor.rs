//! アクター駆動モジュール — inbox メッセージ・CueSink ブリッジ・停止経路・アクター本体。
//!
//! 受け口（単一の出力契約 [`dola::cue::CueSink`] を実装する [`SerikoSink`] ブリッジ）と inbox
//! メッセージ列挙（[`SerikoMsg`]）・停止経路（mpsc チャネル）に加え、アクター本体の spawn
//! （[`spawn_seriko`]）・inbox ハンドラ（解釈 2.1→状態更新 2.2→発行 2.3 を一本の経路で結ぶ）・
//! 単一発行点（[`emit_display`]）を持つ。
//!
//! # 一本の経路（解釈→状態→発行）
//!
//! [`spawn_seriko`] は解決テーブル・静的 bind 集合・出力先を受け、独立スレッド上で
//! [`areka_actor::run_inbox`] ループを回す（1.3）。各発火は到着順（FIFO・単一スレッド）に
//! `cue_target_of` で分類され、Shell 系 `Emote{key}` のみが「解決（[`SurfaceResolver`]）→
//! 状態確定（[`ScopeStates::apply`]）→発行（[`emit_display`]）」の一本経路を通る。状態確定から
//! 表示指令発行までは単一関数 [`emit_display`] に集約され、後続の時間駆動ループ（`seriko-loop`）が
//! 同じ発行点を再利用できる（5.3）。broadcast（D4）で seriko は全 cue を受け取るため、担当外
//! （非 Shell）・純粋 Wait の受信は「正常な担当外受信」＝良性 `debug!`＋skip（action を無視し
//! duration を honor・R2.2/2.3/5.4）、名前形バルーン key（NameForm）は文字の層が読むので `debug!`＋skip、
//! 破損入力（Unresolved/Invalid=`error!`／EntityRef=`warn!`）のみが真の異常。いずれもループは継続する。停止は Close 受領・全 Sender drop の 2 経路（1.4）。
//!
//! # 結線契約（受け口＝単一の出力契約）
//!
//! 演者非依存の**単一の出力契約** [`dola::cue::CueSink`]（task 4.1）を実装する [`SerikoSink`] が
//! 差し込み口となる。`emit` は届いた [`TalkCue`] を [`SerikoMsg::Cue`] として専用 inbox（std mpsc）
//! へ橋渡しする。ghost live-path も `CueSink` 注入で結線する（task 8.1 で旧 `SurfaceSink` 経路を
//! 撤去し `CueSink` 一本へ集約・broadcast＋演者側 relevance ゆえ役割別トレイトは不要）。
//!
//! # 停止経路の形
//!
//! inbox は std mpsc チャネル。停止は 2 経路——(1) [`SerikoMsg::Close`] 受領、(2) 全 [`SerikoSink`]
//! （＝全 `Sender`）drop による受信端の `RecvError`——で、後続 3.2 が結ぶ `run_inbox` ループを
//! 正常終了させる。本タスクは列挙とチャネル形のみ用意し、ループ自体は結線しない。
//!
//! # 失敗経路のログ規律（infallible・silent failure 禁止）
//!
//! 単一の出力契約 [`dola::cue::CueSink`] の `emit` は infallible（`()` 返し）で送出本体
//! [`SerikoSink::deliver`] を用いる。inbox 全受信端が消失した後の送出は `send` が `Err`
//! を返すが、`unwrap`／`expect` で panic させず [`tracing::error!`] で観測して戻る
//! （log-first・R6.3／通常入力で panic しない・R6.4）。

use std::collections::BTreeMap;
use std::ops::ControlFlow;

use areka_sakura::{ActorKey, CueCommand, CueTarget, TalkCue, cue_target_of};

use crate::bind::{
    BindChoicePolicy, BindDirective, BindResolver, parse_bind_directive, scope_namespace,
};
use crate::looper::{LoopRuntime, SerikoLoopConfig};
use crate::output::{DisplayCommand, RebaseKind, RebasedShow, SurfaceOutput};
use crate::resolve::{BalloonResolve, SurfaceResolver, SurfaceTarget, resolve_balloon_key};
use crate::state::{ApplyOutcome, BindApplyOutcome, ScopeStates, Slot};
use crate::table::AnimationTable;

/// seriko アクターの inbox メッセージ（areka-actor inbox 規約・投函経路は inbox 一貫）。
///
/// 共有 Close 型は無い規約に従い、`SakuraMsg::Close` を先例に自前 `Close` を持つ（DD3）。
#[derive(Debug)]
pub enum SerikoMsg {
    /// broadcast された 1 発火（`SerikoSink::emit` が橋渡しする・到着順に適用＝R1.5）。
    Cue(TalkCue),
    /// 時間駆動ループの 1 tick（絶対時刻 ms・素の `u64`＝新規依存なし・D-1）。
    ///
    /// cue と同一 inbox を FIFO 共有し、`SerikoSink::send_tick` が橋渡しする。`handle_message` の
    /// Tick 腕が `LoopRuntime::on_tick` を回し、既存 `emit_display` 単一発行点から発行する（R1.1/6.3）。
    Tick { now_ms: u64 },
    /// 定義の差し替えの依頼（spec: areka-P0-shell-balloon-switch 要件 2.6・2.8・3.5）。
    ///
    /// [`spawn_seriko`] の受信の閉包が [`handle_message`] より先に捌く（定義を所有する殻の仕事）。
    Replace(Box<SerikoReplace>),
    /// kanade 由来の停止指令（areka-actor 停止規約の Close 相当・正常終了させる）。
    Close,
}

/// 定義の差し替えの依頼の中身（spec: areka-P0-shell-balloon-switch 要件 2.6・2.8・3.5）。
///
/// `epoch` は UI の差し替えの世代で、出力の合図 [`DisplayCommand::Rebased`] にそのまま載る。
pub enum SerikoReplace {
    /// シェルの定義（別名表・静的な着せ替え・着せ替えの名前表・シェルのアニメ表）。
    Shell {
        epoch: u64,
        resolver: SurfaceResolver,
        static_binds: areka_emo_compose::BindSet,
        bind_resolver: BindResolver,
        shell_table: AnimationTable,
    },
    /// バルーンの定義（スコープごとのバルーンのアニメ表）。
    Balloon {
        epoch: u64,
        balloon_tables: BTreeMap<ActorKey, AnimationTable>,
    },
}

/// 表は大きいので種別と世代だけを出す（ログ用）。
impl std::fmt::Debug for SerikoReplace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shell { epoch, .. } => write!(f, "SerikoReplace::Shell {{ epoch: {epoch} }}"),
            Self::Balloon { epoch, .. } => {
                write!(f, "SerikoReplace::Balloon {{ epoch: {epoch} }}")
            }
        }
    }
}

/// 単一の出力契約 [`dola::cue::CueSink`] を実装する送出ブリッジ（cue 再生ランタイムが保持する結線契約）。
///
/// inbox（std mpsc）の `Sender` を内包し、届いた発火を [`SerikoMsg::Cue`] として橋渡しする。
/// broadcast された全 cue が本 sink へ届き、担当（Shell）選別は [`handle_message`] の演者側
/// relevance（`cue_target_of`）が行う。
///
/// `Clone` を導出する（内側 `mpsc::Sender<SerikoMsg>` は常に `Clone`）。全 clone は単一の
/// seriko アクター inbox への送信端であり配送意味は同一（cue 再生ランタイム・`areka_ghost::boot`
/// が要求する `dola::cue::CueSink + Clone + Send + 'static` を満たし、dispatcher が talk ごとに
/// sink を clone しても全 cue が同一 inbox へ FIFO 到着する）。
#[derive(Clone)]
pub struct SerikoSink {
    tx: std::sync::mpsc::Sender<SerikoMsg>,
}

impl SerikoSink {
    /// inbox の `Sender` からブリッジを組む。
    ///
    /// 後続 3.2 の `spawn_seriko` がアクター inbox の送信端から構築する。std mpsc の
    /// `Sender` は `Clone`（複製すれば複数 sink 口へ配れる）だが、本タスクでは単一送出端で足りる。
    pub(crate) fn new(tx: std::sync::mpsc::Sender<SerikoMsg>) -> Self {
        Self { tx }
    }

    /// アクターへ [`SerikoMsg::Close`] を送り、正常停止を要求する（R1.4）。
    ///
    /// kanade による停止駆動（単一 Close funnel）の受け口。`spawn_seriko` が返した
    /// `SerikoSink` から停止を送れるようにする最小 API で、終了同期テスト（本タスク・後続 4.1）
    /// が `ActorHandle::join` と対にして使う。受信端消失時は `send` が `Err` を返すが、
    /// アクターは既に停止済み（＝目的達成）ゆえ `error!` は不要——`Ok`/`Err` を呼び手へ返す。
    pub fn close(&self) -> Result<(), std::sync::mpsc::SendError<SerikoMsg>> {
        self.tx.send(SerikoMsg::Close)
    }

    /// 時間駆動ループの 1 tick（絶対時刻 ms）を inbox へ橋渡しする（R1.1）。
    ///
    /// loop ticker（本番）／テストの直接注入がアクターへ tick を届ける最小 API。cue と同一 inbox を
    /// FIFO 共有するため、tick と cue は構造的に直列化され状態競合しない。受信端消失時（アクター
    /// 停止後）は `send` が `Err` を返すが、それは **shutdown 中の期待事象**（PresentBridge 先例）
    /// ゆえ `error!` でなく [`tracing::debug!`] で観測して戻る（silent failure 禁止・panic しない・R7.5/6.3）。
    pub fn send_tick(&self, now_ms: u64) {
        if self.tx.send(SerikoMsg::Tick { now_ms }).is_err() {
            tracing::debug!(
                now_ms,
                "seriko: inbox が消失; tick を配送できず破棄した（shutdown 中の期待事象・PresentBridge 先例・R7.5）"
            );
        }
    }

    /// 定義の差し替えの依頼を inbox へ送る（spec: areka-P0-shell-balloon-switch 要件 2.6・3.5）。
    ///
    /// 受信端が消えていれば `false` を返す（呼び手が `error!` を残す）。
    pub fn send_replace(&self, replace: SerikoReplace) -> bool {
        self.tx.send(SerikoMsg::Replace(Box::new(replace))).is_ok()
    }

    /// 1 発火を inbox へ橋渡しする送出本体（infallible）——単一の出力契約
    /// [`dola::cue::CueSink`] の `emit` が用いる。
    ///
    /// 受信端（inbox／アクター）が消失していると `send` は `Err` を返すが、`unwrap`／`expect`
    /// では panic するため用いず、[`tracing::error!`] で落とした発火を記録して戻る
    /// （silent failure 禁止・R6.3／通常運転で panic しない・R6.4）。
    fn deliver(&mut self, cue: TalkCue) {
        if let Err(err) = self.tx.send(SerikoMsg::Cue(cue)) {
            // Err の内側に move された SerikoMsg::Cue から発火の識別情報を復元してログへ載せる。
            let SerikoMsg::Cue(dropped) = err.0 else {
                unreachable!("emit が送るのは常に SerikoMsg::Cue");
            };
            tracing::error!(
                at = dropped.at,
                scope = %dropped.actor,
                command = ?dropped.command,
                "seriko inbox が消失: surface 発火を配送できず破棄した（受信端全消失）"
            );
        }
    }
}

/// **単一の出力契約**（R11.6・task 4.1）: 演者非依存の [`dola::cue::CueSink`] を実装する。
///
/// `CuePlayer` は登録された全 sink へ全 cue を **broadcast** し、seriko は担当（Shell）か否かを
/// 演者側 relevance（`cue_target_of`）で選別する——action を無視しても duration は honor する
/// （担当外 cue も本 sink 経由で inbox へ届き、[`handle_message`] が良性に読み飛ばす・R2.2/R2.3）。
/// 配送先スロットの 2 分割（旧 `SurfaceSink`/`TextSink`）は broadcast＋演者側 relevance ゆえ廃した
/// （task 8.1 で暫定並存の `SurfaceSink` 実装を撤去し `CueSink` 一本へ集約）。
impl dola::cue::CueSink for SerikoSink {
    fn emit(&mut self, cue: TalkCue) {
        self.deliver(cue);
    }
}

/// 単一発行点（5.3）— 状態確定結果 [`DisplayCommand`] を発行先へ渡す**唯一**の関数。
///
/// `SurfaceOutput::send` を呼ぶのはこの関数だけであり、cue 適用駆動（本タスク）でも後続の
/// 時間駆動ループ（`seriko-loop`）でも、状態確定→表示指令発行はこの一点を通す。分岐ごとに
/// `out.send` を散らさないことで、発行の観測点・不変条件を一箇所に集約する。
fn emit_display<O: SurfaceOutput>(out: &mut O, command: DisplayCommand) {
    out.send(command);
}

/// アクター起動: 解決テーブル＋静的 bind 集合＋bind 名前解決層＋出力先を受け、独立スレッドで
/// 稼働させる（1.3）。
///
/// [`areka_actor::spawn_actor`]`::<SerikoMsg, _>("seriko", body)` で名前付きスレッドを起動し
/// （span `actor="seriko"` は spawn 原語が付与する）、返した `Sender` から組んだ [`SerikoSink`] を
/// 第 1 要素に、[`areka_actor::ActorHandle`] を第 2 要素に返す。`body` は `resolver`・
/// `static_binds`（[`ScopeStates::new`] へ move）・`bind_resolver`（`\![bind]` の名前解決層・
/// [`handle_message`] へ `&` 手渡し）・`out` を単独所有し、[`areka_actor::run_inbox`] で発火を
/// 到着順（FIFO）に処理する。
///
/// `bind_resolver` は additive 追加（D4）。bind 名前表を供給しない既存経路は
/// [`BindResolver::empty`] を渡せば従来と byte 同値。その根拠は宣言集合が空であることではなく、
/// **名前表が空＝[`BindResolver::resolve`] が常に `None`** であること——空リゾルバは
/// 全カテゴリが正典の既定（[`BindChoicePolicy::Default`]＝着衣は排他置換）になるが、そもそも
/// 着せ替え ID を解決できず適用へ到達しないため発行に影響しない（bindopt 設計 D3）。
///
/// # 停止（1.4）
///
/// [`SerikoMsg::Close`] 受領（handler が `Break`）または全 `Sender` drop（inbox 切断）の
/// 2 経路で正常終了する。前者は [`SerikoSink::close`]、後者は全 [`SerikoSink`] drop で駆動する。
///
/// # 失敗経路・担当外受信（6.1/6.2/6.3/6.4）
///
/// 真の異常——解決不能（[`SurfaceTarget::Unresolved`]）・破損バルーン数値（`Invalid`）は
/// `error!`＋skip、防御枝 `EntityRef` は `warn!`＋skip。名前形バルーン key（`NameForm`）は文字の層が
/// 読むので `debug!`＋skip。
/// 対して broadcast の担当外受信——非 Shell（Balloon 系）・純粋 Wait は「正常経路」ゆえ良性
/// `debug!`＋skip（action 無視・duration honor・新ローカル遅延なし・R2.2/2.3/5.4）。いずれも
/// ループを殺さず継続する（silent failure 禁止・入力起因では panic しない）。
pub fn spawn_seriko<O>(
    resolver: SurfaceResolver,
    static_binds: areka_emo_compose::BindSet,
    bind_resolver: BindResolver,
    loop_config: SerikoLoopConfig,
    out: O,
) -> (SerikoSink, areka_actor::ActorHandle)
where
    O: SurfaceOutput + Send + 'static,
{
    spawn_seriko_clocked(
        resolver,
        static_binds,
        bind_resolver,
        loop_config,
        out,
        None,
    )
}

/// 刻みと同じ時計（単調・ミリ秒・spec: areka-P0-animated-image-playback 要件 1.6・2.9・3.1）。
pub type SerikoClock = std::sync::Arc<dyn Fn() -> u64 + Send + Sync>;

/// 時計つきの起動（[`spawn_seriko`] は `clock = None` でここへ委ねる）。
///
/// 台本の合図（`\s`・`\b`・着せ替え）を処理するときに `clock` を読み、その時刻で時計を始める。
/// `clock` が無ければ直前の刻みの時刻を使う（design「時計の開始の時刻」）。経過は「今の時刻 −
/// 開始の時刻」を 0 で止めて求める（別スレッドが読んだ刻みの時刻が開始より僅かに前でも負にならない）。
pub fn spawn_seriko_clocked<O>(
    resolver: SurfaceResolver,
    static_binds: areka_emo_compose::BindSet,
    bind_resolver: BindResolver,
    loop_config: SerikoLoopConfig,
    out: O,
    clock: Option<SerikoClock>,
) -> (SerikoSink, areka_actor::ActorHandle)
where
    O: SurfaceOutput + Send + 'static,
{
    let (tx, actor) = areka_actor::spawn_actor::<SerikoMsg, _>("seriko", move |rx| {
        let mut states = ScopeStates::new(static_binds);
        let mut out = out;
        // 定義（別名表・着せ替えの名前表）は差し替えの依頼でだけ替わる。
        let mut resolver = resolver;
        let mut bind_resolver = bind_resolver;
        // アクター本体が SERIKO ループ統括器を単独所有する（スレッド内・ロック不要・単一所有者）。
        // 表・乱数は `loop_config` から構築して以後この 1 スレッドで進める（発見 C の値渡し解消）。
        let mut loop_runtime = LoopRuntime::new(loop_config).with_clock(clock);
        areka_actor::run_inbox::<SerikoMsg, std::convert::Infallible>(rx, move |msg| {
            // 差し替えの依頼は定義を所有するこの殻が先に捌く（handle_message の署名は不変）。
            if let SerikoMsg::Replace(replace) = msg {
                // 設計の順: 定義 → スコープの状態 → ループの表 → 合図 1 件。
                // `extra` は合図に必ず載せるスコープ（バルーンは新しい表の鍵＝装着の全スコープ）。
                let (epoch, kind, extra) = match *replace {
                    SerikoReplace::Shell {
                        epoch,
                        resolver: new_resolver,
                        static_binds,
                        bind_resolver: new_bind_resolver,
                        shell_table,
                    } => {
                        resolver = new_resolver;
                        bind_resolver = new_bind_resolver;
                        states.rebase_shell(static_binds);
                        loop_runtime.replace_shell_table(shell_table);
                        (epoch, RebaseKind::Shell, Vec::new())
                    }
                    SerikoReplace::Balloon {
                        epoch,
                        balloon_tables,
                    } => {
                        let scopes: Vec<ActorKey> = balloon_tables.keys().cloned().collect();
                        states.rebase_balloon();
                        loop_runtime.replace_balloon_tables(balloon_tables);
                        (epoch, RebaseKind::Balloon, scopes)
                    }
                };
                tracing::info!(epoch, ?kind, "seriko: 定義を差し替えた");
                emit_display(&mut out, rebased(epoch, kind, &states, extra));
                return Ok(ControlFlow::Continue(()));
            }
            Ok(handle_message(
                &resolver,
                &bind_resolver,
                &mut states,
                &mut loop_runtime,
                &mut out,
                msg,
            ))
        });
    });

    (SerikoSink::new(tx), actor)
}

/// 差し替えの後の最初の表示を載せた合図（spec: areka-P0-shell-balloon-switch 要件 2.6・3.5）。
///
/// シェルは各スコープの今の面（無ければ `None`）と新しい既定の着せ替え、バルーンは各スコープの
/// バルーンの今の面（無ければ既定の 0）と空の着せ替えを載せる。スコープは seriko が見た
/// スコープと `extra` の和（`\s`・`\b` の届いていないスコープも起動時に面 0 で確立されている）。
fn rebased(
    epoch: u64,
    kind: RebaseKind,
    states: &ScopeStates,
    extra: Vec<ActorKey>,
) -> DisplayCommand {
    let (slot, binds) = match kind {
        RebaseKind::Shell => (Slot::Shell, states.binds().clone()),
        RebaseKind::Balloon => (Slot::Balloon, areka_emo_compose::BindSet::default()),
    };
    let mut surfaces: BTreeMap<ActorKey, Option<u32>> =
        states.current_surfaces(slot).into_iter().collect();
    for scope in extra {
        surfaces.entry(scope).or_insert(None);
    }
    let shows = surfaces
        .into_iter()
        .map(|(scope, surface_id)| RebasedShow {
            scope,
            surface_id: match kind {
                RebaseKind::Shell => surface_id,
                RebaseKind::Balloon => surface_id.or(Some(0)),
            },
            binds: binds.clone(),
        })
        .collect();
    DisplayCommand::Rebased { epoch, kind, shows }
}

/// inbox メッセージ 1 件を処理し、`run_inbox` 用の [`ControlFlow`] を返す。
///
/// - [`SerikoMsg::Close`] → `Break`（正常終了・1.4）。
/// - [`SerikoMsg::Cue`] → 演者側 relevance（`cue_target_of`・D4 単一権威）で担当（Shell）を選別し、
///   `Emote{key}`／`BalloonSurface` のみが解決層へ進む一本経路。担当外（非 Shell・純粋 Wait）は
///   良性 `debug!`＋skip（正常経路・duration honor）、破損入力は `warn!`/`error!`＋skip。常に `Continue`。
///   `cue_target_of == None` 枝の内側（Wait 判定より前・D1）に `\![bind]` 名前自己選別分岐を持ち、
///   `name == "bind"` のキャリアのみ `bind_resolver` で名前解決し [`ScopeStates::apply_bind`] を通す。
fn handle_message<O: SurfaceOutput>(
    resolver: &SurfaceResolver,
    bind_resolver: &BindResolver,
    states: &mut ScopeStates,
    loop_runtime: &mut LoopRuntime,
    out: &mut O,
    msg: SerikoMsg,
) -> ControlFlow<()> {
    let cue = match msg {
        // 正常停止（1.4）。積み残しは run_inbox の即時 return で破棄される。
        SerikoMsg::Close => return ControlFlow::Break(()),
        // 時間駆動ループの 1 tick（1.1/6.3）: LoopRuntime へ委譲し、返る指令列を**既存**の
        // `emit_display` 単一発行点のみで発行する（新発行点を作らない・R6.3）。表示中 slot が 1 つも
        // ない tick は on_tick が空を返す＝完全 no-op（無発行・2.1 の表示中ゲートの自然帰結）。
        SerikoMsg::Tick { now_ms } => {
            for cmd in loop_runtime.on_tick(now_ms, states) {
                emit_display(out, cmd);
            }
            return ControlFlow::Continue(());
        }
        // 差し替えの依頼は spawn_seriko の殻が先に捌く。ここへ届くのは殻を経ない誤配だけ。
        SerikoMsg::Replace(replace) => {
            tracing::error!(
                ?replace,
                "seriko: 差し替えの依頼が殻を経ずに届いたので捨てる（定義は不変）"
            );
            return ControlFlow::Continue(());
        }
        SerikoMsg::Cue(cue) => cue,
    };

    // 分類（DD1/D4/6.2）: Shell 系のみ本アクターが action する。broadcast（D4）で seriko は
    // 全 cue を受け取るため、担当外（非 Shell）・純粋 Wait の受信は「異常」でなく「正常な担当外
    // 受信」——action を無視しつつ duration を honor（seriko は自前 reveal/timeline を持たず
    // タイミングは焼き込み絶対時刻が担うので、skip が新ローカル遅延を生まない＝否定的 no-op）。
    // ゆえに良性 debug!＋skip（warn/error でない・実害ない水準・R2.2/2.3/5.4）。genuine anomaly
    // （Unresolved/Invalid=error!／EntityRef=warn!）は下流で severity を維持する。
    match cue_target_of(&cue.command) {
        Some(CueTarget::Shell) => {}
        Some(other) => {
            // Balloon 系（Text/NewLine/Clear/ClearAll/Choice→emo-text の担当）。broadcast の
            // 正常な担当外受信ゆえ action を無視し duration は honor（新ローカル遅延なし）して skip。
            tracing::debug!(
                target = ?other,
                command = ?cue.command,
                "seriko: 担当外（非 Shell）cue を broadcast 受信; action を無視し duration を honor して読み飛ばす（正常経路・R2.2/2.3）"
            );
            return ControlFlow::Continue(());
        }
        None => {
            // `cue_target_of==None` は Wait（純粋な待ち・action なし）と Custom（`\!` 汎用キャリア）。
            // bind 消費分岐（D1）: Wait 判定より前に `\![bind]` を名前自己選別で捌く。キャリアを
            // 開封し `name == "bind"` のみが bind パイプラインへ入る。非該当・非キャリアは既存の
            // 良性 skip へフォールスルー（dola の名前写像 API は退役ゆえ参照しない・D1/D10）。
            match cue.command.as_command_carrier() {
                // 非キャリア（純粋 Wait・非正準 params の Custom・その他）。
                None => {
                    if matches!(cue.command, CueCommand::Wait) {
                        // Wait は「分類不能」でなく「どの演者の担当でもない」ゆえ文言を分ける（申し送り是正）。
                        tracing::debug!(
                            command = ?cue.command,
                            "seriko: 純粋 Wait cue を broadcast 受信; どの演者の担当でもない待ち＝action なし・duration は焼き込み絶対時刻が担うため読み飛ばす（正常経路・R5.4）"
                        );
                    } else if let CueCommand::Custom { command, .. } = &cue.command {
                        // 非正準 params の Custom: 宛名規律（D8④）で severity を峻別する。`Custom{command}`
                        // フィールドは開封失敗でも読めるため宛名で判断する——宛名 `bind`＝自分宛の壊れ物
                        // ゆえ warn!（ワイヤ破損は宛名の担当者が報告）・他人宛/未知名＝担当外ゆえ debug!。
                        if command == "bind" {
                            tracing::warn!(
                                command = ?cue.command,
                                "seriko: 自分宛（bind）の非正準 Custom params を読み飛ばす（ワイヤ破損・D8④）"
                            );
                        } else {
                            tracing::debug!(
                                command = ?cue.command,
                                "seriko: 他人宛/未知名の非正準 Custom params を読み飛ばす（担当外・報告責任は宛名の担当者・D8④）"
                            );
                        }
                    } else {
                        // 上記以外の非キャリア（M-boot compile は非生成）。broadcast 下では一律良性ゆえ debug!。
                        tracing::debug!(
                            command = ?cue.command,
                            "seriko: 担当の演者がいない cue を broadcast 受信; 読み飛ばす（正常経路）"
                        );
                    }
                    return ControlFlow::Continue(());
                }
                // 正準キャリア（`Custom` の String Array）。名前自己選別で bind のみ消費する。
                Some((name, tokens)) => {
                    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1
                    if name != "bind" {
                        // 未登記/他担当名（move 等・`bind-noevent` 等も含む）は静かに読み流す
                        // （名前自己選別・R2.5・一意性は areka 消費者台帳が保証・D1/D10）。
                        tracing::debug!(
                            name,
                            command = ?cue.command,
                            "seriko: 担当外コマンド名のキャリア cue を読み飛ばす（名前自己選別・R2.5）"
                        );
                        return ControlFlow::Continue(());
                    }

                    // (step 3) 引数解釈（純関数・不透明保持）。M1 縮退の正典形／破損入力を severity 分けする。
                    let (category, part, on) = match parse_bind_directive(&tokens) {
                        BindDirective::Apply { category, part, on } => (category, part, on),
                        BindDirective::Toggle { .. } | BindDirective::CategoryWide { .. } => {
                            // トグル形・カテゴリ単位形＝M1 未実導出の正当構文（将来 additive シーム・R4.2・D8②）。
                            tracing::warn!(
                                command = ?cue.command,
                                "seriko: bind のトグル形/カテゴリ単位形は M1 未実導出のため読み飛ばす（正当構文・将来 additive・R4.2・D8②）"
                            );
                            return ControlFlow::Continue(());
                        }
                        BindDirective::Malformed => {
                            // カテゴリ欠落・on/off 値破損＝破損入力（D8③）。
                            tracing::error!(
                                command = ?cue.command,
                                "seriko: bind の破損入力（カテゴリ欠落・on/off 値破損）を読み飛ばす（D8③）"
                            );
                            return ControlFlow::Continue(());
                        }
                    };

                    // (step 4) scope→名前空間（D7）。"0"→sakura・"1"→kero・"2"+/非数値→写像なし。
                    let ns = match scope_namespace(&cue.actor) {
                        Some(ns) => ns,
                        None => {
                            // 写像なし（char2+ は M1 未取込・M-dual 拡張シーム・D7・D8⑤）。
                            tracing::warn!(
                                scope = %cue.actor,
                                "seriko: bind の scope 写像なし（scope 2 以降・非数値）で読み飛ばす（M-dual 拡張シーム・D7・D8⑤）"
                            );
                            return ControlFlow::Continue(());
                        }
                    };

                    // (step 5) 名前解決（未宣言は捏造せず None・R3.7）。解決不能は error!＋skip・状態不変・発行なし。
                    let id = match bind_resolver.resolve(ns, &category, &part) {
                        Some(id) => id,
                        None => {
                            tracing::error!(
                                scope = %cue.actor,
                                category = %category,
                                part = %part,
                                "seriko: bind の (カテゴリ, パーツ) を名前解決できず読み飛ばす（未宣言・状態不変・発行なし・R3.7・D8①）"
                            );
                            return ControlFlow::Continue(());
                        }
                    };

                    // (step 6) 適用＋発行（bindopt 設計 D1/D2）。カテゴリの 3 値ポリシーで分岐する:
                    // 複数可（Multiple）でないカテゴリ——mustselect（bindopt 3.1）と既定（非宣言・
                    // 高々 1 個・bindopt 2.1）——の着衣（on=true）は排他置換（同カテゴリ他パーツを
                    // 自動 off）、複数可の着衣は加算（bindopt 3.3）、mustselect 以外の脱衣は除去
                    // （bindopt 2.2/3.3）。Changed のみ単一発行点から発行し、実機 grep マーカーを発火する。
                    let policy = bind_resolver.policy(ns, &category);

                    // mustselect の脱衣は正典「解除不可」: 集合を変えず読み流し、痕跡を warn! で
                    // 残す（bindopt 3.2・bindopt D1）。無言の握り潰しにしないため実機の既定ログ水準（info）で
                    // 見える warn を選ぶ。
                    if !on && policy == BindChoicePolicy::MustSelect {
                        tracing::warn!(
                            scope = %cue.actor,
                            category = %category,
                            part = %part,
                            id,
                            on,
                            "seriko: mustselect カテゴリの脱衣指示を無視（正典・解除不可・bindopt 3.2）"
                        );
                        return ControlFlow::Continue(());
                    }

                    let outcome = if on && policy != BindChoicePolicy::Multiple {
                        // MustSelect（従来どおり・bindopt 3.1）／Default（本 spec の是正・bindopt 2.1）
                        // の着衣は排他置換。
                        let cat_ids = bind_resolver.category_ids(ns, &category);
                        states.apply_bind_exclusive(&cue.actor, &cat_ids, id)
                    } else {
                        // Multiple の着衣＝加算（bindopt 3.3）／MustSelect 以外の脱衣＝除去
                        // （bindopt 2.2/3.3）。
                        states.apply_bind(&cue.actor, id, on)
                    };
                    match outcome {
                        BindApplyOutcome::Changed(command) => {
                            // 部品のコマを新しい着せ替えで載せ直した Show があればそれを、無ければ
                            // apply の指令を 1 件だけ出す（spec: areka-P0-surface-element-nesting 要件 5.12）。
                            // 時計は合図を処理する今読む（spec: areka-P0-animated-image-playback 要件 3.1）。
                            let at_ms = loop_runtime.event_ms();
                            let command = loop_runtime
                                .refresh(&cue.actor, Slot::Shell, at_ms, states)
                                .unwrap_or(command);
                            emit_display(out, command); // 単一発行点（R3.5）
                            // 実機サインオフの grep マーカー（R7.1・有界 auto-exit＋ログ grep 流儀）。
                            tracing::info!(
                                scope = %cue.actor,
                                category = %category,
                                part = %part,
                                id,
                                on,
                                "seriko: bind 適用"
                            );
                        }
                        // 非表示/未知 scope（StateOnly）または同値（Unchanged）は発行しない（D5・flow note (2)）。
                        BindApplyOutcome::StateOnly | BindApplyOutcome::Unchanged => {
                            tracing::debug!(
                                scope = %cue.actor,
                                category = %category,
                                part = %part,
                                id,
                                on,
                                "seriko: bind 集合を更新（非表示/未知 scope または同値ゆえ発行なし）"
                            );
                        }
                    }
                    return ControlFlow::Continue(());
                }
            }
        }
    }

    // バルーン面切替は早期分岐で処理する（key 抽出 match の前段・既存 Emote 経路の形に触れない・R4.6）。
    // 解決→適用→発行を arm 内で完結し値を返さないため、値を返す key 抽出 match には混ぜず早期 return する。
    if let CueCommand::BalloonSurface { key } = &cue.command {
        let target = match resolve_balloon_key(key) {
            BalloonResolve::Show(id) => SurfaceTarget::Show(id),
            BalloonResolve::Hide => SurfaceTarget::Hide,
            BalloonResolve::NameForm => {
                // 名前形（\b[バルーン１]）: 行き先は文字の層（emo-text）が箱の名前で決める正当構文。
                // seriko は debug!＋skip・発行なし・状態不変。名前形についての警告は文字の層だけが出す
                // （shell-balloon 4.8/4.9）。
                tracing::debug!(
                    key = %key,
                    scope = %cue.actor,
                    "seriko: 名前の形の `\\b` は文字の層が読む"
                );
                return ControlFlow::Continue(());
            }
            BalloonResolve::Invalid => {
                // 破損数値（-2・範囲外・u32 超過）: 作者入力の破損＝error!＋skip・発行なし（シェル経路と同水準・R4.5）。
                tracing::error!(
                    key = %key,
                    scope = %cue.actor,
                    "seriko: バルーン面 key が不正な数値で読み飛ばす（破損入力・R4.5）"
                );
                return ControlFlow::Continue(());
            }
        };
        // 状態更新（2.2 の鏡映）＋発行: 状態が実際に変化したときだけ単一発行点から発行する（冪等・R4.3）。
        if let ApplyOutcome::Changed(command) = states.apply_balloon(&cue.actor, target) {
            // バルーン面切替／Hide でループ再生をリセット（当該 slot の playback 全除去・R2.3 表示従属）。
            // PatternState クリアは apply_balloon の責務、playback クリアはループ統括器の責務。
            loop_runtime.on_surface_changed(&cue.actor, Slot::Balloon);
            // 一番上の `always` を出来事の時刻で始めた絵が在ればそれを、無ければ apply の指令を 1 件だけ
            // 出す（spec: areka-P0-animated-image-playback 要件 4.1・4.3）。時計は合図を処理する今読む（要件 3.1）。
            let at_ms = loop_runtime.event_ms();
            let command = loop_runtime
                .refresh(&cue.actor, Slot::Balloon, at_ms, states)
                .unwrap_or(command);
            emit_display(out, command); // 単一発行点共用（R4.1/4.2/4.3）
        }
        return ControlFlow::Continue(());
    }

    // Shell 系の command 内訳。実到来は Emote{key} のみ、EntityRef は防御枝（DD5/Risks）。
    let key = match &cue.command {
        CueCommand::Emote { key } => key,
        CueCommand::EntityRef(entity) => {
            // M-boot では非到来。将来 dola 変更時の catch-all 回避のため明示 skip。
            tracing::warn!(
                entity = entity,
                scope = %cue.actor,
                "seriko: EntityRef は M-boot で未対応; 防御的に読み飛ばす（R6.2）"
            );
            return ControlFlow::Continue(());
        }
        // cue_target_of が Shell と分類する variant は上記 2 つのみ（分類表と整合）。
        // 万一新 variant が Shell 分類されたら記録して skip（非 panic・6.4）。
        other => {
            tracing::warn!(
                command = ?other,
                "seriko: 未知の Shell 系 command を受領; 読み飛ばす（R6.2）"
            );
            return ControlFlow::Continue(());
        }
    };

    // 解釈（2.1）: Emote{key} を SurfaceTarget へ。Unresolved は error!＋skip（状態不変・6.1）。
    let target = resolver.resolve(key);
    if target == SurfaceTarget::Unresolved {
        tracing::error!(
            key = %key,
            scope = %cue.actor,
            "seriko: surface を解決できず読み飛ばす（未知 alias／範囲外など・R6.1）"
        );
        return ControlFlow::Continue(());
    }

    // 状態更新（2.2）＋発行（2.3）: 状態が実際に変化したときだけ単一発行点から発行する（冪等ガード）。
    if let ApplyOutcome::Changed(command) = states.apply(&cue.actor, target) {
        // シェル面切替／Hide でループ再生をリセット（当該 slot の playback 全除去・R2.3 表示従属）。
        // PatternState クリアは apply の責務、playback クリアはループ統括器の責務。部品の時計は捨てない。
        loop_runtime.on_surface_changed(&cue.actor, Slot::Shell);
        // 部品のコマが在ればそれを載せた Show に差し替えて、切り替えの発行は 1 件だけ
        // （spec: areka-P0-surface-element-nesting 要件 5.6・5.9）。時計は合図を処理する今読む
        // （spec: areka-P0-animated-image-playback 要件 3.1）。
        let at_ms = loop_runtime.event_ms();
        let command = loop_runtime
            .refresh(&cue.actor, Slot::Shell, at_ms, states)
            .unwrap_or(command);
        emit_display(out, command);
    }

    ControlFlow::Continue(())
}

#[cfg(test)]
#[path = "actor_bind_loop_tests.rs"]
mod bind_loop_tests;
#[cfg(test)]
#[path = "actor_clock_tests.rs"]
mod clock_tests;
#[cfg(test)]
#[path = "actor_dispatch_tests.rs"]
mod dispatch_tests;
#[cfg(test)]
#[path = "actor_parts_tests.rs"]
mod parts_tests;
#[cfg(test)]
#[path = "actor_replace_tests.rs"]
mod replace_tests;
#[cfg(test)]
#[path = "actor_test_support.rs"]
mod test_support;
