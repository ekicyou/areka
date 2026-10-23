// =============================================================================
// 選択肢つきのトークの時間切れの起点（areka-P0-balloon-text-bugs task 3.2・
// 要件 1.1〜1.9・1.13・4.1）
//
// 選択肢を出したトーク（`ChoiceShown` が届いたトーク）は、選択の待ちが解けて止まった時刻を
// 起点に測る。占有区間の終端（選択肢を出した所）とは比べない——比べると、選択の待ちの間に
// 待ち時間を使い切った形になり、待ちが解けた途端に消える。
//
//   ⑴ 待ちの間（トークの終わりが未着）は、時間がどれだけ進んでも隠さず知らせも作らない（1.1・1.9）
//   ⑵ 占有終端 2.0・止まった時刻 60.0 → 満了予定 90.0。選んだ・選択肢の時間切れ・中断の 3 つは
//      どれも同じ合図の形（止まった時刻つきの終わり）で届く＝名前つきの 3 行（1.2〜1.4・1.8）
//   ⑶ 終わりの後の滞在は保留し、離れた時刻から既定の長さで測り直す（1.5・1.6）
//   ⑷ 次の会話の開始で選択肢つきでないトークの規則へ戻る（1.7）
//   ⑸ このトークの待ち時間の指定（ミリ秒・なし）が選択肢つきのトークにも効く（1.8）
//   ⑹ 時刻なしの終わりは、計測が成り立つ最初のフレームの現在時刻が起点（記録の語）
//   ⑺ 箱にだけ文字が出ている scope も同じ（1.13）
//   ⑻ 受け口に cue を流して落とし、届いた合図の列を判断に通す通しの 1 本
// =============================================================================

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use areka_ghost::sink::BootCueSink;
use dola::cue::{ActorKey, CueCommand, TalkCue};

use super::test_support::*;
use super::*;
use crate::emo2_boot::talk_clock::TalkClock;
use crate::emo2_boot::talk_lifecycle::BalloonLifecycleSink;

/// タイムアウト時間（秒）。既定値の唯一の定義箇所から引く。
const TIMEOUT: f64 = DEFAULT_BALLOON_TIMEOUT_SECS;

/// 占有区間の終端（選択肢を出した所・talk 相対秒）。
const DISPLAY_END: f64 = 2.0;

/// 選択の待ちが解けて受け口が落ちた時刻（talk 相対秒）。
const STOPPED: f64 = 60.0;

/// 箱に文字が出ている scope にする。
fn boxed(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        box_showing: true,
        ..observation
    }
}

/// 選択肢つきのトークを始め、バルーンを出したところまで進めた状態を返す。
fn choice_talk_shown(scope: ScopeObservation) -> BalloonVisibilityState {
    let mut state = BalloonVisibilityState::default();
    let shown = Frame::new(&[(0, scope)])
        .at(0.0)
        .talk_started()
        .display_end(DISPLAY_END)
        .signal(TalkLifecycleSignal::ChoiceShown)
        .run(&mut state);
    assert!(
        state.talk_had_choice,
        "較正: 選択肢つきのトークになっている"
    );
    if !scope.box_showing {
        assert_eq!(
            shown.actions,
            vec![VisibilityAction::Show { scope: 0 }],
            "較正: 窓が出ている"
        );
    }
    state
}

/// 止まった時刻つきのトークの終わり。
fn ended(at: Option<f64>) -> TalkLifecycleSignal {
    TalkLifecycleSignal::TalkEnded { at }
}

fn started(origin: MeasurementOrigin, display_end: f64, deadline: f64) -> VisibilityLogEvent {
    VisibilityLogEvent::MeasurementStarted {
        origin,
        display_end,
        deadline,
    }
}

fn hide_timed_out(scopes: &[u32]) -> VisibilityAction {
    VisibilityAction::HideScopes {
        scopes: scopes.to_vec(),
        trigger: VisibilityTrigger::Timeout,
    }
}

