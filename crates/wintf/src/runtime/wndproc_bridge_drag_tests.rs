//! 入口（`make_wndproc` のクロージャ）を通る、再入のドラッグの決定論テスト。
//!
//! 再入＝画面更新が World を可変で借りている間に、OS から同期でメッセージが届くこと。
//! ここでは `world.borrow_mut()` を握ったまま入口へメッセージを渡してその状態を作り、
//! 借用を返してから本物の知らせを配る段（`dispatch_drag_events`）を回して数える
//! （要件 8.1・8.5）。ハンドラを入口を通さずに直に呼ぶ形は使わない。
//!
//! 組み立て（[`Rig`]）: 本番の World（`EcsWorld::new()`）→ 対象（`Window`＋`DragConfig`）→
//! 押しは `start_preparing` を直に呼ぶ（押しの当たり判定はレイアウトが要り、押しの道は
//! 本 spec の対象外）→ 閾値を越える `WM_MOUSEMOVE` を入口から渡す（開始の種は本番の
//! `mouse_move.rs` が積む）→ 配る段を 1 回回して開始の知らせを配る。実時間の待機は 0 か所。
//!
//! テストの番号は design.md の Testing Strategy「入口を通る再入のテスト」の表の番号。
//! 起床の旗を読むテスト（7・7b）は共有の錠 `TICK_WAKE_TEST_LOCK` を毒化に耐える取り方で取る。

use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::rc::Rc;
use std::time::Instant;

use bevy_ecs::message::{Message, Messages};
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    WM_ACTIVATE, WM_CANCELMODE, WM_CAPTURECHANGED, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE,
};

use super::{WndState, make_wndproc};
use crate::ecs::Point;
use crate::ecs::drag::{
    DragAccumulatorResource, DragConfig, DragEndEvent, DragStartEvent, DragStateSnapshot,
    DraggingState, WindowDragging, cancel_dragging, dispatch_drag_events, snapshot_drag_state,
    start_preparing,
};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::window::{KeepDirectlyAbove, SinkObservationPending, Window, WindowPos};
use crate::ecs::world::{EcsWorld, FrameCount, UISetup, tick_wake};
use crate::executor::util::WindowMessage;

// ============================================================================
// 共通の組み立て
// ============================================================================

/// 押した位置（画面の座標）。窓の `WindowPos.position` は原点なので窓の中の座標と同じ値。
const PRESS: PhysicalPoint = PhysicalPoint { x: 100, y: 100 };
/// 閾値（既定 5px）を十分に越えた位置。
const MOVED: PhysicalPoint = PhysicalPoint { x: 130, y: 140 };

/// 本番の入口のクロージャ（`make_wndproc` の戻り）。
type Entrance = Box<dyn Fn(Pin<&WndState>, WindowMessage) -> Option<LRESULT>>;

/// 本番の World・ドラッグの対象・その窓の入口を 1 組にしたもの。
pub(super) struct Rig {
    pub world: Rc<RefCell<EcsWorld>>,
    /// ドラッグの対象であり、メッセージを受ける窓の entity（`Window`＋`DragConfig`）。
    pub target: Entity,
    state: Pin<Box<WndState>>,
    entrance: Entrance,
}

impl Rig {
    /// 本番の World に対象を 1 つ置く。ドラッグの状態はまだ押していない。
    pub fn new() -> Self {
        // DRAG_STATE は thread_local。前のテストの残りを休む状態へ落としてから始める。
        cancel_dragging();

        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let target = world
            .borrow_mut()
            .world_mut()
            .spawn((
                Window::default(),
                // クライアント領域の左上を画面の原点に置く（窓の中の座標＝画面の座標）。
                // 既定の位置は CW_USEDEFAULT で、閾値の計算が桁あふれする。
                WindowPos {
                    position: Some(Point { x: 0, y: 0 }),
                    ..Default::default()
                },
                // 窓は空の hwnd なので、ドラッグ中に SetWindowPos を呼ばせない。
                DragConfig {
                    move_window: false,
                    ..Default::default()
                },
            ))
            .id();
        let state = Box::pin(WndState {
            world: Rc::downgrade(&world),
            entity: target,
        });
        Self {
            world,
            target,
            state,
            entrance: Box::new(make_wndproc()),
        }
    }

