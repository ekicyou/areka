//! CompositionMode と Window のユニットテスト (Task 11.1)

use wintf::ecs::{CompositionMode, Window};

#[test]
fn test_composition_mode_default_is_ulw() {
    assert_eq!(CompositionMode::default(), CompositionMode::ULW);
}

#[test]
fn test_window_default_composition_mode_is_ulw() {
    let window = Window::default();
    assert_eq!(window.composition_mode(), CompositionMode::ULW);
}

#[test]
fn test_window_dcomp_mode_setter() {
    let window = Window {
        composition_mode: CompositionMode::DComp,
        ..Default::default()
    };
    assert_eq!(window.composition_mode(), CompositionMode::DComp);
}

#[test]
fn test_composition_mode_clone_and_eq() {
    let mode = CompositionMode::DComp;
    let cloned = mode.clone();
    assert_eq!(mode, cloned);
    assert_ne!(CompositionMode::ULW, CompositionMode::DComp);
}

#[test]
fn test_composition_mode_debug() {
    let mode = CompositionMode::ULW;
    let debug_str = format!("{:?}", mode);
    assert!(debug_str.contains("ULW"));
}
