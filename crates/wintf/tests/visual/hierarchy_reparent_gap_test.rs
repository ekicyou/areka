//! visual_hierarchy_sync_system の再ペアレント未検出ギャップの特性化テスト (W3b-V)
//!
//! 本システムの未同期検出は `parent_visual().is_none()` のみであり、既に親 Visual を
//! 持つ子の ChildOf 変更（再ペアレント）は検出されない。既存テスト
//! `test_childof_change_moves_visual_to_new_parent`（hierarchy_sync_test.rs）は
//! `parent_visual().is_some()` しか検証しないためこのギャップを見逃す。
//! 本テストはポインタ同一性（`Interface::as_raw`）で現行挙動を固定する。
//! 修正（検出条件の拡張）は挙動変更のため P47 に記録。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::UI::Composition::{ContainerVisual, Visual as WucVisual};
use windows::core::{Interface, Result};
use wintf::ecs::{Visual, VisualGraphics, visual_hierarchy_sync_system};

use super::common::{WucVisualFactory, setup_graphics};

/// WUC 移行: DComp の add_visual(insertAbove=false, ref=None) は
/// ContainerVisual.Children().InsertAtBottom に対応。
fn add_child(parent: &WucVisual, child: &WucVisual) -> Result<()> {
    parent
        .cast::<ContainerVisual>()?
        .Children()?
        .InsertAtBottom(child)?;
    Ok(())
}

fn run_sync(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_hierarchy_sync_system);
    schedule.run(world);
}

/// 再ペアレント（ChildOf A→B）では parent_visual キャッシュが旧親 A を指し続け、
/// 新親 B へは再接続されない（現行挙動の固定: 同期システムは何も検出しない）
#[test]
fn reparent_keeps_stale_parent_visual_cache() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    let mut world = World::new();
    world.insert_resource(graphics);

    // 親A（同期済み状態を構築）
    let visual_a = factory.create_visual()?;
    let parent_a = world
        .spawn((Visual::default(), VisualGraphics::new(visual_a.clone())))
        .id();

    // 親B
    let visual_b = factory.create_visual()?;
    let parent_b = world
        .spawn((Visual::default(), VisualGraphics::new(visual_b.clone())))
        .id();
    let _ = parent_b; // ECS 階層上の新親（DComp 側は同期されないことを検証する）

    // 子: 親Aに同期済み（parent_visual キャッシュ = visual_a、DComp 上も A に接続）
    let visual_c = factory.create_visual()?;
    let mut child_vg = VisualGraphics::new(visual_c.clone());
    child_vg.set_parent_visual(Some(visual_a.clone()));
    add_child(&visual_a, &visual_c)?;
    let child = world
        .spawn((Visual::default(), child_vg, ChildOf(parent_a)))
        .id();

    // 再ペアレント: A → B
    world.entity_mut(child).insert(ChildOf(parent_b));

    run_sync(&mut world);

    // 現行挙動: parent_visual().is_none() のみが未同期検出条件のため、
    // キャッシュは旧親 A のまま残り、新親 B へは再接続されない
    let cached = world
        .get::<VisualGraphics>(child)
        .unwrap()
        .parent_visual()
        .cloned()
        .expect("cache should remain Some");
    assert_eq!(
        cached.as_raw(),
        visual_a.as_raw(),
        "characterization: cache still points to OLD parent A (reparent not detected)"
    );
    assert_ne!(
        cached.as_raw(),
        visual_b.as_raw(),
        "characterization: cache does NOT point to new parent B"
    );

    Ok(())
}

/// 再ペアレント後に旧親側が（別の未同期子により）再同期されると、移動済みの子は
/// remove_all_visuals で旧親の DComp Visual から切り離されるが、新親へは再接続されず、
/// parent_visual キャッシュは実体と乖離した旧親参照を保持し続ける（現行挙動の固定）
#[test]
fn old_parent_resync_detaches_moved_child_without_reattach() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    let mut world = World::new();
    world.insert_resource(graphics);

    // 親A・親B・同期済みの子（A に接続）
    let visual_a = factory.create_visual()?;
    let parent_a = world
        .spawn((Visual::default(), VisualGraphics::new(visual_a.clone())))
        .id();
    let visual_b = factory.create_visual()?;
    let parent_b = world
        .spawn((Visual::default(), VisualGraphics::new(visual_b.clone())))
        .id();

    let visual_c = factory.create_visual()?;
    let mut child_vg = VisualGraphics::new(visual_c.clone());
    child_vg.set_parent_visual(Some(visual_a.clone()));
    add_child(&visual_a, &visual_c)?;
    let moved_child = world
        .spawn((Visual::default(), child_vg, ChildOf(parent_a)))
        .id();

    // 再ペアレント: A → B（検出されない）
    world.entity_mut(moved_child).insert(ChildOf(parent_b));

    // 新しい未同期の子 D を A の下に追加 → A が affected_parents 入りして再同期される
    let visual_d = factory.create_visual()?;
    let new_child = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(visual_d.clone()),
            ChildOf(parent_a),
        ))
        .id();

    run_sync(&mut world);

    // A の再同期: remove_all_visuals で visual_c も A から切り離され、A.Children には
    // D のみが残るため visual_c は再追加されない（DComp 上はどの親にも属さない孤立 Visual）
    let new_child_cache = world
        .get::<VisualGraphics>(new_child)
        .unwrap()
        .parent_visual()
        .cloned()
        .expect("new child should be synced to A");
    assert_eq!(
        new_child_cache.as_raw(),
        visual_a.as_raw(),
        "new child D should be attached to A"
    );

    // 現行挙動: 移動済みの子のキャッシュは旧親 A を指したまま（実体は切り離し済みで乖離）
    let moved_cache = world
        .get::<VisualGraphics>(moved_child)
        .unwrap()
        .parent_visual()
        .cloned()
        .expect("moved child cache should remain Some");
    assert_eq!(
        moved_cache.as_raw(),
        visual_a.as_raw(),
        "characterization: moved child cache still claims OLD parent A after detach"
    );
    assert_ne!(
        moved_cache.as_raw(),
        visual_b.as_raw(),
        "characterization: moved child is never attached to new parent B"
    );

    Ok(())
}