    /// 入口へメッセージを渡す（hwnd は空。座標を確かめるテストは隠れた実物の窓を使う）。
    pub fn send(&self, msg: u32, wparam: usize, lparam: isize) -> Option<LRESULT> {
        (self.entrance)(
            self.state.as_ref(),
            WindowMessage {
                hwnd: HWND(std::ptr::null_mut()),
                msg,
                wparam: WPARAM(wparam),
                lparam: LPARAM(lparam),
            },
        )
    }

    /// World を可変で借りたまま（＝再入の状態で）入口へ渡し、借用を返す。
    pub fn send_while_borrowed(&self, msg: u32, wparam: usize, lparam: isize) -> Option<LRESULT> {
        let _held = self.world.borrow_mut();
        self.send(msg, wparam, lparam)
    }

    /// 左ボタンの押し。状態へ直に入れる。
    pub fn press(&self) {
        start_preparing(self.target, PRESS, HWND::default());
        assert!(
            matches!(snapshot_drag_state(), DragStateSnapshot::Preparing { .. }),
            "押しの後は準備中: {:?}",
            snapshot_drag_state()
        );
    }

    /// 閾値を越える動きを入口から渡す（借用なし＝ふつうの道）。
    pub fn cross_threshold(&self) {
        self.send(WM_MOUSEMOVE, 0, client_lparam(MOVED));
        assert!(
            matches!(snapshot_drag_state(), DragStateSnapshot::JustStarted { .. }),
            "閾値を越えた後は開始直後: {:?}",
            snapshot_drag_state()
        );
    }

    /// 押して閾値を越え、配る段を 1 回回して開始の知らせを配り終えた状態にする。
    pub fn start_drag(&self) {
        self.press();
        self.cross_threshold();
        let starts = self.dispatch::<DragStartEvent>();
        assert_eq!(starts.len(), 1, "開始の知らせは 1 件: {starts:?}");
        assert!(
            matches!(snapshot_drag_state(), DragStateSnapshot::Dragging { .. }),
            "開始を配った後はドラッグ中: {:?}",
            snapshot_drag_state()
        );
    }

    /// 配る段を 1 回回し、届いた知らせ `E` を読み出す。
    pub fn dispatch<E: Message>(&self) -> Vec<E> {
        let mut w = self.world.borrow_mut();
        let w = w.world_mut();
        dispatch_drag_events(w);
        w.resource_mut::<Messages<E>>().drain().collect()
    }

    /// 配る段を 1 回回し、届いた終了の知らせを読み出す。
    pub fn dispatch_ends(&self) -> Vec<DragEndEvent> {
        self.dispatch::<DragEndEvent>()
    }

    /// World の資源の累積器（同じ実体の複製）。
    pub fn accumulator(&self) -> DragAccumulatorResource {
        self.world
            .borrow()
            .world()
            .resource::<DragAccumulatorResource>()
            .clone()
    }

    /// 対象が部品 `C` を持っているか。
    pub fn target_has<C: bevy_ecs::component::Component>(&self) -> bool {
        self.world
            .borrow()
            .world()
            .entity(self.target)
            .contains::<C>()
    }
}

/// 窓の中の座標を `WM_MOUSEMOVE`／ボタンの lParam の形へ詰める。
pub(super) fn client_lparam(p: PhysicalPoint) -> isize {
    ((p.y as u16 as isize) << 16) | (p.x as u16 as isize)
}

/// 取り消しの 4 種（ESC の押下・メニューやダイアログの割り込み・非活性化・捕捉の喪失）。
#[derive(Clone, Copy)]
pub(super) struct Cancel {
    pub what: &'static str,
    pub msg: u32,
    pub wparam: usize,
}

pub(super) const ESC: Cancel = Cancel {
    what: "ESC の押下（WM_KEYDOWN）",
    msg: WM_KEYDOWN,
    wparam: VK_ESCAPE.0 as usize,
};
pub(super) const CANCELMODE: Cancel = Cancel {
    what: "メニューやダイアログの割り込み（WM_CANCELMODE）",
    msg: WM_CANCELMODE,
    wparam: 0,
};
pub(super) const DEACTIVATE: Cancel = Cancel {
    what: "非活性化（WM_ACTIVATE・WA_INACTIVE）",
    msg: WM_ACTIVATE,
    wparam: 0, // 下位の語が WA_INACTIVE
};
pub(super) const CAPTURE_LOST: Cancel = Cancel {
    what: "捕捉の喪失（WM_CAPTURECHANGED）",
    msg: WM_CAPTURECHANGED,
    wparam: 0,
};
pub(super) const CANCELS: [Cancel; 4] = [ESC, CANCELMODE, DEACTIVATE, CAPTURE_LOST];

