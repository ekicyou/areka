//! えみりの形の固定入力から組んだ配置の全 scope の窓で、マウスを動かしても panic しない
//! （spec: areka-P0-emily-ghost-verification 要件 4.7・6.7）。
//!
//! 窓の scope は手で書かず、配置の設定が返した scope の全部で作る。scope 2 以上の窓が
//! 作られると `char_scope` の `debug_assert!(scope <= 1)` で落ちる。

use std::sync::mpsc;

use wintf::ecs::Point;

use super::*;
use crate::placement::shared_test_support::emily_shaped_scopes;

fn no_region(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: None,
        surface_point: (x, y),
    }
}

#[test]
fn pointer_moves_over_every_emily_shaped_window_do_not_panic() {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::new(tx, RegionSource::Mock(no_region)));
    let windows: Vec<Entity> = emily_shaped_scopes()
        .into_iter()
        .map(|scope| world.spawn(CharWindowMarker { scope }).id())
        .collect();

    let ev = Phase::Bubble(PointerState {
        client_point: Point { x: 10, y: 20 },
        ..Default::default()
    });
    for &e in &windows {
        assert!(on_char_pointer_moved(&mut world, e, e, &ev));
    }

    let scopes: Vec<u32> = rx
        .try_iter()
        .map(|msg| match msg {
            KanadeMsg::Mouse(m) => m.scope,
            _ => panic!("Mouse(Move) を期待"),
        })
        .collect();
    assert_eq!(
        scopes,
        vec![0, 1],
        "窓ごとに 1 件ずつ・scope は 0 と 1 だけ"
    );
}
