use wintf::ecs::Visual;

// ========================================================================
// Visual API unit tests (Task 1.2)
// ========================================================================

#[test]
fn test_visual_default_values() {
    let visual = Visual::default();
    assert_eq!(visual.opacity, 1.0);
    assert!(visual.is_visible);
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
    let low = Visual {
        opacity: -0.5,
        ..Default::default()
    };
    assert_eq!(low.clamped_opacity(), 0.0);

    let high = Visual {
        opacity: 2.0,
        ..Default::default()
    };
    assert_eq!(high.clamped_opacity(), 1.0);
}

#[test]
fn test_visual_set_visible_true() {
    let mut visual = Visual::default();
    visual.set_visible(false);
    assert!(!visual.is_visible);
    visual.set_visible(true);
    assert!(visual.is_visible);
}

#[test]
fn test_visual_set_visible_false() {
    let mut visual = Visual::default();
    visual.set_visible(false);
    assert!(!visual.is_visible);
}

#[test]
fn test_visual_clamped_opacity_clamps_to_unit_range() {
    let test_values = [0.0, 0.25, 0.5, 0.75, 1.0, -0.1, 1.5, -100.0, 100.0];

    for &value in &test_values {
        let visual = Visual {
            opacity: value,
            ..Default::default()
        };
        assert_eq!(
            visual.clamped_opacity(),
            value.clamp(0.0, 1.0),
            "Mismatch for value {}",
            value
        );
    }
}