/// 状態が取り消しの印つきで休んでいるか。
fn rests_cancelled() -> bool {
    matches!(
        snapshot_drag_state(),
        DragStateSnapshot::JustEnded {
            cancelled: true,
            ..
        }
    )
}

// ============================================================================
// テスト 1: 閾値を越えた後の取り消し 4 種で、終了の知らせが取り消しの印つきで 1 件
// （要件 1.1・2.1・2.3）
// ============================================================================

fn cancel_after_threshold_ends_once(kind: Cancel) {
    let rig = Rig::new();
    rig.start_drag();

    let ret = rig.send_while_borrowed(kind.msg, kind.wparam, 0);
    assert!(ret.is_none(), "{}: 入口は既定の手続きへ委ねる", kind.what);
    assert!(
        rests_cancelled(),
        "{}: 再入で届いてもドラッグは取り消しの印つきで休む: {:?}",
        kind.what,
        snapshot_drag_state()
    );

    let ends = rig.dispatch_ends();
    assert_eq!(
        ends.len(),
        1,
        "{}: 再入の取り消しでも終了の知らせは 1 件: {ends:?}",
        kind.what
    );
    let end = &ends[0];
    assert_eq!(end.target, rig.target, "{}: 対象", kind.what);
    assert!(end.cancelled, "{}: 取り消しの印つき", kind.what);
    assert_eq!(end.position, PRESS, "{}: 位置は押した位置", kind.what);
}

#[test]
fn t01_reentrant_esc_after_threshold_ends_once_cancelled() {
    cancel_after_threshold_ends_once(ESC);
}

#[test]
fn t01_reentrant_cancelmode_after_threshold_ends_once_cancelled() {
    cancel_after_threshold_ends_once(CANCELMODE);
}

#[test]
fn t01_reentrant_deactivate_after_threshold_ends_once_cancelled() {
    cancel_after_threshold_ends_once(DEACTIVATE);
}

#[test]
fn t01_reentrant_capture_lost_after_threshold_ends_once_cancelled() {
    cancel_after_threshold_ends_once(CAPTURE_LOST);
}

// ============================================================================
// テスト 3: 取り消しの終了の知らせを配った後、ドラッグ中の印が外れている（要件 2.5）
// ============================================================================

fn cancel_after_threshold_clears_drag_markers(kind: Cancel) {
    let rig = Rig::new();
    rig.start_drag();
    assert!(
        rig.target_has::<DraggingState>() && rig.target_has::<WindowDragging>(),
        "開始を配った後はドラッグ中の印が付いている"
    );

    rig.send_while_borrowed(kind.msg, kind.wparam, 0);
    rig.dispatch_ends();

    assert!(
        !rig.target_has::<DraggingState>(),
        "{}: 配った後 DraggingState が外れている",
        kind.what
    );
    assert!(
        !rig.target_has::<WindowDragging>(),
        "{}: 配った後 WindowDragging が外れている",
        kind.what
    );
}

#[test]
fn t03_reentrant_esc_clears_drag_markers() {
    cancel_after_threshold_clears_drag_markers(ESC);
}

#[test]
fn t03_reentrant_cancelmode_clears_drag_markers() {
    cancel_after_threshold_clears_drag_markers(CANCELMODE);
}

#[test]
fn t03_reentrant_deactivate_clears_drag_markers() {
    cancel_after_threshold_clears_drag_markers(DEACTIVATE);
}

#[test]
fn t03_reentrant_capture_lost_clears_drag_markers() {
    cancel_after_threshold_clears_drag_markers(CAPTURE_LOST);
}

// ============================================================================
// テスト 4: 閾値前の取り消し 4 種で、状態が休み、終了の知らせ 0 件（要件 3.2）
// ============================================================================

fn cancel_before_threshold_rests_without_end(kind: Cancel) {
    let rig = Rig::new();
    rig.press();

    rig.send_while_borrowed(kind.msg, kind.wparam, 0);
    assert!(
        rests_cancelled(),
        "{}: 閾値前でも再入の取り消しで状態は休む: {:?}",
        kind.what,
        snapshot_drag_state()
    );

    let ends = rig.dispatch_ends();
    assert_eq!(
        ends.len(),
        0,
        "{}: 閾値前の取り消しで終了の知らせは 0 件: {ends:?}",
        kind.what
    );
}

