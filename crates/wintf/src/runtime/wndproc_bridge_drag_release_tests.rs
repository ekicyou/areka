//! 入口を通る再入のドラッグの決定論テスト（離し側・その後の操作・記録）。
//!
//! 組み立ては `wndproc_bridge_drag_test_support.rs` の [`Rig`]。座標と捕捉を確かめるテスト
//! （2・5・11・14）は [`Rig::with_real_window`] で隠れた実物の窓を決まった位置に作る。
//! テストの番号は design.md の Testing Strategy「入口を通る再入のテスト」の表の番号
//! （2・5・6・9・10・11・12・14）。実時間の待機は 0 か所。

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{WM_CAPTURECHANGED, WM_LBUTTONUP, WM_MOUSEMOVE};

use super::wndproc_bridge_drag_test_support::{
    CANCELMODE, CANCELS, CAPTURE_LOST, MOVED, PRESS, RealWindow, Rig, client_lparam,
    spawn_drag_window,
};
use super::{WndState, make_wndproc};
use crate::ecs::drag::{
    DragConfig, DragEndEvent, DragStartEvent, DragStateSnapshot, snapshot_drag_state,
    start_preparing,
};
use crate::ecs::pointer::PhysicalPoint;

/// 対象の実物の窓を置く画面の位置。
const AT: PhysicalPoint = PhysicalPoint { x: 400, y: 300 };
/// もう 1 枚の実物の窓を置く画面の位置（対象の窓と重ならない）。
const OTHER_AT: PhysicalPoint = PhysicalPoint { x: 800, y: 300 };
/// 離す位置（窓の中の座標）。押した位置とも動かした位置とも違う。
const RELEASE: PhysicalPoint = PhysicalPoint { x: 150, y: 170 };

/// このスレッドのマウスの捕捉を持つ窓。
fn capture_owner() -> HWND {
    // SAFETY: Win32 境界。このスレッドの捕捉の持ち主を読むだけ。
    unsafe { GetCapture() }
}

/// 状態が取り消しの印なしで休んでいるか。
fn rests_released() -> bool {
    matches!(
        snapshot_drag_state(),
        DragStateSnapshot::JustEnded {
            cancelled: false,
            ..
        }
    )
}

// ============================================================================
// テスト 2: 閾値を越えた後の離しで、終了の知らせが印なしで 1 件。位置は、同じ操作を
// 借用なしで渡した場合と同じ画面の座標（要件 2.2・2.3）
// ============================================================================

/// 実物の窓でドラッグを始め、離しを渡して、離した画面の座標と終了の知らせを返す。
fn release_after_threshold(borrowed: bool) -> (PhysicalPoint, Vec<DragEndEvent>, Entity) {
    let rig = Rig::with_real_window(AT);
    rig.start_drag();
    if borrowed {
        rig.send_while_borrowed(WM_LBUTTONUP, 0, client_lparam(RELEASE));
    } else {
        rig.send(WM_LBUTTONUP, 0, client_lparam(RELEASE));
    }
    (rig.screen(RELEASE), rig.dispatch_ends(), rig.target)
}

#[test]
fn t02_reentrant_release_after_threshold_ends_once_at_the_same_screen_point() {
    // 比べる相手: 借用なしの道は、窓の中の座標＋WindowPos.position の画面の座標で 1 件。
    let (screen, ordinary, target) = release_after_threshold(false);
    assert_eq!(
        ordinary.len(),
        1,
        "借用なしの道: 終了の知らせ 1 件: {ordinary:?}"
    );
    assert_eq!(ordinary[0].target, target, "借用なしの道: 対象");
    assert!(!ordinary[0].cancelled, "借用なしの道: 印なし");
    assert_eq!(
        ordinary[0].position, screen,
        "借用なしの道: 位置は離した点の画面の座標"
    );
    assert_ne!(
        screen, RELEASE,
        "窓の原点が画面の原点でない（窓の中の座標との取り違えを見分けられる）"
    );

    // 再入: 同じ位置に置いた実物の窓へ、同じ離しを World を借りたまま渡す。
    let (_, reentrant, target) = release_after_threshold(true);
    assert_eq!(
        reentrant.len(),
        1,
        "再入の離しでも終了の知らせは 1 件: {reentrant:?}"
    );
    assert_eq!(reentrant[0].target, target, "対象");
    assert!(!reentrant[0].cancelled, "印なし");
    assert_eq!(
        reentrant[0].position, ordinary[0].position,
        "位置は借用なしで渡した場合と同じ画面の座標"
    );
}

