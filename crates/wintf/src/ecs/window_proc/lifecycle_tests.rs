use crate::ecs::dispatch_window_message;
use crate::ecs::window::{OnCloseRequest, OnSessionEnd};
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;
use bevy_ecs::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{WM_ENDSESSION, WM_QUERYENDSESSION};

/// 閉鎖要求の関数が呼ばれた entity を順に記録する（テスト専用）。
#[derive(Resource, Default)]
struct CloseRequestCalls(Vec<Entity>);

fn record_close_request(world: &mut World, entity: Entity) {
    world.resource_mut::<CloseRequestCalls>().0.push(entity);
}

fn send_wm_close(world: &Rc<RefCell<EcsWorld>>, entity: Entity) -> Option<LRESULT> {
    super::WM_CLOSE(
        world,
        entity,
        HWND(std::ptr::null_mut()),
        WPARAM(0),
        LPARAM(0),
    )
}

/// 要件 2.10・2.11: `OnCloseRequest` を差した窓は `WM_CLOSE` で消えずに関数が 1 回呼ばれ、
/// 差さない窓は従来どおり消える。戻り値（既定手続きの `DestroyWindow` 抑止）は両方同じ。
#[test]
fn wm_close_calls_on_close_request_instead_of_despawning() {
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (hooked, plain) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        w.init_resource::<CloseRequestCalls>();
        let hooked = w.spawn(OnCloseRequest(record_close_request)).id();
        let plain = w.spawn_empty().id();
        (hooked, plain)
    };

    let hooked_ret = send_wm_close(&world, hooked);
    let plain_ret = send_wm_close(&world, plain);

    let w = world.borrow();
    let w = w.world();
    assert!(
        w.get_entity(hooked).is_ok(),
        "部品を差した窓が WM_CLOSE で消えた（要件 2.10）"
    );
    assert_eq!(
        w.resource::<CloseRequestCalls>().0,
        vec![hooked],
        "部品の関数が差した窓について 1 回だけ呼ばれていない（要件 2.10・2.11）"
    );
    assert!(
        w.get_entity(plain).is_err(),
        "部品を差さない窓が WM_CLOSE で消えていない（要件 2.11）"
    );
    assert_eq!(hooked_ret, Some(LRESULT(0)));
    assert_eq!(plain_ret, Some(LRESULT(0)));
}

/// セッションの終了の関数が呼ばれた entity を順に記録する（テスト専用）。
#[derive(Resource, Default)]
struct SessionEndCalls(Vec<Entity>);

fn record_session_end(world: &mut World, entity: Entity) {
    world.resource_mut::<SessionEndCalls>().0.push(entity);
}

/// 配送表を通して `msg` を送る（腕の有無ごと固定するため受け手を直に呼ばない）。
fn send(world: &Rc<RefCell<EcsWorld>>, entity: Entity, msg: u32, wparam: usize) -> Option<LRESULT> {
    dispatch_window_message(
        world,
        entity,
        &WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg,
            wparam: WPARAM(wparam),
            lparam: LPARAM(0),
        },
    )
}

/// `OnSessionEnd` を差した entity と差さない entity を持つ World。
fn session_end_world() -> (Rc<RefCell<EcsWorld>>, Entity, Entity) {
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (hooked, plain) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        w.init_resource::<SessionEndCalls>();
        let hooked = w.spawn(OnSessionEnd(record_session_end)).id();
        let plain = w.spawn_empty().id();
        (hooked, plain)
    };
    (world, hooked, plain)
}

fn session_end_calls(world: &Rc<RefCell<EcsWorld>>) -> Vec<Entity> {
    world
        .borrow()
        .world()
        .resource::<SessionEndCalls>()
        .0
        .clone()
}

/// `event` 欄が `name` の記録を水準ごと数える。
fn events_named(events: &[log_capture_kit::CapturedEvent], name: &str) -> Vec<tracing::Level> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .map(|e| e.level)
        .collect()
}