#[test]
fn t04_reentrant_esc_before_threshold_rests_without_end() {
    cancel_before_threshold_rests_without_end(ESC);
}

#[test]
fn t04_reentrant_cancelmode_before_threshold_rests_without_end() {
    cancel_before_threshold_rests_without_end(CANCELMODE);
}

#[test]
fn t04_reentrant_deactivate_before_threshold_rests_without_end() {
    cancel_before_threshold_rests_without_end(DEACTIVATE);
}

#[test]
fn t04_reentrant_capture_lost_before_threshold_rests_without_end() {
    cancel_before_threshold_rests_without_end(CAPTURE_LOST);
}

// ============================================================================
// テスト 7: 再入の終了の知らせは、借用を返した後の 1 回目の配る段で 1 件・2 回目は 0 件。
// 借用なしで渡した場合も同じ 1 回目。再入の扱いの後、起床の旗が立つ（要件 2.4・8.3）
// ============================================================================

/// 閾値を越えたドラッグへ取り消しを渡し、配る段 2 回の件数を返す。
fn ends_per_dispatch(borrowed: bool) -> (usize, usize) {
    let rig = Rig::new();
    rig.start_drag();
    if borrowed {
        rig.send_while_borrowed(CANCELMODE.msg, CANCELMODE.wparam, 0);
    } else {
        rig.send(CANCELMODE.msg, CANCELMODE.wparam, 0);
    }
    (rig.dispatch_ends().len(), rig.dispatch_ends().len())
}

#[test]
fn t07_reentrant_end_is_dispatched_on_the_first_dispatch_like_the_ordinary_path() {
    let _lock = crate::ecs::world::TICK_WAKE_TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());

    // 比べる相手: 借用なしで渡した場合は 1 回目で 1 件、2 回目は 0 件。
    assert_eq!(ends_per_dispatch(false), (1, 0), "借用なしの道");

    // 再入: 旗を掃いてから渡し、扱いの後に旗が立っていることを見る。
    let rig = Rig::new();
    rig.start_drag();
    let _ = tick_wake::take(Instant::now());
    rig.send_while_borrowed(CANCELMODE.msg, CANCELMODE.wparam, 0);
    let woke = tick_wake::take(Instant::now());

    let first = rig.dispatch_ends().len();
    let second = rig.dispatch_ends().len();
    assert_eq!(
        (first, second),
        (1, 0),
        "再入の終了の知らせは借用を返した後の 1 回目の配る段で 1 件・2 回目は 0 件"
    );
    let expected = tick_wake::wake_bits_for_message(CANCELMODE.msg);
    assert!(
        woke.bits & expected.bits() == expected.bits(),
        "再入の扱いの後、配送表の入口と同じ起床の旗が立っている: {:?}",
        woke.bits
    );
}

// ============================================================================
// テスト 7b: 本番の並び。本物の try_tick_world を 2 回回し、1 回目の途中（UISetup）から
// 非活性化を入口へ渡す（要件 2.4・2.5・8.3）
// ============================================================================

thread_local! {
    /// UISetup のテスト用システムが呼ぶ入口（組み立てごと預ける）。
    static STASHED: RefCell<Option<Rc<Rig>>> = const { RefCell::new(None) };
    /// 真のときだけ、次の UISetup で非活性化を 1 回渡す。
    static ARMED: Cell<bool> = const { Cell::new(false) };
    /// テスト用システムが実際に非活性化を渡した回数（空振りの見逃しを防ぐ）。
    static FIRED: Cell<u32> = const { Cell::new(0) };
}

/// UISetup（メインスレッド固定）に置くテスト用システム。画面更新の途中で入口を呼ぶ。
fn deactivate_through_the_entrance_mid_tick() {
    if !ARMED.with(|a| a.replace(false)) {
        return;
    }
    // 預け先の借用は複製を取る間だけ（入口を呼ぶ間は握らない）。
    let rig = STASHED.with(|s| s.borrow().clone());
    if let Some(rig) = rig {
        rig.send(DEACTIVATE.msg, DEACTIVATE.wparam, 0);
        FIRED.with(|f| f.set(f.get() + 1));
    }
}