// ============================================================================
// テスト 5: 閾値前（と閾値に届いた直後）の離しで状態が休み、捕捉がその窓でなくなる（要件 4.1）
// ============================================================================

fn release_while_held_rests_and_frees_capture(cross_threshold: bool) {
    let rig = Rig::with_real_window(AT);
    rig.press();
    if cross_threshold {
        rig.cross_threshold();
    }
    assert_eq!(capture_owner(), rig.hwnd, "押しの後は捕捉がその窓");

    rig.send_while_borrowed(WM_LBUTTONUP, 0, client_lparam(PRESS));
    assert!(
        rests_released(),
        "再入の離しで状態は印なしで休む: {:?}",
        snapshot_drag_state()
    );
    assert_ne!(capture_owner(), rig.hwnd, "再入の離しで捕捉が解放される");
}

#[test]
fn t05_reentrant_release_before_threshold_rests_and_frees_capture() {
    release_while_held_rests_and_frees_capture(false);
}

#[test]
fn t05_reentrant_release_just_after_threshold_rests_and_frees_capture() {
    release_while_held_rests_and_frees_capture(true);
}

// ============================================================================
// テスト 6: 4・5 の後、ボタンを押さずに閾値を越える WM_MOUSEMOVE を渡しても、
// 開始の知らせ 0 件・状態は休んだまま（要件 4.3）
// ============================================================================

/// 押さずに閾値を越えて動かし、開始の知らせが 0 件で状態が休んだままであることを確かめる。
fn moving_without_press_starts_nothing(rig: &Rig, what: &str) {
    rig.send(WM_MOUSEMOVE, 0, client_lparam(MOVED));
    let starts = rig.dispatch::<DragStartEvent>();
    assert_eq!(
        starts.len(),
        0,
        "{what}: 押さずに動かしても開始の知らせは 0 件: {starts:?}"
    );
    assert!(
        !snapshot_drag_state().is_button_held(),
        "{what}: 状態は休んだまま: {:?}",
        snapshot_drag_state()
    );
}

#[test]
fn t06_no_start_without_press_after_reentrant_cancel_before_threshold() {
    for kind in CANCELS {
        let rig = Rig::new();
        rig.press();
        rig.send_while_borrowed(kind.msg, kind.wparam, 0);
        moving_without_press_starts_nothing(&rig, kind.what);
    }
}

#[test]
fn t06_no_start_without_press_after_reentrant_release_before_threshold() {
    let rig = Rig::with_real_window(AT);
    rig.press();
    rig.send_while_borrowed(WM_LBUTTONUP, 0, client_lparam(PRESS));
    moving_without_press_starts_nothing(&rig, "左ボタンを離す（WM_LBUTTONUP）");
}

// ============================================================================
// テスト 9: 再入でドラッグが終わった後、動かさないクリックで終了の知らせ 0 件（要件 3.1）
// ============================================================================

/// 閾値を越えたドラッグを再入で終える 2 通り（取り消し・離し）。
#[derive(Clone, Copy, Debug)]
enum ReentrantEnd {
    Cancel,
    Release,
}

impl ReentrantEnd {
    fn rig(self) -> Rig {
        match self {
            Self::Cancel => Rig::new(),
            Self::Release => Rig::with_real_window(AT),
        }
    }

    /// 閾値を越えたドラッグ中の `rig` へ、World を借りたまま終わりのメッセージを渡し、
    /// 配る段を 1 回回してその分を掃く（件数はテスト 1・2 が確かめる）。
    fn apply(self, rig: &Rig) {
        match self {
            Self::Cancel => rig.send_while_borrowed(CANCELMODE.msg, CANCELMODE.wparam, 0),
            Self::Release => rig.send_while_borrowed(WM_LBUTTONUP, 0, client_lparam(RELEASE)),
        };
        rig.dispatch_ends();
    }
}

