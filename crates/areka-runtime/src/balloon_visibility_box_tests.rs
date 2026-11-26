// =============================================================================
// 判断中核 `decide` の「箱を数に入れた中断と時間切れ」の決定論テスト
// =============================================================================
//
// 箱（シェル内バルーン）にだけ文字が出ている scope が、利用者の中断と時間切れの対象になること、
// 隠す発行が箱に届くかの振り分け（`hide_reaches_boxes`）を固定する。

use super::decision::hide_reaches_boxes;
use super::test_support::*;
use super::*;

/// 隠す発行が箱に届くのは、時間切れと、掛け金が掛かったままの利用者の中断だけ。文字が 0 に
/// 落ちたことによる非表示は窓だけ（箱のあるサーフェスへ切り替えた後も箱の文字は残す）。
#[test]
fn hide_reaches_boxes_on_timeout_and_on_a_latched_break_only() {
    for latch in [false, true] {
        assert!(!hide_reaches_boxes(VisibilityTrigger::Clear, latch));
        assert!(hide_reaches_boxes(VisibilityTrigger::Timeout, latch));
    }
    assert!(hide_reaches_boxes(VisibilityTrigger::UserBreak, true));
    assert!(!hide_reaches_boxes(VisibilityTrigger::UserBreak, false));
}

/// 利用者の中断は、窓が見えている scope に加えて箱だけに文字が出ている scope も対象にする。
/// どちらにも出ていない scope は載せない。
#[test]
fn user_break_targets_scopes_showing_only_boxes() {
    let entries = [
        (0, seen(3, true)),
        (1, boxed(seen(0, false))),
        (2, seen(0, false)),
    ];
    let mut state = BalloonVisibilityState::default();
    let broke = Frame::new(&entries)
        .signal(TalkLifecycleSignal::UserBreak)
        .run(&mut state);
    assert_eq!(
        broke.actions,
        vec![hide(&[0, 1], VisibilityTrigger::UserBreak)]
    );
    assert!(hide_reaches_boxes(
        VisibilityTrigger::UserBreak,
        state.break_latch
    ));

    // 対照: 箱に文字が出ていなければ、窓の見えている scope だけ。
    let mut state = BalloonVisibilityState::default();
    let plain = Frame::new(&[(0, seen(3, true)), (1, seen(0, false)), (2, seen(0, false))])
        .signal(TalkLifecycleSignal::UserBreak)
        .run(&mut state);
    assert_eq!(
        plain.actions,
        vec![hide(&[0], VisibilityTrigger::UserBreak)]
    );
}

/// 中断と次の台詞の始まりが同じ巡に届いたら、掛け金は解けているので箱へは届かない
/// （箱の文字は新しい台詞のもの）。窓の隠れている箱だけの scope は、隠すものが無いので載せない。
#[test]
fn a_break_released_in_the_same_frame_does_not_reach_boxes() {
    let mut state = BalloonVisibilityState::default();
    let decision = Frame::new(&[(1, boxed(seen(0, false)))])
        .signal(TalkLifecycleSignal::UserBreak)
        .signal(TalkLifecycleSignal::TalkStarted { talk_id: None })
        .run(&mut state);
    assert!(decision.actions.is_empty(), "{:?}", decision.actions);
    assert!(decision.logs.is_empty(), "{:?}", decision.logs);
    assert!(!hide_reaches_boxes(
        VisibilityTrigger::UserBreak,
        state.break_latch
    ));
}

/// 箱だけに文字が出ている scope でも、台詞の後の待ち時間を計り、満了で隠す。
#[test]
fn timeout_measures_and_hides_a_scope_showing_only_boxes() {
    let mut state = BalloonVisibilityState::default();
    let armed = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(ms(0))
        .talk_started()
        .ended_at(ms(0))
        .run(&mut state);
    assert!(armed.actions.is_empty(), "箱だけの scope で窓が出た");
    assert_eq!(
        armed.logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::DisplayEnd,
            display_end: ms(0),
            deadline: TIMEOUT,
        }]
    );

    let expired = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        expired.actions,
        vec![hide(&[0], VisibilityTrigger::Timeout)]
    );

    // 対照: 箱に文字が出ていなければ計測は立たず、何も隠さない。
    let mut state = BalloonVisibilityState::default();
    for now in [ms(0), TIMEOUT] {
        let plain = Frame::new(&[(0, seen(0, false))])
            .at(now)
            .talk_started()
            .ended_at(ms(0))
            .run(&mut state);
        assert!(plain.actions.is_empty());
        assert!(plain.logs.is_empty());
    }
}

