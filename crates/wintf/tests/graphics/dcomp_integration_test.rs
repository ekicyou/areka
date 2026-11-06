//! WUC パイプライン統合テスト (Task 12)
//!
//! 窓の下の Visual に描画の部品が自動で入るフックだけを試す（素の World・装置なし）。
//! `invalidate_dependent_components` の連動は `window_pos_systems_test.rs` が確かめる。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::{SurfaceGraphics, SurfaceGraphicsDirty, Visual, VisualGraphics, Window};

// ========================================================================
// 12.1: コンポーネント自動挿入テスト（ECS のフックだけを試すので素の World・装置なし）
// ========================================================================

#[test]
fn window_visual_gets_graphics_parts() {
    let mut world = World::new();

    // Window を生成
    let window_entity = world.spawn(Window::default()).id();

    // Window 配下に Visual を生成
    let visual_entity = world
        .spawn((Visual::default(), ChildOf(window_entity)))
        .id();

    // Commands をフラッシュ
    world.flush();

    // 窓の下の Visual は VisualGraphics, SurfaceGraphics, SurfaceGraphicsDirty が挿入される
    assert!(
        world.get::<VisualGraphics>(visual_entity).is_some(),
        "Window Visual should have VisualGraphics"
    );
    assert!(
        world.get::<SurfaceGraphics>(visual_entity).is_some(),
        "Window Visual should have SurfaceGraphics"
    );
    assert!(
        world.get::<SurfaceGraphicsDirty>(visual_entity).is_some(),
        "Window Visual should have SurfaceGraphicsDirty"
    );
}

#[test]
fn orphan_visual_gets_no_graphics_parts() {
    let mut world = World::new();

    // orphan Visual（Window 祖先なし）
    let visual_entity = world.spawn(Visual::default()).id();

    // Commands をフラッシュ
    world.flush();

    // orphan Visual は描画の部品が挿入されない
    assert!(
        world.get::<VisualGraphics>(visual_entity).is_none(),
        "Orphan Visual should NOT have VisualGraphics"
    );
}
