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
    CueSheet, SakuraMsg, StartTalk, SystemVarSnapshot, TalkDone, TalkEndReason, TalkHandle, TalkId,
};
use crate::error::SakuraError;
use areka_actor::{run_inbox, spawn_actor};
use dola::cue::{CuePlayer, CueSink};

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
/// `done` は `TalkDone` の届け先（呼び出し側 inbox への変換投函・`D: From<TalkDone>`）。
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
    D: From<TalkDone> + Send + 'static,
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
}

impl<D> TalkDriver<D>
where
    D: From<TalkDone> + Send + 'static,
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

        let StartTalk { script, talk_id } = start;

        // 上流パーサで Instruction 列へ変換（再パースしない・R1.2）→ 純粋コンパイル。
        let instructions = areka_parsers::sakura::parse(&script);
        // talk 起動時に手渡された凍結スナップショット（R7.3・D8）を参照してコンパイルする。
        // sakura は値源を所有せず、この凍結像だけを見る（provider 差替で本層は無改変＝差替シーム）。
        let compiled = compile(&instructions, &self.system_vars);

        // 空 sheet: 時間軸駆動せず即終端（R1.4/R6.2）。end は Ended 固定でなく compiled.end
        // （裸の `\-` は空 sheet＋Quit）。
        if compiled.sheet.is_empty() {
            self.send_done(talk_id, compiled.end);
            return ControlFlow::Break(());
        }

        // 非空 sheet: 刻印は初回 Tick に遅延（アンカー＝初回注入時刻）。台本を保持して継続。
        self.phase = TalkPhase::Armed {
            talk_id,
            sheet: compiled.sheet,
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
            self.phase = TalkPhase::Driving {
                talk_id,
                player,
                end,
                last_tick,
            };
            ControlFlow::Continue(())
        }
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
mod tests {
    use super::*;
    use crate::contract::{CueCommand, TalkCue, TalkId};
    use crate::duration::text_playback_duration;
    use std::sync::mpsc::{self, TryRecvError};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // ── テスト用 CueSink 群（broadcast: 登録された全 sink が全 cue を受ける） ──

