//! 再生駆動層（drive）— per-talk transient アクターの起動・自己投函・駆動可能形への変換。
//!
//! [`spawn_talk`] は talk ごとに名前付きスレッド（`sakura-talk-{talk_id}`）を起動し、
//! spawn 直後に [`SakuraMsg::Start`] を自身の inbox へ**自己投函**する（投函経路は inbox
//! 一貫・validation Issue 1 の解決）。以降の `Tick`/`Close` 投函端と join ハンドルを
//! [`TalkHandle`] として呼び出し元へ返す。
//!
//! body は `Start` 受領時に上流 [`areka_parsers::sakura::parse`] → [`crate::compile::compile`]
//! を呼び、発火列を [`to_schedule`] で時刻駆動可能な [`TimedSchedule`] へ変換する。
//! **空 sheet**（発火列が空）なら時間軸駆動を行わず、コンパイル結果の終端理由を伴う
//! [`TalkDone`] を直ちに返して `Break`（R1.4/R6.2・裸の `\-` は空 sheet＋`end=Quit` ゆえ
//! `Ended` を固定送出しない）。
//!
//! # 高々 1 回の唯一機構
//!
//! body は `Option<TalkState>` を所有権スロットとして保持し、全終端経路は
//! 「`state.take()` → `done.send(D::from(TalkDone))` → 直後に `Break`」の対で実装する
//! （`Option::take()` が唯一の高々 1 回機構・終端フラグを持たない）。

use std::ops::ControlFlow;
use std::sync::mpsc::Sender;

use crate::compile::compile;
use crate::contract::{
    cue_target_of, CuePayload, CueSheet, CueTarget, SakuraMsg, StartTalk, TalkCue, TalkDone,
    TalkEndReason, TalkHandle, TalkId,
};
use crate::error::SakuraError;
use crate::sink::{SurfaceSink, TextSink};
use areka_actor::{run_inbox, spawn_actor};
use dola::cue::schedule::{Entry, TimedSchedule};

/// per-talk transient を起動し、`Tick`/`Close` の投函端と join ハンドルを返す。
///
/// [`spawn_actor`]`("sakura-talk-{talk_id}", body)` で talk ごとに名前付きスレッドを
/// 起動し（R10.1）、spawn 直後に [`SakuraMsg::Start`]`(start)` を返された `Sender` へ
/// **自己投函**する（投函経路は inbox 一貫・単一 inbox の全順序で `Start` 先行を保証）。
/// 呼び出し元へは [`TalkHandle`]`{inbox, actor}` を返し、以降 kanade/テストは inbox へ
/// `Tick`/`Close` のみ送る。
///
/// body（[`run_inbox`] ループ）は `Start` 受領時に上流 parse → [`compile`] →
/// [`to_schedule`] を行い発火列を時刻駆動可能形へ変換する。空 sheet なら時間軸駆動せず
/// 即 [`TalkDone`]`{compiled.end}` を送出して `Break`（R1.4/R6.2）。
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
    surface_sink: impl SurfaceSink + Send + 'static,
    text_sink: impl TextSink + Send + 'static,
) -> TalkHandle
where
    D: From<TalkDone> + Send + 'static,
{
    let talk_id = start.talk_id;
    let name = format!("sakura-talk-{}", talk_id.0);

    let (inbox, actor) = spawn_actor::<SakuraMsg, _>(&name, move |rx| {
        let mut driver = TalkDriver::new(surface_sink, text_sink, done);
        run_inbox::<SakuraMsg, std::convert::Infallible>(rx, move |msg| {
            Ok(driver.handle(msg))
        });
    });

    // 投函経路の一貫: spawn 直後に Start を自己投函する（外部からは送らない）。
    // 送信失敗はアクタースレッドが既に消えている場合のみ（通常不到達）。ログして継続。
    if inbox.send(SakuraMsg::Start(start)).is_err() {
        tracing::error!(actor = %name, "failed to self-post Start; actor thread gone");
    }

    TalkHandle { inbox, actor }
}

/// 1 talk の再生状態（body ローカル・他 talk と共有しない・R10.3）。
///
/// `Option<TalkState>` の所有権スロットとして body が保持し、終端時に `take()` して
/// 「高々 1 回」を保証する（`done` の届け先自体は driver 側で保持する `Sender<D>` を使い
/// 回すため move-consume ではなく、`Option::take()` が唯一の高々 1 回機構）。
/// `schedule`/`last_tick` は後続 task 5.2（Tick 駆動ループ）が消費する駆動状態。
// 非空 sheet 経路で確定した駆動状態は、Tick 駆動（task 5.2）と Close 中断 ACK（task 5.3）が
// 消費する。本 task では格納までを行うため全フィールドを dead_code 許容とする。
#[allow(dead_code)]
struct TalkState {
    /// talk 相関 ID（全出力へ対応付け・R1.3/R6.6）。
    talk_id: TalkId,
    /// 時刻駆動可能形（0 起点・`TimedSchedule::new(0.0)`）。task 5.2 が駆動する。
    schedule: TimedSchedule<TalkCue>,
    /// コンパイル時点で確定した終端理由（自然終端で返す reason）。
    end: TalkEndReason,
    /// 直前に処理した `Tick` の時刻（単調・冪等ガード用）。task 5.2 が更新する。
    last_tick: Option<f64>,
}

/// per-talk 駆動アクター本体。`Option<TalkState>` スロットを保持し、`Start` で状態を確定、
/// 空 sheet なら即終端する。非空 sheet の Tick 駆動と Close 中断は後続 task（5.2/5.3）。
struct TalkDriver<S: SurfaceSink, T: TextSink, D> {
    state: Option<TalkState>,
    surface_sink: S,
    text_sink: T,
    /// `TalkDone` の届け先（呼び出し側 inbox への変換投函）。
    done: Sender<D>,
}

