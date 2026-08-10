//! 再生駆動層（drive）— per-talk transient アクターの起動・自己投函・cue 再生ランタイム glue。
//!
//! [`spawn_talk`] は talk ごとに名前付きスレッド（`sakura-talk-{talk_id}`）を起動し、
//! spawn 直後に [`SakuraMsg::Start`] を自身の inbox へ**自己投函**する（投函経路は inbox
//! 一貫・validation Issue 1 の解決）。以降の `Tick`/`Close` 投函端と join ハンドルを
//! [`TalkHandle`] として呼び出し元へ返す。
//!
//! # cue 再生ランタイムへの委譲（task 7.1）
//!
//! 本層は配送・状態機械・完了判定を**自前実装しない**。talk アクターは dola の受動ランタイム
//! [`dola::cue::CuePlayer`] を包み、注入時刻（`Tick`）を渡すだけの薄い glue に縮小する（D7・R11.4）:
//!
//! - `Start` 受領時に上流 [`areka_parsers::sakura::parse`] → [`crate::compile::compile`] で
//!   コンパイル済み台本 [`CueSheet`] を得る（アンカー未刻印）。空 sheet は時間軸駆動せず
//!   即 [`TalkDone`] を返す（R1.4/R6.2・裸 `\-` は空 sheet＋Quit）。
//! - **初回 `Tick(t)` で絶対開始時刻を刻印する**（dispatch 刻印・R9.1/D6）: talk の再生開始時刻
//!   （＝初回注入時刻 `t`）を [`CueSheet::with_absolute_start_time`] でアンカーとして焼き込み、
//!   刻印済み台本から [`CuePlayer`] を構築して両演者 sink を **broadcast** 登録する。各 cue の
//!   絶対発火時刻は `アンカー + 相対 start_time`＝異なる時刻に再生開始した同一台本は異なる絶対
//!   発火時刻で配送される（配送時導出は禁忌・desync 防止）。
//! - 以降の `Tick(t)` は [`CuePlayer::tick`] へ委譲する（broadcast fan-out は CuePlayer が担う・
//!   中央振り分けは廃止）。完了は [`CuePlayer::is_completed`]（占有 horizon gated）で検知し
//!   [`TalkDone`] を返す（entry 枯渇でなく horizon 到達で完了・早期終了しない・R2.5/D6）。
//!
//! # 高々 1 回の唯一機構
//!
//! body は [`TalkPhase`] の所有権スロットを保持し、全終端経路は「phase を [`TalkPhase::Idle`]
//! へ差し替え → `done.send(D::from(TalkDone))` → 直後に `Break`」で実装する（終端後は phase が
//! Idle かつスレッドが `Break` で消えるため二度目の TalkDone は構造的に不能）。

use std::ops::ControlFlow;
use std::sync::mpsc::Sender;

use crate::compile::compile;
use crate::contract::{
    ChoiceWaiting, CueSheet, SakuraMsg, StartTalk, SystemVarSnapshot, TalkDone, TalkEndReason,
    TalkHandle, TalkId,
};
use crate::error::SakuraError;
use areka_actor::{run_inbox, spawn_actor};
use dola::cue::{BarrierKind, CuePlayer, CuePlayerState, CueSink};

