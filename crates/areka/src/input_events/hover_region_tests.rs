//! 出入りの純関数の檻（mouse-click-wheel-events 要件 7.1〜7.5・7.11〜7.13・11.3・design「HoverRegion」の 7 行の表）。

use areka_kanade::HoverEdge;

use super::{HoverSend, HoverSighting, HoverState, plan_hover};

fn over(region: Option<&str>, surface: (i64, i64)) -> HoverSighting {
    HoverSighting::Over {
        scope: 1,
        client: (surface.0 * 2, surface.1 * 2),
        surface,
        region: region.map(str::to_owned),
    }
}

fn state(region: Option<&str>, surface: (i64, i64)) -> HoverState {
    HoverState {
        scope: 1,
        client: (surface.0 * 2, surface.1 * 2),
        surface,
        region: region.map(str::to_owned),
    }
}

fn s(edge: HoverEdge, region: Option<&str>, surface: (i64, i64)) -> HoverSend {
    HoverSend {
        edge,
        surface,
        region: region.map(str::to_owned),
    }
}

/// 表 1 行目: 表に無い→Over(r) は EnterAll(r)→Enter(r)（要件 7.11・7.13）。
#[test]
fn enter_window_onto_region_sends_enter_all_then_enter() {
    let (next, sends) = plan_hover(None, over(Some("Head"), (10, 20)));
    assert_eq!(next, Some(state(Some("Head"), (10, 20))));
    assert_eq!(
        sends,
        vec![
            s(HoverEdge::EnterAll, Some("Head"), (10, 20)),
            s(HoverEdge::Enter, Some("Head"), (10, 20)),
        ]
    );
}

/// 表 1 行目（r が無い）: EnterAll だけ・Ref4 は空（要件 7.11）。
#[test]
fn enter_window_onto_nothing_sends_enter_all_with_empty_region() {
    let (next, sends) = plan_hover(None, over(None, (3, 4)));
    assert_eq!(next, Some(state(None, (3, 4))));
    assert_eq!(sends, vec![s(HoverEdge::EnterAll, None, (3, 4))]);
}

/// 表 2 行目: Over(A)→Over(A) は送らず、位置だけ覚え直す。
#[test]
fn staying_on_same_region_sends_nothing_and_updates_position() {
    let prev = state(Some("Head"), (10, 20));
    let (next, sends) = plan_hover(Some(&prev), over(Some("Head"), (11, 21)));
    assert_eq!(next, Some(state(Some("Head"), (11, 21))));
    assert!(sends.is_empty());
}

/// 表 3 行目: Over(無し)→Over(B) は Enter(B)（要件 7.1）。
#[test]
fn nothing_to_region_sends_enter() {
    let prev = state(None, (1, 1));
    let (next, sends) = plan_hover(Some(&prev), over(Some("Bust"), (5, 6)));
    assert_eq!(next, Some(state(Some("Bust"), (5, 6))));
    assert_eq!(sends, vec![s(HoverEdge::Enter, Some("Bust"), (5, 6))]);
}

/// 表 4 行目: Over(A)→Over(無し) は Leave(A)・座標は今の位置（要件 7.2）。
#[test]
fn region_to_nothing_sends_leave_at_current_position() {
    let prev = state(Some("Head"), (10, 20));
    let (next, sends) = plan_hover(Some(&prev), over(None, (30, 40)));
    assert_eq!(next, Some(state(None, (30, 40))));
    assert_eq!(sends, vec![s(HoverEdge::Leave, Some("Head"), (30, 40))]);
}

/// 表 5 行目: Over(A)→Over(B) は Leave(A)→Enter(B)・どちらも今の位置（要件 7.3）。
#[test]
fn region_to_region_sends_leave_then_enter() {
    let prev = state(Some("Head"), (10, 20));
    let (next, sends) = plan_hover(Some(&prev), over(Some("Bust"), (12, 50)));
    assert_eq!(next, Some(state(Some("Bust"), (12, 50))));
    assert_eq!(
        sends,
        vec![
            s(HoverEdge::Leave, Some("Head"), (12, 50)),
            s(HoverEdge::Enter, Some("Bust"), (12, 50)),
        ]
    );
}

/// 表 6 行目: Over(A)→Away は Leave(A)→LeaveAll(A)・座標は窓の中で最後に見た位置（要件 7.4・7.12・7.13）。
#[test]
fn leaving_window_from_region_sends_leave_then_leave_all_at_last_seen() {
    let prev = state(Some("Head"), (10, 20));
    let (next, sends) = plan_hover(Some(&prev), HoverSighting::Away);
    assert_eq!(next, None);
    assert_eq!(
        sends,
        vec![
            s(HoverEdge::Leave, Some("Head"), (10, 20)),
            s(HoverEdge::LeaveAll, Some("Head"), (10, 20)),
        ]
    );
}

/// 表 6 行目（r が無い）: LeaveAll だけ・Ref4 は空・最後に見た位置（要件 7.12）。
#[test]
fn leaving_window_from_nothing_sends_leave_all_only() {
    let prev = state(None, (7, 8));
    let (next, sends) = plan_hover(Some(&prev), HoverSighting::Away);
    assert_eq!(next, None);
    assert_eq!(sends, vec![s(HoverEdge::LeaveAll, None, (7, 8))]);
}

/// 表 7 行目: 表に無い→Away は何もしない。
#[test]
fn away_without_state_sends_nothing() {
    let (next, sends) = plan_hover(None, HoverSighting::Away);
    assert_eq!(next, None);
    assert!(sends.is_empty());
}