impl<S: SurfaceSink, T: TextSink, D> TalkDriver<S, T, D>
where
    D: From<TalkDone> + Send + 'static,
{
    fn new(surface_sink: S, text_sink: T, done: Sender<D>) -> Self {
        Self {
            state: None,
            surface_sink,
            text_sink,
            done,
        }
    }

    /// inbox メッセージ 1 件を処理し、`run_inbox` 用の `ControlFlow` を返す。
    fn handle(&mut self, msg: SakuraMsg) -> ControlFlow<()> {
        match msg {
            SakuraMsg::Start(start) => self.on_start(start),
            SakuraMsg::Tick(t) => self.on_tick(t),
            SakuraMsg::Close => self.on_close(),
        }
    }

    /// `Start` 受領: parse → compile → to_schedule。空 sheet なら即 `TalkDone`→`Break`、
    /// 非空 sheet なら状態を確定して継続（Tick 駆動は task 5.2）。
    fn on_start(&mut self, start: StartTalk) -> ControlFlow<()> {
        // Start 二重受領は error!＋無視（プロトコル異常・非 panic）。
        if self.state.is_some() {
            tracing::error!("duplicate Start received; ignoring");
            return ControlFlow::Continue(());
        }

        let StartTalk { script, talk_id } = start;

        // 上流パーサで Instruction 列へ変換（再パースしない・R1.2）→ 純粋コンパイル。
        let instructions = areka_parsers::sakura::parse(&script);
        let compiled = compile(&instructions);

        // 空 sheet: 時間軸駆動せず即終端（R1.4/R6.2）。end は Ended 固定でなく compiled.end
        // （裸の `\-` は空 sheet＋Quit）。
        if compiled.sheet.is_empty() {
            let done = TalkDone {
                talk_id,
                reason: compiled.end,
            };
            if self.done.send(D::from(done)).is_err() {
                tracing::error!(talk_id = talk_id.0, "TalkDone done receiver dropped");
            }
            return ControlFlow::Break(());
        }

        // 非空 sheet: 駆動可能形へ変換し状態を確定して継続（Tick 駆動は task 5.2）。
        let schedule = to_schedule(&compiled.sheet);
        self.state = Some(TalkState {
            talk_id,
            schedule,
            end: compiled.end,
            last_tick: None,
        });
        ControlFlow::Continue(())
    }

    /// `Tick(t)` 受領: 単調・有限ガード → `schedule.tick(t)` → `ready()` の各 `TalkCue` を
    /// `cue_target_of` で 2 sink へ振り分け emit → `schedule.is_completed()` なら
    /// `TalkDone{end}` を送出し `Break`（自然終端・R6.1/6.2/6.3/9.1/9.2）。
    ///
    /// # ガード（R11.1/11.2/11.3・受信ループは殺さない）
    ///
    /// - **非有限**（`NaN`/`±inf`）: dola の NaN 全量配信ハザードを遮断するため
    ///   [`SakuraError::NonFiniteTick`] を構築して `tracing::error!` を記録し、`schedule` を
    ///   一切進めずに `Continue`（talk は終端させない）。
    /// - **逆行/同値**（`last_tick` の `Some(prev)` に対し `t <= prev`）: `tracing::debug!` の
    ///   no-op で `Continue`（`TimedSchedule::tick` の冪等 early-return と併せ二重発火を防ぐ）。
    ///   初回 `Tick` は `last_tick == None` ゆえ比較対象が無く必ず通過する（先頭 `at=0` 発火を
    ///   殺さないため `last_tick` を 0.0 で初期化しない・設計 Issue 2）。
    ///
    /// 状態未確定（`Start` 未受領・投函経路上は非到達の防御枝）なら no-op で `Continue`。
    fn on_tick(&mut self, t: f64) -> ControlFlow<()> {
        // 非有限ガード（R11.1/11.2）: schedule を進めず記録＋error ログ、ループは継続。
        if !t.is_finite() {
            let err = SakuraError::NonFiniteTick(t);
            tracing::error!(error = %err, "non-finite Tick ignored; schedule not advanced");
            return ControlFlow::Continue(());
        }

        // 状態未確定は防御枝（投函経路上 Start 先行が保証される・非到達）。
        let Some(state) = self.state.as_mut() else {
            tracing::error!("Tick received before Start; ignoring");
            return ControlFlow::Continue(());
        };

        // 単調ガード（逆行/同値は no-op・冪等）。初回は last_tick==None ゆえ必ず通過する。
        if let Some(prev) = state.last_tick
            && t <= prev
        {
            tracing::debug!(prev, t, "non-monotonic Tick ignored (backward or equal)");
            return ControlFlow::Continue(());
        }
        state.last_tick = Some(t);

        // Phase 1: 時刻前進。Phase 2: 到達済み発火を配送先分類で 2 sink へ振り分ける。
        state.schedule.tick(t);
        for cue in state.schedule.ready() {
            match cue_target_of(&cue.command) {
                Some(CueTarget::Shell) => self.surface_sink.emit(cue.clone()),
                Some(CueTarget::Balloon) => self.text_sink.emit(cue.clone()),
                None => {
                    // M-boot compile は分類不能 command を生成しない（防御・非 panic）。
                    tracing::error!(command = ?cue.command, "unclassifiable cue command; skipping");
                }
            }
        }

        // 自然終端検出: 全エントリ消費済みかつバリア中でない（R6.1/6.2/6.3）。
        // 高々 1 回機構: state.take() → done.send(D::from(TalkDone)) → Break。
        if state.schedule.is_completed() {
            let state = self.state.take().expect("state was Some above");
            let done = TalkDone {
                talk_id: state.talk_id,
                reason: state.end,
            };
            if self.done.send(D::from(done)).is_err() {
                tracing::error!(talk_id = state.talk_id.0, "TalkDone done receiver dropped");
            }
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    /// `Close` 受領: 進行中の再生を即時停止し、中断 ACK を返す（R7.1/7.2/7.3/7.4）。
    ///
    /// 高々 1 回機構（validation Issue 3）に従い、保持状態を `take()` してから
    /// `done.send(D::from(TalkDone{Interrupted}))` を送出、直後に `Break` する。`take()` した
    /// `schedule` は本メソッド終了時に drop され、未発火の残り cue は sink へ届かない
    /// （drain せず破棄＝R7.2）。`Break` は `run_inbox` の即時 return＝積み残し（inbox の
    /// 未処理メッセージ）は rx drop で破棄される（R7.3・停止規約整合）。
    ///
    /// 状態が既に `None`（自然終端は先に `take()`＋`Break` 済みのため通常は非到達＝
    /// `Start` 前の防御枝）なら、二度目の `TalkDone` を送らずログして `Break` する
    /// （通算高々 1 回・R6.4/R7.5）。自然終端後はスレッドが既に消えており Close 自体が
    /// inbox へ届かないため、この分岐は主に `Start` 受領前の防御的経路である。
    fn on_close(&mut self) -> ControlFlow<()> {
        // 高々 1 回機構: state.take() → done.send(D::from(TalkDone{Interrupted})) → Break。
        // take() された schedule の未発火 cue はここで drop＝sink へ届かない（R7.2）。
        if let Some(state) = self.state.take() {
            let done = TalkDone {
                talk_id: state.talk_id,
                reason: TalkEndReason::Interrupted,
            };
            // done 受領側（kanade）が既に drop されていれば error ログ（黙殺しない・R11.1/11.4）。
            if self.done.send(D::from(done)).is_err() {
                tracing::error!(
                    talk_id = state.talk_id.0,
                    "TalkDone done receiver dropped on Close"
                );
            }
        } else {
            // 状態未確定での Close（Start 前の防御枝・投函経路上は通常非到達）。
            // 二度目の TalkDone を送らずログのみ（通算高々 1 回・R6.4/R7.5）。
            tracing::debug!("Close received without active playback state; no ACK sent");
        }
        ControlFlow::Break(())
    }
}

/// 内部: [`CueSheet`] → [`TimedSchedule`]`<TalkCue>`（0 起点・`TimedSchedule::new(0.0)`）。
///
/// [`dola::cue::compile_sheet`] は使わない（min 正規化が先頭待ちを消すため・禁止）。
/// 挿入は [`CueSheet::cues`] の記述順に 1 件ずつ [`TimedSchedule::insert`] で行う
/// （`extend` 禁止）: insert は同一オフセット群の前方へ挿入し末尾 pop が挿入順を保つため、
/// 同一 `at` の cue は `CueSheet` 記述順（FIFO）で配信される（R4.1/4.2）。
///
/// [`CuePayload::Command`] 以外（Barrier/Routing・M-boot compile は非生成）は
/// `tracing::error!` を記録してスキップする（防御・非 panic）。
fn to_schedule(sheet: &CueSheet) -> TimedSchedule<TalkCue> {
    let mut schedule = TimedSchedule::new(0.0);
    for cue in sheet.cues() {
        match &cue.payload {
            CuePayload::Command(command) => {
                let talk_cue = TalkCue {
                    at: cue.start_time,
                    actor: cue.actor.clone(),
                    command: command.clone(),
                };
                // per-cue insert（extend 禁止）: 同一 at 群を記述順（FIFO）で保つ。
                schedule.insert(Entry::Payload(cue.start_time, talk_cue));
            }
            other => {
                // M-boot compile は Command 以外を生成しない防御枝（非到達）。
                tracing::error!(payload = ?other, "non-Command CuePayload in CueSheet; skipping");
            }
        }
    }
    schedule
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{CueCommand, TalkId};
    use crate::sink::MockSink;
    use std::time::Duration;

    /// 空発火列（空 script）の talk は時間軸駆動せず、Tick を一切送らなくても
    /// コンパイル結果の終端理由（空 script＝`Ended`）を伴う `TalkDone` を**即座に**返す
    /// （observable・R1.4）。`talk_id` は起動要求のものがエコーされる（R1.3）。
    #[test]
    fn empty_script_talk_returns_talkdone_immediately_without_tick() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(7);
        let start = StartTalk {
            script: String::new(), // 空 script → 空 Instruction 列 → 空 sheet。
            talk_id,
        };

        // 2 本の mock sink（surface 用・text 用）。空 sheet ゆえ発火は 1 件も無い。
        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        // Tick を一切送らずに spawn_talk を呼ぶ（時間軸駆動を要求しない）。
        let handle = spawn_talk(start, done_tx, surface, text);

        // TalkDone が即座に到達すること（Tick 不要・時間軸駆動なし）。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("空 script の talk は即座に TalkDone を返すべき");

        // talk_id エコー（R1.3）と終端理由（空 script＝Ended・R1.4）。
        assert_eq!(done.talk_id, talk_id, "talk_id がエコーされること");
        assert_eq!(done.reason, TalkEndReason::Ended, "空 script は Ended");

        // 発火は 1 件も無いこと（両 sink 空）。
        assert!(
            surface_records.lock().unwrap().is_empty(),
            "空 sheet では surface 発火が無いこと"
        );
        assert!(
            text_records.lock().unwrap().is_empty(),
            "空 sheet では text 発火が無いこと"
        );

        // join でスレッド終了を同期（Break 後にスレッドが正常終了していること）。
        handle.actor.join().expect("body は正常終了する");
    }

    /// 送出のたびに mpsc へ流すテスト用 sink（発火の**到着を barrier として同期受信**する）。
    ///
    /// `MockSink`（蓄積のみ）と異なり、各発火を 1 件ずつ受信できるため、「due 前の cue が
    /// 保留され、`at` 到達の Tick で初めて発火する」タイミングゲートを**中間観測で決定的に**
    /// アサートできる（sleep や短い timeout による不在判定を用いない）。
    struct ChannelSink {
        tx: std::sync::mpsc::Sender<TalkCue>,
    }
    impl SurfaceSink for ChannelSink {
        fn emit(&mut self, cue: TalkCue) {
            let _ = self.tx.send(cue);
        }
    }
    impl TextSink for ChannelSink {
        fn emit(&mut self, cue: TalkCue) {
            let _ = self.tx.send(cue);
        }
    }

    /// 未 due の発火は Tick を受けても**保留**され、`at` 到達（境界含む・`at <= tick`）の Tick で
    /// 初めて配送されることを、**中間観測で決定的に**検証する（実時計・sleep 非依存）。
    ///
    /// script `\s[10]hello\w[2]probeA\w[2]probeB\w[2]world\e` の発火予定:
    ///   Emote{10}@0.0(surface) / hello@0.0 / probeA@0.1 / probeB@0.2 / world@0.3 (text) / `\e`。
    ///
    /// **barrier 技法**: probe を受信できた時点で当該 Tick の ready 群は出し切られており、
    /// 次の Tick を送るまで後続（未 due）の発火は到着し得ない。ゆえに `try_recv()==Empty` が
    /// **timeout に依らない決定的な「保留」証明**になる（早すぎ発火があれば Empty にならない）。
    ///
    /// これにより、既存の集計ベーステスト（最終カウントのみ）では捕捉できない「1 tick 早い発火」
    /// を封鎖する（発火時刻ゲートの固定・dola `TimedSchedule` の `at <= tick` 境界包含も確認）。
    #[test]
    fn undue_cues_are_withheld_until_their_at_is_reached() {
        use std::sync::mpsc::{self, TryRecvError};

        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(314);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]probeA\w[2]probeB\w[2]world\e".to_string(),
            talk_id,
        };

        let (surface_tx, surface_rx) = mpsc::channel::<TalkCue>();
        let (text_tx, text_rx) = mpsc::channel::<TalkCue>();
        let handle = spawn_talk(
            start,
            done_tx,
            ChannelSink { tx: surface_tx },
            ChannelSink { tx: text_tx },
        );

        // 期待発火時刻。**compile と同一の `as_secs_f64()` 累積**で導出する（設計 Testing Strategy
        // の戒め: 10 進リテラル直書き `from_millis(300)`=0.3 は累積 0.1+0.1+0.1=0.30000000000000004
        // と異なる f64 となり、`Tick(0.3)` が world 未達＝境界包含判定を誤る）。
        let w = Duration::from_millis(100).as_secs_f64(); // `\w[2]`＝2×50ms
        let at0 = 0.0_f64;
        let at_a = w; // probeA: 1×`\w[2]`
        let at_b = at_a + w; // probeB: 2×`\w[2]`（累積）
        let at_w = at_b + w; // world:  3×`\w[2]`（累積）
        let recv = |rx: &mpsc::Receiver<TalkCue>| {
            rx.recv_timeout(Duration::from_secs(5))
                .expect("due な発火は届くこと")
        };

        // ── Tick(0.1): hello@0.0 と probeA@0.1 のみ due。probeB(0.2)/world(0.3) は未 due。 ──
        handle
            .inbox
            .send(SakuraMsg::Tick(at_a))
            .expect("Tick(0.1) 投函");
        // surface: Emote@0.0（1 件のみ・以降 surface 発火なし）。
        let s0 = recv(&surface_rx);
        assert_eq!(s0.at, at0, "surface Emote の発火時刻は 0.0");
        assert_eq!(s0.command, CueCommand::Emote { key: "10".into() });
        // text: hello@0.0 → probeA@0.1（at 昇順）。probeA 受信＝Tick(0.1) の ready 出し切り barrier。
        let hello = recv(&text_rx);
        assert_eq!(hello.command, CueCommand::Text("hello".into()));
        assert_eq!(hello.at, at0);
        let probe_a = recv(&text_rx);
        assert_eq!(probe_a.command, CueCommand::Text("probeA".into()));
        assert_eq!(probe_a.at, at_a);
        // ★保留の決定的証明: probeA barrier 到達時点で probeB(0.2)/world(0.3) は届いていない。
        assert_eq!(
            text_rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "at=0.1 の Tick では未 due の probeB(0.2)/world(0.3) が保留されること"
        );
        assert_eq!(
            surface_rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "surface 側も追加発火が無いこと"
        );

        // ── Tick(0.2): probeB@0.2 のみ新規 due。world(0.3) は依然 未 due。 ──
        handle
            .inbox
            .send(SakuraMsg::Tick(at_b))
            .expect("Tick(0.2) 投函");
        let probe_b = recv(&text_rx);
        assert_eq!(probe_b.command, CueCommand::Text("probeB".into()));
        assert_eq!(probe_b.at, at_b);
        // ★保留の決定的証明: probeB barrier 到達時点でも world(0.3) は未着。
        assert_eq!(
            text_rx.try_recv().unwrap_err(),
            TryRecvError::Empty,
            "at=0.2 の Tick でも未 due の world(0.3) が保留されること"
        );

        // ── Tick(0.3): world@0.3 が due（境界含む `at <= tick`）→ ここで初めて発火。 ──
        handle
            .inbox
            .send(SakuraMsg::Tick(at_w))
            .expect("Tick(0.3) 投函");
        let world = recv(&text_rx);
        assert_eq!(
            world.command,
            CueCommand::Text("world".into()),
            "world は at=0.3 到達で初めて発火する"
        );
        assert_eq!(world.at, at_w, "world の発火時刻は 0.3（境界包含 at<=tick で発火）");

        // 自然終端（`\e`＝Ended）・talk_id エコー。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("末尾到達で TalkDone");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
        handle.actor.join().expect("body は正常終了する");
    }

    /// **同一 `at`・同一 sink 内の発火順が記述順（FIFO）で保たれる**ことを検証する
    /// （`to_schedule` の per-cue `insert`＝`extend` 禁止の load-bearing 性質を統合レベルで固定）。
    ///
    /// script `\s[10]hello\nworld\e`（`\w` 無し）→ 全 cue が `at=0.0`:
    ///   Emote{10}(surface) / Text("hello") / NewLine / Text("world")（text）。
    /// 単一 `Tick(0.0)` で全 due・自然終端。text sink は **hello → NewLine → world** の
    /// 記述順で受信する（`extend` なら同一 at 群が逆順化し hello/world が入れ替わる＝R4.1/4.2 違反）。
    #[test]
    fn same_at_cues_preserve_script_order_fifo_within_a_sink() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(41);
        let start = StartTalk {
            script: r"\s[10]hello\nworld\e".to_string(),
            talk_id,
        };
        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, done_tx, surface, text);
        // 全 cue が at=0.0 ゆえ単一 Tick(0.0) で全 due→自然終端。
        handle
            .inbox
            .send(SakuraMsg::Tick(0.0))
            .expect("Tick(0.0) 投函");
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("単一 Tick で自然終端");
        assert_eq!(done.reason, TalkEndReason::Ended);
        handle.actor.join().expect("body は正常終了する");

        // surface: Emote{10}@0.0 の 1 件のみ。
        let surface = surface_records.lock().unwrap();
        assert_eq!(surface.len(), 1);
        assert_eq!(surface[0].command, CueCommand::Emote { key: "10".into() });

        // text: hello → NewLine → world の**記述順（FIFO）**で 3 件（全て at=0.0）。
        let text = text_records.lock().unwrap();
        assert_eq!(text.len(), 3, "text 発火は hello/NewLine/world の 3 件");
        assert!(text.iter().all(|c| c.at == 0.0), "同一 at=0.0 群であること");
        assert_eq!(
            text[0].command,
            CueCommand::Text("hello".into()),
            "先頭は記述順どおり hello（extend なら逆順化する）"
        );
        assert!(
            matches!(text[1].command, CueCommand::NewLine { .. }),
            "中央は NewLine"
        );
        assert_eq!(
            text[2].command,
            CueCommand::Text("world".into()),
            "末尾は記述順どおり world（extend なら hello と入れ替わる）"
        );
    }

    /// **`Start` の二重受領が無視される**ことを検証する（プロトコルガード・drive.rs `on_start`）。
    ///
    /// 1 本目（script A）で spawn 後、`inbox` へ別 script の 2 本目 `Start`(B) を送る。
    /// B は `state.is_some()` ゆえ無視され、talk は A の内容のみを再生する。**B の cue
    /// （Emote{77}/DIFFERENT）は一切現れず、B 用の done チャンネルは TalkDone を受け取らない**
    /// ことを決定的に確認する（timeout 待ちに依らず、テスト側で明示 drop した B の受信端は
    /// disconnect ＝ `recv` が即 `Err`）。
    #[test]
    fn duplicate_start_is_ignored_and_first_talk_plays_unchanged() {
        let (done_a_tx, done_a_rx) = std::sync::mpsc::channel::<TalkDone>();
        let id_a = TalkId(11);
        let start_a = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: id_a,
        };
        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();
        let handle = spawn_talk(start_a, done_a_tx, surface, text);

        // 2 本目 Start(B)（別 script）を inbox へ。自己投函の Start(A) の後に処理される。
        // B は state.is_some() ゆえ無視される（driver の done は A 起動時に束縛済みの単一
        // Sender のため、B 用に別途 done チャンネルを用意しても driver からは到達不能）。
        // done_b_tx はここで即 drop し、「無視された B には TalkDone が届かない」ことを
        // recv の即時 disconnect Err で決定的に示す（timeout 待ちに依らない）。
        let (done_b_tx, done_b_rx) = std::sync::mpsc::channel::<TalkDone>();
        drop(done_b_tx);
        let id_b = TalkId(99);
        let start_b = StartTalk {
            script: r"\s[77]DIFFERENT\e".to_string(),
            talk_id: id_b,
        };
        handle
            .inbox
            .send(SakuraMsg::Start(start_b))
            .expect("2 本目 Start(B) 投函");

        // A を駆動して自然終端。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.2)).unwrap();
        let done = done_a_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("A の TalkDone");
        assert_eq!(done.talk_id, id_a, "終端は A の talk_id（B に乗っ取られない）");
        assert_eq!(done.reason, TalkEndReason::Ended);
        handle.actor.join().expect("body は正常終了する");

        // A の内容のみが再生されること（B の Emote{77}/DIFFERENT は現れない）。
        let surface = surface_records.lock().unwrap();
        assert_eq!(surface.len(), 1);
        assert_eq!(
            surface[0].command,
            CueCommand::Emote { key: "10".into() },
            "surface は A の \\s[10]（B の 77 でない）"
        );
        let text = text_records.lock().unwrap();
        assert_eq!(text.len(), 2, "text は A の hello/world の 2 件");
        assert_eq!(text[0].command, CueCommand::Text("hello".into()));
        assert_eq!(text[1].command, CueCommand::Text("world".into()));

        // 無視された B は TalkDone を受け取らない（受信端 drop で disconnect＝recv 即 Err）。
        assert!(
            done_b_rx.recv_timeout(Duration::from_secs(5)).is_err(),
            "無視された Start(B) は TalkDone を返さない"
        );
    }

    /// **`to_schedule` が非 `Command` payload（Barrier/Routing）を skip する**防御枝を、
    /// module-private `to_schedule` を直接呼んで検証する（M-boot compile は生成しないが防御固定）。
    ///
    /// `CuePayload::Command` と `CuePayload::Barrier` を同一 `at=0.0` で混在させた `CueSheet` を
    /// 渡し、生成された `TimedSchedule` を `tick(0.0)`→`ready()` で観測すると、**Command 由来の
    /// `TalkCue` 1 件のみ**が出て Barrier は skip されている（非 panic）ことを確認する。
    #[test]
    fn to_schedule_skips_non_command_payload_without_panic() {
        use crate::contract::{ActorKey, BarrierKind, Cue, CuePayload, CueSheet};

        let sheet = CueSheet::new(vec![
            Cue {
                actor: ActorKey::from("0"),
                start_time: 0.0,
                payload: CuePayload::Command(CueCommand::Text("keep".into())),
                duration: 0.0,
            },
            Cue {
                actor: ActorKey::from("0"),
                start_time: 0.0,
                // M-boot compile は生成しない payload（防御枝を叩くため直接投入）。
                payload: CuePayload::Barrier(BarrierKind::WaitForInput { timeout: None }),
                duration: 0.0,
            },
        ]);

        let mut schedule = to_schedule(&sheet);
        schedule.tick(0.0);
        let ready = schedule.ready();

        // Barrier は skip され、Command 由来の TalkCue 1 件のみが due（非 panic）。
        assert_eq!(ready.len(), 1, "非 Command(Barrier) は skip され Command のみ残る");
        assert_eq!(ready[0].command, CueCommand::Text("keep".into()));
        assert_eq!(ready[0].at, 0.0);
    }

    /// **終端時に done 受信端が drop 済みでも body が panic せず clean exit する**ことを検証する
    /// （R11.1/11.4・全終端経路が `done.send(..).is_err()` を error ログで受けるガードの固定）。
    ///
    /// 駆動前に `done_rx` を drop → 自然終端で `done.send(D::from(TalkDone))` が `Err` になるが、
    /// `error!` を記録して `Break`＝スレッドは正常終了（`join` が Ok）。発火自体は正常に行われる
    /// （done drop は終端信号にのみ影響し、sink 配送には影響しない）。
    #[test]
    fn dropped_done_receiver_at_terminal_exits_cleanly_without_panic() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: TalkId(4),
        };
        let surface = MockSink::new();
        let text = MockSink::new();
        let text_records = text.records();
        let handle = spawn_talk(start, done_tx, surface, text);

        // 終端の TalkDone 送出前に受信端を drop（送出は Err になる）。
        drop(done_rx);
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.2)).unwrap();

        // done 受信端 drop でも panic せず正常終了すること（join が Ok・ハングしない）。
        handle
            .actor
            .join()
            .expect("done 受信端 drop でも body は panic せず正常終了する");

        // 発火は正常に行われた（done drop は終端信号にのみ影響）。
        assert_eq!(
            text_records.lock().unwrap().len(),
            2,
            "done drop は発火に影響しない（hello/world は配送済み）"
        );
    }

    /// M-boot 外タグのみで構成され**発火列が空になる** script は、リテラル空 script では
    /// ないにもかかわらず空 sheet へコンパイルされ、時間軸駆動（Tick）を一切要さずに
    /// 末尾到達の終端理由 `Ended`（R1.4）を伴う `TalkDone` を即座に返す。
    ///
    /// これは既存の「リテラル空 script」テスト（`empty_script_talk_returns_talkdone_immediately_without_tick`）
    /// とは別経路の固定である: script には内容（Choice `\q`・SystemVar `%username`・Raw `\0`）が
    /// あるが、これらは全て M-boot 外タグとして compile が無視し cue を生成しないため sheet が
    /// 空になる。終端命令を含まないため末尾到達で `end=Ended`（Quit ではない）。Tick を送らず
    /// とも空 sheet 経路で即終端し、両 sink は空であること・talk_id エコー（R1.3）を確認する。
    #[test]
    fn ignored_tags_only_script_ends_immediately_with_ended_and_no_firing() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(55);
        // Choice `\q[..]`・SystemVar `%username`・bare `\0`(→Raw) の 3 種はいずれも
        // M-boot 外タグ＝compile が無視し cue を生成しない。Text/Surface/終端命令は無い。
        // ⇒ 空 sheet かつ終端命令なし＝末尾到達で end=Ended。リテラル空 script ではない。
        let start = StartTalk {
            script: r"\q[はい,OnYes]%username\0".to_string(),
            talk_id,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        // Tick を一切送らずに spawn_talk を呼ぶ（時間軸駆動を要求しない・R1.4）。
        let handle = spawn_talk(start, done_tx, surface, text);

        // 空 sheet 経路で TalkDone が即座に到達すること（Tick 不要）。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("無視タグのみの script も空 sheet 経路で即座に TalkDone を返すべき");

        // talk_id エコー（R1.3）と終端理由（末尾到達＝Ended・R1.4／Quit ではない）。
        assert_eq!(done.talk_id, talk_id, "talk_id がエコーされること（R1.3）");
        assert_eq!(
            done.reason,
            TalkEndReason::Ended,
            "終端命令のない無視タグのみ script は末尾到達で Ended（R1.4）"
        );

        // 発火は 1 件も無いこと（無視タグは cue を生成しない＝両 sink 空）。
        assert!(
            surface_records.lock().unwrap().is_empty(),
            "無視タグのみでは surface 発火が無いこと"
        );
        assert!(
            text_records.lock().unwrap().is_empty(),
            "無視タグのみでは text 発火が無いこと"
        );

        // join でスレッド終了を同期（Break 後にスレッドが正常終了していること）。
        handle.actor.join().expect("body は正常終了する");
    }

    /// 発火を伴わない quit 相当のみ（先行 cue のない `\-`）の script は空 sheet かつ
    /// `end=Quit` へコンパイルされ、時間軸駆動（Tick）を要さずに **`Quit`（`Ended` ではない）**
    /// を伴う `TalkDone` を即座に返す（R6.2）。
    ///
    /// これは空 sheet 経路の**弁別テスト**である: 駆動器が空 sheet で `Ended` を固定送出して
    /// いれば FAIL する（compile.rs の `bare_quit_yields_empty_sheet_with_quit_end` が示すとおり
    /// `\-` は空 sheet＋`end=Quit`）。無視タグ `\q[..]` を前置しても Choice は cue を生成せず、
    /// `\-` が Quit を確定するため sheet は空・end=Quit のまま。駆動器が `compiled.end` を
    /// 伝播していることを、`reason == Quit` の観測で固定する。
    #[test]
    fn quit_only_script_ends_immediately_with_quit_not_ended() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(56);
        // 先行 cue のない `\-`（無視タグ `\q[..]` を前置しても発火は 0 件）。
        // ⇒ 空 sheet かつ end=Quit。空 sheet 経路で Quit がそのまま伝播すること（Ended 固定でない）。
        let start = StartTalk {
            script: r"\q[やめる,OnCancel]\-".to_string(),
            talk_id,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        // Tick を一切送らずに spawn_talk を呼ぶ（空 sheet ゆえ時間軸駆動不要）。
        let handle = spawn_talk(start, done_tx, surface, text);

        // 空 sheet 経路で TalkDone が即座に到達すること（Tick 不要）。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("quit 相当のみの script も空 sheet 経路で即座に TalkDone を返すべき");

        // talk_id エコー（R1.3）。弁別の核心: 終端理由は Quit であり Ended ではない（R6.2）。
        // 駆動器が空 sheet 経路で Ended を固定送出していれば、この assert が FAIL する。
        assert_eq!(done.talk_id, talk_id, "talk_id がエコーされること（R1.3）");
        assert_eq!(
            done.reason,
            TalkEndReason::Quit,
            "先行 cue のない `\\-` は空 sheet＋Quit＝Ended を固定送出してはならない（R6.2）"
        );
        assert_ne!(
            done.reason,
            TalkEndReason::Ended,
            "空 sheet 経路で Ended を固定送出していないことの明示（R6.2 の弁別）"
        );

        // 発火は 1 件も無いこと（両 sink 空）。
        assert!(
            surface_records.lock().unwrap().is_empty(),
            "quit 相当のみでは surface 発火が無いこと"
        );
        assert!(
            text_records.lock().unwrap().is_empty(),
            "quit 相当のみでは text 発火が無いこと"
        );

        // join でスレッド終了を同期（Break 後にスレッドが正常終了していること）。
        handle.actor.join().expect("body は正常終了する");
    }

    /// fixture 駆動の統合テスト（task 5.2 の主 observable・R9.3）。
    ///
    /// `\s[10]hello\w[2]world\e`（サーフェス切替＋テキスト＋待ち＋テキスト＋終端）を
    /// script 直入力し、注入 `Tick` 列（0.0 → 待ち跨ぎの 0.2）で駆動する。surface mock には
    /// `Emote{key:"10"}`（at=0.0）が、text mock には `Text("hello")`（at=0.0）と
    /// `Text("world")`（at=0.1・`\w[2]`＝100ms 反映）が **at 昇順・FIFO** で届き、
    /// 最後に `TalkDone{Ended}`（talk_id エコー・R6.6）が返ること、を単一 pass で確認する。
    #[test]
    fn fixture_script_drives_two_sinks_and_returns_ended() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(42);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, done_tx, surface, text);

        // 期待発火時刻（`\w[2]`＝100ms＝Duration::from_millis(100).as_secs_f64()）。
        // リテラル直書きの表現誤差を避けるため同一計算で導出する。
        let at_hello = 0.0_f64;
        let at_world = Duration::from_millis(100).as_secs_f64();

        // 注入 Tick 列: 0.0（先頭群 hello/surface を発火）→ 待ち跨ぎ 0.2（world を発火＋末尾到達）。
        handle
            .inbox
            .send(SakuraMsg::Tick(0.0))
            .expect("Tick(0.0) 投函");
        handle
            .inbox
            .send(SakuraMsg::Tick(0.2))
            .expect("Tick(0.2) 投函");

        // 自然終端で TalkDone{Ended} が返ること（talk_id エコー・R6.6）。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("自然終端で TalkDone が返るべき");
        assert_eq!(done.talk_id, talk_id, "talk_id がエコーされること（R6.6）");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");

        // body の正常終了を同期してから観測（全発火が確定済みになる）。
        handle.actor.join().expect("body は正常終了する");

        // surface 系: Emote{key:"10"}（at=0.0・actor="0"）1 件のみ。
        let surface = surface_records.lock().unwrap();
        assert_eq!(surface.len(), 1, "surface 発火は 1 件（サーフェス切替）");
        assert_eq!(surface[0].at, at_hello, "surface 発火時刻は 0.0");
        assert_eq!(surface[0].actor.as_str(), "0", "既定 scope=0 の転写");
        assert_eq!(
            surface[0].command,
            CueCommand::Emote { key: "10".into() },
            "surface 発火は Emote{{key:10}}"
        );

        // text 系: Text("hello")（at=0.0）→ Text("world")（at=0.1）の 2 件が at 昇順・FIFO。
        let text = text_records.lock().unwrap();
        assert_eq!(text.len(), 2, "text 発火は 2 件（hello/world）");
        assert_eq!(text[0].at, at_hello, "hello の発火時刻は 0.0");
        assert_eq!(
            text[0].command,
            CueCommand::Text("hello".into()),
            "先頭 text は hello"
        );
        assert_eq!(text[1].at, at_world, "world の発火時刻は \\w[2]＝100ms");
        assert_eq!(
            text[1].command,
            CueCommand::Text("world".into()),
            "後続 text は world"
        );
        // at 昇順（FIFO・R4.1/R9.2）。
        assert!(text[0].at <= text[1].at, "text 発火は at 昇順");
    }

    /// 冪等/逆行 `Tick` で二重発火しない（設計クリティカルな二重発火ガードの固定・R11.x）。
    ///
    /// 先頭 cue を発火させた後、同値・逆行 `Tick` を送っても surface/text の発火数が
    /// 増えないこと（`TimedSchedule::tick` の冪等 early-return ＋単調ガードの合わせ技）を確認する。
    #[test]
    fn duplicate_and_backward_tick_do_not_double_fire() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(1);
        // 末尾に長めの待ちを残し、0.0/0.1 の Tick 群だけでは終端しない script にする。
        let start = StartTalk {
            script: r"\s[10]hello\w[10]world\e".to_string(),
            talk_id,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, done_tx, surface, text);

        // 同値・逆行 Tick を織り交ぜて先頭群（at=0.0）を発火させる。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap(); // 同値 → no-op
        handle.inbox.send(SakuraMsg::Tick(-1.0)).unwrap(); // 逆行 → no-op
        handle.inbox.send(SakuraMsg::Tick(0.1)).unwrap(); // 前進だが world(at=0.5) 未達

        // 終端まで進めて body を閉じる（\w[10]=500ms を跨ぐ）。
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("終端で TalkDone");
        assert_eq!(done.reason, TalkEndReason::Ended);
        handle.actor.join().expect("body は正常終了する");

        // 二重発火していないこと: surface=1（Emote 1 回）・text=2（hello/world 各 1 回）。
        assert_eq!(
            surface_records.lock().unwrap().len(),
            1,
            "surface が二重発火していないこと"
        );
        assert_eq!(
            text_records.lock().unwrap().len(),
            2,
            "text が二重発火していないこと（hello/world 各 1 回）"
        );
    }

    /// 非有限 `Tick`（`NaN`/`inf`）は無視され再生が破綻しない（R11.1/11.2）。
    ///
    /// dola の NaN 全量配信ハザードを遮断するガードの固定。非有限 Tick を送っても
    /// 発火は起きず（早期全量配信されない）、その後の正常 Tick で通常どおり終端する。
    #[test]
    fn non_finite_tick_is_ignored_and_playback_survives() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(9);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, done_tx, surface, text);

        // 非有限 Tick を先に送る: 無視され（error ログ＋SakuraError 記録）発火 0 件のはず。
        handle.inbox.send(SakuraMsg::Tick(f64::NAN)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(f64::INFINITY)).unwrap();

        // 正常 Tick 列で通常どおり駆動・終端する（ガードがループを殺していないことの証）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.2)).unwrap();

        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("非有限 Tick 後も正常 Tick で終端するべき");
        assert_eq!(done.reason, TalkEndReason::Ended, "再生は破綻せず Ended");
        handle.actor.join().expect("body は正常終了する");

        // 全発火が非有限 Tick で早期全量配信されず、正常 Tick 分だけ届いていること。
        assert_eq!(
            surface_records.lock().unwrap().len(),
            1,
            "非有限 Tick で surface が早期全量配信されていないこと"
        );
        assert_eq!(
            text_records.lock().unwrap().len(),
            2,
            "非有限 Tick で text が早期全量配信されていないこと"
        );
    }

    /// 再生途中の中断（Close）で `TalkDone{Interrupted}` がちょうど 1 回返り、
    /// 未発火分が sink に届かないこと（R7.1/7.2/7.3/7.4・R6.4）。
    ///
    /// `\s[10]hello\w[10]world\e`（world は `\w[10]`＝500ms 後）を先頭群（at=0.0）だけ
    /// 発火させたところで Close を送る。Close 時点で world（at=0.5）は未発火＝以降 sink へ
    /// 届いてはならない。TalkDone{Interrupted} が talk_id エコー付きでちょうど 1 回返る。
    /// 「ちょうど 1 回」は driver 側の `Option<TalkState>::take()`（高々 1 回の唯一機構）が
    /// 保証する（2 度目の `on_close`/`on_tick` 到達時は `state` が既に `None`）＋ body が
    /// Break でスレッド終了することを join で確認して観測する（R6.4/R7.5）。
    #[test]
    fn mid_playback_close_returns_interrupted_once_and_drops_unfired_cues() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(101);
        let start = StartTalk {
            script: r"\s[10]hello\w[10]world\e".to_string(),
            talk_id,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, done_tx, surface, text);

        // 先頭群（at=0.0）だけ発火させる（world は at=0.5・未達）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).expect("Tick(0.0) 投函");

        // 中断（Close）を送る。進行中の再生を即時停止し Interrupted ACK を返すべき。
        handle.inbox.send(SakuraMsg::Close).expect("Close 投函");

        // TalkDone{Interrupted} がちょうど 1 回返ること（talk_id エコー・R7.4/R6.4）。
        // recv は self を consume するため二度目の受領は型で不能＝「ちょうど 1 回」を保証。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("中断で TalkDone{Interrupted} が返るべき");
        assert_eq!(done.talk_id, talk_id, "talk_id がエコーされること（R6.6）");
        assert_eq!(
            done.reason,
            TalkEndReason::Interrupted,
            "中断の終端理由は Interrupted（R7.4）"
        );

        // body の正常終了（Break）を join で確認する。以降 Close を送っても再返信は不能。
        handle.actor.join().expect("body は Break 後に正常終了する");

        // 先頭群のみ届き、未発火分（world）は sink に届いていないこと（R7.2）。
        let surface = surface_records.lock().unwrap();
        assert_eq!(surface.len(), 1, "surface は先頭 Emote のみ（中断前）");
        assert_eq!(
            surface[0].command,
            CueCommand::Emote { key: "10".into() },
            "surface 発火は Emote{{key:10}}"
        );
        let text = text_records.lock().unwrap();
        assert_eq!(text.len(), 1, "text は hello のみ（world は未発火＝破棄）");
        assert_eq!(
            text[0].command,
            CueCommand::Text("hello".into()),
            "中断前に届いた text は hello のみ"
        );
    }

    /// 自然終端後に中断（Close）を受けても追加の `TalkDone` が発生しないこと（R6.4/R7.5）。
    ///
    /// `\s[10]hello\w[2]world\e` を自然終端まで駆動し `TalkDone{Ended}` を受領・body を
    /// join（アクタースレッド終了）した後に inbox へ Close を送る。アクターは既に消えて
    /// いるため `inbox.send(Close)` は `Err`（送出先消滅）になる＝それが「二重終端しない」
    /// 構造的保証（Break でスレッドが消え、`state` も既に `take()` 済み）の証。
    #[test]
    fn close_after_natural_end_produces_no_extra_talkdone() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
        let talk_id = TalkId(102);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
        };

        let surface = MockSink::new();
        let text = MockSink::new();

        let handle = spawn_talk(start, done_tx, surface, text);

        // 自然終端まで駆動する（0.0 → 待ち跨ぎ 0.2）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).expect("Tick(0.0) 投函");
        handle.inbox.send(SakuraMsg::Tick(0.2)).expect("Tick(0.2) 投函");

        // 自然終端で TalkDone{Ended} が返ること（recv が done_rx を consume）。
        let done = done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("自然終端で TalkDone{Ended} が返るべき");
        assert_eq!(done.talk_id, talk_id, "talk_id エコー");
        assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");

        // body の終了を同期する（Break でアクタースレッドが消える）。
        handle.actor.join().expect("body は自然終端後に正常終了する");

        // 自然終端後の Close: アクターは既に消えているため inbox.send は Err になる。
        // これが「二重終端しない」構造的保証の証＝Close は処理されず追加 TalkDone は不能。
        let send_result = handle.inbox.send(SakuraMsg::Close);
        assert!(
            send_result.is_err(),
            "自然終端後はアクターが消えており Close 送出は失敗する（二重終端不能の証）"
        );
    }

    /// 複数 talk を異なる相関 ID・独立 sink で同時駆動し、各 talk の `TalkDone` に**起動時と
    /// 同一**の `talk_id` が対応付けられ、出力が talk 間で混線しないことを確認する（R1.3/R6.6）。
    ///
    /// 2 本の talk（`TalkId(7)`＝Ended 経路の `\s[10]hello\w[2]world\e`、`TalkId(42)`＝
    /// Quit 経路の `\s[20]bye\w[2]done\-`）を**別々の done channel と別々の mock sink 対**で
    /// 起動し、それぞれ独立の注入 Tick 列で終端まで駆動する。契約上 `TalkCue` は talk_id を
    /// 帯びない（各 talk が自前 sink を持つ per-actor 構成ゆえ）ので、相関の観測は
    /// **出力側 = `TalkDone.talk_id` が `StartTalk.talk_id` に一致**と**各 talk の cue が自分の
    /// sink にのみ届く**の 2 点で行う。id・script・終端理由・cue 内容を talk 間で相違させ、
    /// 万一の取り違えが検出可能な形にする。
    #[test]
    fn multiple_talks_echo_own_talk_id_without_cross_talk_mixing() {
        // talk A: TalkId(7)・Ended 経路。
        let (done_a_tx, done_a_rx) = std::sync::mpsc::channel::<TalkDone>();
        let id_a = TalkId(7);
        let start_a = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: id_a,
        };
        let surface_a = MockSink::new();
        let text_a = MockSink::new();
        let surface_a_records = surface_a.records();
        let text_a_records = text_a.records();

        // talk B: TalkId(42)・Quit 経路（末尾 `\-`）・異なる surface/text 内容。
        let (done_b_tx, done_b_rx) = std::sync::mpsc::channel::<TalkDone>();
        let id_b = TalkId(42);
        let start_b = StartTalk {
            script: r"\s[20]bye\w[2]done\-".to_string(),
            talk_id: id_b,
        };
        let surface_b = MockSink::new();
        let text_b = MockSink::new();
        let surface_b_records = surface_b.records();
        let text_b_records = text_b.records();

        // 2 talk を並行起動（それぞれ独立スレッド・独立 sink 対）。
        let handle_a = spawn_talk(start_a, done_a_tx, surface_a, text_a);
        let handle_b = spawn_talk(start_b, done_b_tx, surface_b, text_b);

        // 各 talk を自分の注入 Tick 列で終端まで駆動する。
        handle_a.inbox.send(SakuraMsg::Tick(0.0)).expect("A Tick(0.0)");
        handle_a.inbox.send(SakuraMsg::Tick(0.2)).expect("A Tick(0.2)");
        handle_b.inbox.send(SakuraMsg::Tick(0.0)).expect("B Tick(0.0)");
        handle_b.inbox.send(SakuraMsg::Tick(0.2)).expect("B Tick(0.2)");

        // 各 done channel には自分の talk_id を帯びた TalkDone がちょうど返る。
        let done_a = done_a_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("talk A は TalkDone を返すべき");
        let done_b = done_b_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("talk B は TalkDone を返すべき");

        // talk_id エコーの相関（R1.3/R6.6）: 各 TalkDone は自分の起動 id を帯びる（取り違えなし）。
        assert_eq!(done_a.talk_id, id_a, "talk A の TalkDone は id_a をエコー");
        assert_eq!(done_b.talk_id, id_b, "talk B の TalkDone は id_b をエコー");
        assert_ne!(
            done_a.talk_id, done_b.talk_id,
            "2 talk の TalkDone は相異なる id（混線していない）"
        );
        // 終端理由も talk 固有（A=Ended・B=Quit）＝script 相違が正しく各 talk に反映される。
        assert_eq!(done_a.reason, TalkEndReason::Ended, "A の `\\e` は Ended");
        assert_eq!(done_b.reason, TalkEndReason::Quit, "B の `\\-` は Quit");

        handle_a.actor.join().expect("A body 正常終了");
        handle_b.actor.join().expect("B body 正常終了");

        // 出力側の相関: 各 talk の cue は自分の sink にのみ届く（talk 間で内容が混ざらない）。
        let surface_a = surface_a_records.lock().unwrap();
        let text_a = text_a_records.lock().unwrap();
        let surface_b = surface_b_records.lock().unwrap();
        let text_b = text_b_records.lock().unwrap();

        // talk A の sink: Emote{key:"10"} 1 件＋Text("hello")/Text("world")。
        assert_eq!(surface_a.len(), 1, "A surface は 1 件");
        assert_eq!(
            surface_a[0].command,
            CueCommand::Emote { key: "10".into() },
            "A surface は Emote{{key:10}}"
        );
        assert_eq!(
            text_a.iter().map(|c| c.command.clone()).collect::<Vec<_>>(),
            vec![
                CueCommand::Text("hello".into()),
                CueCommand::Text("world".into()),
            ],
            "A text は hello/world"
        );

        // talk B の sink: Emote{key:"20"} 1 件＋Text("bye")/Text("done")。
        assert_eq!(surface_b.len(), 1, "B surface は 1 件");
        assert_eq!(
            surface_b[0].command,
            CueCommand::Emote { key: "20".into() },
            "B surface は Emote{{key:20}}"
        );
        assert_eq!(
            text_b.iter().map(|c| c.command.clone()).collect::<Vec<_>>(),
            vec![
                CueCommand::Text("bye".into()),
                CueCommand::Text("done".into()),
            ],
            "B text は bye/done"
        );

        // 相互排他の明示: A の内容が B の sink に、B の内容が A の sink に混入していないこと。
        assert!(
            !text_a.iter().any(|c| matches!(&c.command, CueCommand::Text(s) if s.as_str() == "bye" || s.as_str() == "done")),
            "A text sink に B の cue が混入していないこと"
        );
        assert!(
            !text_b.iter().any(|c| matches!(&c.command, CueCommand::Text(s) if s.as_str() == "hello" || s.as_str() == "world")),
            "B text sink に A の cue が混入していないこと"
        );
    }

    /// 同一 fixture script＋同一注入 Tick 列を N 回実行し、毎回**同一の観測結果**（surface cue 列・
    /// text cue 列・発火時刻・actor key・順序・終端理由）が得られることを確認する（R9.4・決定的再現）。
    ///
    /// 各 run で `spawn_talk`＋mock を作り直し（新スレッド・新 sink）、同一 script
    /// `\s[10]hello\w[2]world\e` を同一 Tick 列（0.0 → 0.2）で駆動する。run ごとに
    /// (surface cue 列, text cue 列, TalkDone.reason) を取り出し、run 0 を基準に全 run が
    /// **完全一致**することを assert する。`at` は f64 完全一致で比較する（compile は決定的な
    /// IEEE-754 累算ゆえ run 間で bit 単位に一致するべき）。時刻注入は Tick のみ（sleep/Instant なし）。
    #[test]
    fn same_fixture_and_tick_sequence_produces_identical_observation_each_run() {
        // 1 run 分の観測を (surface, text, reason) のタプルで取り出すヘルパ。
        // TalkCue は PartialEq 導出前提でないため、比較キーを (at, actor, command) の Vec に射影する。
        type CueKey = (f64, String, CueCommand);
        fn project(records: &std::sync::Arc<std::sync::Mutex<Vec<TalkCue>>>) -> Vec<CueKey> {
            records
                .lock()
                .unwrap()
                .iter()
                .map(|c| (c.at, c.actor.as_str().to_string(), c.command.clone()))
                .collect()
        }

        fn run_once() -> (Vec<CueKey>, Vec<CueKey>, TalkEndReason) {
            let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
            let start = StartTalk {
                script: r"\s[10]hello\w[2]world\e".to_string(),
                talk_id: TalkId(7),
            };
            let surface = MockSink::new();
            let text = MockSink::new();
            let surface_records = surface.records();
            let text_records = text.records();

            let handle = spawn_talk(start, done_tx, surface, text);
            // 同一注入 Tick 列（sleep なし・決定的）。
            handle.inbox.send(SakuraMsg::Tick(0.0)).expect("Tick(0.0)");
            handle.inbox.send(SakuraMsg::Tick(0.2)).expect("Tick(0.2)");

            let done = done_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("自然終端で TalkDone");
            handle.actor.join().expect("body 正常終了");

            (project(&surface_records), project(&text_records), done.reason)
        }

        // N 回実行し、run 0 を基準に全 run が完全一致することを確認する。
        const RUNS: usize = 3;
        let baseline = run_once();
        // baseline 自体が期待形（surface=Emote 1 件・text=hello/world・Ended）であることを固定する。
        assert_eq!(baseline.0.len(), 1, "baseline surface は 1 件");
        assert_eq!(baseline.1.len(), 2, "baseline text は 2 件");
        assert_eq!(baseline.2, TalkEndReason::Ended, "baseline は Ended");

        for run in 1..RUNS {
            let observed = run_once();
            assert_eq!(
                observed, baseline,
                "run {run} の観測が baseline と完全一致すること（surface/text cue 列・at・actor・順序・終端理由が全て同一・R9.4）"
            );
        }
    }
}
