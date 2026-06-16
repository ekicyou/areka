//! layout コンポーネントのライフサイクルフック（on_add 連鎖）のギャップテスト
//!
//! 対象:
//! - `LayoutRoot` on_add フック（mod.rs::on_layout_root_add）: Arrangement 自動挿入と既存保持
//! - `Arrangement` on_add フック（arrangement.rs::on_arrangement_add）:
//!   GlobalArrangement / ArrangementTreeChanged の自動挿入

use bevy_ecs::prelude::*;
use wintf::ecs::layout::{
    Arrangement, ArrangementTreeChanged, GlobalArrangement, LayoutRoot, LayoutScale, Offset, Size,
};

/// LayoutRoot 単独 spawn で Arrangement::default() が自動挿入され、
/// さらに Arrangement の on_add 連鎖で GlobalArrangement / ArrangementTreeChanged も付与される
#[test]
fn test_layout_root_on_add_inserts_default_arrangement_chain() {
    let mut world = World::new();
    let entity = world.spawn(LayoutRoot).id();
    world.flush();

    let arrangement = world
        .get::<Arrangement>(entity)
        .expect("LayoutRoot on_add で Arrangement が自動挿入されるべき");
    assert_eq!(*arrangement, Arrangement::default());

    assert!(
        world.get::<GlobalArrangement>(entity).is_some(),
        "Arrangement on_add 連鎖で GlobalArrangement が自動挿入されるべき"
    );
    assert!(
        world.get::<ArrangementTreeChanged>(entity).is_some(),
        "Arrangement on_add 連鎖で ArrangementTreeChanged が自動挿入されるべき"
    );
}

/// LayoutRoot と同時に明示的な Arrangement を spawn した場合、
/// フックは既存の Arrangement を上書きしない
#[test]
fn test_layout_root_on_add_preserves_existing_arrangement() {
    let custom = Arrangement {
        offset: Offset { x: 12.0, y: 34.0 },
        scale: LayoutScale { x: 2.0, y: 2.0 },
        size: Size {
            width: 800.0,
            height: 600.0,
        },
    };

    let mut world = World::new();
    let entity = world.spawn((LayoutRoot, custom)).id();
    world.flush();

    let arrangement = world
        .get::<Arrangement>(entity)
        .expect("Arrangement が存在するべき");
    assert_eq!(
        *arrangement, custom,
        "明示的に指定した Arrangement がフックで上書きされてはならない"
    );
}

/// Arrangement 単独 spawn で GlobalArrangement（default 値）と
/// ArrangementTreeChanged が自動挿入される
#[test]
fn test_arrangement_on_add_inserts_global_and_tree_changed() {
    let mut world = World::new();
    let entity = world
        .spawn(Arrangement {
            offset: Offset { x: 10.0, y: 20.0 },
            scale: LayoutScale::default(),
            size: Size {
                width: 100.0,
                height: 50.0,
            },
        })
        .id();
    world.flush();

    let global = world
        .get::<GlobalArrangement>(entity)
        .expect("Arrangement on_add で GlobalArrangement が自動挿入されるべき");
    // 挿入時点では伝播前のため default 値（恒等変換・ゼロ bounds）
    assert_eq!(*global, GlobalArrangement::default());

    assert!(
        world.get::<ArrangementTreeChanged>(entity).is_some(),
        "Arrangement on_add で ArrangementTreeChanged が自動挿入されるべき"
    );
}