/// per-talk transient を起動し、`Tick`/`Close` の投函端と join ハンドルを返す。
///
/// [`spawn_actor`]`("sakura-talk-{talk_id}", body)` で talk ごとに名前付きスレッドを
/// 起動し（R10.1）、spawn 直後に [`SakuraMsg::Start`]`(start)` を返された `Sender` へ
/// **自己投函**する（投函経路は inbox 一貫・単一 inbox の全順序で `Start` 先行を保証）。
/// 呼び出し元へは [`TalkHandle`]`{inbox, actor}` を返し、以降 kanade/テストは inbox へ
/// `Tick`/`Close` のみ送る。
///
/// `sinks` は演者非依存の単一出力契約 [`CueSink`] の可変長列（S-3）で、初回 `Tick` 時に
/// [`CuePlayer`] へ**登録順のまま**登録され、以降 broadcast で全 cue を受ける（どの action を
/// 演じるかは演者側 relevance の責務・中央振り分けなし・D4）。順序が broadcast 順を決めるため
/// 呼び出し側（ghost の boot 結線）が決定論的な登録順を与える。
///
/// `system_vars` は **talk 起動時に ⓪ghost から手渡される名前→値の凍結スナップショット**
/// （プロパティシステム読み口の凍結像・R7.3）で、`Start` 受領時のコンパイルへそのまま渡す。
/// sakura は値源を所有せず（永続化層・SHIORI・OS 環境を直接読まない）このスナップショットを
/// 参照するだけである（D8: スナップショットは `StartTalk` でなく talk 起動境界で手渡す）。
///
/// `done` は talk からの**完了通知ポート**（呼び出し側 inbox への変換投函）。境界は
/// `D: From<TalkDone> + From<ChoiceWaiting>` で、[`TalkDone`]（通算高々 1 回）と
/// [`ChoiceWaiting`]（選択待ち成立・DD-6）が**同一ポート**を流れる——同一 talk についての
/// 「選択待ち成立」と「再生完了」の因果順が単一 FIFO で保存される（別チャンネルへ分離しない）。
///
/// # Preconditions
///
/// `done` の受信端が生存している（kanade or テスト）。
///
/// # Postconditions
///
/// 終端・中断のいずれでも `TalkDone`（`D` へ変換して）を高々 1 回 `done` へ送り
/// body 復帰（スレッド終了）。受信端が既に drop 済みなら送信は `Err` になるが、
/// `error!` を記録して黙殺せず talk を終える（受信端不在は致命ではない）。
pub fn spawn_talk<D>(
    start: StartTalk,
    done: Sender<D>,
    sinks: Vec<Box<dyn CueSink + Send>>,
    system_vars: SystemVarSnapshot,
) -> TalkHandle
where
    D: From<TalkDone> + From<ChoiceWaiting> + Send + 'static,
{
    let talk_id = start.talk_id;
    let name = format!("sakura-talk-{}", talk_id.0);

    let (inbox, actor) = spawn_actor::<SakuraMsg, _>(&name, move |rx| {
        // 演者 sink（Send 境界付き＝thread へ move 可）を register 順のまま保持する。
        // 初回 Tick で刻印済み台本の CuePlayer へ broadcast 登録する（4.3 register_sink）。
        let mut driver = TalkDriver::new(sinks, system_vars, done);
        run_inbox::<SakuraMsg, std::convert::Infallible>(rx, move |msg| Ok(driver.handle(msg)));
    });

    // 投函経路の一貫: spawn 直後に Start を自己投函する（外部からは送らない）。
    // 送信失敗はアクタースレッドが既に消えている場合のみ（通常不到達）。ログして継続。
    if inbox.send(SakuraMsg::Start(start)).is_err() {
        tracing::error!(actor = %name, "failed to self-post Start; actor thread gone");
    }

    TalkHandle { inbox, actor }
}

/// 1 talk の駆動状態機械（body ローカル・他 talk と共有しない・R10.3）。
///
/// `Start` 受領で `Armed`（コンパイル済み台本を保持・アンカー未刻印）へ、初回 `Tick` で
/// アンカーを刻印して `Driving`（[`CuePlayer`] を保持）へ遷移する。全終端経路は phase を
/// [`TalkPhase::Idle`] へ差し替えて `Break` する（高々 1 回の唯一機構）。
enum TalkPhase {
    /// `Start` 未受領（初期）。終端後もこの状態へ戻す（phase 差し替えの受け皿）。
    Idle,
    /// `Start` 受領・初回 `Tick` 前。アンカー刻印待ちのコンパイル済み台本を保持する。
    Armed {
        /// talk 相関 ID（全出力へ対応付け・R1.3/R6.6）。
        talk_id: TalkId,
        /// コンパイル済み台本（アンカー未刻印＝0.0・初回 Tick で刻印する）。
        sheet: CueSheet,
        /// コンパイル時点で確定した終端理由（自然終端で返す reason）。
        end: TalkEndReason,
    },
    /// 初回 `Tick` 後・駆動中。刻印済みアンカーで構築した [`CuePlayer`] を保持する。
    Driving {
        /// talk 相関 ID。
        talk_id: TalkId,
        /// cue 再生ランタイム（broadcast・完了 horizon・バリア seam を内包）。
        player: CuePlayer,
        /// 終端理由（自然終端で返す reason）。
        end: TalkEndReason,
        /// 直前に処理した `Tick` の時刻（単調・冪等ガード用）。
        last_tick: f64,
    },
}