fn click_without_move_after_reentrant_end_sends_no_end(end: ReentrantEnd) {
    let rig = end.rig();
    rig.start_drag();
    end.apply(&rig);

    // 動かさないクリック: 押し（状態へ直に入れる）→ 同じ点で離す（借用なし＝ふつうの道）。
    start_preparing(rig.target, rig.screen(PRESS), rig.hwnd);
    rig.send(WM_LBUTTONUP, 0, client_lparam(PRESS));
    let ends = rig.dispatch_ends();
    assert_eq!(
        ends.len(),
        0,
        "{end:?} の後の動かさないクリックで終了の知らせは 0 件: {ends:?}"
    );
}

#[test]
fn t09_click_without_move_after_reentrant_cancel_sends_no_end() {
    click_without_move_after_reentrant_end_sends_no_end(ReentrantEnd::Cancel);
}

#[test]
fn t09_click_without_move_after_reentrant_release_sends_no_end() {
    click_without_move_after_reentrant_end_sends_no_end(ReentrantEnd::Release);
}

// ============================================================================
// テスト 10: 再入でドラッグが終わった後、次の閾値越えのドラッグで開始 1 件 → 終了 1 件、
// 対象は新しいドラッグのもの（要件 3.3）
// ============================================================================

fn next_drag_after_reentrant_end_is_reported_once_each(end: ReentrantEnd) {
    let rig = end.rig();
    rig.start_drag();
    end.apply(&rig);

    // 新しいドラッグの対象: 同じ窓の中の部品（DragConfig だけを持つ子）。
    let next = rig
        .world
        .borrow_mut()
        .world_mut()
        .spawn((
            DragConfig {
                move_window: false,
                ..Default::default()
            },
            ChildOf(rig.target),
        ))
        .id();

    start_preparing(next, rig.screen(PRESS), rig.hwnd);
    rig.send(WM_MOUSEMOVE, 0, client_lparam(MOVED));
    let starts: Vec<Entity> = rig
        .dispatch::<DragStartEvent>()
        .iter()
        .map(|s| s.target)
        .collect();
    assert_eq!(
        starts,
        vec![next],
        "{end:?} の後の次のドラッグ: 開始の知らせは新しい対象について 1 件"
    );

    rig.send(WM_LBUTTONUP, 0, client_lparam(RELEASE));
    let ends = rig.dispatch_ends();
    assert_eq!(
        ends.iter().map(|e| e.target).collect::<Vec<_>>(),
        vec![next],
        "{end:?} の後の次のドラッグ: 終了の知らせは新しい対象について 1 件: {ends:?}"
    );
    assert!(!ends[0].cancelled, "{end:?} の後の次のドラッグ: 印なし");
    assert_eq!(
        ends[0].position,
        rig.screen(RELEASE),
        "{end:?} の後の次のドラッグ: 位置は離した点の画面の座標"
    );
}

#[test]
fn t10_next_drag_after_reentrant_cancel_starts_and_ends_once_for_the_new_target() {
    next_drag_after_reentrant_end_is_reported_once_each(ReentrantEnd::Cancel);
}

#[test]
fn t10_next_drag_after_reentrant_release_starts_and_ends_once_for_the_new_target() {
    next_drag_after_reentrant_end_is_reported_once_each(ReentrantEnd::Release);
}

// ============================================================================
// テスト 11: 捕捉を取った窓と違う窓の離しでは、借用の有無に関わらず終えない
// （隠れた実物の窓 2 枚・要件 4.2）
// ============================================================================

