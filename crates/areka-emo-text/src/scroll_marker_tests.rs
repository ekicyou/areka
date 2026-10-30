//! `PlaceScroll` の送りの規則の檻（design.md「純粋な判断（`scroll_marker_tests.rs`）」）。
//!
//! 自動の先頭行 `auto_first` は本来 `window_for` が毎フレーム覚え直す。ここでは私有の欄を
//! 直接置いて、送りの規則だけを固定する。

use super::*;

/// 自動の先頭行が `auto_first` の、手動に入っていない状態。
fn auto_at(auto_first: usize) -> PlaceScroll {
    PlaceScroll {
        auto_first,
        ..PlaceScroll::default()
    }
}

// ---- 矢印の送り（要件 5.1〜5.4） ----

#[test]
fn arrow_step_is_two_lines() {
    assert_eq!(ARROW_STEP_LINES, 2);
}

#[test]
fn older_from_auto_starts_manual_two_lines_back() {
    let mut s = auto_at(5);
    assert_eq!(
        s.step(ScrollDir::Older, ARROW_STEP_LINES),
        ScrollChange::Started
    );
    assert_eq!(s.manual, Some(3));
    assert_eq!(
        s.step(ScrollDir::Older, ARROW_STEP_LINES),
        ScrollChange::Moved
    );
    assert_eq!(s.manual, Some(1));
}

#[test]
fn older_overshoot_is_clamped_to_line_zero_then_stops() {
    let mut s = auto_at(3);
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Started);
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Moved);
    assert_eq!(s.manual, Some(0), "行き過ぎは行 0 に詰める");
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Unchanged);
    assert_eq!(s.manual, Some(0));
}

#[test]
fn older_from_auto_with_one_hidden_line_clamps_to_zero() {
    let mut s = auto_at(1);
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Started);
    assert_eq!(s.manual, Some(0));
}

#[test]
fn newer_reaching_auto_drops_manual() {
    let mut s = auto_at(5);
    s.step(ScrollDir::Older, 2); // 3
    s.step(ScrollDir::Older, 2); // 1
    assert_eq!(s.step(ScrollDir::Newer, 2), ScrollChange::Moved);
    assert_eq!(s.manual, Some(3));
    assert_eq!(
        s.step(ScrollDir::Newer, 2),
        ScrollChange::Stopped(StopReason::ReachedAuto),
        "3+2=5 は自動の先頭行に達する＝自動へ戻る"
    );
    assert_eq!(s.manual, None);
}

#[test]
fn newer_past_auto_also_drops_manual() {
    let mut s = auto_at(4);
    s.step(ScrollDir::Older, 1); // 3
    assert_eq!(
        s.step(ScrollDir::Newer, 2),
        ScrollChange::Stopped(StopReason::ReachedAuto)
    );
    assert_eq!(s.manual, None);
}

#[test]
fn newer_without_manual_does_nothing() {
    let mut s = auto_at(5);
    assert_eq!(s.step(ScrollDir::Newer, 2), ScrollChange::Unchanged);
    assert_eq!(s, auto_at(5));
}

#[test]
fn nothing_hidden_means_no_step() {
    let mut s = auto_at(0);
    assert!(!s.has_hidden_lines());
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Unchanged);
    assert_eq!(s.step(ScrollDir::Newer, 2), ScrollChange::Unchanged);
    assert_eq!(s, auto_at(0));
}

#[test]
fn has_hidden_lines_when_pushed_out_or_manual() {
    assert!(auto_at(1).has_hidden_lines());
    let mut s = auto_at(2);
    s.step(ScrollDir::Older, 2);
    s.auto_first = 0; // 手動が在れば新しい側に隠れた行が在る
    assert!(s.has_hidden_lines());
}

// ---- ホイール（要件 6.3〜6.6） ----

