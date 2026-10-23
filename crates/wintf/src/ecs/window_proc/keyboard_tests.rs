//! `WM_ACTIVATE` の非活性化がドラッグを中断する範囲と、閾値前（準備中）の取り消しで
//! 終了の知らせを配らないことの決定論テスト。
//!
//! 知らせの件数は `dispatch_drag_events` を 1 回呼んだ後の `Messages<DragEndEvent>` を
//! `drain` して数える。累積器の返す形（`FlushResult`）には触らない。

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use bevy_ecs::message::Messages;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    WM_ACTIVATE, WM_CANCELMODE, WM_CAPTURECHANGED, WM_KEYDOWN,
};

use crate::ecs::drag::{
    DragAccumulatorResource, DragEndEvent, DragStateSnapshot, DragTransition, cancel_dragging,
    dispatch_drag_events, snapshot_drag_state, start_dragging, start_preparing,
};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;

/// 閾値到達の直後（`JustStarted`＝次の tick の配送前）に窓が非アクティブになっても、
/// `Dragging` と同じく `Ended { cancelled: true }` を積んでからドラッグを中断する。
#[test]
fn deactivation_right_after_the_threshold_cancels_the_drag() {
    // DRAG_STATE は thread_local。前のテストの残りを JustEnded へ落としてから始める。
    cancel_dragging();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (entity, accumulator) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        // EcsWorld::new が資源と wndproc 側の控えへ同じ累積器を置いている。入れ直すと控えとずれるので、
        // 資源の複製（同じ実体）を使う。
        let accumulator = w.resource::<DragAccumulatorResource>().clone();
        (w.spawn_empty().id(), accumulator)
    };

    start_preparing(entity, PhysicalPoint::new(10, 20), HWND::default());
    start_dragging(PhysicalPoint::new(40, 50));
    assert!(matches!(
        snapshot_drag_state(),
        DragStateSnapshot::JustStarted { .. }
    ));
    // mouse_move.rs が閾値越えで積む開始の種の代わりに手で積み、配った後の遷移だけを見る。
    accumulator.set_transition(DragTransition::Started {
        entity,
        start_pos: PhysicalPoint::new(10, 20),
        timestamp: Instant::now(),
    });
    accumulator.flush();

    let message = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg: WM_ACTIVATE,
        wparam: WPARAM(0), // WA_INACTIVE
        lparam: LPARAM(0),
    };
    crate::ecs::dispatch_window_message(&world, entity, &message);

    assert!(
        matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded {
                cancelled: true,
                ..
            }
        ),
        "JustStarted のまま非活性化してもドラッグは中断される"
    );
    let transitions = accumulator
        .flush()
        .map(|flushed| flushed.transitions)
        .unwrap_or_default();
    assert!(
        matches!(
            transitions.as_slice(),
            [DragTransition::Ended {
                cancelled: true,
                ..
            }]
        ),
        "中断の遷移が積まれている: {transitions:?}"
    );
}

/// 準備に入った（閾値前の）窓を 1 つ用意する。
fn setup_preparing() -> (Rc<RefCell<EcsWorld>>, Entity) {
    // DRAG_STATE は thread_local。前のテストの残りを JustEnded へ落としてから始める。
    cancel_dragging();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    // 累積器は EcsWorld::new が資源と wndproc 側の控えへ置いたものをそのまま使う（入れ直すと控えとずれる）。
    let entity = world.borrow_mut().world_mut().spawn_empty().id();
    start_preparing(entity, PhysicalPoint::new(10, 20), HWND::default());
    assert!(matches!(
        snapshot_drag_state(),
        DragStateSnapshot::Preparing { .. }
    ));
    (world, entity)
}

/// メッセージを wndproc の経路へ配る。
fn send(world: &Rc<RefCell<EcsWorld>>, entity: Entity, msg: u32, wparam: usize) {
    let message = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg,
        wparam: WPARAM(wparam),
        lparam: LPARAM(0),
    };
    crate::ecs::dispatch_window_message(world, entity, &message);
}

/// 1 回の画面更新として配り、届いた終了の知らせを読み出す。
fn dispatch_once(world: &Rc<RefCell<EcsWorld>>) -> Vec<DragEndEvent> {
    let mut w = world.borrow_mut();
    let w = w.world_mut();
    dispatch_drag_events(w);
    w.resource_mut::<Messages<DragEndEvent>>().drain().collect()
}

