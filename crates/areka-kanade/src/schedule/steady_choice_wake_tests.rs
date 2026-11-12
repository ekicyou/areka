use super::test_support::{choice_input_of, config, expect_get_call, expect_ledger, steady_some};
use super::*;
use crate::schedule::log_capture::{assert_logged, capture};
use crate::schedule::step;
use tracing::Level;

// ============================================================
// 選択待ちの期限の知らせ（要件 9.5）
// ============================================================
//
// 期限が決まった時に起こしの頼みを 1 つ返し、期限の知らせ（`Input::ChoiceTimeoutDue`）で
// 期限ちょうどに `OnChoiceTimeout` を出す。時刻は注入値だけで決まり、実時間を待たない。

/// 表示の終わり 10,001・指令 `secs` の選択待ちの通知（トーク 3 番・候補 `OnMenu`）。
fn choice_waiting(secs: f64) -> Input {
    Input::ChoiceWaiting {
        talk_id: TalkId(3),
        choice_ids: vec!["OnMenu".to_string()],
        display_end: MonotonicMs(10_001),
        timeout_directive_secs: Some(secs),
    }
}

/// 指令 0.5 秒の選択待ちを作った `Steady{Some}`（期限 10,501）。
fn waiting_until_10_501() -> State {
    let (s, _) = step(steady_some(TalkId(3), 4), choice_waiting(0.5), &config());
    s
}

fn due(now: u64) -> Input {
    Input::ChoiceTimeoutDue {
        now: MonotonicMs(now),
    }
}

/// 期限つきの帳簿を作った時に `ArmChoiceWake(期限)` を 1 つだけ返し、無効化の指令（0・-1）では返さない。
#[test]
fn choice_waiting_with_a_deadline_asks_for_a_wake_at_the_deadline() {
    let (_, actions) = step(steady_some(TalkId(3), 4), choice_waiting(0.5), &config());
    assert!(
        matches!(
            actions.as_slice(),
            [Action::ArmChoiceWake(MonotonicMs(10_501))]
        ),
        "期限 10,501 の起こしの頼みが 1 つだけ出る"
    );
    for secs in [0.0, -1.0] {
        let (next, actions) = step(steady_some(TalkId(3), 4), choice_waiting(secs), &config());
        assert!(actions.is_empty(), "無期限（指令 {secs}）は頼みを出さない");
        assert!(
            expect_ledger(&next).deadline.is_none(),
            "帳簿は無期限で立つ"
        );
    }
}

/// 知らせは期限の前（10,500）では出さず、期限ちょうど（10,501）で 1 回だけ出す。後の Tick で二度目は出ない。
#[test]
fn the_deadline_notice_fires_exactly_at_the_deadline() {
    let (s, actions) = step(waiting_until_10_501(), due(10_500), &config());
    assert!(actions.is_empty(), "期限の前の知らせは何も出さない");
    assert!(matches!(expect_ledger(&s).phase, ChoicePhase::Waiting));
    assert_eq!(
        s.last_now,
        Some(MonotonicMs(500)),
        "知らせは last_now を動かさない"
    );

    let (s, actions) = step(s, due(10_501), &config());
    assert_eq!(actions.len(), 1, "期限ちょうどで GET を 1 件だけ出す");
    let (id, _) = expect_get_call(&actions[0]);
    assert_eq!(id, "OnChoiceTimeout");
    assert!(matches!(
        expect_ledger(&s).phase,
        ChoicePhase::TimeoutInFlight
    ));

    let (_, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(11_000),
        },
        &config(),
    );
    for a in &actions {
        if let Action::ShioriRequest(ShioriCall::Get { id, .. }) = a {
            assert_ne!(id.as_str(), "OnChoiceTimeout", "二度目の時間切れは出ない");
        }
    }
}

/// 選択を確定した後に遅れて届いた知らせは空振り（trace だけ・OnSecondChange も出ない）。定常の外でも同じ。
#[test]
fn a_late_notice_after_the_choice_is_accepted_does_nothing() {
    let (s, _) = step(
        waiting_until_10_501(),
        Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
        &config(),
    );
    let mut out = None;
    let ev = capture(|| out = Some(step(s, due(10_501), &config())));
    let (_, actions) = out.expect("step は結果を返す");
    assert!(actions.is_empty(), "確定の後の知らせは何も出さない");
    assert_logged(&ev, Level::TRACE, "choice_timeout_due_not_fired");

    let idle = State::initial();
    let mut out = None;
    let ev = capture(|| out = Some(step(idle, due(10_501), &config())));
    let (_, actions) = out.expect("step は結果を返す");
    assert!(actions.is_empty(), "定常の外の知らせは何も出さない");
    assert_logged(&ev, Level::TRACE, "choice_timeout_due_ignored");
}
