//! DComp パイプライン統合テスト (Task 12)

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::world::FrameCount;
use wintf::ecs::{
    CompositionMode, DCompGraphicsResource, GraphicsCore, SurfaceGraphics, SurfaceGraphicsDirty,
    Visual, VisualGraphics, Window,
};

/// テスト用ワールドをセットアップ
fn setup_world() -> World {
    let graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");
    let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
    let dcomp_resource = DCompGraphicsResource::new(d2d).expect("DCompGraphicsResource作成失敗");

    let mut world = World::new();
    world.insert_resource(graphics);
    world.insert_resource(dcomp_resource);
    world.insert_resource(FrameCount(1));
    world.insert_resource(wintf::ecs::SurfaceCreationStats::default());
    world
}

// ========================================================================
// 12.1: コンポーネント自動挿入テスト
// ========================================================================

#[test]
fn test_dcomp_window_visual_gets_dcomp_components() {
    let mut world = setup_world();

    // DComp Window を生成
    let window_entity = world
        .spawn(Window {
            composition_mode: CompositionMode::DComp,
            ..Default::default()
        })
        .id();

    // DComp Window 配下に Visual を生成
    let visual_entity = world
        .spawn((Visual::default(), ChildOf(window_entity)))
        .id();

    // Commands をフラッシュ
    world.flush();

    // DComp モードの Visual は VisualGraphics, SurfaceGraphics, SurfaceGraphicsDirty が挿入される
    assert!(
        world.get::<VisualGraphics>(visual_entity).is_some(),
        "DComp Visual should have VisualGraphics"
    );
    assert!(
        world.get::<SurfaceGraphics>(visual_entity).is_some(),
        "DComp Visual should have SurfaceGraphics"
    );
    assert!(
        world.get::<SurfaceGraphicsDirty>(visual_entity).is_some(),
        "DComp Visual should have SurfaceGraphicsDirty"
    );
}

#[test]
fn test_ulw_window_visual_does_not_get_dcomp_components() {
    let mut world = setup_world();

    // ULW Window を生成（デフォルト）
    let window_entity = world.spawn(Window::default()).id();

    // ULW Window 配下に Visual を生成
    let visual_entity = world
        .spawn((Visual::default(), ChildOf(window_entity)))
        .id();

    // Commands をフラッシュ
    world.flush();

    // ULW モードの Visual は DComp コンポーネントが挿入されない
    assert!(
        world.get::<VisualGraphics>(visual_entity).is_none(),
        "ULW Visual should NOT have VisualGraphics"
    );
    assert!(
        world.get::<SurfaceGraphics>(visual_entity).is_none(),
        "ULW Visual should NOT have SurfaceGraphics"
    );
    assert!(
        world.get::<SurfaceGraphicsDirty>(visual_entity).is_none(),
        "ULW Visual should NOT have SurfaceGraphicsDirty"
    );
}

#[test]
fn test_orphan_visual_does_not_get_dcomp_components() {
    let mut world = setup_world();

    // orphan Visual（Window 祖先なし）
    let visual_entity = world.spawn(Visual::default()).id();

    // Commands をフラッシュ
    world.flush();

    // orphan Visual は DComp コンポーネントが挿入されない
    assert!(
        world.get::<VisualGraphics>(visual_entity).is_none(),
        "Orphan Visual should NOT have VisualGraphics"
    );
}

// ========================================================================
// 12.2: invalidate_dependent_components 連動テスト
// ========================================================================

#[test]
fn test_invalidate_dependent_components_invalidates_dcomp_resource() {
    let mut world = setup_world();

    // DCompGraphicsResource が valid であることを確認
    {
        let dcr = world.resource::<DCompGraphicsResource>();
        assert!(dcr.is_valid(), "DCompGraphicsResource should start valid");
    }

    // GraphicsCore を無効化
    world.resource_mut::<GraphicsCore>().invalidate();

    // invalidate_dependent_components を実行
    let mut schedule = Schedule::default();
    schedule.add_systems(wintf::ecs::invalidate_dependent_components);
    schedule.run(&mut world);

    // DCompGraphicsResource が無効化されていることを確認
    let dcr = world.resource::<DCompGraphicsResource>();
    assert!(
        !dcr.is_valid(),
        "DCompGraphicsResource should be invalidated after GraphicsCore invalidation"
    );
}
