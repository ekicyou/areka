//! ダブルクリックの 2 回目の押下（`WM_*BUTTONDBLCLK`）からドラッグの準備を始める決まりの決定論テスト。
//!
//! W1: ドラッグを許した窓へ左のダブルクリックの押下を入れると準備に入り、押した位置は画面座標。
//!     閾値を越えて動かせば開始が積まれ、越えずに離せば開始も終了も積まれない。
//! W2: ドラッグを許していない窓（`DragConfig` 無し・無効・左ボタン不可）と、右ボタンのダブルクリックでは
//!     準備に入らない。

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::message::Messages;
use bevy_ecs::prelude::{Entity, World};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDBLCLK,
};
use windows_numerics::Matrix3x2;

use crate::ecs::drag::{
    DragAccumulatorResource, DragConfig, DragEndEvent, DragStartEvent, DragStateSnapshot,
    cancel_dragging, dispatch_drag_events, snapshot_drag_state,
};
use crate::ecs::layout::GlobalArrangement;
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::window::{Window, WindowPos};
use crate::ecs::world::EcsWorld;
use crate::ecs::{Point, Rect, SizeI};
use crate::executor::util::WindowMessage;

/// 窓の左上の画面座標。
const WINDOW_X: i32 = 100;
const WINDOW_Y: i32 = 200;

fn arrangement(left: f32, top: f32, right: f32, bottom: f32) -> GlobalArrangement {
    GlobalArrangement {
        transform: Matrix3x2::translation(left, top),
        bounds: Rect {
            left,
            top,
            right,
            bottom,
        },
    }
}

/// 当たり判定の取れる窓（画面 (100,200) に 400x300）と、その子の部品（画面 (150,250)〜(250,300)）を作る。
/// `config` は窓に付ける（子の部品には付けない＝祖先を探す道を踏む）。返すのは (world, 窓)。
fn setup(config: Option<DragConfig>) -> (Rc<RefCell<EcsWorld>>, Entity) {
    // DRAG_STATE は thread_local。前のテストの残りを落としてから始める。
    cancel_dragging();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let window = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        w.insert_resource(DragAccumulatorResource::new());
        let window = w
            .spawn((
                Window::default(),
                arrangement(100.0, 200.0, 500.0, 500.0),
                WindowPos {
                    position: Some(Point {
                        x: WINDOW_X,
                        y: WINDOW_Y,
                    }),
                    size: Some(SizeI {
                        width: 400,
                        height: 300,
                    }),
                    ..Default::default()
                },
            ))
            .id();
        if let Some(config) = config {
            w.entity_mut(window).insert(config);
        }
        let widget = w.spawn(arrangement(150.0, 250.0, 250.0, 300.0)).id();
        w.entity_mut(window).add_children(&[widget]);
        window
    };
    (world, window)
}

/// クライアント座標 (x, y) のメッセージを wndproc の経路へ配る。
fn send(world: &Rc<RefCell<EcsWorld>>, window: Entity, msg: u32, x: i32, y: i32) {
    let message = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg,
        wparam: WPARAM(0),
        lparam: LPARAM((x as isize) | ((y as isize) << 16)),
    };
    crate::ecs::dispatch_window_message(world, window, &message);
}

/// 1 回の画面更新として配り、届いた開始・終了の知らせを読み出す。
fn dispatch_once(world: &Rc<RefCell<EcsWorld>>) -> (Vec<DragStartEvent>, Vec<DragEndEvent>) {
    let mut w = world.borrow_mut();
    let w = w.world_mut();
    dispatch_drag_events(w);
    (drain::<DragStartEvent>(w), drain::<DragEndEvent>(w))
}

fn drain<E: bevy_ecs::message::Message>(world: &mut World) -> Vec<E> {
    world.resource_mut::<Messages<E>>().drain().collect()
}

fn is_preparing() -> bool {
    matches!(snapshot_drag_state(), DragStateSnapshot::Preparing { .. })
}