/// ⑴ 選択を待つ間はトークの終わりが届かない＝時間がどれだけ進んでも隠さない。
#[test]
fn no_hide_while_the_choice_wait_is_alive_however_long() {
    let mut state = choice_talk_shown(seen(3, false));
    for now in [DISPLAY_END, DISPLAY_END + TIMEOUT, 1_000.0, 100_000.0] {
        let decision = Frame::new(&[(0, seen(3, true))]).at(now).run(&mut state);
        assert!(decision.actions.is_empty(), "{now}: {decision:?}");
        assert!(decision.logs.is_empty(), "{now}: {decision:?}");
        assert_eq!(decision.timeout_notice, None, "{now}: 知らせを作った");
        assert_eq!(state.deadline, None, "{now}: 計測が立った");
    }
}

/// ⑵ 選んだ・選択肢の時間切れ・中断のどれでも、止まった時刻 60.0 を起点に 90.0 で隠して
/// 知らせを返す。終わりを観測したフレームの遅れ（`observed_at`）は満了に効かない。
#[test]
fn choice_talk_expires_from_the_stop_time_in_every_way_the_wait_ends() {
    // (名前, 終わりを観測したフレームの現在時刻)
    let rows: [(&str, f64); 3] = [
        ("選んだ（応答は新しい台詞を始めない）", STOPPED),
        (
            "選択肢の時間切れ（OnChoiceTimeout の応答は台詞を始めない）",
            STOPPED + 1.0,
        ),
        ("中断（台本の中断・割り込み）", STOPPED + 15.0),
    ];
    let mut failures = Vec::new();
    for (name, observed_at) in rows {
        let mut state = choice_talk_shown(seen(3, false));
        let start = Frame::new(&[(0, seen(3, true))])
            .at(observed_at)
            .signal(ended(Some(STOPPED)))
            .run(&mut state);
        let expected_start = vec![started(
            MeasurementOrigin::ChoiceWaitEndedAt(STOPPED),
            DISPLAY_END,
            STOPPED + TIMEOUT,
        )];
        if start.logs != expected_start || !start.actions.is_empty() {
            failures.push(format!("{name}: 計測の開始が {start:?}"));
        }

        let before = Frame::new(&[(0, seen(3, true))])
            .at(STOPPED + TIMEOUT - 0.25)
            .run(&mut state);
        if !before.actions.is_empty() || before.timeout_notice.is_some() {
            failures.push(format!("{name}: 90.0 の直前で {before:?}"));
        }

        let expired = Frame::new(&[(0, seen(3, true))])
            .at(STOPPED + TIMEOUT)
            .run(&mut state);
        if expired.actions != vec![hide_timed_out(&[0])] || expired.timeout_notice != Some(TALK_ID)
        {
            failures.push(format!("{name}: 90.0 で {expired:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// ⑶ 終わりの後の満了で滞在していれば保留し、離れた時刻から既定の長さで測り直す。
/// 抑止の種別は滞在だけ（選択肢の行が残っていることは数えない）。
#[test]
fn hover_holds_and_the_measurement_restarts_from_leaving() {
    let mut state = choice_talk_shown(seen(3, false));
    Frame::new(&[(0, seen(3, true))])
        .at(STOPPED)
        .signal(ended(Some(STOPPED)))
        .run(&mut state);

    let held = Frame::new(&[(0, hovered(seen(3, true)))])
        .at(STOPPED + TIMEOUT)
        .run(&mut state);
    assert!(held.actions.is_empty(), "{held:?}");
    assert_eq!(
        held.logs,
        vec![VisibilityLogEvent::TimeoutSuppressed {
            kinds: SuppressionKinds {
                hover: true,
                ..SuppressionKinds::default()
            },
            deadline: STOPPED + TIMEOUT,
        }]
    );

    let left_at = STOPPED + TIMEOUT + 5.0;
    let left = Frame::new(&[(0, seen(3, true))])
        .at(left_at)
        .run(&mut state);
    assert_eq!(
        left.logs,
        vec![VisibilityLogEvent::MeasurementRestarted {
            now: left_at,
            deadline: left_at + TIMEOUT,
        }]
    );
    let expired = Frame::new(&[(0, seen(3, true))])
        .at(left_at + TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.timeout_notice, Some(TALK_ID));
}

/// ⑷ 選択の応答が新しい台詞を始めたら、そのトークは選択肢つきでない今の規則で測る
/// （占有終端と止まった時刻の早い方）。
#[test]
fn next_talk_start_reverts_to_the_ordinary_rule() {
    let mut state = choice_talk_shown(seen(3, false));
    let next = Frame::new(&[(0, seen(3, true))])
        .at(STOPPED)
        .signal(ended(Some(STOPPED)))
        .talk_started()
        .display_end(STOPPED + 5.0)
        .run(&mut state);
    assert!(!state.talk_had_choice, "次の開始で印が戻る");
    assert!(next.logs.is_empty(), "{next:?}");

    let reply_end = Frame::new(&[(0, seen(5, true))])
        .at(STOPPED + 20.0)
        .signal(ended(Some(STOPPED + 20.0)))
        .run(&mut state);
    assert_eq!(
        reply_end.logs,
        vec![started(
            MeasurementOrigin::DisplayEnd,
            STOPPED + 5.0,
            STOPPED + 5.0 + TIMEOUT,
        )]
    );
}

/// ⑸ `\![set,balloontimeout]` のミリ秒は選択肢つきのトークにも効き、「なし」は計測しない。
#[test]
fn per_talk_wait_setting_applies_to_choice_talks() {
    let mut millis = choice_talk_shown(seen(3, false));
    let start = Frame::new(&[(0, seen(3, true))])
        .at(STOPPED)
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Millis(
            5_000,
        )))
        .signal(ended(Some(STOPPED)))
        .run(&mut millis);
    assert_eq!(
        start.logs,
        vec![started(
            MeasurementOrigin::ChoiceWaitEndedAt(STOPPED),
            DISPLAY_END,
            STOPPED + 5.0,
        )]
    );
    let expired = Frame::new(&[(0, seen(3, true))])
        .at(STOPPED + 5.0)
        .run(&mut millis);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);

    let mut never = choice_talk_shown(seen(3, false));
    for now in [STOPPED, STOPPED + TIMEOUT, 10_000.0] {
        let mut frame = Frame::new(&[(0, seen(3, true))]).at(now);
        if now == STOPPED {
            frame = frame
                .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Never))
                .signal(ended(Some(STOPPED)));
        }
        let decision = frame.run(&mut never);
        assert!(decision.actions.is_empty(), "{now}: {decision:?}");
        assert_eq!(never.deadline, None, "{now}: 「なし」なのに計測が立った");
    }
}

