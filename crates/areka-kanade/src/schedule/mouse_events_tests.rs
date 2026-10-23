//! マウスの解放・ホイール・出入りの受理と、クリックの段の列の表のテスト
//! （mouse-click-wheel-events 要件 1.1・1.7・2.1・2.4・2.6・3.2・6.1・6.2・7.5・8.2・8.3・9.1）。
//!
//! 表（[`plan_click`]）と受理（[`on_mouse_extended`]）は直に、定常の入口の委譲・閉じの握手の保留・
//! 定常以外の棄却は最上位の `step` から通す。

use super::*;
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriCall};
use crate::schedule::log_capture::{assert_not_logged, capture, logged_once};
use crate::schedule::{ActiveTalk, Input, Phase, step};
use crate::talk::TalkId;
use tracing::Level;

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// 定常（話していない）。
fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 7,
        ..State::initial()
    }
}

/// 定常（話している最中）。
fn steady_talking() -> State {
    State {
        phase: Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id: TalkId(3),
                origin: "OnSecondChange",
                script: String::new(),
            }),
        },
        ..steady_none()
    }
}

fn input(kind: MouseEventKind) -> MouseInput {
    MouseInput {
        scope: 1,
        x: 12,
        y: 34,
        region: Some("Head".to_string()),
        kind,
    }
}

fn up(button: ClickButton, second_of_double: bool) -> MouseEventKind {
    MouseEventKind::ButtonUp {
        button,
        second_of_double,
    }
}

/// 一括が GET 1 本だけであることを確かめ、(イベント名, Reference の並び) を返す。
fn only_get(actions: &[Action]) -> (String, Vec<String>) {
    match actions {
        [Action::ShioriRequest(ShioriCall::Get { id, references, .. })] => {
            (id.as_str().to_string(), references.clone())
        }
        _ => panic!("GET 1 本だけのはず（{} 件）", actions.len()),
    }
}

fn names(steps: &[ClickStep]) -> Vec<(&'static str, &'static str)> {
    steps.iter().map(|s| (s.id, s.ref5)).collect()
}

