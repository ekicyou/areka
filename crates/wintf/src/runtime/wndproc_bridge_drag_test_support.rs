//! 入口（`make_wndproc` のクロージャ）を通る再入のドラッグのテストの、共通の組み立て。
//!
//! 再入＝画面更新が World を可変で借りている間に、OS から同期でメッセージが届くこと。
//! テストは `world.borrow_mut()` を握ったまま入口へメッセージを渡してその状態を作り、
//! 借用を返してから本物の知らせを配る段（`dispatch_drag_events`）を回して数える
//! （要件 8.1・8.5）。ハンドラを入口を通さずに直に呼ぶ形は使わない。
//!
//! 組み立て（[`Rig`]）: 本番の World（`EcsWorld::new()`）→ 対象（`Window`＋`DragConfig`）→
//! 押しは `start_preparing` を直に呼ぶ（押しの当たり判定はレイアウトが要り、押しの道は
//! 本 spec の対象外）→ 閾値を越える `WM_MOUSEMOVE` を入口から渡す（開始の種は本番の
//! `mouse_move.rs` が積む）→ 配る段を 1 回回して開始の知らせを配る。実時間の待機は 0 か所。
//!
//! 座標と捕捉を確かめるテストは [`Rig::with_real_window`] で、隠れた実物の窓を決まった
//! 位置に作る（窓の手続きは本番の入口そのもの）。テストは「取り消し」
//! （`wndproc_bridge_drag_cancel_tests.rs`）と「離し」（`wndproc_bridge_drag_release_tests.rs`）
//! の 2 ファイルに分けてある。テストの番号は design.md の Testing Strategy の表の番号。

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::{Rc, Weak};

use bevy_ecs::message::{Message, Messages};
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos, WINDOW_EX_STYLE, WM_ACTIVATE, WM_CANCELMODE,
    WM_CAPTURECHANGED, WM_KEYDOWN, WM_MOUSEMOVE,
};

use super::{WndState, make_wndproc};
use crate::ecs::Point;
use crate::ecs::drag::{
    DragAccumulatorResource, DragConfig, DragEndEvent, DragStartEvent, DragStateSnapshot,
    cancel_dragging, dispatch_drag_events, snapshot_drag_state, start_preparing,
};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::window::{Window, WindowHandle, WindowPos};
use crate::ecs::world::EcsWorld;
use crate::executor::util::{Window as LibWindow, WindowMessage, WindowType, get_instance_handle};

/// 押した位置（窓の中の座標）。画面の座標は [`Rig::screen`] で窓の原点を足す。
pub(super) const PRESS: PhysicalPoint = PhysicalPoint { x: 100, y: 100 };
/// 閾値（既定 5px）を十分に越えた位置（窓の中の座標）。
pub(super) const MOVED: PhysicalPoint = PhysicalPoint { x: 130, y: 140 };

/// 本番の入口のクロージャ（`make_wndproc` の戻り）。
type Entrance = Box<dyn Fn(Pin<&WndState>, WindowMessage) -> Option<LRESULT>>;

/// 本番の World・ドラッグの対象・その窓の入口を 1 組にしたもの。
pub(super) struct Rig {
    /// 隠れた実物の窓（[`Rig::with_real_window`] のときだけ）。World より先に落とす。
    window: Option<RealWindow>,
    pub world: Rc<RefCell<EcsWorld>>,
    /// ドラッグの対象であり、メッセージを受ける窓の entity（`Window`＋`DragConfig`）。
    pub target: Entity,
    /// メッセージに載せる窓（空の窓か、隠れた実物の窓）。
    pub hwnd: HWND,
    /// クライアント領域の左上の画面の座標（`WindowPos.position` と同じ値）。
    pub origin: PhysicalPoint,
    state: Pin<Box<WndState>>,
    entrance: Entrance,
}

impl Rig {
    /// 本番の World に対象を 1 つ置く。窓は空の hwnd・原点は画面の原点。
    /// ドラッグの状態はまだ押していない。
    pub fn new() -> Self {
        // DRAG_STATE は thread_local。前のテストの残りを休む状態へ落としてから始める。
        cancel_dragging();

        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let target = spawn_drag_window(&world, PhysicalPoint::new(0, 0));
        let state = Box::pin(WndState {
            world: Rc::downgrade(&world),
            entity: target,
        });
        Self {
            window: None,
            world,
            target,
            hwnd: HWND(std::ptr::null_mut()),
            origin: PhysicalPoint::new(0, 0),
            state,
            entrance: Box::new(make_wndproc()),
        }
    }

    /// 対象の窓を、画面の `at` に置いた隠れた実物の窓にする。窓の手続きは本番の入口で、
    /// `WindowPos.position` には窓のクライアント領域の左上の画面の座標を入れる
    /// （借用なしの道の画面の座標＝窓の中の座標＋この原点）。`WindowHandle` も付けるので、
    /// 配る段がドラッグ中の状態へこの窓の hwnd を写す（本番と同じ）。
    pub fn with_real_window(at: PhysicalPoint) -> Self {
        let mut rig = Self::new();
        let window = RealWindow::new(&rig.world, rig.target, at);
        rig.hwnd = window.hwnd();
        rig.origin = window.client_origin();
        rig.world
            .borrow_mut()
            .world_mut()
            .entity_mut(rig.target)
            .insert((
                WindowPos {
                    position: Some(Point {
                        x: rig.origin.x,
                        y: rig.origin.y,
                    }),
                    ..Default::default()
                },
                WindowHandle {
                    hwnd: rig.hwnd,
                    instance: get_instance_handle(),
                },
            ));
        rig.window = Some(window);
        rig
    }