/// per-talk 駆動アクター本体。[`TalkPhase`] スロットと未登録の演者 sink を保持し、cue 再生の
/// 制御（配送・状態機械・完了）は [`CuePlayer`] へ委譲する（自前実装しない・D7）。
struct TalkDriver<D> {
    /// 駆動状態機械。
    phase: TalkPhase,
    /// 初回 `Tick` で [`CuePlayer`] へ登録する演者 sink（register 順＝呼び出し側が与える broadcast 順）。
    /// 初回 `Tick` で drain して CuePlayer へ move する（それ以降は空）。
    sinks: Vec<Box<dyn CueSink + Send>>,
    /// talk 起動時に ⓪ghost から手渡された名前→値の凍結スナップショット（R7.3・値源非所有）。
    /// `Start` 受領時のコンパイルへ参照渡しする（sakura は値源を所有しない・D8）。
    system_vars: SystemVarSnapshot,
    /// `TalkDone` の届け先（呼び出し側 inbox への変換投函）。
    done: Sender<D>,
    /// 現在の選択待ちバリアについて [`ChoiceWaiting`] を送出済みか（一度きり検出・DD-6）。
    ///
    /// `WaitingForChoice` へ遷移した tick で `true` にし、以降の tick では再送しない
    /// （`CuePlayer` は待機中 tick を早期 return するため状態は待機のまま維持される）。
    /// [`on_resolve_choice`](Self::on_resolve_choice) の解決成功で `false` へ戻す——M1 の
    /// compile は talk あたり高々 1 個の選択待ちバリアしか発行しないため実際には再成立しないが、
    /// 1 トークに複数バリアを持つ将来拡張のシームとしてリセットを残す。
    choice_notified: bool,
}

