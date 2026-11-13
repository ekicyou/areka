// =============================================================================
// 判断中核 `decide` の「箱を数に入れた中断と時間切れ」の決定論テスト（areka-P0-shell-balloon 10.1）
// =============================================================================
//
// 箱（シェル内バルーン）にだけ文字が出ている scope が、利用者の中断と時間切れの対象になること、
// 隠す発行が箱に届くかの振り分け（`hide_reaches_boxes`）を固定する（要件 6.10・6.11）。
// 箱に文字が出ていない入力（`box_showing` が偽）の期待値は既存のテスト群が持つ。

use super::decision::hide_reaches_boxes;
use super::test_support::*;
use super::*;

/// タイムアウト時間（秒）。既定値の唯一の定義箇所から引く。
const TIMEOUT: f64 = DEFAULT_BALLOON_TIMEOUT_SECS;

/// 箱に文字が出ている scope にする。
fn boxed(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        box_showing: true,
        ..observation
    }
}

/// 表示の合図の線へ任意の信号を混ぜて 1 巡進める（並びは線の上の到着順）。
fn run_with_signals(
    state: &mut BalloonVisibilityState,
    entries: &[(u32, ScopeObservation)],
    signals: &[TalkLifecycleSignal],
    now: Option<f64>,
) -> VisibilityDecision {
    let mut obs = observations(entries);
    obs.lifecycle.extend_from_slice(signals);
    decide(state, &obs, now, TIMEOUT)
}

fn hide(scopes: &[u32], trigger: VisibilityTrigger) -> VisibilityAction {
    VisibilityAction::HideScopes {
        scopes: scopes.to_vec(),
        trigger,
    }
}

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
    let broke = run_with_signals(
        &mut state,
        &entries,
        &[TalkLifecycleSignal::UserBreak],
        None,
    );
    assert_eq!(
        broke.actions,
        vec![hide(&[0, 1], VisibilityTrigger::UserBreak)]
    );
    assert!(hide_reaches_boxes(
        VisibilityTrigger::UserBreak,
        state.break_latch
    ));

    // 対照: 箱に文字が出ていなければ、箱を足す前と同じく窓の見えている scope だけ。
    let mut state = BalloonVisibilityState::default();
    let plain = run_with_signals(
        &mut state,
        &[(0, seen(3, true)), (1, seen(0, false)), (2, seen(0, false))],
        &[TalkLifecycleSignal::UserBreak],
        None,
    );
    assert_eq!(
        plain.actions,
        vec![hide(&[0], VisibilityTrigger::UserBreak)]
    );
}

/// 中断と次の台詞の始まりが同じ巡に届いたら、掛け金は解けているので箱へは届かない
/// （箱の文字は新しい台詞のもの）。
#[test]
fn a_break_released_in_the_same_frame_does_not_reach_boxes() {
    let mut state = BalloonVisibilityState::default();
    let decision = run_with_signals(
        &mut state,
        &[(1, boxed(seen(0, false)))],
        &[
            TalkLifecycleSignal::UserBreak,
            TalkLifecycleSignal::TalkStarted { talk_id: None },
        ],
        None,
    );
    assert_eq!(
        decision.actions,
        vec![hide(&[1], VisibilityTrigger::UserBreak)]
    );
    assert!(!hide_reaches_boxes(
        VisibilityTrigger::UserBreak,
        state.break_latch
    ));
}

/// 箱だけに文字が出ている scope でも、台詞の後の待ち時間を計り、満了で隠す（要件 6.10）。
#[test]
fn timeout_measures_and_hides_a_scope_showing_only_boxes() {
    let mut state = BalloonVisibilityState::default();
    let armed = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(0.0)
        .talk_started()
        .ended_at(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(armed.actions.is_empty(), "箱だけの scope で窓が出た");
    assert_eq!(
        armed.logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::DisplayEnd,
            display_end: 0.0,
            deadline: TIMEOUT,
        }]
    );

    let expired = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        expired.actions,
        vec![hide(&[0], VisibilityTrigger::Timeout)]
    );

    // 対照: 箱に文字が出ていなければ計測は立たず、何も隠さない。
    let mut state = BalloonVisibilityState::default();
    for now in [0.0, TIMEOUT] {
        let plain = Frame::new(&[(0, seen(0, false))])
            .at(now)
            .talk_started()
            .ended_at(0.0)
            .timeout(TIMEOUT)
            .run(&mut state);
        assert!(plain.actions.is_empty());
        assert!(plain.logs.is_empty());
    }
}

/// 文字の出ている箱の上にポインタが居るあいだは、普通のバルーンと同じ規則で満了を保留する
/// （要件 6.11）。
#[test]
fn hover_on_a_box_holds_the_expired_deadline() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, boxed(seen(0, false)))])
        .at(0.0)
        .talk_started()
        .ended_at(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);

    let held = Frame::new(&[(0, hovered(boxed(seen(0, false))))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
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
/// （箱のあるサーフェスへ切り替えた形・要件 5.1 と 6.10）。
#[test]
fn clearing_the_window_keeps_the_measurement_while_boxes_show() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(2, false))])
        .at(0.0)
        .talk_started()
        .ended_at(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);

    let cleared = Frame::new(&[(0, boxed(seen(0, true)))])
        .at(2.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(cleared.actions, vec![hide(&[0], VisibilityTrigger::Clear)]);
    assert_eq!(
        state.deadline,
        Some(TIMEOUT),
        "箱が出ているのに計測が捨てられた"
    );

    let expired = Frame::new(&[(0, boxed(seen(0, false)))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
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
        .at(0.0)
        .talk_started()
        .ended_at(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);

    let broke = run_with_signals(
        &mut state,
        &[(0, boxed(seen(0, false)))],
        &[TalkLifecycleSignal::UserBreak],
        Some(TIMEOUT),
    );
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
    let decision = run_with_signals(
        &mut state,
        &[(1, boxed(seen(0, false)))],
        &[
            TalkLifecycleSignal::UserBreak,
            TalkLifecycleSignal::TalkStarted { talk_id: None },
            TalkLifecycleSignal::DisplayEndAt(0.0),
            TalkLifecycleSignal::TalkEnded { at: Some(0.0) },
        ],
        Some(0.0),
    );
    assert_eq!(
        decision.actions,
        vec![hide(&[1], VisibilityTrigger::UserBreak)]
    );
    assert!(
        decision
            .logs
            .contains(&VisibilityLogEvent::MeasurementStarted {
                origin: MeasurementOrigin::DisplayEnd,
                display_end: 0.0,
                deadline: TIMEOUT,
            }),
        "掛け金の解けた中断で箱だけの scope が計測から外れた: {:?}",
        decision.logs
    );
}
