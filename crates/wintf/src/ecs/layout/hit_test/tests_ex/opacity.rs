use super::super::*;
use super::make_global_arrangement;
use bevy_ecs::world::World;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

// ========================================================================
// Opacity/Brushes α値判定テスト（HitTestMode::Bounds）
// ========================================================================

/// Visual.opacity=0.502 * foreground.a=1.0 → 合成α ≈ 0.502 ≥ 128/255 → HTCLIENT（ヒット）
#[test]
fn test_hit_test_entity_bounds_alpha_boundary_above() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.502,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
        ))
        .id();
    // Visual.on_add → Arrangement.on_add が GlobalArrangement::default() を挿入するため、
    // flush 後にテスト用の値で上書きする
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 0.502 * 1.0 = 0.502 ≥ 128/255 ≈ 0.50196 → ヒット
    assert!(hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=0.501 * foreground.a=1.0 → 合成α ≈ 0.501 < 128/255 → 透明領域（ミス）
#[test]
fn test_hit_test_entity_bounds_alpha_boundary_below() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.501,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 0.501 * 1.0 = 0.501 < 128/255 ≈ 0.50196 → 透明
    assert!(!hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=0.4 * foreground.a=1.0 → 合成α = 0.4 < 128/255 → 透明領域
#[test]
fn test_hit_test_entity_bounds_low_opacity() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.4,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            }),
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 0.4 * 1.0 = 0.4 < 128/255 → 透明
    assert!(!hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=1.0 * foreground.a=0.4 → 合成α = 0.4 < 128/255 → 透明領域（foreground側が低い）
#[test]
fn test_hit_test_entity_bounds_low_foreground_alpha() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 1.0,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 0.4,
            }),
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 1.0 * 0.4 = 0.4 < 128/255 → 透明
    assert!(!hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Opacity なし + Brushes なし → デフォルト（1.0 * 1.0 = 1.0）→ ヒット
#[test]
fn test_hit_test_entity_bounds_no_opacity_no_brushes() {
    let mut world = World::new();
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::bounds(),
        ))
        .id();

    // Opacity=1.0(default), foreground.a=1.0(DEFAULT_FOREGROUND=BLACK) → ヒット
    assert!(hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=0.502 + Brushes::Inherit → DEFAULT_FOREGROUND (a=1.0) → 合成α ≥ 閾値 → ヒット
#[test]
fn test_hit_test_entity_bounds_inherit_foreground() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.502,
                ..Default::default()
            },
            Brushes::default(), // foreground = Inherit
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // Inherit → DEFAULT_FOREGROUND (BLACK, a=1.0)
    // 合成α = 0.502 * 1.0 ≥ 128/255 → ヒット
    assert!(hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}
