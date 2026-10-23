//! mouse_events — マウスの解放・ホイール・出入りの受理と、クリックの段の列（mouse-click-wheel-events）。
//!
//! - [`plan_click`]: ボタンと「2 回目の解放か」から送る段の列を決める（10 通りの表）。
//! - [`on_mouse_extended`]: 3 つの知らせの受理。解放は段の列の 1 段目の GET を積んで段の記憶を
//!   [`State::mouse_click`](super::State) に置く。ホイールと出入りは GET を 1 本積むだけ。
//!
//! 呼び手（[`super::steady::on_mouse`]）が閉じの握手の保留を確かめた後に渡す。定常以外の棄却は
//! 運行表の入口が持つ。話の最中かどうかでは分けない（要件 8.2）。

use super::{Action, State, events};
use crate::msg::{ClickButton, HoverEdge, MouseEventKind, MouseInput};

/// 段 1 つ（送るイベントの名前と Ref5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClickStep {
    pub id: &'static str,
    pub ref5: &'static str,
}

/// ボタンと「2 回目の解放か」から段の列を決める。
///
/// 中のふつうは `Ex` の 2 段の後に互換の 2 段。2 回目の解放は `Click` の系を抜く（要件 3.2）。
/// 中の 2 回目は `Ex` に返事が無ければ互換が続くよう `OnMouseUpEx` → `OnMouseUp`(2)（要件 2.4・§8 に登記）。
pub(crate) fn plan_click(button: ClickButton, second_of_double: bool) -> &'static [ClickStep] {
    const fn s(id: &'static str, ref5: &'static str) -> ClickStep {
        ClickStep { id, ref5 }
    }
    const UP: &str = "OnMouseUp";
    const CLICK: &str = "OnMouseClick";
    const UP_EX: &str = "OnMouseUpEx";
    const CLICK_EX: &str = "OnMouseClickEx";
    const LEFT: &[ClickStep] = &[s(UP, "0"), s(CLICK, "0")];
    const RIGHT: &[ClickStep] = &[s(UP, "1"), s(CLICK, "1")];
    const MIDDLE: &[ClickStep] = &[
        s(UP_EX, "middle"),
        s(CLICK_EX, "middle"),
        s(UP, "2"),
        s(CLICK, "2"),
    ];
    const MIDDLE_SECOND: &[ClickStep] = &[s(UP_EX, "middle"), s(UP, "2")];
    const X1: &[ClickStep] = &[s(UP_EX, "xbutton1"), s(CLICK_EX, "xbutton1")];
    const X2: &[ClickStep] = &[s(UP_EX, "xbutton2"), s(CLICK_EX, "xbutton2")];
    match (button, second_of_double) {
        (ClickButton::Left, false) => LEFT,
        (ClickButton::Right, false) => RIGHT,
        (ClickButton::Middle, false) => MIDDLE,
        (ClickButton::Middle, true) => MIDDLE_SECOND,
        (ClickButton::XButton1, false) => X1,
        (ClickButton::XButton2, false) => X2,
        // 左・右・拡張の 2 回目は 1 段目だけ（`Click` の系を送らない）。
        (ClickButton::Left, true) => &LEFT[..1],
        (ClickButton::Right, true) => &RIGHT[..1],
        (ClickButton::XButton1, true) => &X1[..1],
        (ClickButton::XButton2, true) => &X2[..1],
    }
}

/// 送っている途中の段の列（Reference の材料と、次に送る段の位置）。
// 応答の側（段を進める）は 2.4 の on_click_reply が読む。それまで本体のビルドでは読み手が居ない。
#[cfg_attr(not(test), expect(dead_code, reason = "2.4 の on_click_reply が読む"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClickChain {
    pub(super) scope: u32,
    pub(super) x: i64,
    pub(super) y: i64,
    pub(super) region: Option<String>,
    pub(super) steps: &'static [ClickStep],
    pub(super) next: usize,
    pub(super) second_of_double: bool,
}

/// 3 つの知らせ（解放・ホイール・出入り）の受理。
///
/// 解放は段の列の 1 段目の GET を積み、段の記憶を置く（次は 2 段目）。ホイール（`OnMouseWheel`・
/// Ref2＝回転量）と出入り（4 語・Ref2＝0）は GET を 1 本積むだけ。Ref5 は解放で段の値、他は "0"。
/// 送ったら `mouse_event_sent` を残す（クリックの系は info・ホイールと出入りは debug・要件 9.1）。
pub(super) fn on_mouse_extended(mut state: State, input: MouseInput) -> (State, Vec<Action>) {
    let snapshot = state.snapshot();
    let region = input.region.as_deref();
    let (x, y, scope) = (input.x, input.y, input.scope);
    let call = match input.kind {
        MouseEventKind::ButtonUp {
            button,
            second_of_double,
        } => {
            let steps = plan_click(button, second_of_double);
            let first = steps[0];
            tracing::info!(
                target: "kanade",
                event = "mouse_event_sent",
                id = first.id,
                scope,
                region,
                ref5 = first.ref5,
                "マウスの解放——クリックの段の列の 1 段目を送る"
            );
            let call = events::mouse_get(first.id, x, y, "0", scope, region, first.ref5, &snapshot);
            state.mouse_click = Some(ClickChain {
                scope,
                x,
                y,
                region: input.region,
                steps,
                next: 1,
                second_of_double,
            });
            call
        }
        MouseEventKind::Wheel { delta } => {
            debug_sent("OnMouseWheel", scope, region, delta);
            events::mouse_get(
                "OnMouseWheel",
                x,
                y,
                &delta.to_string(),
                scope,
                region,
                "0",
                &snapshot,
            )
        }
        MouseEventKind::Hover(edge) => {
            let id = match edge {
                HoverEdge::Enter => "OnMouseEnter",
                HoverEdge::Leave => "OnMouseLeave",
                HoverEdge::EnterAll => "OnMouseEnterAll",
                HoverEdge::LeaveAll => "OnMouseLeaveAll",
            };
            debug_sent(id, scope, region, 0);
            events::mouse_get(id, x, y, "0", scope, region, "0", &snapshot)
        }
        // 呼び手は 3 つの腕だけを渡す。
        MouseEventKind::Move
        | MouseEventKind::DoubleClick { .. }
        | MouseEventKind::DragStart
        | MouseEventKind::DragEnd => {
            tracing::warn!(target: "kanade", event = "mouse_extended_unexpected_kind", ?input, "解放・ホイール・出入り以外が届いた——送らない");
            return (state, Vec::new());
        }
    };
    (state, vec![Action::ShioriRequest(call)])
}

/// ホイールと出入りの `mouse_event_sent`（カーソルの動きにつれて出るので debug）。
fn debug_sent(id: &'static str, scope: u32, region: Option<&str>, delta: i32) {
    tracing::debug!(
        target: "kanade",
        event = "mouse_event_sent",
        id,
        scope,
        region,
        delta,
        "マウスのホイール・出入りを送る"
    );
}

#[cfg(test)]
#[path = "mouse_events_tests.rs"]
mod tests;
