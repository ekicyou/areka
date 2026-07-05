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
//! 「`state.take()` → `reply.send(TalkDone)` → 直後に `Break`」の対で実装する
//! （`ReplySender::send(self)` の move-consume が唯一の高々 1 回機構・終端フラグを持たない）。

use std::ops::ControlFlow;

use crate::compile::compile;
use crate::contract::{
    cue_target_of, CuePayload, CueSheet, CueTarget, SakuraMsg, StartTalk, TalkCue, TalkDone,
    TalkEndReason, TalkHandle, TalkId,
};
use crate::error::SakuraError;
use crate::sink::{SurfaceSink, TextSink};
use areka_actor::{run_inbox, spawn_actor, ReplySender};
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
/// # Preconditions
///
/// `start.reply` は生存する `ReplyReceiver` と対（kanade or テスト）。
///
/// # Postconditions
///
/// 終端・中断のいずれでも `TalkDone` を高々 1 回返し body 復帰（スレッド終了）。
pub fn spawn_talk(
    start: StartTalk,
    surface_sink: impl SurfaceSink + Send + 'static,
    text_sink: impl TextSink + Send + 'static,
) -> TalkHandle {
    let talk_id = start.talk_id;
    let name = format!("sakura-talk-{}", talk_id.0);

    let (inbox, actor) = spawn_actor::<SakuraMsg, _>(&name, move |rx| {
        let mut driver = TalkDriver::new(surface_sink, text_sink);
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
/// `reply` を move-consume する（高々 1 回の唯一機構）。`schedule`/`last_tick` は
/// 後続 task 5.2（Tick 駆動ループ）が消費する駆動状態。
// 非空 sheet 経路で確定した駆動状態は、Tick 駆動（task 5.2）と Close 中断 ACK（task 5.3）が
// 消費する。本 task では格納までを行うため全フィールドを dead_code 許容とする。
#[allow(dead_code)]
struct TalkState {
    /// talk 相関 ID（全出力へ対応付け・R1.3/R6.6）。
    talk_id: TalkId,
    /// `TalkDone` の返信端（move-consume が唯一の高々 1 回機構）。
    reply: ReplySender<TalkDone>,
    /// 時刻駆動可能形（0 起点・`TimedSchedule::new(0.0)`）。task 5.2 が駆動する。
    schedule: TimedSchedule<TalkCue>,
    /// コンパイル時点で確定した終端理由（自然終端で返す reason）。
    end: TalkEndReason,
    /// 直前に処理した `Tick` の時刻（単調・冪等ガード用）。task 5.2 が更新する。
    last_tick: Option<f64>,
}

/// per-talk 駆動アクター本体。`Option<TalkState>` スロットを保持し、`Start` で状態を確定、
/// 空 sheet なら即終端する。非空 sheet の Tick 駆動と Close 中断は後続 task（5.2/5.3）。
struct TalkDriver<S: SurfaceSink, T: TextSink> {
    state: Option<TalkState>,
    surface_sink: S,
    text_sink: T,
}

impl<S: SurfaceSink, T: TextSink> TalkDriver<S, T> {
    fn new(surface_sink: S, text_sink: T) -> Self {
        Self {
            state: None,
            surface_sink,
            text_sink,
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

        let StartTalk {
            script,
            talk_id,
            reply,
        } = start;

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
            if reply.send(done).is_err() {
                tracing::error!(talk_id = talk_id.0, "TalkDone reply receiver dropped");
            }
            return ControlFlow::Break(());
        }

        // 非空 sheet: 駆動可能形へ変換し状態を確定して継続（Tick 駆動は task 5.2）。
        let schedule = to_schedule(&compiled.sheet);
        self.state = Some(TalkState {
            talk_id,
            reply,
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
        // 高々 1 回機構: state.take() → reply.send(TalkDone) → Break。
        if state.schedule.is_completed() {
            let state = self.state.take().expect("state was Some above");
            let done = TalkDone {
                talk_id: state.talk_id,
                reason: state.end,
            };
            if state.reply.send(done).is_err() {
                tracing::error!(talk_id = state.talk_id.0, "TalkDone reply receiver dropped");
            }
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    /// `Close` 受領: 進行中の再生を即時停止し、中断 ACK を返す（R7.1/7.2/7.3/7.4）。
    ///
    /// 高々 1 回機構（validation Issue 3）に従い、保持状態を `take()` して `reply` を
    /// move-consume し `TalkDone{Interrupted}` を送出、直後に `Break` する。`take()` した
    /// `schedule` は本メソッド終了時に drop され、未発火の残り cue は sink へ届かない
    /// （drain せず破棄＝R7.2）。`Break` は `run_inbox` の即時 return＝積み残し（inbox の
    /// 未処理メッセージ）は rx drop で破棄される（R7.3・停止規約整合）。
    ///
    /// 状態が既に `None`（自然終端は先に `take()`＋`Break` 済みのため通常は非到達＝
    /// `Start` 前の防御枝）なら、二度目の `TalkDone` を送らずログして `Break` する
    /// （通算高々 1 回・R6.4/R7.5）。自然終端後はスレッドが既に消えており Close 自体が
    /// inbox へ届かないため、この分岐は主に `Start` 受領前の防御的経路である。
    fn on_close(&mut self) -> ControlFlow<()> {
        // 高々 1 回機構: state.take() → reply.send(TalkDone{Interrupted}) → Break。
        // take() された schedule の未発火 cue はここで drop＝sink へ届かない（R7.2）。
        if let Some(state) = self.state.take() {
            let done = TalkDone {
                talk_id: state.talk_id,
                reason: TalkEndReason::Interrupted,
            };
            // reply 受領側（kanade）が既に drop されていれば error ログ（黙殺しない・R11.1/11.4）。
            if state.reply.send(done).is_err() {
                tracing::error!(
                    talk_id = state.talk_id.0,
                    "TalkDone reply receiver dropped on Close"
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
    use areka_actor::reply_channel;
    use std::time::Duration;

    /// 空発火列（空 script）の talk は時間軸駆動せず、Tick を一切送らなくても
    /// コンパイル結果の終端理由（空 script＝`Ended`）を伴う `TalkDone` を**即座に**返す
    /// （observable・R1.4）。`talk_id` は起動要求のものがエコーされる（R1.3）。
    #[test]
    fn empty_script_talk_returns_talkdone_immediately_without_tick() {
        let (reply_tx, reply_rx) = reply_channel::<TalkDone>();
        let talk_id = TalkId(7);
        let start = StartTalk {
            script: String::new(), // 空 script → 空 Instruction 列 → 空 sheet。
            talk_id,
            reply: reply_tx,
        };

        // 2 本の mock sink（surface 用・text 用）。空 sheet ゆえ発火は 1 件も無い。
        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        // Tick を一切送らずに spawn_talk を呼ぶ（時間軸駆動を要求しない）。
        let handle = spawn_talk(start, surface, text);

        // TalkDone が即座に到達すること（Tick 不要・時間軸駆動なし）。
        let done = reply_rx
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

    /// fixture 駆動の統合テスト（task 5.2 の主 observable・R9.3）。
    ///
    /// `\s[10]hello\w[2]world\e`（サーフェス切替＋テキスト＋待ち＋テキスト＋終端）を
    /// script 直入力し、注入 `Tick` 列（0.0 → 待ち跨ぎの 0.2）で駆動する。surface mock には
    /// `Emote{key:"10"}`（at=0.0）が、text mock には `Text("hello")`（at=0.0）と
    /// `Text("world")`（at=0.1・`\w[2]`＝100ms 反映）が **at 昇順・FIFO** で届き、
    /// 最後に `TalkDone{Ended}`（talk_id エコー・R6.6）が返ること、を単一 pass で確認する。
    #[test]
    fn fixture_script_drives_two_sinks_and_returns_ended() {
        let (reply_tx, reply_rx) = reply_channel::<TalkDone>();
        let talk_id = TalkId(42);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
            reply: reply_tx,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, surface, text);

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
        let done = reply_rx
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
        let (reply_tx, reply_rx) = reply_channel::<TalkDone>();
        let talk_id = TalkId(1);
        // 末尾に長めの待ちを残し、0.0/0.1 の Tick 群だけでは終端しない script にする。
        let start = StartTalk {
            script: r"\s[10]hello\w[10]world\e".to_string(),
            talk_id,
            reply: reply_tx,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, surface, text);

        // 同値・逆行 Tick を織り交ぜて先頭群（at=0.0）を発火させる。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap(); // 同値 → no-op
        handle.inbox.send(SakuraMsg::Tick(-1.0)).unwrap(); // 逆行 → no-op
        handle.inbox.send(SakuraMsg::Tick(0.1)).unwrap(); // 前進だが world(at=0.5) 未達

        // 終端まで進めて body を閉じる（\w[10]=500ms を跨ぐ）。
        handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
        let done = reply_rx
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
        let (reply_tx, reply_rx) = reply_channel::<TalkDone>();
        let talk_id = TalkId(9);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
            reply: reply_tx,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, surface, text);

        // 非有限 Tick を先に送る: 無視され（error ログ＋SakuraError 記録）発火 0 件のはず。
        handle.inbox.send(SakuraMsg::Tick(f64::NAN)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(f64::INFINITY)).unwrap();

        // 正常 Tick 列で通常どおり駆動・終端する（ガードがループを殺していないことの証）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
        handle.inbox.send(SakuraMsg::Tick(0.2)).unwrap();

        let done = reply_rx
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
    /// 「ちょうど 1 回」は `ReplyReceiver`（oneshot・recv で consume）＋ `ReplySender::send`
    /// の move-consume が型で保証する（2 度目の送信は構造的に不能）＋ body が Break で
    /// スレッド終了することを join で確認して観測する（R6.4/R7.5）。
    #[test]
    fn mid_playback_close_returns_interrupted_once_and_drops_unfired_cues() {
        let (reply_tx, reply_rx) = reply_channel::<TalkDone>();
        let talk_id = TalkId(101);
        let start = StartTalk {
            script: r"\s[10]hello\w[10]world\e".to_string(),
            talk_id,
            reply: reply_tx,
        };

        let surface = MockSink::new();
        let text = MockSink::new();
        let surface_records = surface.records();
        let text_records = text.records();

        let handle = spawn_talk(start, surface, text);

        // 先頭群（at=0.0）だけ発火させる（world は at=0.5・未達）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).expect("Tick(0.0) 投函");

        // 中断（Close）を送る。進行中の再生を即時停止し Interrupted ACK を返すべき。
        handle.inbox.send(SakuraMsg::Close).expect("Close 投函");

        // TalkDone{Interrupted} がちょうど 1 回返ること（talk_id エコー・R7.4/R6.4）。
        // recv は self を consume するため二度目の受領は型で不能＝「ちょうど 1 回」を保証。
        let done = reply_rx
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
    /// 構造的保証（Break でスレッドが消え、reply も既に move-consume 済み）の証。
    #[test]
    fn close_after_natural_end_produces_no_extra_talkdone() {
        let (reply_tx, reply_rx) = reply_channel::<TalkDone>();
        let talk_id = TalkId(102);
        let start = StartTalk {
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id,
            reply: reply_tx,
        };

        let surface = MockSink::new();
        let text = MockSink::new();

        let handle = spawn_talk(start, surface, text);

        // 自然終端まで駆動する（0.0 → 待ち跨ぎ 0.2）。
        handle.inbox.send(SakuraMsg::Tick(0.0)).expect("Tick(0.0) 投函");
        handle.inbox.send(SakuraMsg::Tick(0.2)).expect("Tick(0.2) 投函");

        // 自然終端で TalkDone{Ended} が返ること（recv が reply_rx を consume）。
        let done = reply_rx
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
}
