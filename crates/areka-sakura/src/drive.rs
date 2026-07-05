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
    CuePayload, CueSheet, SakuraMsg, StartTalk, TalkCue, TalkDone, TalkEndReason, TalkHandle,
    TalkId,
};
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

    /// `Tick` 受領（本 task ではスケルトンのみ・完全な駆動ループは task 5.2）。
    ///
    /// body が panic しないことのみ保証する（状態未確定なら no-op）。実際の
    /// `schedule.tick` → `ready` 振り分け → 完了判定は task 5.2 で実装する。
    fn on_tick(&mut self, _t: f64) -> ControlFlow<()> {
        // task 5.2 でここに単調・有限ガード＋schedule 駆動＋2 sink 振り分けを実装する。
        // 現状は sink フィールドの未使用警告を避けつつ非 panic を保証する no-op。
        let _ = (&mut self.surface_sink, &mut self.text_sink);
        ControlFlow::Continue(())
    }

    /// `Close` 受領（本 task ではスケルトンのみ・完全な中断 ACK は task 5.3）。
    ///
    /// 停止規約に従い即時 `Break` する（積み残しは rx drop で破棄）。`TalkDone{Interrupted}`
    /// の送出は task 5.3 で実装する。
    fn on_close(&mut self) -> ControlFlow<()> {
        // task 5.3 で state.take() → reply.send(TalkDone{Interrupted}) → Break を実装する。
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
    use crate::contract::TalkId;
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
}
