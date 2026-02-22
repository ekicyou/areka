use bevy_ecs::prelude::*;
use windows_numerics::Vector2;
use wintf::ecs::Visual;
use wintf::ecs::visual_resource_management_system;
use wintf::ecs::world::FrameCount;
use wintf::ecs::{GraphicsCore, VisualGraphics};

#[test]
fn test_visual_component_definition() {
    let mut world = World::new();

    // Test Default
    let entity = world.spawn(Visual::default()).id();

    let visual = world.get::<Visual>(entity).unwrap();

    // Check default values
    assert_eq!(visual.is_visible, true);
    assert_eq!(visual.opacity, 1.0);
    assert_eq!(visual.transform_origin.X, 0.0);
    assert_eq!(visual.transform_origin.Y, 0.0);
}

#[test]
fn test_visual_component_properties() {
    let visual = Visual {
        is_visible: false,
        opacity: 0.5,
        transform_origin: Vector2::new(10.0, 20.0),
    };

    assert_eq!(visual.is_visible, false);
    assert_eq!(visual.opacity, 0.5);
    assert_eq!(visual.transform_origin.X, 10.0);
    assert_eq!(visual.transform_origin.Y, 20.0);
}

/// Phase 6: Visualリソース作成テスト
///
/// visual_resource_management_systemはVisualGraphicsのみを作成し、
/// SurfaceGraphicsは作成しない（deferred_surface_creation_systemで遅延作成）。
///
/// Phase 2: on_visual_addフックはVisualGraphics自動挿入を停止したため、
/// テスト内で明示的にVisualGraphicsを挿入する。
#[test]
fn test_visual_resource_creation() {
    let mut world = World::new();

    // Setup GraphicsCore
    // Note: This requires a valid Windows environment with DComp support.
    // If running in CI without GPU, this might fail.
    // But local environment seems to have it.
    let graphics = GraphicsCore::new().expect("Failed to create GraphicsCore");
    world.insert_resource(graphics);

    // Setup FrameCount resource (required by visual_resource_management_system)
    world.insert_resource(FrameCount(1));

    // Setup Schedule
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_resource_management_system);

    // Spawn entity with Visual + VisualGraphics
    // Phase 2: on_visual_addはVisualGraphicsを自動挿入しなくなったため明示的に追加
    let entity = world
        .spawn((Visual::default(), VisualGraphics::default()))
        .id();

    // Run schedule
    schedule.run(&mut world);

    // Phase 6: VisualGraphicsのみが作成される（Surfaceは遅延作成）
    assert!(
        world.get::<VisualGraphics>(entity).is_some(),
        "VisualGraphics should be created by visual_resource_management_system"
    );
    // SurfaceGraphicsはdeferred_surface_creation_systemで作成されるため、
    // ここでは存在しない
}

// ========================================================================
// Visual API unit tests (Task 1.2)
// ========================================================================

#[test]
fn test_visual_default_values() {
    let visual = Visual::default();
    assert_eq!(visual.opacity, 1.0);
    assert_eq!(visual.is_visible, true);
    assert_eq!(visual.clamped_opacity(), 1.0);
}

#[test]
fn test_visual_set_opacity_normal_range() {
    let mut visual = Visual::default();
    visual.set_opacity(0.5);
    assert_eq!(visual.opacity, 0.5);
    assert_eq!(visual.clamped_opacity(), 0.5);
}

#[test]
fn test_visual_set_opacity_boundary_zero() {
    let mut visual = Visual::default();
    visual.set_opacity(0.0);
    assert_eq!(visual.opacity, 0.0);
    assert_eq!(visual.clamped_opacity(), 0.0);
}

#[test]
fn test_visual_set_opacity_boundary_one() {
    let mut visual = Visual::default();
    visual.set_opacity(1.0);
    assert_eq!(visual.opacity, 1.0);
    assert_eq!(visual.clamped_opacity(), 1.0);
}

#[test]
fn test_visual_set_opacity_clamps_negative() {
    let mut visual = Visual::default();
    visual.set_opacity(-0.1);
    assert_eq!(visual.opacity, 0.0);
    assert_eq!(visual.clamped_opacity(), 0.0);
}

#[test]
fn test_visual_set_opacity_clamps_above_one() {
    let mut visual = Visual::default();
    visual.set_opacity(1.5);
    assert_eq!(visual.opacity, 1.0);
    assert_eq!(visual.clamped_opacity(), 1.0);
}

#[test]
fn test_visual_clamped_opacity_direct_field_out_of_range() {
    // 直接フィールドアクセスで範囲外値を設定した場合でも clamped_opacity() はクランプを保証
    let mut visual = Visual::default();
    visual.opacity = -0.5;
    assert_eq!(visual.clamped_opacity(), 0.0);

    visual.opacity = 2.0;
    assert_eq!(visual.clamped_opacity(), 1.0);
}

#[test]
fn test_visual_set_visible_true() {
    let mut visual = Visual::default();
    visual.set_visible(false);
    assert_eq!(visual.is_visible, false);
    visual.set_visible(true);
    assert_eq!(visual.is_visible, true);
}

#[test]
fn test_visual_set_visible_false() {
    let mut visual = Visual::default();
    visual.set_visible(false);
    assert_eq!(visual.is_visible, false);
}

#[test]
fn test_visual_clamped_opacity_matches_opacity_clamped() {
    // Visual.clamped_opacity() が Opacity::clamped() と同等の動作をすることを検証
    #[allow(deprecated)]
    use wintf::ecs::layout::Opacity;

    let test_values = [0.0, 0.25, 0.5, 0.75, 1.0, -0.1, 1.5, -100.0, 100.0];

    for &value in &test_values {
        #[allow(deprecated)]
        let opacity = Opacity(value);
        let visual = Visual {
            opacity: value,
            ..Default::default()
        };
        assert_eq!(
            opacity.clamped(),
            visual.clamped_opacity(),
            "Mismatch for value {}",
            value
        );
    }
}
