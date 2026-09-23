use crate::ecs::window::OnCloseRequest;
use crate::ecs::world::EcsWorld;
use bevy_ecs::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

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
