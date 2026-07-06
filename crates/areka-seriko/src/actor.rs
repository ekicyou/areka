//! アクター駆動モジュール — inbox メッセージ・SurfaceSink ブリッジ・停止経路・アクター本体。
//!
//! 受け口（`SurfaceSink` を実装する [`SerikoSink`] ブリッジ）と inbox メッセージ列挙
//! （[`SerikoMsg`]）・停止経路（mpsc チャネル）に加え、アクター本体の spawn
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
//! 同じ発行点を再利用できる（5.3）。解釈できない／分類できない入力は記録を残して読み飛ばし
//! （6.1/6.2）、ループは継続する。停止は Close 受領・全 Sender drop の 2 経路（1.4）。
//!
//! # 結線契約（受け口＝trait 実装）
//!
//! sakura（④）の surface 系出力先は [`areka_sakura::SurfaceSink`] 一本であり、その実装
//! [`SerikoSink`] が唯一の差し込み口となる（追加の口を設けない・ghost-setup 期待）。
//! `SerikoSink::emit` は届いた [`TalkCue`] を [`SerikoMsg::Cue`] として専用 inbox（std mpsc）
//! へ橋渡しする。
//!
//! # 停止経路の形
//!
//! inbox は std mpsc チャネル。停止は 2 経路——(1) [`SerikoMsg::Close`] 受領、(2) 全 [`SerikoSink`]
//! （＝全 `Sender`）drop による受信端の `RecvError`——で、後続 3.2 が結ぶ `run_inbox` ループを
//! 正常終了させる。本タスクは列挙とチャネル形のみ用意し、ループ自体は結線しない。
//!
//! # 失敗経路のログ規律（infallible・silent failure 禁止）
//!
//! `SurfaceSink::emit` は infallible（`()` 返し）。inbox 全受信端が消失した後の送出は
//! `send` が `Err` を返すが、`unwrap`／`expect` で panic させず [`tracing::error!`] で
//! 観測して戻る（log-first・R6.3／通常入力で panic しない・R6.4）。

use std::ops::ControlFlow;

use areka_sakura::{cue_target_of, CueCommand, CueTarget, SurfaceSink, TalkCue};

use crate::output::{DisplayCommand, SurfaceOutput};
use crate::resolve::{SurfaceResolver, SurfaceTarget};
use crate::state::{ApplyOutcome, ScopeStates};

/// seriko アクターの inbox メッセージ（areka-actor inbox 規約・投函経路は inbox 一貫）。
///
/// 共有 Close 型は無い規約に従い、`SakuraMsg::Close` を先例に自前 `Close` を持つ（DD3）。
#[derive(Debug)]
pub enum SerikoMsg {
    /// surface 系発火（`SerikoSink::emit` が橋渡しする・到着順に適用＝R1.5）。
    Cue(TalkCue),
    /// kanade 由来の停止指令（areka-actor 停止規約の Close 相当・正常終了させる）。
    Close,
}

/// [`SurfaceSink`] を実装する送出ブリッジ（sakura dispatcher が保持する結線契約）。
///
/// inbox（std mpsc）の `Sender` を内包し、届いた発火を [`SerikoMsg::Cue`] として橋渡しする。
/// この trait 実装が唯一の受け口＝差し込み口であり、追加の注入メソッドは設けない。
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
}