/// W1 前半: 左のダブルクリックの押下で準備に入る。準備の対象は `DragConfig` を持つ祖先（窓）、
/// 押した位置はクライアント座標 (60,60) に窓の位置を足した画面座標 (160,260)。
/// 閾値（既定 5px）を越えて動かすと、開始の知らせが 1 件届く。
#[test]
fn w1_left_dblclick_prepares_and_crossing_threshold_starts() {
    let (world, window) = setup(Some(DragConfig::default()));

    send(&world, window, WM_LBUTTONDBLCLK, 60, 60);

    match snapshot_drag_state() {
        DragStateSnapshot::Preparing {
            entity, start_pos, ..
        } => {
            assert_eq!(entity, window, "準備の対象は DragConfig を持つ祖先の窓");
            assert_eq!(
                start_pos,
                PhysicalPoint::new(60 + WINDOW_X, 60 + WINDOW_Y),
                "押した位置は画面座標"
            );
        }
        other => panic!("左のダブルクリックの押下で準備に入るはず: {other:?}"),
    }

    send(&world, window, WM_MOUSEMOVE, 70, 60);
    let (starts, ends) = dispatch_once(&world);
    assert_eq!(
        starts.len(),
        1,
        "閾値を越えて動かすと開始の知らせは 1 件: {starts:?}"
    );
    assert_eq!(starts[0].target, window);
    assert_eq!(ends.len(), 0, "まだ離していないので終了は 0 件: {ends:?}");
}

/// W1 後半: 左のダブルクリックの押下から閾値を越えずに離すと、開始も終了も積まれない。
/// （準備に入ったことを先に確かめ、離しの後の 0 件が「準備しなかったから」でないことを示す。）
#[test]
fn w1_left_dblclick_released_without_move_dispatches_nothing() {
    let (world, window) = setup(Some(DragConfig::default()));

    send(&world, window, WM_LBUTTONDBLCLK, 60, 60);
    assert!(
        is_preparing(),
        "押下で準備に入る: {:?}",
        snapshot_drag_state()
    );

    send(&world, window, WM_MOUSEMOVE, 62, 61);
    send(&world, window, WM_LBUTTONUP, 62, 61);

    assert!(
        matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded {
                cancelled: false,
                ..
            }
        ),
        "閾値前の離しで状態は JustEnded{{cancelled:false}}: {:?}",
        snapshot_drag_state()
    );
    let (starts, ends) = dispatch_once(&world);
    assert_eq!(starts.len(), 0, "開始の知らせは 0 件: {starts:?}");
    assert_eq!(ends.len(), 0, "終了の知らせは 0 件: {ends:?}");
}

/// W2: ドラッグを許していない窓（`DragConfig` 無し・無効・左ボタン不可）では、
/// 左のダブルクリックの押下で準備に入らない。
#[test]
fn w2_left_dblclick_on_window_without_drag_does_not_prepare() {
    let cases = [
        ("DragConfig 無し", None),
        (
            "無効",
            Some(DragConfig {
                enabled: false,
                ..Default::default()
            }),
        ),
        (
            "左ボタン不可",
            Some(DragConfig {
                left_button: false,
                ..Default::default()
            }),
        ),
    ];
    for (label, config) in cases {
        let (world, window) = setup(config);
        send(&world, window, WM_LBUTTONDBLCLK, 60, 60);
        assert!(
            !is_preparing(),
            "{label}: 準備に入らない: {:?}",
            snapshot_drag_state()
        );
    }
}

/// W2: ドラッグを許した窓でも、右ボタンのダブルクリックの押下では準備に入らない。
#[test]
fn w2_right_dblclick_does_not_prepare() {
    let (world, window) = setup(Some(DragConfig {
        right_button: true,
        ..Default::default()
    }));

    send(&world, window, WM_RBUTTONDBLCLK, 60, 60);

    assert!(
        !is_preparing(),
        "右ボタンのダブルクリックでは準備に入らない: {:?}",
        snapshot_drag_state()
    );
}