/// 本番と同じく World を可変で借りたまま 1 回の画面更新を回し、届いた終了の知らせを読む。
fn tick(rig: &Rig) -> Vec<DragEndEvent> {
    let mut w = rig.world.borrow_mut();
    assert!(w.try_tick_world(), "画面更新が回る");
    w.world_mut()
        .resource_mut::<Messages<DragEndEvent>>()
        .drain()
        .collect()
}

/// テスト用システムを UISetup へ置いた組み立てを作り、ドラッグ中にして預ける。
fn stashed_dragging_rig() -> Rc<Rig> {
    let rig = Rc::new(Rig::new());
    rig.world
        .borrow_mut()
        .add_systems(UISetup, deactivate_through_the_entrance_mid_tick);
    rig.start_drag();
    STASHED.with(|s| *s.borrow_mut() = Some(rig.clone()));
    FIRED.with(|f| f.set(0));
    rig
}

#[test]
fn t07b_deactivate_inside_a_real_tick_ends_on_the_next_tick() {
    let _lock = crate::ecs::world::TICK_WAKE_TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());

    // 比べる相手: 同じ非活性化を 1 回目と 2 回目の間に借用なしで渡すと、2 回目で 1 件。
    {
        let rig = stashed_dragging_rig();
        assert_eq!(tick(&rig).len(), 0, "借用なしの道: 1 回目は何も起きない");
        rig.send(DEACTIVATE.msg, DEACTIVATE.wparam, 0);
        assert_eq!(tick(&rig).len(), 1, "借用なしの道: 2 回目で 1 件");
        STASHED.with(|s| *s.borrow_mut() = None);
    }

    let rig = stashed_dragging_rig();
    let _ = tick_wake::take(Instant::now());
    ARMED.with(|a| a.set(true));

    // 1 回目: 途中の UISetup から非活性化が入口へ届く（World は借用中＝再入）。
    let first = tick(&rig);
    assert_eq!(
        FIRED.with(|f| f.get()),
        1,
        "テスト用システムが 1 回目の途中で入口を呼んだ"
    );
    let woke = tick_wake::take(Instant::now());
    assert_eq!(
        first.len(),
        0,
        "1 回目の終わりでは終了の知らせ 0 件: {first:?}"
    );
    assert!(
        rests_cancelled(),
        "1 回目の終わりで状態は取り消しの印つきで休んでいる: {:?}",
        snapshot_drag_state()
    );
    let expected = tick_wake::wake_bits_for_message(DEACTIVATE.msg);
    assert!(
        woke.bits & expected.bits() == expected.bits(),
        "1 回目の終わりで再入の扱いの起床の旗が立っている: {:?}",
        woke.bits
    );

    // 2 回目: 終了の知らせ 1 件・ドラッグ中の印が外れる。
    let second = tick(&rig);
    assert_eq!(second.len(), 1, "2 回目で終了の知らせ 1 件: {second:?}");
    assert!(second[0].cancelled, "取り消しの印つき");
    assert_eq!(second[0].target, rig.target, "対象");
    assert!(
        !rig.target_has::<DraggingState>(),
        "2 回目の後 DraggingState が外れている"
    );
    assert!(
        !rig.target_has::<WindowDragging>(),
        "2 回目の後 WindowDragging が外れている"
    );

    STASHED.with(|s| *s.borrow_mut() = None);
}

// ============================================================================
// テスト 8: 5 種でないメッセージは、借用中は状態も累積器も変えず None・
// `drag_reentry_handled` の記録 0 件（要件 1.4・6.3）
// ============================================================================

/// 5 種に入らないメッセージ（押す・動かす・ESC でないキー・活性化）。
const NOT_DRAG_MESSAGES: [(&str, u32, usize); 4] = [
    ("WM_MOUSEMOVE", WM_MOUSEMOVE, 0),
    ("WM_LBUTTONDOWN", WM_LBUTTONDOWN, 0),
    ("ESC でない WM_KEYDOWN（A）", WM_KEYDOWN, 0x41),
    ("活性化の WM_ACTIVATE（WA_ACTIVE）", WM_ACTIVATE, 1),
];