fn refs(ref2: &str, ref5: &str) -> Vec<String> {
    ["12", "34", ref2, "1", "Head", ref5, "mouse"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// --- 段の列の表 ---

/// 要件 1.1・2.1・2.4・2.6・3.2: ボタン 5 つ × ふつう／2 回目の 10 通り。
#[test]
fn plan_click_table_has_ten_rows() {
    use ClickButton::*;
    type Row = (ClickButton, bool, &'static [(&'static str, &'static str)]);
    let table: [Row; 10] = [
        (Left, false, &[("OnMouseUp", "0"), ("OnMouseClick", "0")]),
        (Left, true, &[("OnMouseUp", "0")]),
        (Right, false, &[("OnMouseUp", "1"), ("OnMouseClick", "1")]),
        (Right, true, &[("OnMouseUp", "1")]),
        (
            Middle,
            false,
            &[
                ("OnMouseUpEx", "middle"),
                ("OnMouseClickEx", "middle"),
                ("OnMouseUp", "2"),
                ("OnMouseClick", "2"),
            ],
        ),
        (
            Middle,
            true,
            &[("OnMouseUpEx", "middle"), ("OnMouseUp", "2")],
        ),
        (
            XButton1,
            false,
            &[("OnMouseUpEx", "xbutton1"), ("OnMouseClickEx", "xbutton1")],
        ),
        (XButton1, true, &[("OnMouseUpEx", "xbutton1")]),
        (
            XButton2,
            false,
            &[("OnMouseUpEx", "xbutton2"), ("OnMouseClickEx", "xbutton2")],
        ),
        (XButton2, true, &[("OnMouseUpEx", "xbutton2")]),
    ];
    for (button, second, want) in table {
        assert_eq!(
            names(plan_click(button, second)),
            want.to_vec(),
            "{button:?} second_of_double={second}"
        );
    }
}

// --- 解放の受理 ---

/// 要件 1.1・1.7・2.1・9.1: 1 段目の GET を 1 本だけ積み（Reference の 7 並び）、段の記憶を置き、
/// `mouse_event_sent` を info で残す。ボタン 5 つ × 2 回目か、のどれでも同じ形。
#[test]
fn button_up_sends_first_step_and_keeps_the_chain() {
    use ClickButton::*;
    for button in [Left, Right, Middle, XButton1, XButton2] {
        for second in [false, true] {
            let steps = plan_click(button, second);
            let mut out = None;
            let events =
                capture(|| out = Some(on_mouse_extended(steady_none(), input(up(button, second)))));
            let (state, actions) = out.unwrap();
            let (id, references) = only_get(&actions);
            assert_eq!(id, steps[0].id, "{button:?} {second}");
            assert_eq!(references, refs("0", steps[0].ref5), "{button:?} {second}");
            assert_eq!(
                state.mouse_click,
                Some(ClickChain {
                    scope: 1,
                    x: 12,
                    y: 34,
                    region: Some("Head".to_string()),
                    steps,
                    next: 1,
                    second_of_double: second,
                })
            );
            let ev = logged_once(&events, Level::INFO, "mouse_event_sent");
            assert_eq!(ev.fields.get("id").map(String::as_str), Some(steps[0].id));
        }
    }
}

/// 要件 1.1: 当たり判定の無い解放は Ref4 が空。
#[test]
fn button_up_without_region_leaves_ref4_empty() {
    let mut m = input(up(ClickButton::Left, false));
    m.region = None;
    let (_, actions) = on_mouse_extended(steady_none(), m);
    assert_eq!(only_get(&actions).1[4], "");
}

// --- ホイールと出入り ---

/// 要件 6.1・6.2・9.1: ホイールは `OnMouseWheel`（Ref2＝回転量・Ref5＝0）を 1 本だけ。段の記憶は置かない。
#[test]
fn wheel_sends_one_get_with_delta_in_ref2() {
    for delta in [120, -240] {
        let mut out = None;
        let events = capture(|| {
            out = Some(on_mouse_extended(
                steady_none(),
                input(MouseEventKind::Wheel { delta }),
            ))
        });
        let (state, actions) = out.unwrap();
        let (id, references) = only_get(&actions);
        assert_eq!(id, "OnMouseWheel");
        assert_eq!(references, refs(&delta.to_string(), "0"));
        assert!(state.mouse_click.is_none());
        logged_once(&events, Level::DEBUG, "mouse_event_sent");
    }
}

/// 要件 7.5・9.1: 出入りは 4 語のどれか（Ref2・Ref5＝0）を 1 本だけ。段の記憶は置かない。
#[test]
fn hover_sends_one_of_the_four_names() {
    for (edge, want) in [
        (HoverEdge::Enter, "OnMouseEnter"),
        (HoverEdge::Leave, "OnMouseLeave"),
        (HoverEdge::EnterAll, "OnMouseEnterAll"),
        (HoverEdge::LeaveAll, "OnMouseLeaveAll"),
    ] {
        let mut out = None;
        let events = capture(|| {
            out = Some(on_mouse_extended(
                steady_none(),
                input(MouseEventKind::Hover(edge)),
            ))
        });
        let (state, actions) = out.unwrap();
        let (id, references) = only_get(&actions);
        assert_eq!(id, want);
        assert_eq!(references, refs("0", "0"));
        assert!(state.mouse_click.is_none());
        logged_once(&events, Level::DEBUG, "mouse_event_sent");
    }
}

// --- 定常の入口からの委譲 ---

/// 要件 8.2: 話の最中でも送る（入口の委譲を `step` から通す）。
#[test]
fn step_delegates_in_steady_even_while_talking() {
    for state in [steady_none(), steady_talking()] {
        let (state, actions) = step(
            state,
            Input::Mouse(input(up(ClickButton::Right, false))),
            &config(),
        );
        assert_eq!(only_get(&actions).0, "OnMouseUp");
        assert!(state.mouse_click.is_some());
    }
}

/// 要件 8.3: 閉じの握手の保留中は 3 つとも送らない（段の記憶も置かない）。
#[test]
fn step_sends_nothing_while_close_is_pending() {
    for kind in [
        up(ClickButton::Left, false),
        MouseEventKind::Wheel { delta: 120 },
        MouseEventKind::Hover(HoverEdge::Enter),
    ] {
        let state = State {
            pending_close: Some(CloseReason::User { scope: 0 }),
            ..steady_none()
        };
        let mut out = None;
        let events = capture(|| out = Some(step(state, Input::Mouse(input(kind)), &config())));
        let (state, actions) = out.unwrap();
        assert!(actions.is_empty(), "{kind:?}");
        assert!(state.mouse_click.is_none());
        assert_not_logged(&events, "mouse_event_sent");
    }
}

/// 要件 8.3: 定常以外では 3 つとも送らない。
#[test]
fn step_sends_nothing_outside_steady() {
    for kind in [
        up(ClickButton::Left, false),
        MouseEventKind::Wheel { delta: 120 },
        MouseEventKind::Hover(HoverEdge::LeaveAll),
    ] {
        let state = State {
            phase: Phase::Idle,
            ..steady_none()
        };
        let (state, actions) = step(state, Input::Mouse(input(kind)), &config());
        assert!(actions.is_empty(), "{kind:?}");
        assert!(state.mouse_click.is_none());
    }
}