#[test]
fn t11_release_on_another_window_does_not_end_with_or_without_borrow() {
    for dragging in [false, true] {
        for borrowed in [false, true] {
            let case = format!(
                "{}・{}",
                if dragging {
                    "ドラッグ中"
                } else {
                    "準備中"
                },
                if borrowed {
                    "借用中"
                } else {
                    "借用なし"
                }
            );
            let rig = Rig::with_real_window(AT);
            let other_entity = spawn_drag_window(&rig.world, OTHER_AT);
            let other = RealWindow::new(&rig.world, other_entity, OTHER_AT);
            let other_state = Box::pin(WndState {
                world: Rc::downgrade(&rig.world),
                entity: other_entity,
            });
            assert_ne!(other.hwnd(), rig.hwnd, "{case}: 窓は 2 枚");

            if dragging {
                rig.start_drag();
            } else {
                rig.press();
            }
            let before = format!("{:?}", snapshot_drag_state());

            {
                let _held = borrowed.then(|| rig.world.borrow_mut());
                rig.send_as(
                    other_state.as_ref(),
                    other.hwnd(),
                    WM_LBUTTONUP,
                    0,
                    client_lparam(RELEASE),
                );
            }

            assert_eq!(
                format!("{:?}", snapshot_drag_state()),
                before,
                "{case}: 違う窓の離しでは状態は変わらない"
            );
            assert_eq!(capture_owner(), rig.hwnd, "{case}: 捕捉は取った窓のまま");
            let ends = rig.dispatch_ends();
            assert_eq!(ends.len(), 0, "{case}: 終了の知らせは 0 件: {ends:?}");
        }
    }
}

// ============================================================================
// テスト 12: 再入の扱いで `drag_reentry_handled` が 1 行（終えた・取り消したとき warn!・
// 何もしなかったとき debug!）。空の hwnd の離しで `drag_reentry_pos_unreadable` が 1 行
// （要件 7.1・7.2）
// ============================================================================

/// 記録のうち `event` 欄が `name` のもの。
fn lines<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    name: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 欄の文字（文字列で載せた欄は生値、`?` で載せた欄は Debug 表現）。欠落は `None`。
fn text<'a>(e: &'a log_capture_kit::CapturedEvent, name: &str) -> Option<&'a str> {
    e.field_str(name).or_else(|| e.field(name))
}

/// `drag_reentry_handled` の 1 行の中身を確かめる（design.md Monitoring の欄
/// `msg`・`entity`・`window`・`action`）。`entity` が `None` のときは欄があることだけを見る
/// （何もしなかったときは対象が無ければ空）。`msg` の綴りは決めていないので空でないことだけ。
fn assert_handled_line(
    line: &log_capture_kit::CapturedEvent,
    action: &str,
    level: tracing::Level,
    entity: Option<Entity>,
    window: Entity,
) {
    assert_eq!(line.field_str("action"), Some(action), "action: {line:?}");
    assert_eq!(line.level, level, "{action} の行の level: {line:?}");
    assert!(
        text(line, "msg").is_some_and(|m| !m.is_empty()),
        "{action} の行にメッセージの種類（msg）がある: {line:?}"
    );
    match entity {
        Some(entity) => assert_eq!(
            text(line, "entity"),
            Some(format!("{entity:?}").as_str()),
            "{action} の行のドラッグの対象（entity）: {line:?}"
        ),
        None => assert!(
            text(line, "entity").is_some(),
            "{action} の行に entity の欄がある: {line:?}"
        ),
    }
    assert_eq!(
        text(line, "window"),
        Some(format!("{window:?}").as_str()),
        "{action} の行の受けた窓（window）: {line:?}"
    );
}

#[test]
fn t12_reentrant_handling_logs_exactly_one_line() {
    // 取り消した: warn! で 1 行・action は "cancelled"。
    let rig = Rig::new();
    rig.start_drag();
    let (_, events) =
        log_capture_kit::capture(|| rig.send_while_borrowed(CANCELMODE.msg, CANCELMODE.wparam, 0));
    let handled = lines(&events, "drag_reentry_handled");
    assert_eq!(
        handled.len(),
        1,
        "取り消したとき drag_reentry_handled は 1 行: {handled:?}"
    );
    assert_handled_line(
        handled[0],
        "cancelled",
        tracing::Level::WARN,
        Some(rig.target),
        rig.target,
    );

    // 何もしなかった（休んでいる状態で捕捉の喪失）: debug! で 1 行・action は "none"。
    let rig = Rig::new();
    let (_, events) = log_capture_kit::capture(|| {
        rig.send_while_borrowed(CAPTURE_LOST.msg, CAPTURE_LOST.wparam, 0)
    });
    let handled = lines(&events, "drag_reentry_handled");
    assert_eq!(
        handled.len(),
        1,
        "何もしなかったときも drag_reentry_handled は 1 行: {handled:?}"
    );
    assert_handled_line(handled[0], "none", tracing::Level::DEBUG, None, rig.target);
}