/// 今の状態のまま 5 種でないメッセージを借用中に渡し、何も変わらないことを確かめる。
fn not_drag_messages_change_nothing(rig: &Rig, phase: &str) {
    let acc = rig.accumulator();
    // 前に積まれた分を掃き、以後に積まれたものだけを見る。
    let before_flush = acc.flush().expect("累積器の錠は毒化していない");
    let before_state = format!("{:?}", snapshot_drag_state());

    let ((), events) = log_capture_kit::capture(|| {
        for (what, msg, wparam) in NOT_DRAG_MESSAGES {
            // 閾値を越える位置を渡す（閾値と比べたなら開始してしまう形）。
            let ret = rig.send_while_borrowed(msg, wparam, client_lparam(MOVED));
            assert!(ret.is_none(), "{phase}: {what} は借用中は None");
        }
    });

    assert_eq!(
        format!("{:?}", snapshot_drag_state()),
        before_state,
        "{phase}: 5 種でないメッセージは状態を変えない"
    );
    let after = acc.flush().expect("累積器の錠は毒化していない");
    assert!(
        after.transitions.is_empty(),
        "{phase}: 遷移は積まれない: {:?}",
        after.transitions
    );
    assert_eq!(
        after.delta,
        PhysicalPoint::new(0, 0),
        "{phase}: 動きは積まれない"
    );
    assert_eq!(
        after.current_dragging_entity, before_flush.current_dragging_entity,
        "{phase}: ドラッグ中の対象は変わらない"
    );
    assert_eq!(
        after.current_position, before_flush.current_position,
        "{phase}: 位置は変わらない"
    );
    let handled = events
        .iter()
        .filter(|e| e.field_str("event") == Some("drag_reentry_handled"))
        .count();
    assert_eq!(handled, 0, "{phase}: drag_reentry_handled の記録は 0 件");
}

#[test]
fn t08_non_drag_messages_while_borrowed_change_nothing() {
    // 準備中（閾値と比べる前）。
    let rig = Rig::new();
    rig.press();
    not_drag_messages_change_nothing(&rig, "準備中");

    // ドラッグ中（動きを積む状態）。
    let rig = Rig::new();
    rig.start_drag();
    not_drag_messages_change_nothing(&rig, "ドラッグ中");
}

// ============================================================================
// テスト 13: 再入の扱いは None を返し、非活性化でも沈降の観測の目印が付かない。
// 借用は握られたまま。画面更新の回数は増えない（要件 1.2・1.3）
// ============================================================================

#[test]
fn t13_reentrant_handling_returns_none_keeps_the_borrow_and_skips_side_work() {
    let release = Cancel {
        what: "左ボタンを離す（WM_LBUTTONUP）",
        msg: WM_LBUTTONUP,
        wparam: 0,
    };
    for kind in CANCELS.into_iter().chain([release]) {
        let rig = Rig::new();
        // 対象の窓をペアの相手に持つ宣言側（非活性化の目印はここへ付く）。
        let balloon = rig
            .world
            .borrow_mut()
            .world_mut()
            .spawn(KeepDirectlyAbove { peer: rig.target })
            .id();
        rig.start_drag();
        let frames = rig.world.borrow().world().resource::<FrameCount>().0;

        {
            let _held = rig.world.borrow_mut();
            let ret = rig.send(kind.msg, kind.wparam, client_lparam(MOVED));
            assert!(ret.is_none(), "{}: 再入の扱いは None を返す", kind.what);
            assert!(
                rig.world.try_borrow().is_err(),
                "{}: 扱いの後も借用は握られたまま（横取りしない）",
                kind.what
            );
        }

        let w = rig.world.borrow();
        assert_eq!(
            w.world().resource::<FrameCount>().0,
            frames,
            "{}: 画面更新の回数は増えない",
            kind.what
        );
        assert!(
            !w.world()
                .entity(balloon)
                .contains::<SinkObservationPending>(),
            "{}: 再入では沈降の観測の目印が付かない",
            kind.what
        );
    }

    // 対照: 借用なしの非活性化では目印が付く（付かない側の主張が恒真でない）。
    let rig = Rig::new();
    let balloon = rig
        .world
        .borrow_mut()
        .world_mut()
        .spawn(KeepDirectlyAbove { peer: rig.target })
        .id();
    rig.send(DEACTIVATE.msg, DEACTIVATE.wparam, 0);
    assert!(
        rig.world
            .borrow()
            .world()
            .entity(balloon)
            .contains::<SinkObservationPending>(),
        "借用なしの非活性化では目印が付く"
    );
}