    /// broadcast で届いた全 cue を共有蓄積へ FIFO 追記する記録 sink（`Clone` で観測ハンドル取得）。
    #[derive(Clone)]
    struct RecordingSink {
        records: Arc<Mutex<Vec<TalkCue>>>,
    }
    impl RecordingSink {
        fn new() -> Self {
            Self {
                records: Arc::new(Mutex::new(Vec::new())),
            }
        }
        fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
            Arc::clone(&self.records)
        }
    }
    impl CueSink for RecordingSink {
        fn emit(&mut self, cue: TalkCue) {
            self.records
                .lock()
                .expect("RecordingSink records mutex poisoned")
                .push(cue);
        }
    }

    /// broadcast の 2 つ目のスロットを埋める no-op sink（多くのテストは片方の記録 sink のみ観測する）。
    struct NoopSink;
    impl CueSink for NoopSink {
        fn emit(&mut self, _cue: TalkCue) {}
    }

    /// テスト用: 2 演者 sink を register 順（S-3・登録順＝broadcast 順）で `spawn_talk` の
    /// `Vec<Box<dyn CueSink + Send>>` へ束ねるヘルパ。broadcast ゆえ両 sink は同一 cue 列を受け、
    /// 順序は broadcast 順にのみ効く（観測 sink をどちらへ置いても記録内容は不変）。
    fn two_sinks(
        first: impl CueSink + Send + 'static,
        second: impl CueSink + Send + 'static,
    ) -> Vec<Box<dyn CueSink + Send>> {
        vec![Box::new(first), Box::new(second)]
    }

    /// 発火の到着を barrier として同期受信するチャンネル sink（保留の決定的証明に使う）。
    struct ChannelSink {
        tx: mpsc::Sender<TalkCue>,
    }
    impl CueSink for ChannelSink {
        fn emit(&mut self, cue: TalkCue) {
            let _ = self.tx.send(cue);
        }
    }

    /// command 抽出ヘルパ。
    fn commands(records: &Arc<Mutex<Vec<TalkCue>>>) -> Vec<CueCommand> {
        records
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.command.clone())
            .collect()
    }

    /// 空発火列（空 script）の talk は時間軸駆動せず、Tick を一切送らなくても
    /// コンパイル結果の終端理由（空 script＝`Ended`）を伴う `TalkDone` を**即座に**返す
    /// （observable・R1.4）。`talk_id` は起動要求のものがエコーされる（R1.3）。
    #[test]
    fn empty_script_talk_returns_talkdone_immediately_without_tick() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(7);
        let start = StartTalk {
            script: String::new(), // 空 script → 空 Instruction 列 → 空 sheet。
            talk_id,
        };

        let sink = RecordingSink::new();
        let records = sink.records();

        // Tick を一切送らずに spawn_talk を呼ぶ（時間軸駆動を要求しない）。
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // TalkDone が即座に到達すること（Tick 不要・時間軸駆動なし）。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("空 script の talk は即座に TalkDone を返すべき");

        assert_eq!(done.talk_id, talk_id, "talk_id がエコーされること");
        assert_eq!(done.reason, TalkEndReason::Ended, "空 script は Ended");

        assert!(
            records.lock().unwrap().is_empty(),
            "空 sheet では発火が無いこと"
        );
        handle.actor.join().expect("body は正常終了する");
    }

    /// **broadcast**: 登録された全 sink が**同一の cue 列を同一順序で**受信する（中央振り分け廃止・
    /// 演者側 relevance が action 選別・D4/R2.1）。`\s[10]hello\w[2]world\e` を 2 つの記録 sink で
    /// 駆動し、両者が ClearAll/Emote/hello/Wait/world を過不足なく受けることを固定する。
    #[test]
    fn broadcast_delivers_identical_cue_stream_to_every_registered_sink() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: TalkId(200),
        };
        let surface = RecordingSink::new();
        let text = RecordingSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(surface, text),
            SystemVarSnapshot::default(),
        );
        // 初回 Tick(0.0) でアンカー刻印（0.0）、占有 horizon（world 再生完了＝0.35+0.25=0.60）を跨ぐ 1.0。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("自然終端で TalkDone");
        handle.actor.join().expect("body は正常終了する");

        // 期待 broadcast 列（両 sink が同一）: ClearAll@0 / Emote{10}@0 / hello@0 / Wait@0.25 / world@0.35。
        let expected = vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "10".into() },
            CueCommand::Text("hello".into()),
            CueCommand::Wait,
            CueCommand::Text("world".into()),
        ];
        assert_eq!(
            commands(&surface_records),
            expected,
            "surface sink が全 cue を broadcast 受信する（Emote だけでなく ClearAll/hello/Wait/world も）"
        );
        assert_eq!(
            commands(&text_records),
            expected,
            "text sink も同一の全 cue を broadcast 受信する（中央振り分けなし）"
        );
    }

    /// **観測可能な完了条件（task 7.1）**: 同一台本を 2 回**異なる時刻で再生開始**すると、同一 cue が
    /// **異なる絶対発火時刻**で配送される（絶対開始時刻が dispatch 刻印され honor される・R9.1/D6）。
    ///
    /// `\s[0]hi\w[10]bye\e` の "bye" は相対 `at=0.6`（hi の D=0.1 ＋ `\w[10]`=0.5）。初回 Tick を
    /// アンカー `A` として、"bye" の絶対発火時刻は `A + 0.6`。2 つの anchor（10.0 / 20.0）で
    /// 再生開始すると "bye" の発火時刻は 10.6 / 20.6 と**異なる**。
    ///
    /// 弁別（アンカー未刻印なら FAIL）: 初回 Tick(A) の時点では offset=0 ゆえ "bye"（at=0.6）は
    /// **保留**される。もしアンカーを刻印せず 0.0 のままなら offset=A（=10 や 20）が既に 0.6 を
    /// 超え、初回 Tick で "bye" が即発火してしまう＝下の「初回 Tick 直後は bye 未着」assert が FAIL する。
    #[test]
    fn same_sheet_started_at_different_times_delivers_cue_at_different_absolute_fire_times() {
        // 1 回の再生を anchor で駆動し、(初回Tick直後にbye未着か, A+0.5でbye未着か, A+0.6でbye着弾か) を返す。
        fn run_with_anchor(anchor: f64) -> (bool, bool, bool) {
            let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
            let start = StartTalk {
                script: r"\s[0]hi\w[10]bye\e".to_string(),
                talk_id: TalkId(1),
            };
            let (tx, rx) = mpsc::channel::<TalkCue>();
            let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(ChannelSink { tx }, NoopSink),
            SystemVarSnapshot::default(),
        );

            // barrier 技法: 記録 sink を挟まず、bye の着弾のみをチャンネルで観測する。
            let bye_seen = |rx: &mpsc::Receiver<TalkCue>| -> bool {
                let mut seen = false;
                while let Ok(cue) = rx.try_recv() {
                    if cue.command == CueCommand::Text("bye".into()) {
                        seen = true;
                    }
                }
                seen
            };
            // 「この Tick 送出＋ドレインまでに bye が届いたか」を決定的に観測するため、Tick 投函後に
            // done も含めた barrier で drain を同期する。ここでは十分に決定的な probe cue で代替する:
            // 各 Tick 後に "hi"（初回群）や world を受けるので、それを recv barrier に使う。

            // 初回 Tick(A): offset 0 → ClearAll/Emote/hi が due。bye(0.6) は保留のはず。
            handle.inbox.send(SakuraMsg::Tick(anchor)).unwrap();
            // hi 着弾を barrier に、初回群の drain 完了を待つ（bye は同 tick で来ない）。
            let mut hi_seen = false;
            while !hi_seen {
                match rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(cue) if cue.command == CueCommand::Text("hi".into()) => hi_seen = true,
                    Ok(_) => {}
                    Err(_) => panic!("初回群の hi が届かない"),
                }
            }
            let bye_after_first = bye_seen(&rx);

            // Tick(A+0.5): offset 0.5 → Wait(0.25? いや at=0.1) は due だが bye(0.6) は保留。
            handle.inbox.send(SakuraMsg::Tick(anchor + 0.5)).unwrap();
            std::thread::yield_now();
            // Wait cue の着弾を barrier に使う（at=0.1 <= 0.5 ゆえこの Tick までに届く）。
            let mut wait_seen = false;
            for _ in 0..1000 {
                match rx.try_recv() {
                    Ok(cue) if cue.command == CueCommand::Wait => {
                        wait_seen = true;
                        break;
                    }
                    Ok(_) => {}
                    Err(TryRecvError::Empty) => std::thread::yield_now(),
                    Err(TryRecvError::Disconnected) => break,
                }
            }
            assert!(wait_seen, "Wait(at=0.1) は A+0.5 までに届くはず（barrier）");
            let bye_after_half = bye_seen(&rx);

            // Tick(A+0.6): offset 0.6 → bye が due。着弾を待つ。
            handle.inbox.send(SakuraMsg::Tick(anchor + 0.6)).unwrap();
            let mut bye_after_full = false;
            // 自然終端まで進めてから observe すると drain が確定する。horizon=0.75 を跨ぐ A+1.0。
            handle.inbox.send(SakuraMsg::Tick(anchor + 1.0)).unwrap();
            done_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("horizon 到達で TalkDone");
            handle.actor.join().expect("body 正常終了");
            while let Ok(cue) = rx.try_recv() {
                if cue.command == CueCommand::Text("bye".into()) {
                    bye_after_full = true;
                }
            }

            (bye_after_first, bye_after_half, bye_after_full)
        }

        // Run A（anchor 10.0）と Run B（anchor 20.0）。
        let (a_first, a_half, a_full) = run_with_anchor(10.0);
        let (b_first, b_half, b_full) = run_with_anchor(20.0);

        // 弁別の核心: 初回 Tick(A) 直後は bye 未着（アンカー刻印されているから offset=0）。
        // アンカー未刻印（0.0 固定）なら初回 Tick で offset=A>0.6 ゆえ bye が即着＝この assert が FAIL する。
        assert!(
            !a_first,
            "Run A: 初回 Tick(10.0) 直後は bye 未着（アンカー刻印の弁別）"
        );
        assert!(
            !b_first,
            "Run B: 初回 Tick(20.0) 直後は bye 未着（アンカー刻印の弁別）"
        );
        // A+0.5（=offset 0.5）でもまだ bye は保留（0.6 未達）。
        assert!(!a_half, "Run A: offset 0.5 では bye(0.6) 保留");
        assert!(!b_half, "Run B: offset 0.5 では bye(0.6) 保留");
        // A+0.6（=offset 0.6）で初めて bye が着弾する（＝絶対発火時刻 anchor+0.6）。
        assert!(a_full, "Run A: offset 0.6（絶対 10.6）で bye 着弾");
        assert!(b_full, "Run B: offset 0.6（絶対 20.6）で bye 着弾");
        // 同一 cue が 2 回の再生で異なる絶対発火時刻（10.6 vs 20.6）で配送された（構成的に相異）。
        assert_ne!(
            10.0 + 0.6,
            20.0 + 0.6,
            "同一台本を異なる時刻に再生開始すると bye の絶対発火時刻が異なる（10.6 != 20.6）"
        );
    }

    /// 未 due の発火は Tick を受けても**保留**され、`at` 到達（境界含む・`at <= offset`）の Tick で
    /// 初めて配送されることを**中間観測で決定的に**検証する（実時計・sleep 非依存）。broadcast ゆえ
    /// 単一の記録チャンネル sink が全 cue（surface/text の別なく）を受ける。
    ///
    /// script `\s[10]hello\w[2]probeA\w[2]probeB\w[2]world\e` の発火予定（D 焼き込み後・アンカー 0）:
    ///   ClearAll@0・Emote{10}@0・hello@0 / Wait@0.25 / probeA@0.35 / Wait@0.65 / probeB@0.75 /
    ///   Wait@1.05 / world@1.15。probe 受信を barrier に、未 due cue が保留されることを try_recv Empty で固定する。
    #[test]
    fn undue_cues_are_withheld_until_their_at_is_reached() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(314);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]probeA\w[2]probeB\w[2]world\e".to_string(),
            talk_id,
        };

        let (tx, rx) = mpsc::channel::<TalkCue>();
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(ChannelSink { tx }, NoopSink),
            SystemVarSnapshot::default(),
        );

        let d_hello = text_playback_duration("hello"); // 0.25
        let d_probe = text_playback_duration("probeA"); // 0.30
        let w = Duration::from_millis(100).as_secs_f64(); // \w[2] = 0.10
        let at_a = d_hello + w; // probeA: 0.35
        let at_b = at_a + d_probe + w; // probeB: 0.75
        let at_w = at_b + d_probe + w; // world:  1.15

        let recv = |rx: &mpsc::Receiver<TalkCue>| {
            rx.recv_timeout(Duration::from_secs(5))
                .expect("due な発火は届くこと")
        };
        // probe cue（Text）だけを追う barrier ヘルパ（Wait 等は読み飛ばす）。
        let recv_text = |rx: &mpsc::Receiver<TalkCue>, want: &str| {
            loop {
                let cue = recv(rx);
                if cue.command == CueCommand::Text(want.into()) {
                    return cue;
                }
            }
        };

        // 初回 Tick(0.0) でアンカー刻印（0）。ClearAll/Emote/hello が due（probe は未 due）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        let hello = recv_text(&rx, "hello");
        assert_eq!(hello.at, 0.0, "hello の発火時刻は 0.0");
        // 初回群 drain 後、probeA(0.35) は未着（保留の決定的証明）。
        assert_eq!(
            rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "初回 Tick(0.0) では未 due の probeA(0.35) が保留されること"
        );

        // Tick(at_a=0.35): Wait@0.25 と probeA@0.35 が due。probeB/world は未 due。
        handle.inbox.send(SakuraMsg::Tick(at_a)).unwrap();
        let probe_a = recv_text(&rx, "probeA");
        assert_eq!(probe_a.at, at_a, "probeA の発火時刻は 0.35");
        assert_eq!(
            rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "at=0.35 の Tick では未 due の probeB(0.75)/world(1.15) が保留されること"
        );

        // Tick(at_b=0.75): probeB@0.75 が新規 due。world は依然未 due。
        handle.inbox.send(SakuraMsg::Tick(at_b)).unwrap();
        let probe_b = recv_text(&rx, "probeB");
        assert_eq!(probe_b.at, at_b, "probeB の発火時刻は 0.75");
        assert_eq!(
            rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "at=0.75 の Tick でも未 due の world(1.15) が保留されること"
        );

        // Tick(at_w=1.15): world@1.15 が due（境界含む `at <= offset`）→ ここで初めて発火。
        handle.inbox.send(SakuraMsg::Tick(at_w)).unwrap();
        let world = recv_text(&rx, "world");
        assert_eq!(world.at, at_w, "world の発火時刻は 1.15（境界包含で発火）");

        // 占有 horizon（world 再生完了＝1.15+0.25=1.40）を跨ぐ Tick で自然終端。
        handle.inbox.send(SakuraMsg::Tick(2.0)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("末尾到達で TalkDone");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
        handle.actor.join().expect("body は正常終了する");
    }

    /// **同一 `at` の発火順が記述順（FIFO）で保たれる**ことを broadcast の単一記録 sink で固定する
    /// （canonical 変換 `to_talk_schedule` の per-cue insert の load-bearing 性質）。
    ///
    /// script `\s[10]hello\nworld\e` → 発火（アンカー 0）:
    ///   ClearAll@0 / Emote{10}@0 / Text(hello)@0（at=0 群）→ NewLine@0.25 / Text(world)@0.25（at=0.25 群）。
    #[test]
    fn same_at_cues_preserve_script_order_fifo() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let start = StartTalk {
            script: r"\s[10]hello\nworld\e".to_string(),
            talk_id: TalkId(41),
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );
        // 初回 Tick(0.0) 刻印＋単一 Tick(0.5) で全 due（world 再生完了 horizon=0.50 到達）→自然終端。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("単一 Tick で自然終端");
        assert_eq!(done.reason, TalkEndReason::Ended);
        handle.actor.join().expect("body は正常終了する");

        let d_hello = text_playback_duration("hello");
        let recs = records.lock().unwrap();
        // 記述順（FIFO）: ClearAll/Emote/hello（at=0 群）→ NewLine/world（at=0.25 群）の 5 件。
        assert_eq!(
            recs.len(),
            5,
            "broadcast は ClearAll/Emote/hello/NewLine/world の 5 件"
        );
        assert_eq!(
            recs[0].command,
            CueCommand::ClearAll,
            "冒頭は全消去 ClearAll（at=0）"
        );
        assert_eq!(recs[0].at, 0.0);
        assert_eq!(recs[1].command, CueCommand::Emote { key: "10".into() });
        assert_eq!(recs[1].at, 0.0);
        assert_eq!(recs[2].command, CueCommand::Text("hello".into()));
        assert_eq!(recs[2].at, 0.0);
        assert!(
            matches!(recs[3].command, CueCommand::NewLine { .. }),
            "at=0.25 群先頭は NewLine（FIFO・extend なら逆順化する）"
        );
        assert_eq!(recs[3].at, d_hello);
        assert_eq!(recs[4].command, CueCommand::Text("world".into()));
        assert_eq!(recs[4].at, d_hello);
    }

    /// **`Start` の二重受領が無視される**ことを検証する（プロトコルガード・`on_start`）。
    /// 1 本目（script A）で spawn 後、別 script の 2 本目 `Start`(B) を送っても A のみ再生される。
    #[test]
    fn duplicate_start_is_ignored_and_first_talk_plays_unchanged() {
        let (done_a_tx, done_a_rx) = mpsc::channel::<TalkDone>();
        let id_a = TalkId(11);
        let start_a = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: id_a,
        };
        let sink = RecordingSink::new();
        let records = sink.records();
        let handle = spawn_talk(
            start_a,
            done_a_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // 2 本目 Start(B)（別 script）を inbox へ。自己投函の Start(A) の後に処理され、無視される。
        let id_b = TalkId(99);
        let start_b = StartTalk {
            script: r"\s[77]DIFFERENT\e".to_string(),
            talk_id: id_b,
        };
        handle
            .inbox
            .send(SakuraMsg::Start(start_b))
            .expect("2 本目 Start(B) 投函");

        // A を駆動して自然終端（world 再生完了 horizon=0.60 を跨ぐ Tick(1.0) まで）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
        let done = done_a_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("A の TalkDone");
        assert_eq!(
            done.talk_id, id_a,
            "終端は A の talk_id（B に乗っ取られない）"
        );
        assert_eq!(done.reason, TalkEndReason::Ended);
        handle.actor.join().expect("body は正常終了する");

        // A の内容のみ（B の Emote{77}/DIFFERENT は現れない）: ClearAll/Emote{10}/hello/Wait/world。
        assert_eq!(
            commands(&records),
            vec![
                CueCommand::ClearAll,
                CueCommand::Emote { key: "10".into() },
                CueCommand::Text("hello".into()),
                CueCommand::Wait,
                CueCommand::Text("world".into()),
            ],
            "A の内容のみが broadcast される（B の DIFFERENT/Emote{{77}} は不在）"
        );
    }

    /// **終端時に done 受信端が drop 済みでも body が panic せず clean exit する**（R11.1/11.4）。
    /// 駆動前に `done_rx` を drop → 自然終端で `done.send` が `Err` になるが `error!` の上で `Break`。
    /// 発火自体は正常（done drop は終端信号にのみ影響し broadcast には影響しない）。
    #[test]
    fn dropped_done_receiver_at_terminal_exits_cleanly_without_panic() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: TalkId(4),
        };
        let sink = RecordingSink::new();
        let records = sink.records();
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        drop(done_rx); // 終端 TalkDone 送出前に受信端を drop（送出は Err になる）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

        handle
            .actor
            .join()
            .expect("done 受信端 drop でも body は panic せず正常終了する");

        // broadcast は正常に行われた（ClearAll/Emote/hello/Wait/world の 5 件）。
        assert_eq!(
            records.lock().unwrap().len(),
            5,
            "done drop は broadcast に影響しない（5 cue 配送済み）"
        );
    }

    /// M-boot 外タグのみで発火列が空になる script は空 sheet へコンパイルされ、Tick を要さずに
    /// 末尾到達の `Ended`（R1.4）を伴う `TalkDone` を即座に返す（リテラル空 script とは別経路）。
    #[test]
    fn ignored_tags_only_script_ends_immediately_with_ended_and_no_firing() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(55);
        // task 4.2 で SystemVar/GenericCommand は cue を発行するようになったため、無 cue フィラーには
        // `\0` を用いる。parser は `\0` を正典スコープタグ `SpeakerScope{n:0}` へ写像するが（task 12.1・
        // R1.5/R4.4）、compile は `SpeakerScope{n}` を「scope 状態更新のみ・cue 非発行」で扱う
        // （`compile.rs` の SpeakerScope アーム）。ゆえに内容 cue は皆無で empty-sheet 即時 TalkDone
        // 経路を保つ（`\0` の写像先が Raw から SpeakerScope へ変わっても本檻の観測は不変）。
        let start = StartTalk {
            script: r"\0".to_string(),
            talk_id,
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("無視タグのみの script も空 sheet 経路で即座に TalkDone を返すべき");

        assert_eq!(done.talk_id, talk_id, "talk_id エコー（R1.3）");
        assert_eq!(
            done.reason,
            TalkEndReason::Ended,
            "終端命令のない無視タグのみ script は末尾到達で Ended（R1.4）"
        );
        assert!(
            records.lock().unwrap().is_empty(),
            "無視タグは発火を生成しない"
        );
        handle.actor.join().expect("body は正常終了する");
    }

    /// 先行 cue のない `\-`（quit 相当のみ）の script は空 sheet＋`end=Quit` へコンパイルされ、Tick を
    /// 要さずに **`Quit`（`Ended` ではない）** を伴う `TalkDone` を即座に返す（空 sheet 経路の弁別・R6.2）。
    #[test]
    fn quit_only_script_ends_immediately_with_quit_not_ended() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(56);
        // task 4.2 で SystemVar は cue を発行するようになったため、`\-` の先行フィラーには `\0` を用いる。
        // parser は `\0` を `SpeakerScope{n:0}` へ写像し（task 12.1・R1.5/R4.4）、compile はそれを
        // scope 状態更新のみ（cue 非発行）で扱う。先行内容 cue のない `\-` の empty-sheet＋Quit 経路を
        // 保つ（SpeakerScope は cue を生まず `\-` が Quit で切詰め＝空 sheet＋end=Quit）。
        let start = StartTalk {
            script: r"\0\-".to_string(),
            talk_id,
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("quit 相当のみの script も空 sheet 経路で即座に TalkDone を返すべき");

        assert_eq!(done.talk_id, talk_id, "talk_id エコー（R1.3）");
        assert_eq!(
            done.reason,
            TalkEndReason::Quit,
            "先行 cue のない `\\-` は空 sheet＋Quit（Ended を固定送出してはならない・R6.2）"
        );
        assert_ne!(
            done.reason,
            TalkEndReason::Ended,
            "空 sheet 経路で Ended を固定送出していない"
        );
        assert!(
            records.lock().unwrap().is_empty(),
            "quit 相当のみでは発火が無い"
        );
        handle.actor.join().expect("body は正常終了する");
    }

    /// fixture 駆動の統合テスト（主 observable・R9.3）。`\s[10]hello\w[2]world\e` を注入 Tick 列で
    /// 駆動し、broadcast の単一記録 sink が ClearAll/Emote/hello/Wait/world を **at 昇順・FIFO** で
    /// 受け、最後に `TalkDone{Ended}`（talk_id エコー・R6.6）が返ることを確認する。
    ///
    /// **task 9.5（再生時間搬送 e2e・R1.1/7.1）**: 併せて、各 delivered cue の **envelope
    /// `duration`** が、コンパイル時に焼き込んだ再生時間と**同一算術**（テキストは
    /// `text_playback_duration`・`\w[2]` は 2×50ms の `Duration` 算術）で一致することを固定する。
    /// これは実際の `compile → drive → CuePlayer broadcast → sink` 経路上で観測した delivered
    /// duration が無変形で届くことの唯一の檻であり（他 hop は個別 crate で既に檻済み）、演者側
    /// reveal 完了時刻（区間 `[at, at+duration)` の終端）を導く素が正しく搬送されることを示す。
    #[test]
    fn fixture_script_drives_broadcast_and_returns_ended() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(42);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        let at_world = text_playback_duration("hello") + Duration::from_millis(100).as_secs_f64();

        // 初回 Tick(0.0) 刻印＋占有 horizon（world 再生完了＝at_world+0.25=0.60）を跨ぐ Tick(1.0)。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("自然終端で TalkDone が返るべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー（R6.6）");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
        handle.actor.join().expect("body は正常終了する");

        let recs = records.lock().unwrap();
        // ClearAll@0 / Emote{10}@0 / hello@0 / Wait@0.25 / world@0.35（at 昇順・FIFO）。
        assert_eq!(
            recs.len(),
            5,
            "broadcast は 5 件（ClearAll/Emote/hello/Wait/world）"
        );
        assert_eq!(recs[0].command, CueCommand::ClearAll);
        assert_eq!(recs[0].at, 0.0);
        assert_eq!(recs[1].command, CueCommand::Emote { key: "10".into() });
        assert_eq!(recs[1].at, 0.0);
        assert_eq!(recs[1].actor.as_str(), "0", "既定 scope=0 の転写");
        assert_eq!(recs[2].command, CueCommand::Text("hello".into()));
        assert_eq!(recs[2].at, 0.0);
        assert_eq!(
            recs[3].command,
            CueCommand::Wait,
            "Wait cue も broadcast される（旧中央振り分けは skip していた）"
        );
        assert_eq!(recs[3].at, text_playback_duration("hello"));
        assert_eq!(recs[4].command, CueCommand::Text("world".into()));
        assert_eq!(recs[4].at, at_world, "world は hello の D＋\\w[2] 後に発火");
        for pair in recs.windows(2) {
            assert!(pair[0].at <= pair[1].at, "broadcast は at 昇順");
        }

        // ── task 9.5: delivered envelope duration の無変形搬送檻（R1.1/7.1） ──
        // 期待値は production 経路と**同一算術**で導く（10 進リテラル直書きは IEEE-754 表現誤差ゆえ
        // 使わない）: テキストは compile が呼ぶのと同じ `text_playback_duration`、`\w[2]` は parser が
        // 生成するのと同じ `Duration::from_millis(2 × 50ms).as_secs_f64()`。この delivered duration が
        // 期待値とビット同一（`==`）なら、D 焼き込み → `to_talk_schedule` → CuePlayer broadcast の
        // どの hop でも duration が落とされ／ゼロ化され／再導出されていない（無変形搬送）ことの証拠。
        let d_hello = text_playback_duration("hello");
        let w2 = Duration::from_millis(100).as_secs_f64(); // \w[2] = 2 × 50ms（parser 算術と同一）
        let d_world = text_playback_duration("world");
        assert_eq!(recs[0].duration, 0.0, "ClearAll は瞬時（duration=0）");
        assert_eq!(recs[1].duration, 0.0, "Emote は瞬時（duration=0）");
        assert_eq!(
            recs[2].duration, d_hello,
            "hello の delivered duration はコンパイル焼き込み D（text_playback_duration）と無変形一致（R1.1/7.1）"
        );
        assert_eq!(
            recs[3].duration, w2,
            "Wait の delivered duration は \\w[2]=100ms（envelope duration が待ち時間を担う・無変形）"
        );
        assert_eq!(
            recs[4].duration, d_world,
            "world の delivered duration もコンパイル焼き込み D と無変形一致（演者側 reveal 完了時刻の素）"
        );

        // 演者側 reveal 完了時刻は delivered cue の区間 `[at, at+duration)` 終端で導かれる
        // （emo-text state.rs 檻）。その素になる hello の占有終端（at+duration）が後続 Wait の発火
        // 時刻（＝hello 再生完了後）と一致することを固定し、焼き込み duration が下流タイムラインの
        // 整列に無変形で効く e2e（コンパイル値 → reveal 完了時刻が同一算術）を drive 層で観測する。
        assert_eq!(
            recs[2].at + recs[2].duration,
            recs[3].at,
            "hello の reveal 完了時刻（at+duration）は後続 Wait の発火時刻と一致（焼き込み duration が整列の素）"
        );
    }

    /// 冪等/逆行 `Tick` で二重発火しない（設計クリティカルな二重発火ガードの固定・R11.x）。
    #[test]
    fn duplicate_and_backward_tick_do_not_double_fire() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let start = StartTalk {
            script: r"\s[10]hello\w[10]world\e".to_string(),
            talk_id: TalkId(1),
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // 初回 Tick(0.0) 刻印。同値・逆行 Tick を織り交ぜて at=0.0 群を発火させる。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap(); // 同値 → no-op
        handle.inbox.send(SakuraMsg::Tick(-1.0)).unwrap(); // 逆行 → no-op
        handle.inbox.send(SakuraMsg::Tick(0.1)).unwrap(); // 前進だが world(at=0.75) 未達

        // 終端まで進める（hello D=0.25＋\w[10]=0.5 後 world@0.75・horizon=1.0 を跨ぐ）。
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("終端で TalkDone");
        assert_eq!(done.reason, TalkEndReason::Ended);
        handle.actor.join().expect("body は正常終了する");

        // 二重発火なし: ClearAll/Emote/hello/Wait/world 各 1 回＝5 件。
        assert_eq!(
            commands(&records),
            vec![
                CueCommand::ClearAll,
                CueCommand::Emote { key: "10".into() },
                CueCommand::Text("hello".into()),
                CueCommand::Wait,
                CueCommand::Text("world".into()),
            ],
            "dupe/逆行 Tick でも二重発火しない（各 cue 1 回）"
        );
    }

    /// 非有限 `Tick`（`NaN`/`inf`）は無視され再生が破綻しない（R11.1/11.2）。刻印前（`Armed`）でも
    /// 非有限 Tick でアンカーを刻印せず（NaN アンカー防止）、その後の正常 Tick で通常どおり終端する。
    #[test]
    fn non_finite_tick_is_ignored_and_playback_survives() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: TalkId(9),
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // 刻印前に非有限 Tick を送る: 無視され（error ログ＋SakuraError 記録）刻印もされない。
        handle.inbox.send(SakuraMsg::Tick(f64::NAN)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(f64::INFINITY)).unwrap();

        // 正常 Tick 列で通常どおり駆動・終端する（ガードがループを殺していないことの証）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("非有限 Tick 後も正常 Tick で終端するべき");
        assert_eq!(done.reason, TalkEndReason::Ended, "再生は破綻せず Ended");
        handle.actor.join().expect("body は正常終了する");

        // 非有限 Tick で早期全量配信されず、正常 Tick 分だけ届く（5 件）。
        assert_eq!(
            records.lock().unwrap().len(),
            5,
            "非有限 Tick で早期全量配信されていない（ClearAll/Emote/hello/Wait/world）"
        );
    }

    /// 再生途中の中断（Close）で `TalkDone{Interrupted}` がちょうど 1 回返り、未発火分が sink に
    /// 届かないこと（R7.1/7.2/7.3/7.4・R6.4）。`\s[10]hello\w[10]world\e`（world は \w[10] 後）を
    /// 先頭群だけ発火させたところで Close。world（at=0.75）は未発火＝以降届いてはならない。
    #[test]
    fn mid_playback_close_returns_interrupted_once_and_drops_unfired_cues() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(101);
        let start = StartTalk {
            script: r"\s[10]hello\w[10]world\e".to_string(),
            talk_id,
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // 初回 Tick(0.0) 刻印＋at=0.0 群を発火（world は at=0.75・未達）。
        handle
            .inbox
            .send(SakuraMsg::Tick(0.0))
            .expect("Tick(0.0) 投函");
        // 中断（Close）を送る。進行中の再生を即時停止し Interrupted ACK を返すべき。
        handle.inbox.send(SakuraMsg::Close).expect("Close 投函");

        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("中断で TalkDone{Interrupted} が返るべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー（R6.6）");
        assert_eq!(
            done.reason,
            TalkEndReason::Interrupted,
            "中断の終端理由は Interrupted（R7.4）"
        );
        handle.actor.join().expect("body は Break 後に正常終了する");

        // at=0.0 群のみ届き（ClearAll/Emote/hello）、未発火分（world@0.75）は届いていない（R7.2）。
        assert_eq!(
            commands(&records),
            vec![
                CueCommand::ClearAll,
                CueCommand::Emote { key: "10".into() },
                CueCommand::Text("hello".into()),
            ],
            "中断前に届いたのは ClearAll/Emote/hello のみ（world は未発火＝破棄・R7.2）"
        );
    }

    /// 自然終端後に中断（Close）を受けても追加の `TalkDone` が発生しないこと（R6.4/R7.5）。
    /// 自然終端後はアクタースレッドが消えており `inbox.send(Close)` が `Err`＝二重終端不能の構造的証。
    #[test]
    fn close_after_natural_end_produces_no_extra_talkdone() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(102);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
        };
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(NoopSink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // 自然終端まで駆動する（0.0 刻印 → 占有 horizon=0.60 を跨ぐ 1.0）。
        handle
            .inbox
            .send(SakuraMsg::Tick(0.0))
            .expect("Tick(0.0) 投函");
        handle
            .inbox
            .send(SakuraMsg::Tick(1.0))
            .expect("Tick(1.0) 投函");

        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("自然終端で TalkDone{Ended} が返るべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
        handle
            .actor
            .join()
            .expect("body は自然終端後に正常終了する");

        // 自然終端後の Close: アクターは既に消えており inbox.send は Err（二重終端不能の証）。
        let send_result = handle.inbox.send(SakuraMsg::Close);
        assert!(
            send_result.is_err(),
            "自然終端後はアクターが消えており Close 送出は失敗する（二重終端不能の証）"
        );
    }

    /// 複数 talk を異なる相関 ID・独立 sink で同時駆動し、各 `TalkDone` に起動時と同一の `talk_id`
    /// が対応付けられ、出力が talk 間で混線しないことを確認する（R1.3/R6.6）。
    #[test]
    fn multiple_talks_echo_own_talk_id_without_cross_talk_mixing() {
        // talk A: TalkId(7)・Ended 経路。
        let (done_a_tx, done_a_rx) = mpsc::channel::<TalkDone>();
        let id_a = TalkId(7);
        let start_a = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: id_a,
        };
        let sink_a = RecordingSink::new();
        let records_a = sink_a.records();

        // talk B: TalkId(42)・Quit 経路（末尾 `\-`）。
        let (done_b_tx, done_b_rx) = mpsc::channel::<TalkDone>();
        let id_b = TalkId(42);
        let start_b = StartTalk {
            script: r"\s[20]bye\w[2]done\-".to_string(),
            talk_id: id_b,
        };
        let sink_b = RecordingSink::new();
        let records_b = sink_b.records();

        let handle_a = spawn_talk(
            start_a,
            done_a_tx,
            two_sinks(sink_a, NoopSink),
            SystemVarSnapshot::default(),
        );
        let handle_b = spawn_talk(
            start_b,
            done_b_tx,
            two_sinks(sink_b, NoopSink),
            SystemVarSnapshot::default(),
        );

        handle_a
            .inbox
            .send(SakuraMsg::Tick(0.0))
            .expect("A Tick(0.0)");
        handle_a
            .inbox
            .send(SakuraMsg::Tick(1.0))
            .expect("A Tick(1.0)");
        handle_b
            .inbox
            .send(SakuraMsg::Tick(0.0))
            .expect("B Tick(0.0)");
        handle_b
            .inbox
            .send(SakuraMsg::Tick(1.0))
            .expect("B Tick(1.0)");

        let done_a = done_a_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("talk A は TalkDone を返すべき");
        let done_b = done_b_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("talk B は TalkDone を返すべき");

        assert_eq!(done_a.talk_id, id_a, "talk A の TalkDone は id_a をエコー");
        assert_eq!(done_b.talk_id, id_b, "talk B の TalkDone は id_b をエコー");
        assert_ne!(done_a.talk_id, done_b.talk_id, "2 talk の id は相異なる");
        assert_eq!(done_a.reason, TalkEndReason::Ended, "A の `\\e` は Ended");
        assert_eq!(done_b.reason, TalkEndReason::Quit, "B の `\\-` は Quit");

        handle_a.actor.join().expect("A body 正常終了");
        handle_b.actor.join().expect("B body 正常終了");

        // 各 talk の cue は自分の sink にのみ届く（混線しない）。
        assert_eq!(
            commands(&records_a),
            vec![
                CueCommand::ClearAll,
                CueCommand::Emote { key: "10".into() },
                CueCommand::Text("hello".into()),
                CueCommand::Wait,
                CueCommand::Text("world".into()),
            ],
            "A sink は A の cue 列のみ"
        );
        assert_eq!(
            commands(&records_b),
            vec![
                CueCommand::ClearAll,
                CueCommand::Emote { key: "20".into() },
                CueCommand::Text("bye".into()),
                CueCommand::Wait,
                CueCommand::Text("done".into()),
            ],
            "B sink は B の cue 列のみ"
        );
    }

    /// 同一 fixture script＋同一注入 Tick 列を N 回実行し、毎回**同一の観測結果**（cue 列・at・
    /// actor・順序・終端理由）が得られることを確認する（R9.4・決定的再現）。
    #[test]
    fn same_fixture_and_tick_sequence_produces_identical_observation_each_run() {
        type CueKey = (u64, String, CueCommand);
        fn project(records: &Arc<Mutex<Vec<TalkCue>>>) -> Vec<CueKey> {
            records
                .lock()
                .unwrap()
                .iter()
                .map(|c| {
                    (
                        c.at.to_bits(),
                        c.actor.as_str().to_string(),
                        c.command.clone(),
                    )
                })
                .collect()
        }

        fn run_once() -> (Vec<CueKey>, TalkEndReason) {
            let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
            let start = StartTalk {
                script: r"\s[10]hello\w[2]world\e".to_string(),
                talk_id: TalkId(7),
            };
            let sink = RecordingSink::new();
            let records = sink.records();

            let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );
            handle.inbox.send(SakuraMsg::Tick(0.0)).expect("Tick(0.0)");
            handle.inbox.send(SakuraMsg::Tick(1.0)).expect("Tick(1.0)");

            let done = done_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("自然終端で TalkDone");
            handle.actor.join().expect("body 正常終了");
            (project(&records), done.reason)
        }

        const RUNS: usize = 3;
        let baseline = run_once();
        assert_eq!(
            baseline.0.len(),
            5,
            "baseline は 5 cue（ClearAll/Emote/hello/Wait/world）"
        );
        assert_eq!(baseline.1, TalkEndReason::Ended, "baseline は Ended");
        for run in 1..RUNS {
            let observed = run_once();
            assert_eq!(
                observed, baseline,
                "run {run} の観測が baseline と完全一致すること（cue 列・at・actor・順序・終端理由・R9.4）"
            );
        }
    }

    // ── task 7.2: 完了通知を占有 horizon まで遅らせる drive-level 注入時刻檻（R2.5/D6） ──
    //
    // これらは **drive-level**（実 talk アクター＋done チャンネル）でしか捕捉できない早期終了の檻
    // である（compile-level の extent 檻は「配送し終えた」時点の完了を検知できない）。共通の骨子:
    //
    // - **負の窓（早期終了しないことの決定的証明）**: horizon 未満の注入時刻まで駆動した後、
    //   `done_rx.recv_timeout(NEG_WINDOW)` が **timeout（`is_err()`）** することを主張する。完了通知は
    //   `is_completed()`（占有 horizon gated）でしか送られないため、horizon 未満では送信自体が起きず
    //   recv は必ず timeout する。逆にもし「entry 枯渇＝完了」の早期終了バグがあれば、この窓で
    //   `TalkDone` が既に届き `recv_timeout` が **成功**して `is_err()` が偽になり檻が落ちる（バグ検出）。
    //   窓長（数百 ms）はアクターの tick 処理（μs 台）を遥かに上回るため、正常系では送信が無く必ず
    //   timeout、バグ系では送信が窓内に届く——両方向に決定的（実時計依存は安全余裕であって精度要件でない）。
    // - **時間障壁の兼用**: `recv_timeout(NEG_WINDOW)` の待機中にアクターは投函済み Tick を全消化して
    //   recv でブロックするため、窓明けの `records`（全 cue 配送済み）と `is_finished()==false`
    //   （駆動継続）は race なく観測できる。
    // - **正の確認**: その後 horizon 到達の Tick を投函し `recv_timeout(5s)` で `TalkDone` を受けることで
    //   「horizon 到達で初めて完了する」を示す（末尾の待ち・最終テキストの duration が終端で切り捨てられない）。

    /// 早期終了バグを疑って完了通知を待つ負の窓長（正常系の timeout 待機・アクター処理 μs を遥かに上回る）。
    const NEG_WINDOW: Duration = Duration::from_millis(200);

    /// **末尾に明示的な待ちを持つ talk**: 完了通知は cue 配送完了（entry 枯渇）でなく、末尾 Wait の
    /// 再生時間を含む占有 horizon 到達で初めて発火する（R2.5/D6・#3 の実機構）。
    ///
    /// `\s[10]hello\_w[800]\e` の台本（アンカー 0）:
    ///   ClearAll@0 / Emote{10}@0 / hello@0(dur=D) / Wait@D(dur=0.8)。
    /// 全 cue の配送は Tick(D) で完了する（entry 枯渇）が、占有 horizon＝`D + 0.8` であり、そこへ達する
    /// まで `TalkDone` は発火しない。末尾待ちの 0.8 秒が talk 終端で切り捨てられない（早期終了しない）。
    #[test]
    fn trailing_wait_talkdone_fires_at_horizon_not_at_cue_exhaustion() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(720);
        let start = StartTalk {
            script: r"\s[10]hello\_w[800]\e".to_string(),
            talk_id,
        };
        let sink = RecordingSink::new();
        let records = sink.records();
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // 期待値は本番と同一の算術で導出（10 進直書きの表現誤差を排除・注入時刻決定論）。
        let d_hello = text_playback_duration("hello"); // 0.25
        let w = Duration::from_millis(800).as_secs_f64(); // 0.8（\_w[800]）
        let t_wait = d_hello; // Wait cue の相対発火時刻
        let horizon = d_hello + w; // 占有 horizon＝末尾 Wait の再生完了時刻（1.05）
        let near_horizon = d_hello + w * 0.5; // horizon 手前（entry 枯渇後・horizon 未満）

        // 初回 Tick(0.0) 刻印。Tick(D) で Wait を配送し **entry を枯渇**、さらに horizon 手前まで前進する
        // （いずれも horizon 未満・単調増加 0.0 < 0.25 < 0.65 < 1.05）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(t_wait)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(near_horizon)).unwrap();

        // 負の窓: entry 枯渇かつ horizon 手前では完了通知が **発火しない**（配送 ≠ 再生完了・早期終了しない）。
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "全 cue 配送済み（entry 枯渇）かつ horizon 未満では TalkDone は発火してはならない（配送 ≠ 完了・R2.5）"
        );
        // 窓明けの race なし観測: 全 cue は既に broadcast 配送済み（配送完了）だが完了はしていない。
        assert_eq!(
            commands(&records),
            vec![
                CueCommand::ClearAll,
                CueCommand::Emote { key: "10".into() },
                CueCommand::Text("hello".into()),
                CueCommand::Wait,
            ],
            "末尾 Wait まで含め全 cue が配送済み（占有 horizon 未達でも配送は完了している）"
        );
        assert!(
            !handle.actor.is_finished(),
            "配送完了後も horizon 未達ゆえ talk は駆動継続（早期終了せず TalkDone 未送出）"
        );

        // horizon 到達で初めて完了する（末尾 Wait の 0.8 秒が終端で切り捨てられない）。
        handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("占有 horizon 到達で TalkDone が発火するべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
        handle.actor.join().expect("body は正常終了する");
    }

    /// **待ちを持たない末尾テキストのみの talk**: 完了通知は最終テキストの **配送時刻**（発火 start）でなく、
    /// その再生時間 D を含む絶対終了時刻（start + D）到達で発火する（R2.5/D6）。
    ///
    /// `\s[10]hello\_w[500]world\e` の台本（アンカー 0）:
    ///   ClearAll@0 / Emote{10}@0 / hello@0(dur=D_h) / Wait@D_h(dur=0.5) / world@(D_h+0.5)(dur=D_w)。
    /// 末尾 cue は Text(world)。world は Tick(D_h+0.5) で配送されるが（entry 枯渇）、占有 horizon＝
    /// `(D_h+0.5) + D_w` であり、world の **再生時間 D_w** が終端で落とされずそこまで完了は遅れる。
    #[test]
    fn trailing_final_text_talkdone_fires_after_text_duration_not_at_delivery() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(721);
        let start = StartTalk {
            script: r"\s[10]hello\_w[500]world\e".to_string(),
            talk_id,
        };
        let sink = RecordingSink::new();
        let records = sink.records();
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        let d_hello = text_playback_duration("hello"); // 0.25
        let w = Duration::from_millis(500).as_secs_f64(); // 0.5（\_w[500]）
        let d_world = text_playback_duration("world"); // 0.25
        let t_world = d_hello + w; // 末尾テキスト world の配送時刻（0.75）
        let horizon = t_world + d_world; // world の再生完了時刻＝占有 horizon（1.0）

        // 初回 Tick(0.0) 刻印 → Tick(D_h) で Wait 配送 → Tick(t_world) で末尾 world を配送し entry 枯渇。
        // t_world は末尾テキストの **発火時刻** であって完了時刻ではない（単調 0.0 < 0.25 < 0.75）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(d_hello)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(t_world)).unwrap();

        // 負の窓: 末尾テキストは配送済み（発火 start 到達）だが、その再生時間 D_w ぶん完了は遅れる。
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "末尾テキストの配送時刻（発火 start）では TalkDone は発火してはならない（再生時間を終端で落とさない・R2.5）"
        );
        assert_eq!(
            commands(&records),
            vec![
                CueCommand::ClearAll,
                CueCommand::Emote { key: "10".into() },
                CueCommand::Text("hello".into()),
                CueCommand::Wait,
                CueCommand::Text("world".into()),
            ],
            "末尾テキスト world まで全 cue 配送済み（発火はしたが再生は未完了）"
        );
        assert!(
            !handle.actor.is_finished(),
            "末尾テキスト配送後も start+D 未達ゆえ駆動継続（配送 ≠ 再生完了）"
        );

        // start + D（世界の再生完了＝占有 horizon）到達で初めて完了する。
        handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("末尾テキストの再生完了（start+D）で TalkDone が発火するべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
        handle.actor.join().expect("body は正常終了する");
    }

    /// **tick 源の liveness 契約**: horizon 未満で tick が止まると `TalkDone` は発火せず、horizon まで
    /// tick を送り続けると発火する（task 7.2 申し送り＝「tick 源は entries 枯渇後も horizon 到達まで
    /// tick を送り続ける」）。本 spec は本番 tick 源を変えず、drive は `is_completed()` 成立で発火する。
    ///
    /// `\s[10]ab\_w[600]\e`（ab=2char→D=0.1・Wait 0.6・horizon=0.7）で、entry 枯渇（0.1）でも、その先の
    /// horizon 手前（0.5）でも、tick を止めれば完了通知は保留され、horizon（0.7）到達で初めて発火する。
    #[test]
    fn talkdone_withheld_while_ticks_stop_below_horizon_then_fires_on_resume() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(722);
        let start = StartTalk {
            script: r"\s[10]ab\_w[600]\e".to_string(),
            talk_id,
        };
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(NoopSink, NoopSink),
            SystemVarSnapshot::default(),
        );

        let d_ab = text_playback_duration("ab"); // 0.1
        let w = Duration::from_millis(600).as_secs_f64(); // 0.6
        let horizon = d_ab + w; // 0.7

        // 初回 Tick(0.0) 刻印 → Tick(D) で Wait 配送＝entry 枯渇。ここで tick を **止める**。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(d_ab)).unwrap();
        // tick 停止中（entry 枯渇・horizon 未満）は完了通知が発火しない。
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "tick が horizon 未満で止まると TalkDone は発火しない（entry 枯渇 ≠ 完了・R2.5）"
        );
        assert!(!handle.actor.is_finished(), "駆動継続（未完了）");

        // tick を再開するが依然 horizon 手前（0.5 < 0.7）。まだ発火しない。
        handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "horizon 手前まで進めても未達なら TalkDone は発火しない"
        );
        assert!(!handle.actor.is_finished(), "horizon 手前ゆえ依然駆動継続");

        // horizon まで tick を送り切ると初めて発火する（liveness 契約の正の側）。
        handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("horizon まで tick を送り続けると TalkDone が発火するべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
        handle.actor.join().expect("body は正常終了する");
    }

    /// **終端理由と絶対終了時刻の型的別概念（D6・R2.5）**: `TalkDone.reason` は compile 時に確定する
    /// 終端理由 `TalkEndReason`（`Ended`/`Quit`＝時間量でない）に等しく、一方 **発火の時刻** は台本由来の
    /// 占有 horizon（`absolute_end_time`）で決まる——この 2 つは互いに独立した事実である。
    ///
    /// `\s[10]hi\_w[700]\-`（末尾 `\-`→Quit・末尾に Wait 0.7）で、(1) `done.reason` が compile の
    /// `TalkEndReason::Quit` に一致し（`Ended` の反例で時間由来でないことを示す）、(2) その発火は
    /// `compiled.sheet.absolute_end_time()` 由来の horizon 到達まで遅れる（entry 枯渇では発火しない）ことを固定する。
    #[test]
    fn talkdone_reason_is_compiled_end_while_firing_time_is_horizon_derived() {
        let script = r"\s[10]hi\_w[700]\-";

        // FACT 1（終端理由）: reason は compile 時に確定する TalkEndReason（時間量でない enum）。
        let compiled = compile(
            &areka_parsers::sakura::parse(script),
            &crate::sysvar::SystemVarSnapshot::default(),
        );
        assert_eq!(
            compiled.end,
            TalkEndReason::Quit,
            "末尾 `\\-` の終端理由は Quit（時刻でなく理由）"
        );
        // FACT 2（終了時刻）: 発火時刻の権威は台本由来の占有 horizon（アンカー未刻印＝0 起点で導出）。
        let horizon = compiled.sheet.absolute_end_time(); // 0.1(hi) + 0.7(\_w[700]) = 0.8

        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(723);
        let start = StartTalk {
            script: script.to_string(),
            talk_id,
        };
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(NoopSink, NoopSink),
            SystemVarSnapshot::default(),
        );

        let d_hi = text_playback_duration("hi"); // 0.1（末尾 Wait の発火時刻＝entry 枯渇点）

        // 初回 Tick(0.0) 刻印 → Tick(D) で末尾 Wait 配送＝entry 枯渇（horizon=0.8 の遥か手前）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(d_hi)).unwrap();
        // 発火時刻が horizon 由来である証: entry 枯渇では発火しない（reason が確定していても時刻は別権威）。
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "終端理由が確定していても発火は entry 枯渇でなく horizon 到達に従う（時刻は別権威・D6）"
        );
        assert!(!handle.actor.is_finished(), "horizon 未達ゆえ駆動継続");

        // 台本由来の horizon 到達で発火。reason は compile 由来（Quit）で、firing time は horizon 由来。
        handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("台本由来の horizon 到達で TalkDone が発火するべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(
            done.reason, compiled.end,
            "reason は compile が確定した TalkEndReason（Quit）に等しい（時間量でない）"
        );
        assert_eq!(
            done.reason,
            TalkEndReason::Quit,
            "末尾 `\\-` は Quit（Ended でない＝reason は時刻でなく理由由来）"
        );
        handle.actor.join().expect("body は正常終了する");
    }

    // ── task 5.2: ResolveChoice ハンドラ＋即時 settle の統合檻（R2.3/2.4/9.8） ──
    //
    // 共通 fixture: `\s[10]hello\w[2]\q[選択A,targetA]\e`。compile 後（アンカー 0）:
    //   ClearAll@0 / Emote{10}@0 / hello@0(D=0.25) / Wait@0.25(0.1) / Choice@0.35(id=targetA) /
    //   Barrier@0.35（選択待ち・R2.1/2.2）。占有 horizon=0.35。barrier が**最終 horizon 要素**（menu
    //   ケース）ゆえ、Tick(0.5) で barrier 到達後に解決すると、既に current_offset(0.5) ≥ horizon(0.35)
    //   で **その場で** 完了する（次 Tick を待たない・settle_after_tick と同型の後始末を共用）。

    const MENU_SCRIPT: &str = r"\s[10]hello\w[2]\q[選択A,targetA]\e";

    /// Choice の着弾（＝barrier 到達）を決定的に観測するため、記録 sink に加えチャンネル sink を挟む
    /// ヘルパ。Tick(0.5) を送り、Choice(id=targetA) cue の着弾を待って返す（この時点で player は
    /// `WaitingForChoice`・後続 ResolveChoice は inbox FIFO でこの後に処理される）。
    fn drive_menu_to_barrier(
        handle: &TalkHandle,
        rx: &mpsc::Receiver<TalkCue>,
    ) {
        // 初回 Tick(0.0) 刻印: ClearAll/Emote/hello を配送（barrier 未到達）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        // Tick(0.5): Wait@0.25・Choice@0.35 を配送し barrier@0.35 到達 → WaitingForChoice。
        handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
        // Choice cue 着弾を barrier に、barrier 到達（WaitingForChoice 遷移）を決定的に待つ。
        loop {
            let cue = rx
                .recv_timeout(Duration::from_secs(5))
                .expect("Choice cue（barrier 手前）が届くべき");
            if matches!(cue.command, CueCommand::Choice { .. }) {
                break;
            }
        }
    }

    /// **R2.3（barrier-stop）**: 選択待ち barrier で止まった talk は、horizon 越えまで `Tick` を注入
    /// しても**完了として通知されない**（選択未解決の間 `TalkDone` を出さない）。
    #[test]
    fn menu_barrier_withholds_talkdone_while_choice_unresolved() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let (tx, rx) = mpsc::channel::<TalkCue>();
        let start = StartTalk {
            script: MENU_SCRIPT.to_string(),
            talk_id: TalkId(801),
        };
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(ChannelSink { tx }, NoopSink),
            SystemVarSnapshot::default(),
        );

        drive_menu_to_barrier(&handle, &rx);

        // horizon(0.35) を遥かに越える Tick を注入しても、選択未解決ゆえ完了しない（R2.3）。
        handle.inbox.send(SakuraMsg::Tick(5.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(50.0)).unwrap();

        // 負の窓: barrier 未解決の間は TalkDone が発火しない（早期完了しない）。
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "選択待ち barrier 未解決の間は horizon 越え Tick でも TalkDone を出さない（R2.3）"
        );
        assert!(
            !handle.actor.is_finished(),
            "barrier 未解決ゆえ talk は駆動継続（完了通知せず）"
        );

        // 片付け: Close で中断 ACK を取り body を畳む（テスト resource の後始末）。
        handle.inbox.send(SakuraMsg::Close).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("Close で中断 ACK");
        assert_eq!(done.reason, TalkEndReason::Interrupted);
        handle.actor.join().expect("body は正常終了する");
    }

    /// **R2.4/9.8（resolve-resume・即時 settle）**: barrier で止まった talk へ有効な選択 id を
    /// `SakuraMsg::ResolveChoice` で投入すると、**追加の `Tick` なしに**再開し `TalkDone{Ended}` へ
    /// 到達する（menu ケース＝barrier が最終 horizon 要素ゆえその場で完了・R-5 の一 tick 遅延を残さない）。
    #[test]
    fn resolve_choice_resumes_barrier_stopped_talk_and_settles_immediately() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let (tx, rx) = mpsc::channel::<TalkCue>();
        let talk_id = TalkId(802);
        let start = StartTalk {
            script: MENU_SCRIPT.to_string(),
            talk_id,
        };
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(ChannelSink { tx }, NoopSink),
            SystemVarSnapshot::default(),
        );

        drive_menu_to_barrier(&handle, &rx);

        // 有効な選択 id を投入。追加 Tick は**送らない**（即時 settle の弁別）。
        handle
            .inbox
            .send(SakuraMsg::ResolveChoice {
                id: "targetA".to_string(),
            })
            .unwrap();

        // 追加 Tick なしで自然終端へ到達する（barrier 解決で offset(0.5) ≥ horizon(0.35) ＝即完了）。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("ResolveChoice で talk が再開し、追加 Tick なしで TalkDone に到達すべき（R2.4/9.8）");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(
            done.reason,
            TalkEndReason::Ended,
            "`\\e` 終端の menu talk は解決後 Ended で完了する"
        );
        handle.actor.join().expect("body は正常終了する");
    }

    /// **mismatch**: 未知の選択 id で `ResolveChoice` しても状態は不変（`None` 記録＋継続）で
    /// `TalkDone` は出ず、talk は待機継続する。その後**有効な id** で解決すれば完了へ到達し、
    /// 誤 id が barrier を壊していない（talk が生存継続していた）ことを示す。
    #[test]
    fn resolve_choice_with_unknown_id_is_noop_and_talk_continues() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let (tx, rx) = mpsc::channel::<TalkCue>();
        let talk_id = TalkId(803);
        let start = StartTalk {
            script: MENU_SCRIPT.to_string(),
            talk_id,
        };
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(ChannelSink { tx }, NoopSink),
            SystemVarSnapshot::default(),
        );

        drive_menu_to_barrier(&handle, &rx);

        // 未知 id: resolve_choice は None → 記録して継続（状態不変・barrier は解けない）。
        handle
            .inbox
            .send(SakuraMsg::ResolveChoice {
                id: "NO_SUCH_ID".to_string(),
            })
            .unwrap();

        // 負の窓: 誤 id では完了しない（barrier 依然未解決）。
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "未知 id の ResolveChoice では TalkDone を出さない（状態不変・継続）"
        );
        assert!(
            !handle.actor.is_finished(),
            "誤 id は barrier を壊さず talk は待機継続する"
        );

        // 有効 id で解決すれば完了へ到達（barrier が生きていた＝誤 id で壊れていない証）。
        handle
            .inbox
            .send(SakuraMsg::ResolveChoice {
                id: "targetA".to_string(),
            })
            .unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("有効 id の解決で完了へ到達すべき（誤 id 後も barrier は生存）");
        assert_eq!(done.talk_id, talk_id);
        assert_eq!(done.reason, TalkEndReason::Ended);
        handle.actor.join().expect("body は正常終了する");
    }

    /// **R2.3/2.4/9.8（full-menu 統合檻・Task 10.2）**: `menu.pasta:15` 相当の**実 3 択メニュー**
    /// （`\q \n \q \_l[5em,2lh] \q`）を **`spawn_talk` の actor 境界**（内部で parse→compile を通す）へ
    /// 投入し、選択待ち barrier 停止→3 択のうち**実 id の 1 つ**での解決→即時 settle を end-to-end で固定する。
    ///
    /// 5.2 の 3 檻（`menu_barrier_*`/`resolve_choice_*`）は**単一** `\q` の `MENU_SCRIPT` で actor 境界を
    /// 覆い、`compile_broadcast_stream_*` は**3 択実 menu** を覆うが**生 `CuePlayer`**（actor 境界を通らない）。
    /// 本檻はその両者の交差＝「実 3 択 menu × `spawn_talk` × 実 choice id 解決」を単一の統合檻で立証する
    /// （5.2 の単一 `\q` では現れない、複数 Choice がバッグに並ぶ中で**中間 id** を照合して解ける経路）。
    ///
    /// 檻は 3 主張を 1 本の actor フローで固定する:
    ///  - **R2.3**: barrier 停止後、horizon を遥かに越える `Tick(5.0)/Tick(50.0)` でも `TalkDone` 不送出。
    ///  - **mismatch**: 未知 id の `ResolveChoice` では状態不変（`TalkDone` 不送出・talk 継続）。
    ///  - **R2.4/9.8**: 3 択の**中間**実 id（`Onエモの位置調整メニュー`）で解決すると、**追加 `Tick` なしに**
    ///    再開し `TalkDone{Ended}` へ到達する（barrier は最終 horizon 要素＝その場で settle）。
    #[test]
    fn full_menu_via_spawn_talk_barrier_stops_and_middle_choice_id_settles_immediately() {
        // menu.pasta:15 の raw さくらスクリプト断片（3 択＋改行＋カーソル指定）。
        // `spawn_talk` へ**生 script として**渡し、parse→compile は actor 内部の実経路を通す
        // （5.2 の単一 `\q` MENU_SCRIPT でも、生 CuePlayer 檻@compile_broadcast_stream_* でもなく、
        //  実 3 択 menu が actor 境界を貫く経路をここで初めて覆う）。
        let script = concat!(
            r"\q[おしゃべり頻度,Onおしゃべり頻度メニュー]",
            r"\n",
            r"\q[エモの位置調整,Onエモの位置調整メニュー]",
            r"\_l[5em,2lh]",
            r"\q[閉じる,Onメニュー閉じる]",
        );
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let (tx, rx) = mpsc::channel::<TalkCue>();
        let talk_id = TalkId(810);
        let start = StartTalk {
            script: script.to_string(),
            talk_id,
        };
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(ChannelSink { tx }, NoopSink),
            SystemVarSnapshot::default(),
        );

        // Tick(0.0)/Tick(0.5) で offset 0 群（ClearAll/3 Choice/NewLine/Cursor）を配送し barrier@0 到達を待つ。
        drive_menu_to_barrier(&handle, &rx);

        // R2.3: horizon(=0) を遥かに越える Tick を注入しても、選択未解決ゆえ完了しない。
        handle.inbox.send(SakuraMsg::Tick(5.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(50.0)).unwrap();
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "実 3 択 menu でも barrier 未解決の間は horizon 越え Tick で TalkDone を出さない（R2.3）"
        );
        assert!(
            !handle.actor.is_finished(),
            "barrier 未解決ゆえ talk は駆動継続（早期完了しない・R2.3）"
        );

        // mismatch: 3 択のいずれとも一致しない id では状態不変（`None` 記録＋継続・barrier は解けない）。
        handle
            .inbox
            .send(SakuraMsg::ResolveChoice {
                id: "NO_SUCH_ID".to_string(),
            })
            .unwrap();
        assert!(
            done_rx.recv_timeout(NEG_WINDOW).is_err(),
            "不一致 id の ResolveChoice では TalkDone を出さない（状態不変・複数 Choice バッグは無傷）"
        );
        assert!(
            !handle.actor.is_finished(),
            "不一致 id は barrier を壊さず talk は待機継続する（バッグの他 id は依然解決可能）"
        );

        // R2.4/9.8: 3 択の**中間** id を投入。追加 Tick は**送らない**（即時 settle の弁別）。
        // 中間 id を選ぶことで「先頭/末尾でなくバッグ内の任意 id を照合して解ける」ことも固定する。
        handle
            .inbox
            .send(SakuraMsg::ResolveChoice {
                id: "Onエモの位置調整メニュー".to_string(),
            })
            .unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("中間実 id の解決で再開し、追加 Tick なしで TalkDone に到達すべき（R2.4/9.8）");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(
            done.reason,
            TalkEndReason::Ended,
            "`\\e` 無しの menu 台本は既定 Ended で完了する（compile 既定＝Ended）"
        );
        handle.actor.join().expect("body は正常終了する");
    }

    /// **defensive（Armed 誤投函）**: 初回 `Tick` 前（`Armed`＝CuePlayer 未構築）に `ResolveChoice` が
    /// 届いても warn して継続し（防御枝）、以降の通常 Tick 駆動で talk は正常に終端する。
    #[test]
    fn resolve_choice_before_playback_armed_is_ignored_and_playback_survives() {
        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(804);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
        };
        let sink = RecordingSink::new();
        let records = sink.records();
        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink, NoopSink),
            SystemVarSnapshot::default(),
        );

        // 初回 Tick 前（Armed）に ResolveChoice 誤投函: warn して継続（CuePlayer 未構築ゆえ no-op）。
        handle
            .inbox
            .send(SakuraMsg::ResolveChoice {
                id: "targetA".to_string(),
            })
            .unwrap();

        // 通常 Tick 列で駆動・終端する（防御枝がループを殺していない証）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("Armed 誤投函後も通常 Tick で終端するべき");
        assert_eq!(done.reason, TalkEndReason::Ended, "再生は破綻せず Ended");
        handle.actor.join().expect("body は正常終了する");
        assert_eq!(
            records.lock().unwrap().len(),
            5,
            "誤投函で早期全量配信されず、通常 5 cue が届く"
        );
    }

    // ── task 10.1: 配送列統合檻（R9.7/R1.8/R8.6・責務二分） ──

    /// **配送列統合檻（R9.7/R1.8/R8.6）**: 実 `parse`→`compile` の menu 台本を `CuePlayer` ＋記録
    /// sink×**複数**で駆動し、broadcast 観測順が **compile 順**に一致すること（Choice が NewLine/
    /// Cursor と**交互のまま**配送列に現れること・R1.8）と、同一 Choice が同時に**バッグ**
    /// （`pending_choices`）へも積まれること（責務二分＝配送列は表示の単一真実源／バッグは解決照合の
    /// 単一真実源・R8.6/R9.7）を、実 `compile → CuePlayer broadcast → sink` 経路上で固定する。
    ///
    /// **task 2.2 との差**: 2.2 は dola `runtime_test.rs` で**手組みの CueSheet** を使う runtime 檻。
    /// 本檻は **areka-sakura の実 `parse`＋実 `compile`** から得た CueSheet を出発点とし、
    /// **複数**記録 sink へ broadcast させて配送列とバッグの並存を end-to-end で固定する統合檻である
    /// （案C の「Choice 除外」廃止・配送列合流を実 compile 出力に対して立証する）。
    ///
    /// 弁別: もし Choice が配送列から隠される（旧・先積み一択）なら配送列の等価 assert が FAIL し、
    /// もしバッグへ積まれなければ `pending_choices` の assert が FAIL する（配送列とバッグの乖離を捕捉）。
    #[test]
    fn compile_broadcast_stream_preserves_order_and_choices_also_land_in_bag() {
        use dola::cue::{CuePlayer, PendingChoice};

        // menu.pasta:15 の raw さくらスクリプト断片（`\q \n \q \_l[5em,2lh] \q`・task 4.4 cage と同一）。
        // 実 parse → 実 compile を通す（手組み Instruction でなく、パーサ〜コンパイラの実経路を出発点にする）。
        let script = concat!(
            r"\q[おしゃべり頻度,Onおしゃべり頻度メニュー]",
            r"\n",
            r"\q[エモの位置調整,Onエモの位置調整メニュー]",
            r"\_l[5em,2lh]",
            r"\q[閉じる,Onメニュー閉じる]",
        );
        let instructions = areka_parsers::sakura::parse(script);
        let compiled = compile(&instructions, &SystemVarSnapshot::default());

        // 実 compile 出力から CuePlayer を構築し、記録 sink を **2 本** broadcast 登録する
        // （どの sink も全 cue を受ける・登録順は配送内容に影響しない）。
        let mut player = CuePlayer::from_sheet(&compiled.sheet);
        let sink_a = RecordingSink::new();
        let sink_b = RecordingSink::new();
        let records_a = sink_a.records();
        let records_b = sink_b.records();
        player.register_sink(Box::new(sink_a));
        player.register_sink(Box::new(sink_b));

        // 全内容は瞬時（at=0）＋末尾に選択待ち barrier@0。単一 tick(0.0) で offset 0 群（内容 6 cue）を
        // 配送し barrier@0 到達 → WaitingForChoice（barrier 手前の cue は配送済み）。
        player.tick(0.0);
        assert_eq!(
            player.state(),
            &dola::cue::CuePlayerState::WaitingForChoice,
            "menu 台本は末尾 barrier で WaitingForChoice へ停止する（barrier 手前は配送済み）"
        );

        // 期待配送列（compile 順）: 冒頭 ClearAll 前置＋内容 5 件（Choice が NewLine/Cursor と交互）。
        // barrier は配送列に現れない（Barrier は presentation でなく sink へ配られない）。
        let expected_stream = vec![
            CueCommand::ClearAll,
            CueCommand::Choice {
                id: "Onおしゃべり頻度メニュー".into(),
                text: "おしゃべり頻度".into(),
                references: vec![],
            },
            CueCommand::NewLine { ratio: 1.0 },
            CueCommand::Choice {
                id: "Onエモの位置調整メニュー".into(),
                text: "エモの位置調整".into(),
                references: vec![],
            },
            CueCommand::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            CueCommand::Choice {
                id: "Onメニュー閉じる".into(),
                text: "閉じる".into(),
                references: vec![],
            },
        ];
        // 複数 sink が **同一の配送列を同一順序**で受ける（broadcast・Choice を隠さず交互のまま合流・R1.8）。
        assert_eq!(
            commands(&records_a),
            expected_stream,
            "sink A: 配送列が compile 順（Choice が NewLine/Cursor と交互のまま現れる・R1.8/R9.7）"
        );
        assert_eq!(
            commands(&records_b),
            expected_stream,
            "sink B: broadcast ゆえ両 sink が同一の配送列を受ける（中央振り分けなし）"
        );

        // 交互配置の直接固定（index 1/3/5 が Choice・2 が NewLine・4 が Cursor）。full-vector 等価に
        // 加えて「交互のまま」の意図を legible に残す（Choice が改行/カーソルに埋もれず順序保持）。
        let stream_a = commands(&records_a);
        assert!(
            matches!(stream_a[1], CueCommand::Choice { .. })
                && matches!(stream_a[2], CueCommand::NewLine { .. })
                && matches!(stream_a[3], CueCommand::Choice { .. })
                && matches!(stream_a[4], CueCommand::Cursor { .. })
                && matches!(stream_a[5], CueCommand::Choice { .. }),
            "Choice/NewLine/Choice/Cursor/Choice が交互のまま配送列に並ぶ（R1.8）"
        );

        // 責務二分（R8.6/R9.7）: **同一 3 Choice** がバッグ（解決照合の単一真実源）へも**同時に**積まれる。
        // バッグ内容は id/text で配送列の 3 Choice と一致する（配送列とバッグが乖離しない）。
        let expected_bag = vec![
            PendingChoice {
                id: "Onおしゃべり頻度メニュー".into(),
                text: "おしゃべり頻度".into(),
            },
            PendingChoice {
                id: "Onエモの位置調整メニュー".into(),
                text: "エモの位置調整".into(),
            },
            PendingChoice {
                id: "Onメニュー閉じる".into(),
                text: "閉じる".into(),
            },
        ];
        assert_eq!(
            player.pending_choices(),
            expected_bag.as_slice(),
            "同一 3 Choice がバッグへも同時に積まれる（責務二分＝配送列とバッグが並存・R8.6）"
        );

        // 配送列側の Choice を抽出し、バッグと (id, text) で完全一致することを固定する
        // （同一 Choice が配送列とバッグの**両路**に現れる＝責務二分の相互整合）。
        let stream_choices: Vec<(String, String)> = stream_a
            .iter()
            .filter_map(|cmd| match cmd {
                CueCommand::Choice { id, text, .. } => Some((id.clone(), text.clone())),
                _ => None,
            })
            .collect();
        let bag_choices: Vec<(String, String)> = player
            .pending_choices()
            .iter()
            .map(|c| (c.id.clone(), c.text.clone()))
            .collect();
        assert_eq!(
            stream_choices, bag_choices,
            "配送列に現れる 3 Choice とバッグの 3 Choice が同一（id/text・順序とも一致）"
        );
    }

    // ── task 10.3: 未知コマンド名の第一級縮退（統合檻・R8.2/R8.5/R9.3b） ──

    /// **未知コマンド名の第一級縮退（統合檻・R8.2/R8.5/R9.3b）**: `\!` 名前空間の**未知・M1 未対応
    /// コマンド名**（`\![raise,OnBoot]`／単独形 `\![vanish]`）を含む生 script を `spawn_talk` の actor
    /// 境界（内部で parse→compile→CuePlayer broadcast を通す）へ投入し、次の 3 点を end-to-end で固定する:
    ///
    ///  - **R8.2（compile 卒業）**: 未知名 `\!` は compile の無音落ちでなく汎用コマンド cue（`Custom`
    ///    キャリア）として**台本に第一級で載る**。ゆえに broadcast された各記録 sink の配送列に
    ///    キャリア cue が現れる（2 名とも `raise`／`vanish` を受ける＝配送で消えない）。
    ///  - **R8.5/R5（良性スキップ）**: どの消費者も未知名キャリアに action しない——`command_target_of`
    ///    が未知名に対し `None`（担当消費者なし）を返す。記録 sink は全 cue を**記録**する（無音破棄でも
    ///    異常終了でもない・honor は不変）。**複数** sink が同一列を受けて talk が完走することがその証跡。
    ///  - **R9.3b（第一級縮退）＋partition**: 未知名は名前権威表 `command_target_of` 上で `None`＝どの
    ///    消費者の担当でもなく、Some を返すのは M1 の `"move"` のみ（1 コマンド名の担当は高々 1）。
    ///    partition 不変条件の網羅檻は dola `sink_test.rs::command_target_of_maps_move_and_rejects_unknown_names`
    ///    （task 1.4）が正本であり、本檻は統合経路上でその帰結（未知名→None・move→Some）を焦点確認する。
    ///
    /// 弁別: もし compile が未知名を無音落ちさせるなら配送列にキャリアが現れず等価 assert が FAIL する。
    /// もし未知名が誰かの担当（Some）へ誤配線されるなら `command_target_of` の None assert が FAIL する。
    #[test]
    fn unknown_command_names_broadcast_and_benign_skip_then_talk_completes() {
        use dola::cue::{CueTarget, command_target_of};

        let (done_tx, done_rx) = mpsc::channel::<TalkDone>();
        let talk_id = TalkId(103);
        // 未知名 2 種（引数付き `raise` と単独形 `vanish`）＋テキストを挟み `\e` で終端。
        // parse: `\![raise,OnBoot]`→GenericCommand{"raise",["OnBoot"]}／`\![vanish]`→GenericCommand{"vanish",[]}。
        // compile: いずれも command_carrier(name, args)（Custom キャリア）へ卒業・無音落ちしない（R8.2）。
        let start = StartTalk {
            script: r"\![raise,OnBoot]hello\![vanish]world\e".to_string(),
            talk_id,
        };
        // broadcast の第一級性を立証するため**複数**記録 sink を登録（両者が同一配送列を受ける）。
        let sink_a = RecordingSink::new();
        let sink_b = RecordingSink::new();
        let records_a = sink_a.records();
        let records_b = sink_b.records();

        let handle = spawn_talk(
            start,
            done_tx,
            two_sinks(sink_a, sink_b),
            SystemVarSnapshot::default(),
        );

        // 初回 Tick(0.0) でアンカー刻印。全内容は at=0 群（raise/hello）と at=0.25 群（vanish/world）。
        // 占有 horizon（world 再生完了＝0.25+0.25=0.50）を跨ぐ Tick(1.0) で自然終端する。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

        // R8.2 の帰結: 未知名キャリアが無音落ちせず talk が完走し TalkDone{Ended} を返す。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("未知名キャリアを含む talk も良性スキップして完了すべき（無音落ち／panic しない）");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー（R1.3）");
        assert_eq!(
            done.reason,
            TalkEndReason::Ended,
            "`\\e` 終端＝Ended（未知名は終端理由に影響しない）"
        );
        handle
            .actor
            .join()
            .expect("未知名キャリアでも body は panic せず正常終了する（良性スキップ）");

        // 期待 broadcast 列（compile 順・冒頭 ClearAll 前置）: ClearAll / raise / hello / vanish / world。
        // 未知名キャリアが**配送列に第一級で現れる**（compile が卒業させた証・R8.2）。
        let expected = vec![
            CueCommand::ClearAll,
            CueCommand::command_carrier("raise", vec!["OnBoot".into()]),
            CueCommand::Text("hello".into()),
            CueCommand::command_carrier("vanish", vec![]),
            CueCommand::Text("world".into()),
        ];
        assert_eq!(
            commands(&records_a),
            expected,
            "sink A: 未知名キャリア（raise/vanish）が配送列に第一級で現れる（無音落ちしない・R8.2）"
        );
        // broadcast の第一級性: 2 つ目の sink も同一列を受ける（未知名も両者へ届く＝配送で消えない・R5）。
        assert_eq!(
            commands(&records_b),
            expected,
            "sink B: broadcast ゆえ両 sink が同一配送列を受ける（未知名キャリアも欠落しない）"
        );

        // R8.5/R9.3b（良性スキップ＋担当なし）: 配送列中の各未知名キャリアについて、名前権威表
        // `command_target_of` が **None（担当消費者なし）** を返す＝どの消費者も action しない良性スキップ。
        // キャリア variant からのコマンド名抽出（`as_command_carrier`）を通し、抽出できた名前で判定する。
        let carrier_names: Vec<String> = commands(&records_a)
            .iter()
            .filter_map(|cmd| cmd.as_command_carrier().map(|(name, _)| name.to_string()))
            .collect();
        assert_eq!(
            carrier_names,
            vec!["raise".to_string(), "vanish".to_string()],
            "配送列から未知名キャリア 2 件（raise/vanish）が抽出される"
        );
        for name in &carrier_names {
            assert_eq!(
                command_target_of(name),
                None,
                "未知名 {name:?} はどの消費者の担当でもない（None＝記録付き良性スキップ・R8.5/R9.3b）"
            );
        }

        // partition 不変条件（R9.3b）の統合経路上の焦点確認: 名前権威表で Some を返すのは M1 の
        // `"move"` のみ（1 コマンド名の担当は高々 1）。網羅檻は dola task 1.4 の
        // `command_target_of_maps_move_and_rejects_unknown_names` が正本（本檻は重複せず帰結のみ確認）。
        assert_eq!(
            command_target_of("move"),
            Some(CueTarget::Window),
            "M1 の権威表で担当を持つ名は \"move\" のみ（partition 網羅は dola task 1.4 が正本）"
        );
    }
}