/// 準備中の取り消しは状態を取り消しの印つきの休む状態へ落とし、終了の知らせを配らない。
fn assert_cancel_before_threshold_dispatches_no_drag_end(msg: u32, wparam: usize, what: &str) {
    let (world, entity) = setup_preparing();
    send(&world, entity, msg, wparam);

    assert!(
        matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded {
                cancelled: true,
                ..
            }
        ),
        "閾値前の{what}で状態は JustEnded{{cancelled:true}} になる: {:?}",
        snapshot_drag_state()
    );
    let ends = dispatch_once(&world);
    assert_eq!(
        ends.len(),
        0,
        "閾値前の{what}で終了の知らせは 0 件のはず: {ends:?}"
    );
}

#[test]
fn esc_before_threshold_dispatches_no_drag_end() {
    assert_cancel_before_threshold_dispatches_no_drag_end(WM_KEYDOWN, VK_ESCAPE.0 as usize, "ESC");
}

#[test]
fn cancelmode_before_threshold_dispatches_no_drag_end() {
    assert_cancel_before_threshold_dispatches_no_drag_end(
        WM_CANCELMODE,
        0,
        "メニューやダイアログの割り込み（WM_CANCELMODE）",
    );
}

/// `WM_CAPTURECHANGED` は `mark_released()` を先に呼ぶので `ReleaseCapture` は走らない。
#[test]
fn capturechanged_before_threshold_dispatches_no_drag_end() {
    assert_cancel_before_threshold_dispatches_no_drag_end(
        WM_CAPTURECHANGED,
        0,
        "捕捉の喪失（WM_CAPTURECHANGED）",
    );
}

/// 準備中の非活性化は今も終了の知らせを配らない（修正の前後とも緑）。
#[test]
fn deactivation_before_threshold_dispatches_no_drag_end() {
    let (world, entity) = setup_preparing();
    send(&world, entity, WM_ACTIVATE, 0); // WA_INACTIVE

    let ends = dispatch_once(&world);
    assert_eq!(
        ends.len(),
        0,
        "閾値前の非活性化で終了の知らせは 0 件: {ends:?}"
    );
}

// ============================================================================
// OnKeyDown の受け渡し（要件 8.1・8.2・8.4）
// ============================================================================

use super::{deliver_key_down, key_down_from};
use crate::ecs::window::{KeyDown, OnKeyDown};
use crate::ecs::window_proc::lifecycle::DESPAWNED_SKIP_TAG;
use bevy_ecs::prelude::{Resource, World};
use log_capture_kit::CapturedEvent;
use tracing::Level;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_F10;
use windows::Win32::UI::WindowsAndMessaging::WM_SYSKEYDOWN;

/// 受け手が呼ばれるたびに渡された押下を積む（テスト専用）。資源が無い＝1 度も呼ばれていない。
#[derive(Resource, Default)]
struct ReceivedKeys(Vec<KeyDown>);

fn record(world: &mut World, key: KeyDown) {
    world
        .get_resource_or_insert_with(ReceivedKeys::default)
        .0
        .push(key);
}

fn consume_key(world: &mut World, _entity: Entity, key: KeyDown) -> bool {
    record(world, key);
    true
}

fn pass_key(world: &mut World, _entity: Entity, key: KeyDown) -> bool {
    record(world, key);
    false
}

/// 消費する受け手・消費しない受け手・受け手なしの 3 つの窓を持つ World。
fn key_world() -> (Rc<RefCell<EcsWorld>>, Entity, Entity, Entity) {
    cancel_dragging();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (consuming, passing, plain) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        (
            w.spawn(OnKeyDown(consume_key)).id(),
            w.spawn(OnKeyDown(pass_key)).id(),
            w.spawn_empty().id(),
        )
    };
    (world, consuming, passing, plain)
}

fn received_keys(world: &Rc<RefCell<EcsWorld>>) -> Option<Vec<KeyDown>> {
    world
        .borrow()
        .world()
        .get_resource::<ReceivedKeys>()
        .map(|r| r.0.clone())
}

fn key_message(msg: u32, vk: u16, lparam: isize) -> WindowMessage {
    WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg,
        wparam: WPARAM(vk as usize),
        lparam: LPARAM(lparam),
    }
}

fn f10() -> KeyDown {
    KeyDown {
        vk: VK_F10.0,
        ctrl: true,
        shift: false,
        alt: true,
        repeat: false,
    }
}