#[test]
fn wheel_one_notch_with_lines_3_sends_three_lines() {
    let mut s = auto_at(10);
    assert_eq!(s.wheel(120, WheelLines::Lines(3), 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(7));
    assert_eq!(
        s.wheel(-120, WheelLines::Lines(3), 4),
        ScrollChange::Stopped(StopReason::ReachedAuto)
    );
    assert_eq!(s.manual, None);
}

#[test]
fn wheel_one_notch_with_lines_1_sends_one_line() {
    let mut s = auto_at(10);
    assert_eq!(s.wheel(120, WheelLines::Lines(1), 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(9));
}

#[test]
fn wheel_page_sends_visible_line_count() {
    let mut s = auto_at(10);
    assert_eq!(s.wheel(120, WheelLines::Page, 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(6));
}

#[test]
fn wheel_two_notches_at_once_send_twice() {
    let mut s = auto_at(10);
    s.wheel(240, WheelLines::Lines(3), 4);
    assert_eq!(s.manual, Some(4));
}

#[test]
fn wheel_remainder_is_carried_while_sendable() {
    let mut s = auto_at(10);
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s.wheel_remainder, 60);
    assert_eq!(s.manual, None);
    assert_eq!(s.wheel(60, WheelLines::Lines(3), 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(7));
    assert_eq!(s.wheel_remainder, 0);
    // 端数つきの回転: 180 = 1 目盛り＋60
    assert_eq!(s.wheel(180, WheelLines::Lines(3), 4), ScrollChange::Moved);
    assert_eq!(s.manual, Some(4));
    assert_eq!(s.wheel_remainder, 60);
    // 逆向きの回転は端数と打ち消し合う
    assert_eq!(
        s.wheel(-60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s.wheel_remainder, 0);
}

#[test]
fn wheel_does_nothing_and_drops_remainder_without_hidden_lines() {
    let mut s = auto_at(0);
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s.wheel_remainder, 0, "送れない間は端数を持ち越さない");
    assert_eq!(
        s.wheel(120, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s, auto_at(0));
}

#[test]
fn wheel_remainder_is_dropped_when_the_notch_cannot_send() {
    let mut s = auto_at(2);
    s.wheel(120, WheelLines::Lines(3), 4); // 行 0 に詰まる
    assert_eq!(s.manual, Some(0));
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(
        s.wheel_remainder, 0,
        "古い側の端で送れなかった目盛りの端数は捨てる"
    );
    assert_eq!(s.manual, Some(0));
}

#[test]
fn wheel_newer_clamps_to_auto_with_limits() {
    let mut s = auto_at(4);
    s.step(ScrollDir::Older, 2); // 2
    assert_eq!(
        s.wheel(-120, WheelLines::Lines(3), 4),
        ScrollChange::Stopped(StopReason::ReachedAuto),
        "行き過ぎても自動へ戻るだけ"
    );
    assert_eq!(s.manual, None);
}

// ---- 見える字の増減（要件 8.1） ----

#[test]
fn growing_visible_text_drops_manual_with_new_text() {
    let mut s = auto_at(5);
    assert_eq!(s.observe_visible(10), None, "手動でなければ理由は返さない");
    s.step(ScrollDir::Older, 2);
    assert_eq!(s.observe_visible(10), None, "同じ数なら何もしない");
    assert_eq!(s.manual, Some(3));
    assert_eq!(s.observe_visible(11), Some(StopReason::NewText));
    assert_eq!(s.manual, None);
    assert_eq!(s.last_visible, 11);
}

#[test]
fn shrinking_visible_text_only_remembers_the_count() {
    let mut s = auto_at(5);
    s.observe_visible(10);
    s.step(ScrollDir::Older, 2);
    assert_eq!(s.observe_visible(4), None);
    assert_eq!(s.manual, Some(3), "減ったときは手動を捨てない");
    assert_eq!(s.last_visible, 4);
    assert_eq!(
        s.observe_visible(5),
        Some(StopReason::NewText),
        "覚え直した数より増えたら捨てる"
    );
}

#[test]
fn forget_drops_manual_and_remainder() {
    let mut s = auto_at(10);
    s.wheel(180, WheelLines::Lines(3), 4);
    assert_eq!(
        s.forget(StopReason::Cleared),
        ScrollChange::Stopped(StopReason::Cleared)
    );
    assert_eq!(s.manual, None);
    assert_eq!(s.wheel_remainder, 0);
    assert_eq!(s.forget(StopReason::Cleared), ScrollChange::Unchanged);
}