impl SurfaceSink for SerikoSink {
    /// 1 発火を inbox へ橋渡しする（infallible）。
    ///
    /// 受信端（inbox／アクター）が消失していると `send` は `Err` を返すが、`unwrap`／`expect`
    /// では panic するため用いず、[`tracing::error!`] で落とした発火を記録して戻る
    /// （silent failure 禁止・R6.3／通常運転で panic しない・R6.4）。
    fn emit(&mut self, cue: TalkCue) {
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

/// 単一発行点（5.3）— 状態確定結果 [`DisplayCommand`] を発行先へ渡す**唯一**の関数。
///
/// `SurfaceOutput::send` を呼ぶのはこの関数だけであり、cue 適用駆動（本タスク）でも後続の
/// 時間駆動ループ（`seriko-loop`）でも、状態確定→表示指令発行はこの一点を通す。分岐ごとに
/// `out.send` を散らさないことで、発行の観測点・不変条件を一箇所に集約する。
fn emit_display<O: SurfaceOutput>(out: &mut O, command: DisplayCommand) {
    out.send(command);
}

/// アクター起動: 解決テーブル＋静的 bind 集合＋出力先を受け、独立スレッドで稼働させる（1.3）。
///
/// [`areka_actor::spawn_actor`]`::<SerikoMsg, _>("seriko", body)` で名前付きスレッドを起動し
/// （span `actor="seriko"` は spawn 原語が付与する）、返した `Sender` から組んだ [`SerikoSink`] を
/// 第 1 要素に、[`areka_actor::ActorHandle`] を第 2 要素に返す。`body` は `resolver`・
/// `static_binds`（[`ScopeStates::new`] へ move）・`out` を単独所有し、[`areka_actor::run_inbox`]
/// で発火を到着順（FIFO）に処理する。
///
/// # 停止（1.4）
///
/// [`SerikoMsg::Close`] 受領（handler が `Break`）または全 `Sender` drop（inbox 切断）の
/// 2 経路で正常終了する。前者は [`SerikoSink::close`]、後者は全 [`SerikoSink`] drop で駆動する。
///
/// # 失敗経路（6.1/6.2/6.3/6.4）
///
/// 解決不能（[`SurfaceTarget::Unresolved`]）は `error!`＋skip、非 Shell／分類不能／防御枝の
/// `EntityRef` は `warn!`＋skip で、いずれもループを殺さず継続する（silent failure 禁止・
/// 入力起因では panic しない）。
pub fn spawn_seriko<O>(
    resolver: SurfaceResolver,
    static_binds: areka_emo_compose::BindSet,
    out: O,
) -> (SerikoSink, areka_actor::ActorHandle)
where
    O: SurfaceOutput + Send + 'static,
{
    let (tx, actor) = areka_actor::spawn_actor::<SerikoMsg, _>("seriko", move |rx| {
        let mut states = ScopeStates::new(static_binds);
        let mut out = out;
        areka_actor::run_inbox::<SerikoMsg, std::convert::Infallible>(rx, move |msg| {
            Ok(handle_message(&resolver, &mut states, &mut out, msg))
        });
    });

    (SerikoSink::new(tx), actor)
}

/// inbox メッセージ 1 件を処理し、`run_inbox` 用の [`ControlFlow`] を返す。
///
/// - [`SerikoMsg::Close`] → `Break`（正常終了・1.4）。
/// - [`SerikoMsg::Cue`] → 分類（`cue_target_of`）・解決・状態更新・発行の一本経路。Shell 系
///   `Emote{key}` のみが解決層へ進み、それ以外は記録を残して skip（6.1/6.2）。常に `Continue`。
fn handle_message<O: SurfaceOutput>(
    resolver: &SurfaceResolver,
    states: &mut ScopeStates,
    out: &mut O,
    msg: SerikoMsg,
) -> ControlFlow<()> {
    let cue = match msg {
        // 正常停止（1.4）。積み残しは run_inbox の即時 return で破棄される。
        SerikoMsg::Close => return ControlFlow::Break(()),
        SerikoMsg::Cue(cue) => cue,
    };

    // 分類（DD1/6.2）: Shell 系のみ本アクターが扱う。非 Shell・分類不能は warn!＋skip。
    match cue_target_of(&cue.command) {
        Some(CueTarget::Shell) => {}
        Some(other) => {
            // Balloon 等（→emo text-layer）は本アクターの管轄外。到来しない想定だが防御的に skip。
            tracing::warn!(
                target = ?other,
                command = ?cue.command,
                "seriko: 非 Shell 系 cue を受領; surface 系ではないため読み飛ばす（R6.2）"
            );
            return ControlFlow::Continue(());
        }
        None => {
            // 分類不能（Custom 等・M-boot compile は非生成）。記録を残して skip。
            tracing::warn!(
                command = ?cue.command,
                "seriko: 分類できない cue command を受領; 読み飛ばす（R6.2）"
            );
            return ControlFlow::Continue(());
        }
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
        emit_display(out, command);
    }

    ControlFlow::Continue(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::{ActorKey, CueCommand, SurfaceSink, TalkCue};
    use crate::output::{DisplayCommand, MockSurfaceOutput};

    /// テスト用の TalkCue（Shell 系 Emote・at/actor 込み）を組む。
    fn emote_cue(at: f64, scope: &str, key: &str) -> TalkCue {
        TalkCue {
            at,
            actor: ActorKey::from(scope),
            command: CueCommand::Emote { key: key.into() },
        }
    }

    /// 受信端が生きていれば `emit` が `SerikoMsg::Cue` を inbox へ橋渡しする（R1.1/1.2）。
    /// trait 実装＝結線契約（追加の口を設けず SurfaceSink 経由でのみ届く）。
    #[test]
    fn emit_forwards_cue_to_inbox() {
        let (tx, rx) = std::sync::mpsc::channel::<SerikoMsg>();
        let mut sink = SerikoSink::new(tx);

        SurfaceSink::emit(&mut sink, emote_cue(1.5, "0", "smile"));

        match rx.recv() {
            Ok(SerikoMsg::Cue(cue)) => {
                assert_eq!(cue.at, 1.5, "at が保たれる");
                assert_eq!(cue.actor, ActorKey::from("0"), "actor が保たれる");
                assert!(
                    matches!(cue.command, CueCommand::Emote { ref key } if key == "smile"),
                    "Emote{{key}} が保たれる: {:?}",
                    cue.command
                );
            }
            other => panic!("Cue が inbox へ届くこと: {other:?}"),
        }
    }

    /// 受信端（inbox/アクター）消失後に `emit` を呼んでも、`error!` ログを残しつつ
    /// panic せず正常に戻る（infallible 契約・send 失敗を黙殺しない・R6.3/6.4）。
    #[test]
    fn emit_after_receiver_gone_logs_no_panic() {
        let (tx, rx) = std::sync::mpsc::channel::<SerikoMsg>();
        let mut sink = SerikoSink::new(tx);
        // 受信端を drop＝アクター停止後（inbox 全受信端消失）を模す。
        drop(rx);

        // emit は infallible。send 失敗経路（Err）を通り、panic せず戻ること自体が合格条件。
        let logs = capture_logs(|| {
            SurfaceSink::emit(&mut sink, emote_cue(0.0, "0", "smile"));
        });

        // silent failure 禁止（R6.3）: send 失敗が error! として観測できること。
        assert!(
            logs.contains("level=ERROR"),
            "send 失敗が error! ログとして発火すること: {logs}"
        );
        assert!(
            logs.contains("target=areka_seriko"),
            "本クレート target で発火すること: {logs}"
        );
    }

    /// 単発シナリオ（本タスクの主 observable・R1.3/5.3）: 単純な発火 1 件を入力すると、
    /// 期待どおりの表示指令 1 件が観測用出力先へちょうど記録される。
    ///
    /// 解決層（2.1）・状態層（2.2）・発行層（2.3）を独立スレッド上で一本の経路で結び、
    /// `emit_display` 単一発行点から発行されることを end-to-end で固定する。同期は
    /// `Close`→`ActorHandle::join`（Break 後のスレッド終了待ち）で決定論的に行い、sleep を
    /// 一切用いない（先に送った Cue は FIFO 単一スレッドゆえ join 復帰時に処理済み）。
    #[test]
    fn single_cue_emits_one_display_command() {
        use crate::resolve::SurfaceResolver;
        use areka_emo_compose::BindSet;
        use std::collections::BTreeMap;

        // 解決表（"通常"→2100 の 1 件だけ持つ小さな alias 表）と静的 bind 集合。
        let mut aliases: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        aliases.insert("通常".to_string(), vec![2100]);
        let resolver = SurfaceResolver::new(aliases);
        let binds = BindSet::from_ids([1100, 1207]);

        // 観測用出力先。records() ハンドルを move 前に取得しておく。
        let out = MockSurfaceOutput::new();
        let records = out.records();

        // アクター起動→単純な Shell 系 Emote 1 件を emit→Close→join で終了同期。
        let (mut sink, handle) = spawn_seriko(resolver, binds.clone(), out);
        SurfaceSink::emit(&mut sink, emote_cue(0.0, "0", "2100"));
        sink.close().expect("Close を送れること");
        handle.join().expect("Close で正常終了する");

        // 表示指令がちょうど 1 件、期待値どおり記録されていること（全値比較）。
        let recorded = records.lock().expect("records mutex poisoned");
        assert_eq!(recorded.len(), 1, "単発発火で表示指令はちょうど 1 件");
        assert_eq!(
            recorded[0],
            DisplayCommand::Show {
                scope: ActorKey::from("0"),
                surface_id: 2100,
                binds: BindSet::from_ids([1100, 1207]),
            },
            "解決→状態確定→単一発行点発行の一本経路の結果が期待どおり"
        );
    }

    /// テスト専用 tracing 捕捉ハーネス（emo-compose/kanade の log_capture 流儀・
    /// スレッドローカル `with_default` ゆえ並行テスト安全）。
    fn capture_logs<F: FnOnce()>(f: F) -> String {
        use std::sync::{Arc, Mutex};
        use tracing::field::{Field, Visit};
        use tracing_subscriber::prelude::*;

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<String>>>);

        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
            fn on_event(
                &self,
                ev: &tracing::Event<'_>,
                _: tracing_subscriber::layer::Context<'_, S>,
            ) {
                let meta = ev.metadata();
                let mut line = format!("level={} target={}", meta.level(), meta.target());
                struct V<'a>(&'a mut String);
                impl Visit for V<'_> {
                    fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                        use std::fmt::Write;
                        let _ = write!(self.0, " {}={:?}", f.name(), v);
                    }
                }
                ev.record(&mut V(&mut line));
                self.0.lock().unwrap().push(line);
            }
        }

        let cap = Capture::default();
        let logs = cap.0.clone();
        let subscriber = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(subscriber, f);
        let guard = logs.lock().unwrap();
        guard.join("\n")
    }
}