/// 要件 8.1: `WM_KEYDOWN` と `WM_SYSKEYDOWN` のどちらでも受け手が仮想キーつきで呼ばれ、
/// 真を返せば押下は消費される（`Some(LRESULT(0))`＝DefWindowProc へ渡らない）。
#[test]
fn both_key_messages_reach_the_receiver_and_true_consumes() {
    for msg in [WM_KEYDOWN, WM_SYSKEYDOWN] {
        let (world, consuming, _, _) = key_world();
        let ret = crate::ecs::dispatch_window_message(
            &world,
            consuming,
            &key_message(msg, VK_F10.0, 1 << 30),
        );
        assert_eq!(ret, Some(LRESULT(0)), "msg={msg:#x} が消費されていない");
        let keys = received_keys(&world).expect("受け手が呼ばれていない");
        assert_eq!(keys.len(), 1, "msg={msg:#x}: {keys:?}");
        assert_eq!(keys[0].vk, VK_F10.0);
        assert!(
            keys[0].repeat,
            "lparam の 30 ビット目が repeat に写っていない"
        );
    }
}

/// 要件 8.4: 偽を返す受け手・受け手の無い窓では押下は素通し（`None`＝DefWindowProc へ）。
#[test]
fn false_or_no_receiver_passes_through() {
    for msg in [WM_KEYDOWN, WM_SYSKEYDOWN] {
        let (world, _, passing, plain) = key_world();
        let m = key_message(msg, VK_F10.0, 0);
        assert_eq!(
            crate::ecs::dispatch_window_message(&world, passing, &m),
            None
        );
        assert_eq!(
            received_keys(&world).map(|k| k.len()),
            Some(1),
            "偽を返す受け手も呼ばれる"
        );
        assert_eq!(crate::ecs::dispatch_window_message(&world, plain, &m), None);
        assert_eq!(
            received_keys(&world).map(|k| k.len()),
            Some(1),
            "受け手の無い窓で何かが呼ばれた"
        );
    }
}

/// 修飾キーと repeat は値で渡ったとおりに受け手へ届く（OS は読まない）。
#[test]
fn deliver_passes_the_key_by_value() {
    let (world, consuming, _, _) = key_world();
    let key = key_down_from(WPARAM(VK_F10.0 as usize), LPARAM(0), (true, false, true));
    assert_eq!(key, f10());
    assert_eq!(deliver_key_down(&world, consuming, key), Some(LRESULT(0)));
    assert_eq!(received_keys(&world), Some(vec![f10()]));
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 要件 8.1: World の借用中は受け手を呼ばず、`key_down_dropped`（vk・reason=world_busy）の
/// `warn!` を 1 行出して素通しする。
#[test]
fn world_busy_logs_and_does_not_deliver() {
    let (world, consuming, _, _) = key_world();

    let held = world.borrow_mut();
    let (ret, events) = log_capture_kit::capture(|| deliver_key_down(&world, consuming, f10()));
    drop(held);

    assert_eq!(ret, None);
    assert_eq!(received_keys(&world), None, "借用中に受け手が呼ばれた");
    let hits = events_named(&events, "key_down_dropped");
    assert_eq!(hits.len(), 1, "key_down_dropped が 1 行でない: {events:?}");
    assert_eq!(hits[0].level, Level::WARN);
    assert_eq!(hits[0].field("vk"), Some(VK_F10.0.to_string().as_str()));
    assert_eq!(hits[0].field_str("reason"), Some("world_busy"));
}

/// 窓が消えた後に届いた押下は呼ばずに `debug!` で打ち切る（警告 0）。
#[test]
fn despawned_window_skips_with_debug() {
    let (world, consuming, _, _) = key_world();
    assert!(world.borrow_mut().world_mut().despawn(consuming));

    let (ret, events) = log_capture_kit::capture(|| deliver_key_down(&world, consuming, f10()));

    assert_eq!(ret, None);
    assert_eq!(received_keys(&world), None);
    let skips: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(DESPAWNED_SKIP_TAG))
        .collect();
    assert_eq!(skips.len(), 1, "{events:?}");
    assert_eq!(skips[0].level, Level::DEBUG);
    assert!(events.iter().all(|e| e.level > Level::WARN), "{events:?}");
}

/// Esc の取り消しは受け手を差した窓でも今どおり先に行われ、その後で受け手が呼ばれる。
#[test]
fn esc_still_cancels_the_drag_before_delivery() {
    let (world, consuming, _, _) = key_world();
    start_preparing(consuming, PhysicalPoint::new(10, 20), HWND::default());

    let ret = crate::ecs::dispatch_window_message(
        &world,
        consuming,
        &key_message(WM_KEYDOWN, VK_ESCAPE.0, 0),
    );

    assert!(matches!(
        snapshot_drag_state(),
        DragStateSnapshot::JustEnded {
            cancelled: true,
            ..
        }
    ));
    assert_eq!(ret, Some(LRESULT(0)));
    assert_eq!(
        received_keys(&world).map(|k| k.iter().map(|k| k.vk).collect::<Vec<_>>()),
        Some(vec![VK_ESCAPE.0])
    );
}