impl<D> TalkDriver<D>
where
    D: From<TalkDone> + From<ChoiceWaiting> + Send + 'static,
{
    fn new(
        sinks: Vec<Box<dyn CueSink + Send>>,
        system_vars: SystemVarSnapshot,
        done: Sender<D>,
    ) -> Self {
        Self {
            phase: TalkPhase::Idle,
            sinks,
            system_vars,
            done,
            choice_notified: false,
        }
    }

    /// inbox メッセージ 1 件を処理し、`run_inbox` 用の `ControlFlow` を返す。
    fn handle(&mut self, msg: SakuraMsg) -> ControlFlow<()> {
        match msg {
            SakuraMsg::Start(start) => self.on_start(start),
            SakuraMsg::Tick(t) => self.on_tick(t),
            SakuraMsg::Close => self.on_close(),
            SakuraMsg::ResolveChoice { id } => self.on_resolve_choice(id),
        }
    }

    /// `Start` 受領: parse → compile。空 sheet なら即 `TalkDone`→`Break`、非空 sheet なら
    /// `Armed`（アンカー刻印待ち）へ遷移して継続（刻印・CuePlayer 構築は初回 `Tick`）。
    fn on_start(&mut self, start: StartTalk) -> ControlFlow<()> {
        // Start 二重受領は error!＋無視（プロトコル異常・非 panic）。
        if !matches!(self.phase, TalkPhase::Idle) {
            tracing::error!("duplicate Start received; ignoring");
            return ControlFlow::Continue(());
        }

        let StartTalk {
            script,
            talk_id,
            epilogue,
        } = start;

        // 上流パーサで Instruction 列へ変換（再パースしない・R1.2）→ 純粋コンパイル。
        let instructions = areka_parsers::sakura::parse(&script);
        // talk 起動時に手渡された凍結スナップショット（R7.3・D8）を参照してコンパイルする。
        // sakura は値源を所有せず、この凍結像だけを見る（provider 差替で本層は無改変＝差替シーム）。
        let compiled = compile(&instructions, &self.system_vars);

        // epilogue を compile 後・空判定**前**に末尾 carrier cue として付加する（design C12・R3.4）。
        // `epilogue.is_empty()` なら恒等（既存経路完全不変）。epilogue-only talk（空 script＋epilogue）は
        // ここで 1 cue 以上の非空 sheet になり、通常再生して即時完走する（空判定を通過する）。
        let sheet = crate::compile::append_epilogue(compiled.sheet, &epilogue);

        // 空 sheet: 時間軸駆動せず即終端（R1.4/R6.2）。end は Ended 固定でなく compiled.end
        // （裸の `\-` は空 sheet＋Quit）。epilogue 付加後も空なら epilogue も空（恒等）ゆえ従来挙動。
        if sheet.is_empty() {
            self.send_done(talk_id, compiled.end);
            return ControlFlow::Break(());
        }

        // 非空 sheet: 刻印は初回 Tick に遅延（アンカー＝初回注入時刻）。台本を保持して継続。
        self.phase = TalkPhase::Armed {
            talk_id,
            sheet,
            end: compiled.end,
        };
        ControlFlow::Continue(())
    }

    /// `Tick(t)` 受領: 有限・単調ガードの後、cue 再生ランタイムへ委譲する。
    ///
    /// - **初回 `Tick`（`Armed`）**: `t` を絶対開始時刻としてアンカー刻印し（dispatch 刻印・
    ///   R9.1/D6）、刻印済み台本から [`CuePlayer`] を構築、両演者 sink を broadcast 登録して
    ///   `player.tick(t)`。以降の cue 絶対発火時刻は `t + 相対 start_time`。
    /// - **以降の `Tick`（`Driving`）**: 単調ガード後 `player.tick(t)`（broadcast は CuePlayer 内）。
    ///
    /// いずれも [`CuePlayer::is_completed`]（占有 horizon gated）が真なら `TalkDone{end}` を
    /// 送出し `Break`（自然終端・entry 枯渇でなく horizon 到達で完了・R2.5/D6）。
    ///
    /// # ガード（R11.1/11.2/11.3・受信ループは殺さない）
    ///
    /// - **非有限**（`NaN`/`±inf`）: dola の NaN 全量配信ハザードを遮断し、かつ NaN による
    ///   アンカー刻印を防ぐため、[`SakuraError::NonFiniteTick`] を `tracing::error!` で記録し
    ///   `schedule` を一切進めず `Continue`（phase 不変・talk は終端させない）。
    /// - **逆行/同値**（`Driving` の `last_tick` に対し `t <= last_tick`）: `tracing::debug!` の
    ///   no-op で `Continue`。初回 `Tick`（`Armed`）は比較対象が無く必ず通過する（先頭 `at=0`
    ///   発火を殺さないため 0.0 で初期化しない・設計 Issue 2）。
    ///
    /// 状態未確定（`Idle`＝`Start` 未受領・投函経路上は非到達の防御枝）なら no-op で `Continue`。
    fn on_tick(&mut self, t: f64) -> ControlFlow<()> {
        // 非有限ガード（R11.1/11.2）: schedule を進めず記録＋error ログ、ループは継続。
        // NaN/±inf アンカー刻印もここで塞ぐ（刻印前の初回 Tick も本ガードを通す）。
        if !t.is_finite() {
            let err = SakuraError::NonFiniteTick(t);
            tracing::error!(error = %err, "non-finite Tick ignored; schedule not advanced");
            return ControlFlow::Continue(());
        }

        // phase を所有権ごと取り出して分岐する（終端時は Idle のまま・継続時は書き戻す）。
        match std::mem::replace(&mut self.phase, TalkPhase::Idle) {
            TalkPhase::Idle => {
                // 状態未確定は防御枝（投函経路上 Start 先行が保証される・非到達）。
                tracing::error!("Tick received before Start; ignoring");
                ControlFlow::Continue(())
            }
            TalkPhase::Armed {
                talk_id,
                sheet,
                end,
            } => {
                // 初回 Tick: dispatch 刻印（アンカー＝初回注入時刻 t・R9.1/D6）。相対 start_time は
                // 書き換えず、その上にアンカーが載る。刻印済み台本から CuePlayer を構築する。
                let sheet = sheet.with_absolute_start_time(t);
                let mut player = CuePlayer::from_sheet(&sheet);
                // 演者 sink を broadcast 登録（register 順: surface, text・4.3）。
                for sink in self.sinks.drain(..) {
                    player.register_sink(sink);
                }
                player.tick(t);
                self.settle_after_tick(talk_id, player, end, t)
            }
            TalkPhase::Driving {
                talk_id,
                mut player,
                end,
                last_tick,
            } => {
                // 単調ガード（逆行/同値は no-op・冪等）。phase を書き戻して Continue。
                if t <= last_tick {
                    tracing::debug!(
                        prev = last_tick,
                        t,
                        "non-monotonic Tick ignored (backward or equal)"
                    );
                    self.phase = TalkPhase::Driving {
                        talk_id,
                        player,
                        end,
                        last_tick,
                    };
                    return ControlFlow::Continue(());
                }
                player.tick(t);
                self.settle_after_tick(talk_id, player, end, t)
            }
        }
    }

    /// `player.tick` の後始末: 占有 horizon 到達（[`CuePlayer::is_completed`]）なら
    /// `TalkDone{end}` を送出し `Break`（phase は既に Idle）、未完了なら `Driving` を書き戻して
    /// `Continue`。完了検知は entry 枯渇でなく horizon 到達で真になる（早期終了しない・R2.5/D6）。
    ///
    /// 未完了側では書き戻しの直前に**選択待ち成立の検出**（[`notify_choice_waiting_if_newly_waiting`]
    /// (Self::notify_choice_waiting_if_newly_waiting)）を挟む（DD-6・R7.1/7.2）。完了側で検出しないのは
    /// [`CuePlayerState`] が排他ゆえ——`WaitingForChoice` なら `is_completed()` は必ず偽である。
    fn settle_after_tick(
        &mut self,
        talk_id: TalkId,
        player: CuePlayer,
        end: TalkEndReason,
        last_tick: f64,
    ) -> ControlFlow<()> {
        if player.is_completed() {
            // 自然終端: player を drop（残り無し）。phase は Idle のまま。高々 1 回機構。
            self.send_done(talk_id, end);
            ControlFlow::Break(())
        } else {
            // 選択待ち成立の一度きり検出＋通知（tick 経路。`on_resolve_choice` からの合流でも
            // 呼ばれるが、`resolve_choice` 成功直後の状態は `Playing`/`Completed` であって
            // `WaitingForChoice` にはなり得ないため構造的に no-op である）。
            self.notify_choice_waiting_if_newly_waiting(talk_id, &player);
            self.phase = TalkPhase::Driving {
                talk_id,
                player,
                end,
                last_tick,
            };
            ControlFlow::Continue(())
        }
    }

    /// 選択待ちバリアの成立（[`CuePlayerState::WaitingForChoice`] 遷移）を**一度きり**検出し、
    /// [`ChoiceWaiting`] を done ポートへ送出する（DD-6・R7.1/7.2）。
    ///
    /// 通知の真実源は**再生層**（[`CuePlayer`]）であり、本メソッドは検出時点の player から
    /// 3 つの事実をその場で写し取って送る:
    ///
    /// - **候補選択肢 ID 列**: [`CuePlayer::pending_choices`] の表示順（照合は下流・DD-7。talk 側
    ///   [`CuePlayer::resolve_choice`] の id 照合は二重防御として温存する）。
    /// - **表示完了時刻**: [`CuePlayer::occupancy_horizon`]（アンカー込みの絶対値＝duration 権威）。
    ///   注入 Tick の現在時刻ではない——タイムアウト計測の起点に再生層以外の時間基準を持ち込まない
    ///   （R7.2）。**捕捉は送出時点**で行う: [`CuePlayer::stop`] 後は schedule が clear されて
    ///   horizon がアンカーへ落ちるため、停止前のこの時点で読むことが値の正しさを担保する。
    /// - **タイムアウト指令**: [`BarrierKind::WaitForChoice`] の `timeout` をそのまま搬送する
    ///   （写像も既定値の適用も本層では行わない・DD-8）。
    ///
    /// 二重送出は [`choice_notified`](Self::choice_notified) が塞ぐ。送信失敗は `error!` を記録して
    /// 黙殺せず、talk は終端させない（`TalkDone` 送出と同規律・受信端不在は致命ではない）。
    fn notify_choice_waiting_if_newly_waiting(&mut self, talk_id: TalkId, player: &CuePlayer) {
        if self.choice_notified || !matches!(player.state(), CuePlayerState::WaitingForChoice) {
            return;
        }

        let choice_ids: Vec<String> = player
            .pending_choices()
            .iter()
            .map(|choice| choice.id.clone())
            .collect();
        let timeout_directive_secs = match player.current_barrier() {
            Some(BarrierKind::WaitForChoice { timeout }) => *timeout,
            other => {
                // 非到達（`WaitingForChoice` は `WaitForChoice` バリアでのみ成立する）。黙って
                // 既定へ倒さず記録したうえで「未指定」として搬送する（防御枝・DD-8）。
                tracing::warn!(
                    talk_id = talk_id.0,
                    barrier = ?other,
                    "WaitingForChoice なのに WaitForChoice バリアではない; タイムアウト指令を未指定として扱う"
                );
                None
            }
        };
        // 停止（`stop`）前のこの時点で占有 horizon を捕捉する（送出時点での捕捉）。
        let display_end_elapsed_secs = player.occupancy_horizon();

        let waiting = ChoiceWaiting {
            talk_id,
            choice_ids,
            display_end_elapsed_secs,
            timeout_directive_secs,
        };
        tracing::info!(
            talk_id = talk_id.0,
            choice_count = waiting.choice_ids.len(),
            display_end_elapsed_secs,
            ?timeout_directive_secs,
            "choice barrier reached; notifying ChoiceWaiting"
        );
        if self.done.send(D::from(waiting)).is_err() {
            tracing::error!(talk_id = talk_id.0, "ChoiceWaiting done receiver dropped");
        }
        self.choice_notified = true;
    }

    /// `Close` 受領: 進行中の再生を即時停止し、中断 ACK を返す（R7.1/7.2/7.3/7.4）。
    ///
    /// `Driving` なら [`CuePlayer::stop`] で残 entry を破棄してから（未発火 cue は sink へ届かない・
    /// R7.2）、`Armed`（初回 Tick 前）なら停止対象が無いので直接、`TalkDone{Interrupted}` を送出し
    /// `Break` する。phase は取り出しで Idle へ差し替わるため二度目の ACK は不能（通算高々 1 回・
    /// R6.4/R7.5）。`Idle`（`Start` 前の防御枝・自然終端後は既にスレッド消滅で Close 未達）は
    /// 二度目の TalkDone を送らずログのみ。
    fn on_close(&mut self) -> ControlFlow<()> {
        match std::mem::replace(&mut self.phase, TalkPhase::Idle) {
            TalkPhase::Driving {
                talk_id,
                mut player,
                ..
            } => {
                // 残 entry を破棄（以降配送しない・R7.2）。interrupt-vs-natural の区別は本層が持つ。
                player.stop();
                self.send_interrupted(talk_id);
                ControlFlow::Break(())
            }
            TalkPhase::Armed { talk_id, .. } => {
                // 初回 Tick 前の中断: CuePlayer 未構築ゆえ stop 対象なし。ACK のみ返す。
                self.send_interrupted(talk_id);
                ControlFlow::Break(())
            }
            TalkPhase::Idle => {
                // 状態未確定での Close（Start 前の防御枝・投函経路上は通常非到達）。
                // 二度目の TalkDone を送らずログのみ（通算高々 1 回・R6.4/R7.5）。
                tracing::debug!("Close received without active playback state; no ACK sent");
                ControlFlow::Break(())
            }
        }
    }

    /// `ResolveChoice{id}` 受領: 選択待ち（barrier）で止まった talk へ選択 id を投入する型付き口
    /// （R2.7）。W5（`areka-P0-choice-select-events`）の解決入力の唯一の到達点であり、
    /// [`CuePlayer::resolve_choice`] を外部から直接呼ぶ経路は存在しない（アクター内に閉じる）。
    ///
    /// - **`Driving`**（駆動中＝唯一 `resolve_choice` を呼ぶ状態）: `player.resolve_choice(&id)` へ
    ///   委譲する（id 照合＋一致時の先積みクリアは [`CuePlayer`] の責務）。
    ///   - `Some`: 選択が解決され `Playing` へ戻った。**その場で** [`CuePlayer::is_completed`] を
    ///     確認し、解決後に既に占有 horizon 到達（menu ケース＝barrier が最終 horizon 要素）なら
    ///     `TalkDone{end}` を送出して `Break`——[`settle_after_tick`] と同型の後始末を共用し、次 Tick
    ///     を待たない（R-5 の一 tick 遅延を残さない・R2.4/9.8）。未完了なら `Driving` を書き戻して継続。
    ///   - `None`（id 不一致・非待機）: 状態を変えず記録して継続する（barrier は解けない・R2.3 継続）。
    /// - **`Armed`/`Idle`**（初回 Tick 前・Start 前 or 終端後＝CuePlayer 未構築 or talk 不在）:
    ///   投函経路上は非到達の**誤投函**（W5 の mis-post）。`warn!` して継続する（防御枝・talk を
    ///   終端させない）。
    fn on_resolve_choice(&mut self, id: String) -> ControlFlow<()> {
        match std::mem::replace(&mut self.phase, TalkPhase::Idle) {
            TalkPhase::Driving {
                talk_id,
                mut player,
                end,
                last_tick,
            } => match player.resolve_choice(&id) {
                Some(_) => {
                    // 選択待ち成立の検出フラグを戻す（DD-6 のシーム）。M1 の compile は talk あたり
                    // 高々 1 個のバリアしか発行しないため同一 talk で再成立しないが、1 トークに複数
                    // バリアを持つ将来拡張ではこのリセットが次バリアの通知を可能にする。
                    self.choice_notified = false;
                    // 選択解決成功。解決後に既に占有 horizon 到達なら即時 settle（次 Tick を待たない）。
                    // settle_after_tick と同型（同一 TalkDone{end} 構築・同一 reason・同一片付け）を
                    // 共用し、tick 完了経路と分岐させない。last_tick は未完了時の書き戻し用に温存する。
                    self.settle_after_tick(talk_id, player, end, last_tick)
                }
                None => {
                    // id 不一致・非待機: 状態不変で記録して継続（barrier は解けない・R2.3 継続）。
                    tracing::debug!(
                        choice_id = %id,
                        "ResolveChoice: no matching pending choice (id mismatch or not waiting); continuing"
                    );
                    self.phase = TalkPhase::Driving {
                        talk_id,
                        player,
                        end,
                        last_tick,
                    };
                    ControlFlow::Continue(())
                }
            },
            TalkPhase::Armed {
                talk_id,
                sheet,
                end,
            } => {
                // 誤投函（初回 Tick 前・CuePlayer 未構築）: warn して継続（防御枝・W5 mis-post 検出）。
                tracing::warn!(
                    choice_id = %id,
                    "ResolveChoice received before playback started (Armed); ignoring"
                );
                self.phase = TalkPhase::Armed {
                    talk_id,
                    sheet,
                    end,
                };
                ControlFlow::Continue(())
            }
            TalkPhase::Idle => {
                // 誤投函（Start 前 or 終端後＝talk 不在）: warn して継続（防御枝）。phase は Idle のまま。
                tracing::warn!(
                    choice_id = %id,
                    "ResolveChoice received with no active talk (Idle); ignoring"
                );
                ControlFlow::Continue(())
            }
        }
    }

    /// 自然終端の `TalkDone{reason}` を送出する（受信端 drop は error ログ・黙殺しない・R11.1/11.4）。
    fn send_done(&self, talk_id: TalkId, reason: TalkEndReason) {
        let done = TalkDone { talk_id, reason };
        if self.done.send(D::from(done)).is_err() {
            tracing::error!(talk_id = talk_id.0, "TalkDone done receiver dropped");
        }
    }

    /// 中断の `TalkDone{Interrupted}` を送出する（受信端 drop は error ログ・R11.1/11.4）。
    fn send_interrupted(&self, talk_id: TalkId) {
        let done = TalkDone {
            talk_id,
            reason: TalkEndReason::Interrupted,
        };
        if self.done.send(D::from(done)).is_err() {
            tracing::error!(
                talk_id = talk_id.0,
                "TalkDone done receiver dropped on Close"
            );
        }
    }
}

#[cfg(test)]
#[path = "drive_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "drive_delivery_tests.rs"]
mod delivery_tests;
#[cfg(test)]
#[path = "drive_lifecycle_tests.rs"]
mod lifecycle_tests;
#[cfg(test)]
#[path = "drive_choice_tests.rs"]
mod choice_tests;