/// 要件 12.9・12.10: `WM_ENDSESSION`（wParam 真）は部品を差した entity の関数を 1 回呼び、
/// 0 を返す（処理した）。部品を差さない entity では呼ばず 0 を返す。
#[test]
fn wm_endsession_true_calls_on_session_end_once() {
    let (world, hooked, plain) = session_end_world();

    let (rets, events) = log_capture_kit::capture(|| {
        (
            send(&world, hooked, WM_ENDSESSION, 1),
            send(&world, plain, WM_ENDSESSION, 1),
        )
    });

    assert_eq!(
        session_end_calls(&world),
        vec![hooked],
        "部品を差した entity について 1 回だけ呼ばれていない（要件 12.9・12.10）"
    );
    assert_eq!(rets, (Some(LRESULT(0)), Some(LRESULT(0))));
    assert_eq!(
        events_named(&events, "os_session_end"),
        vec![tracing::Level::INFO],
        "関数を呼ぶ前の info! がちょうど 1 件でない"
    );
}

/// 要件 12.10: 取りやめ（wParam 偽）では呼ばず、0 を返す。
#[test]
fn wm_endsession_false_is_cancelled_without_call() {
    let (world, hooked, _) = session_end_world();

    let (ret, events) = log_capture_kit::capture(|| send(&world, hooked, WM_ENDSESSION, 0));

    assert!(
        session_end_calls(&world).is_empty(),
        "取りやめで関数が呼ばれた"
    );
    assert_eq!(ret, Some(LRESULT(0)));
    assert_eq!(
        events_named(&events, "os_session_end_cancelled"),
        vec![tracing::Level::DEBUG]
    );
}

/// 要件 12.10: 最初の受け手が全ゴースト窓の entity を破棄した後に残りの窓へ届いた
/// 2 通目以降は、破棄済みの打ち切り（debug!）で止まり、呼ばず panic もしない。
#[test]
fn wm_endsession_on_despawned_entity_skips() {
    let (world, hooked, _) = session_end_world();
    assert!(world.borrow_mut().world_mut().despawn(hooked));

    let (ret, events) = log_capture_kit::capture(|| send(&world, hooked, WM_ENDSESSION, 1));

    assert!(session_end_calls(&world).is_empty());
    assert_eq!(ret, Some(LRESULT(0)));
    let skips: Vec<_> = events
        .iter()
        .filter(|e| {
            e.fields_map()
                .get("message")
                .is_some_and(|m| m.contains(super::DESPAWNED_SKIP_TAG))
        })
        .map(|e| e.level)
        .collect();
    assert_eq!(
        skips,
        vec![tracing::Level::DEBUG],
        "破棄済みの打ち切りが 1 件でない"
    );
}

/// 要件 12.10: World の借用中に届いたら呼べない。黙らず帰結を書いた warn! を 1 件残し、0 を返す。
#[test]
fn wm_endsession_while_world_borrowed_warns_without_call() {
    let (world, hooked, _) = session_end_world();

    let held = world.borrow_mut();
    let (ret, events) = log_capture_kit::capture(|| send(&world, hooked, WM_ENDSESSION, 1));
    drop(held);

    assert!(
        session_end_calls(&world).is_empty(),
        "借用中に関数が呼ばれた"
    );
    assert_eq!(ret, Some(LRESULT(0)));
    assert_eq!(
        events_named(&events, "os_session_end_world_busy"),
        vec![tracing::Level::WARN]
    );
}

/// 要件 12.10: `WM_QUERYENDSESSION` は配送表に腕を持たず既定の手続き（終了を許す）へ委ねる。
#[test]
fn wm_queryendsession_is_left_to_default_procedure() {
    let (world, hooked, _) = session_end_world();

    assert_eq!(send(&world, hooked, WM_QUERYENDSESSION, 0), None);
    assert!(session_end_calls(&world).is_empty());
}
