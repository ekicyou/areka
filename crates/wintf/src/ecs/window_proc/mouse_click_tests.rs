//! 左解放（`WM_LBUTTONUP`）を wndproc の経路で配ったときに届くドラッグの知らせの決定論テスト。
//!
//! 知らせの件数は `dispatch_drag_events` を 1 回呼んだ後の `Messages<DragStartEvent>`／
//! `Messages<DragEndEvent>` を `drain` して数える。累積器の返す形（`FlushResult`）には触らない。

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use bevy_ecs::message::Messages;
use bevy_ecs::prelude::{Entity, World};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP;

use crate::ecs::drag::{
    DragAccumulatorResource, DragEndEvent, DragStartEvent, DragStateSnapshot, DragTransition,
    cancel_dragging, dispatch_drag_events, snapshot_drag_state, start_dragging, start_preparing,
};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::window::Window;
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;

/// 窓の印（`Window`）を持つ entity を 1 つ作り、World の累積器の複製と一緒に返す。
///
/// 累積器は `EcsWorld::new` が資源と wndproc 側の控えへ同じ実体で置いたものを使う
/// （入れ直すと控えとずれ、休ませる関数が積む終了の種が見えなくなる）。
///
/// 素の `EcsWorld` では当たり判定が取れないので、左解放は `handle_button_message` の予備の枝を通る。
/// 離しを終えるかは `end_dragging_on_release` が「離した窓の `hwnd` が捕捉を取った窓の `hwnd` と
/// 同じか」で決めるため、押し（`start_preparing`）と離しのメッセージに同じ `hwnd`（空）を渡す。
fn setup() -> (Rc<RefCell<EcsWorld>>, Entity, DragAccumulatorResource) {
    // DRAG_STATE は thread_local。前のテストの残りを JustEnded へ落としてから始める。
    cancel_dragging();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (entity, accumulator) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        let accumulator = w.resource::<DragAccumulatorResource>().clone();
        (w.spawn(Window::default()).id(), accumulator)
    };
    (world, entity, accumulator)
}

/// 左解放をクライアント座標 (x, y) で wndproc の経路へ配る。
fn release_left(world: &Rc<RefCell<EcsWorld>>, entity: Entity, x: i32, y: i32) {
    let message = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg: WM_LBUTTONUP,
        wparam: WPARAM(0),
        lparam: LPARAM((x as isize) | ((y as isize) << 16)),
    };
    crate::ecs::dispatch_window_message(world, entity, &message);
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

/// 閾値に届かずに離した（クリックだけの）ときは、終了の知らせを配らない。
/// ドラッグの状態は今どおり離しの印つきの休む状態（`JustEnded { cancelled: false }`）になる。
#[test]
fn release_before_threshold_dispatches_no_drag_end() {
    let (world, entity, _accumulator) = setup();

    start_preparing(entity, PhysicalPoint::new(10, 20), HWND::default());
    release_left(&world, entity, 10, 20);

    assert!(
        matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded {
                cancelled: false,
                ..
            }
        ),
        "閾値前の左解放で状態は JustEnded{{cancelled:false}} になる: {:?}",
        snapshot_drag_state()
    );

    let (starts, ends) = dispatch_once(&world);
    assert_eq!(
        starts.len(),
        0,
        "閾値前の左解放で開始の知らせは届かない: {starts:?}"
    );
    assert_eq!(
        ends.len(),
        0,
        "閾値前の左解放（クリックだけ）で終了の知らせは 0 件のはず: {ends:?}"
    );
}

/// 閾値を越えた直後（`JustStarted`）に離したときは、同じ画面更新の中で
/// 開始の知らせと終了の知らせが 1 件ずつ届き、状態は `JustEnded { cancelled: false }` になる。
#[test]
fn release_after_threshold_dispatches_start_and_end_once_each() {
    let (world, entity, accumulator) = setup();

    let start_pos = PhysicalPoint::new(10, 20);
    start_preparing(entity, start_pos, HWND::default());
    start_dragging(PhysicalPoint::new(40, 50));
    // mouse_move.rs が閾値越えで積む開始の種の代わりに、手で積む。
    accumulator.set_transition(DragTransition::Started {
        entity,
        start_pos,
        timestamp: Instant::now(),
    });
    release_left(&world, entity, 40, 50);

    assert!(
        matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded {
                cancelled: false,
                ..
            }
        ),
        "閾値後の左解放で状態は JustEnded{{cancelled:false}} になる: {:?}",
        snapshot_drag_state()
    );

    let (starts, ends) = dispatch_once(&world);
    assert_eq!(
        ends.len(),
        1,
        "閾値後の左解放で終了の知らせは 1 件: {ends:?}"
    );
    assert!(
        !ends[0].cancelled && ends[0].target == entity,
        "終了の知らせは中断でなく、対象はドラッグした entity: {:?}",
        ends[0]
    );
    assert_eq!(
        starts.len(),
        1,
        "同じ画面更新で積まれた開始の種も配られ、開始の知らせは 1 件のはず: {starts:?}"
    );
    assert_eq!(starts[0].target, entity);
}