/// ⑹ 時刻なしの終わりは、終わりを畳んだ後で計測が成り立つ最初のフレームの現在時刻が起点。
/// 時刻の分からないフレームでは計測しない。
#[test]
fn timeless_end_starts_from_the_first_measurable_frame() {
    let mut state = choice_talk_shown(seen(3, false));
    let unknown_now = Frame::new(&[(0, seen(3, true))])
        .signal(ended(None))
        .run(&mut state);
    assert!(unknown_now.logs.is_empty(), "{unknown_now:?}");
    assert_eq!(state.deadline, None);

    let first = 70.0;
    let start = Frame::new(&[(0, seen(3, true))]).at(first).run(&mut state);
    assert_eq!(
        start.logs,
        vec![started(
            MeasurementOrigin::ChoiceWaitEndTimeMissing,
            DISPLAY_END,
            first + TIMEOUT,
        )]
    );
    assert_eq!(
        MeasurementOrigin::ChoiceWaitEndTimeMissing.as_str(),
        "choice_wait_end_time_missing"
    );
    assert_eq!(
        MeasurementOrigin::ChoiceWaitEndedAt(STOPPED).as_str(),
        "choice_wait_ended_at"
    );

    let expired = Frame::new(&[(0, seen(3, true))])
        .at(first + TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
}

/// ⑺ 箱にだけ文字が出ている scope でも、待ちの間は隠さず、止まった時刻から測る。
#[test]
fn box_only_scope_follows_the_same_rule() {
    let mut state = choice_talk_shown(boxed(seen(0, false)));
    let waiting = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(STOPPED - 1.0)
        .run(&mut state);
    assert!(waiting.actions.is_empty(), "{waiting:?}");

    let start = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(STOPPED)
        .signal(ended(Some(STOPPED)))
        .run(&mut state);
    assert_eq!(
        start.logs,
        vec![started(
            MeasurementOrigin::ChoiceWaitEndedAt(STOPPED),
            DISPLAY_END,
            STOPPED + TIMEOUT,
        )]
    );
    let before = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(STOPPED + TIMEOUT - 0.25)
        .run(&mut state);
    assert!(before.actions.is_empty(), "{before:?}");
    let expired = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(STOPPED + TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.timeout_notice, Some(TALK_ID));
}

/// ⑻ 通し: 受け口に cue（文字 → 選択肢）を流し、止まった時刻 60.0 で落とし、届いた合図の列を
/// そのまま判断に通すと 90.0 で隠れる。
#[test]
fn sink_signals_drive_the_decision_to_expire_from_the_stop_time() {
    let wall = Arc::new(Mutex::new(0.0_f64));
    let wall_reader = Arc::clone(&wall);
    let clock = TalkClock::new(Arc::new(move || *wall_reader.lock().expect("時計")));
    // 起点は文字の受け口が観測で決める（本番は `ClockedTextSink`）。壁時刻 0.0 の cue で 0 起点。
    clock.observe_cue(0.0);
    let (tx, rx) = channel();
    let registered = BalloonLifecycleSink::new(tx, clock);

    let mut sink: Box<dyn BootCueSink> = registered.clone_box();
    sink.begin_talk(TALK_ID, areka_sakura::contract::ScriptOrigin::GHOST);
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("どれ？".into()),
        duration: DISPLAY_END,
    });
    sink.emit(TalkCue {
        at: DISPLAY_END,
        actor: ActorKey::from("0"),
        command: CueCommand::Choice {
            id: "OnA".into(),
            text: "はい".into(),
            references: vec![],
        },
        duration: 0.0,
    });
    let during_wait: Vec<TalkLifecycleSignal> = rx.try_iter().collect();

    *wall.lock().expect("時計") = STOPPED;
    drop(sink);
    let at_end: Vec<TalkLifecycleSignal> = rx.try_iter().collect();
    assert_eq!(at_end, vec![ended(Some(STOPPED))], "較正: 落ちた時刻つき");

    let mut state = BalloonVisibilityState::default();
    let mut shown = Frame::new(&[(0, seen(3, false))]).at(DISPLAY_END);
    for signal in during_wait {
        shown = shown.signal(signal);
    }
    shown.run(&mut state);
    assert!(state.talk_had_choice, "較正: 選択肢の合図が届いている");

    let waiting = Frame::new(&[(0, seen(3, true))])
        .at(STOPPED - 1.0)
        .run(&mut state);
    assert!(waiting.actions.is_empty(), "待ちの間に隠した: {waiting:?}");

    let mut end_frame = Frame::new(&[(0, seen(3, true))]).at(STOPPED);
    for signal in at_end {
        end_frame = end_frame.signal(signal);
    }
    let start = end_frame.run(&mut state);
    assert_eq!(
        start.logs,
        vec![started(
            MeasurementOrigin::ChoiceWaitEndedAt(STOPPED),
            DISPLAY_END,
            STOPPED + TIMEOUT,
        )]
    );

    let before = Frame::new(&[(0, seen(3, true))])
        .at(STOPPED + TIMEOUT - 0.25)
        .run(&mut state);
    assert!(before.actions.is_empty(), "{before:?}");
    let expired = Frame::new(&[(0, seen(3, true))])
        .at(STOPPED + TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.timeout_notice, Some(TALK_ID));
}
