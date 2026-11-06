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
fn test_visual_set_opacity_stores_clamped_value() {
    for (value, expected) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (-0.1, 0.0), (1.5, 1.0)] {
        let mut visual = Visual::default();
        visual.set_opacity(value);
        assert_eq!(visual.opacity, expected, "set_opacity({value})");
    }
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