#[test]
fn t12_reentrant_release_on_an_empty_window_logs_pos_unreadable() {
    // 空の hwnd では画面の座標へ直せない（ClientToScreen が失敗する）。
    let rig = Rig::new();
    rig.start_drag();
    let (_, events) = log_capture_kit::capture(|| {
        rig.send_while_borrowed(WM_LBUTTONUP, 0, client_lparam(RELEASE))
    });
    let unreadable = lines(&events, "drag_reentry_pos_unreadable");
    assert_eq!(
        unreadable.len(),
        1,
        "位置を読めなかった記録は 1 行: {unreadable:?}"
    );
    assert_eq!(
        unreadable[0].level,
        tracing::Level::WARN,
        "読めなかった情報は warn!"
    );
    let handled = lines(&events, "drag_reentry_handled");
    assert_eq!(handled.len(), 1, "扱いの記録も 1 行: {handled:?}");
    assert_handled_line(
        handled[0],
        "ended",
        tracing::Level::WARN,
        Some(rig.target),
        rig.target,
    );
}

// ============================================================================
// テスト 14: start_preparing の後、状態は準備中で捕捉はその窓。捕捉を取る間に
// ドラッグの状態が借りられていない（SetCapture が同期で送る WM_CAPTURECHANGED を
// 入口から受けても落ちない・不変条件 2）
// ============================================================================

#[test]
fn t14_start_preparing_takes_capture_without_falling_over_on_capture_changed() {
    let rig = Rig::with_real_window(AT);

    // 先に捕捉を持つ窓。窓の手続きは本番の入口で、届いた WM_CAPTURECHANGED を数え、
    // 入口の中の panic を受け止めて記録する（窓の手続きの外へ巻き戻させない）。
    let seen = Rc::new(Cell::new(0u32));
    let fell_over = Rc::new(Cell::new(false));
    let holder_entity = rig.world.borrow_mut().world_mut().spawn(()).id();
    let holder = RealWindow::with_wndproc(&rig.world, holder_entity, OTHER_AT, {
        let (seen, fell_over) = (seen.clone(), fell_over.clone());
        let entrance = make_wndproc();
        move |state, msg| {
            if msg.msg == WM_CAPTURECHANGED {
                seen.set(seen.get() + 1);
            }
            catch_unwind(AssertUnwindSafe(|| entrance(state, msg))).unwrap_or_else(|_| {
                fell_over.set(true);
                None
            })
        }
    });
    // SAFETY: Win32 境界。このスレッドの自分の窓に捕捉を持たせるだけ。
    unsafe { SetCapture(holder.hwnd()) };
    assert_eq!(
        capture_owner(),
        holder.hwnd(),
        "先に捕捉を持つ窓が捕捉している"
    );

    // 本番の押しは、World を可変で借りたまま start_preparing を呼ぶ（handle_button_message）。
    {
        let _held = rig.world.borrow_mut();
        start_preparing(rig.target, rig.screen(PRESS), rig.hwnd);
    }

    assert_eq!(
        seen.get(),
        1,
        "SetCapture が前の持ち主へ WM_CAPTURECHANGED を同期で送り、入口が受けた"
    );
    assert!(
        !fell_over.get(),
        "捕捉を取る間に届いた WM_CAPTURECHANGED を入口が受けても落ちない"
    );
    assert!(
        matches!(snapshot_drag_state(), DragStateSnapshot::Preparing { .. }),
        "押しの後は準備中: {:?}",
        snapshot_drag_state()
    );
    assert_eq!(capture_owner(), rig.hwnd, "押しの後は捕捉がその窓");
}