    /// 窓の中の座標を画面の座標へ直す（原点を足す）。
    pub fn screen(&self, client: PhysicalPoint) -> PhysicalPoint {
        PhysicalPoint::new(client.x + self.origin.x, client.y + self.origin.y)
    }

    /// 入口へメッセージを渡す（対象の窓として・hwnd は [`Rig::hwnd`]）。
    pub fn send(&self, msg: u32, wparam: usize, lparam: isize) -> Option<LRESULT> {
        self.send_as(self.state.as_ref(), self.hwnd, msg, wparam, lparam)
    }

    /// 入口へメッセージを渡す（受ける窓の entity と hwnd を指定する）。
    pub fn send_as(
        &self,
        state: Pin<&WndState>,
        hwnd: HWND,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> Option<LRESULT> {
        (self.entrance)(
            state,
            WindowMessage {
                hwnd,
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

    /// 左ボタンの押し。状態へ直に入れる（捕捉は [`Rig::hwnd`] が取る）。
    pub fn press(&self) {
        start_preparing(self.target, self.screen(PRESS), self.hwnd);
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

/// ドラッグの対象になる窓の entity（`Window`＋`WindowPos`＋`DragConfig`）を置く。
pub(super) fn spawn_drag_window(world: &Rc<RefCell<EcsWorld>>, origin: PhysicalPoint) -> Entity {
    world
        .borrow_mut()
        .world_mut()
        .spawn((
            Window::default(),
            // クライアント領域の左上の画面の座標。既定の位置は CW_USEDEFAULT で、
            // 閾値の計算が桁あふれする。
            WindowPos {
                position: Some(Point {
                    x: origin.x,
                    y: origin.y,
                }),
                ..Default::default()
            },
            // ドラッグ中に SetWindowPos を呼ばせない（窓を動かすのは本 spec の対象外）。
            DragConfig {
                move_window: false,
                ..Default::default()
            },
        ))
        .id()
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
pub(super) fn rests_cancelled() -> bool {
    matches!(
        snapshot_drag_state(),
        DragStateSnapshot::JustEnded {
            cancelled: true,
            ..
        }
    )
}

// ============================================================================
// 隠れた実物の窓
// ============================================================================

/// 本番と同じライブラリの窓（`Window::new_ex`）を、本番の入口を窓の手続きにして作る。
/// 表示はしない（隠れたまま）。落とすと `DestroyWindow` する。
///
/// 作る・動かす・壊す間は World を借りておく。そのとき OS が同期で送るメッセージ
/// （`WM_CREATE`・`WM_WINDOWPOSCHANGED`・`WM_DESTROY` など）は、本番の `create_windows`
/// と同じく入口の安全スキップで捨てられ、World に何も書かない。
pub(super) struct RealWindow {
    window: Option<LibWindow<WndState>>,
    world: Weak<RefCell<EcsWorld>>,
}

/// 実物の窓の大きさ（物理ピクセル）。
const REAL_WINDOW_SIZE: i32 = 240;

impl RealWindow {
    /// 画面の `at` に、`entity` の窓として作る。
    pub fn new(world: &Rc<RefCell<EcsWorld>>, entity: Entity, at: PhysicalPoint) -> Self {
        Self::with_wndproc(world, entity, at, make_wndproc())
    }

    /// 窓の手続きを指定して作る（本番の入口を包んで、届いたメッセージを数えるときに使う）。
    pub fn with_wndproc(
        world: &Rc<RefCell<EcsWorld>>,
        entity: Entity,
        at: PhysicalPoint,
        wndproc: impl Fn(Pin<&WndState>, WindowMessage) -> Option<LRESULT> + 'static,
    ) -> Self {
        let _held = world.borrow_mut();
        let state = WndState {
            world: Rc::downgrade(world),
            entity,
        };
        let window = LibWindow::new_ex(WindowType::TopLevel, WINDOW_EX_STYLE(0), state, wndproc)
            .expect("隠れた実物の窓を作れる");
        // SAFETY: Win32 境界。作ったばかりの自分の窓を、表示も活性化もせずに置くだけ。
        unsafe {
            SetWindowPos(
                window.hwnd(),
                None,
                at.x,
                at.y,
                REAL_WINDOW_SIZE,
                REAL_WINDOW_SIZE,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .expect("窓を決まった位置へ置ける");
        }
        Self {
            window: Some(window),
            world: Rc::downgrade(world),
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.window.as_ref().expect("窓は生きている").hwnd()
    }

    /// クライアント領域の左上の画面の座標（OS から読む）。
    pub fn client_origin(&self) -> PhysicalPoint {
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: Win32 境界。生きている自分の窓の座標を読むだけ。
        let ok = unsafe { ClientToScreen(self.hwnd(), &mut p) };
        assert!(ok.as_bool(), "ClientToScreen が実物の窓で失敗した");
        PhysicalPoint::new(p.x, p.y)
    }
}

impl Drop for RealWindow {
    fn drop(&mut self) {
        // World が生きていれば借りてから壊す（壊すときのメッセージを入口で捨てさせる）。
        let world = self.world.upgrade();
        let _held = world.as_ref().and_then(|w| w.try_borrow_mut().ok());
        drop(self.window.take());
    }
}