/// 文字の出ている箱の上にポインタが居るあいだは、普通のバルーンと同じ規則で満了を保留する。
#[test]
fn hover_on_a_box_holds_the_expired_deadline() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, boxed(seen(0, false)))])
        .at(ms(0))
        .talk_started()
        .ended_at(ms(0))
        .run(&mut state);

    let held = Frame::new(&[(0, hovered(boxed(seen(0, false))))])
        .at(TIMEOUT)
        .run(&mut state);
    assert!(held.actions.is_empty(), "箱の上の滞在で満了が保留されない");
    assert_eq!(
        held.logs,
        vec![VisibilityLogEvent::TimeoutSuppressed {
            kinds: SuppressionKinds {
                hover: true,
                ..SuppressionKinds::default()
            },
            deadline: TIMEOUT,
        }]
    );
}

/// 窓の文字が 0 に落ちて窓を隠しても、箱に文字が出ていれば計測は続き、満了で隠す
/// （箱のあるサーフェスへ切り替えた形）。
#[test]
fn clearing_the_window_keeps_the_measurement_while_boxes_show() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(2, false))])
        .at(ms(0))
        .talk_started()
        .ended_at(ms(0))
        .run(&mut state);

    let cleared = Frame::new(&[(0, boxed(seen(0, true)))])
        .at(ms(2_000))
        .run(&mut state);
    assert_eq!(cleared.actions, vec![hide(&[0], VisibilityTrigger::Clear)]);
    assert_eq!(
        state.deadline,
        Some(TIMEOUT),
        "箱が出ているのに計測が捨てられた"
    );

    let expired = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        expired.actions,
        vec![hide(&[0], VisibilityTrigger::Timeout)]
    );
}

/// 掛け金の掛かった中断で箱も隠したなら、箱だけの scope は時間切れの数からも外れ、消す対象が
/// 無くなった計測は捨てる（隠した箱を満了で隠し直さない）。
#[test]
fn a_latched_break_takes_box_only_scopes_out_of_the_measurement() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, boxed(seen(0, false)))])
        .at(ms(0))
        .talk_started()
        .ended_at(ms(0))
        .run(&mut state);

    let broke = Frame::new(&[(0, boxed(seen(0, false)))])
        .signal(TalkLifecycleSignal::UserBreak)
        .at(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        broke.actions,
        vec![hide(&[0], VisibilityTrigger::UserBreak)]
    );
    assert_eq!(
        broke.logs.last(),
        Some(&VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::NoVisibleScope,
            deadline: TIMEOUT,
        })
    );
}

/// 中断と次の台詞の始まりが同じ巡に届いて掛け金が解けたなら、箱だけの scope は時間切れの数に
/// 残る（箱の文字は新しい台詞のもの）——新しい台詞の計測がその箱を対象に立つ。
#[test]
fn a_released_break_keeps_box_only_scopes_in_the_measurement() {
    let mut state = BalloonVisibilityState::default();
    let decision = Frame::new(&[(1, boxed(seen(0, false)))])
        .signal(TalkLifecycleSignal::UserBreak)
        .signal(TalkLifecycleSignal::TalkStarted { talk_id: None })
        .ended_at(ms(0))
        .at(ms(0))
        .run(&mut state);
    assert!(decision.actions.is_empty(), "{:?}", decision.actions);
    assert!(
        decision
            .logs
            .contains(&VisibilityLogEvent::MeasurementStarted {
                origin: MeasurementOrigin::DisplayEnd,
                display_end: ms(0),
                deadline: TIMEOUT,
            }),
        "掛け金の解けた中断で箱だけの scope が計測から外れた: {:?}",
        decision.logs
    );
}

/// 文字の観測が取れなかったフレームは計測に触れない。窓が隠れて箱にだけ文字が出ている scope は、
/// そのフレームに箱の観測も失うので、見えるものが無いと読むと測り直した満了予定を捨ててしまう。
#[test]
fn an_unobserved_frame_keeps_the_restarted_deadline_while_a_box_is_showing() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, boxed(seen(0, false)))])
        .ended_at(ms(0))
        .at(ms(0))
        .run(&mut state);
    Frame::new(&[(0, hovered(boxed(seen(0, false))))])
        .at(TIMEOUT + ms(5_000))
        .run(&mut state);
    Frame::new(&[(0, boxed(seen(0, false)))])
        .at(ms(50_000))
        .run(&mut state);

    let blind = Frame::new(&[(0, unobserved(false))])
        .at(ms(60_000))
        .run(&mut state);
    let next = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(ms(61_000))
        .run(&mut state);

    assert_eq!(
        (blind, next, state.deadline),
        (
            VisibilityDecision::default(),
            VisibilityDecision::default(),
            Some(ms(50_000) + TIMEOUT)
        ),
        "(観測の取れなかったフレームの判断, 次のフレームの判断, 満了予定)"
    );
}
