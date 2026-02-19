//! DCompGraphicsResource 状態遷移テスト (Task 11.3)

use wintf::ecs::DCompGraphicsResource;

#[test]
fn test_new_empty_is_not_valid() {
    let resource = DCompGraphicsResource::new_empty();
    assert!(!resource.is_valid());
}

#[test]
fn test_new_empty_dcomp_returns_none() {
    let resource = DCompGraphicsResource::new_empty();
    assert!(resource.dcomp().is_none());
}

#[test]
fn test_new_empty_desktop_returns_none() {
    let resource = DCompGraphicsResource::new_empty();
    assert!(resource.desktop().is_none());
}

#[test]
fn test_invalidate_on_empty_is_safe() {
    let mut resource = DCompGraphicsResource::new_empty();
    resource.invalidate(); // should not panic
    assert!(!resource.is_valid());
}

#[test]
fn test_new_from_d2d_device_is_valid() {
    // GPU テスト: D2D デバイスが利用可能な環境のみ
    let graphics = wintf::ecs::GraphicsCore::new().expect("GraphicsCore作成失敗");
    let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
    let resource = DCompGraphicsResource::new(d2d).expect("DCompGraphicsResource作成失敗");

    assert!(resource.is_valid());
    assert!(resource.dcomp().is_some());
    assert!(resource.desktop().is_some());
}

#[test]
fn test_invalidate_after_init() {
    let graphics = wintf::ecs::GraphicsCore::new().expect("GraphicsCore作成失敗");
    let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
    let mut resource = DCompGraphicsResource::new(d2d).expect("DCompGraphicsResource作成失敗");

    assert!(resource.is_valid());

    resource.invalidate();

    assert!(!resource.is_valid());
    assert!(resource.dcomp().is_none());
    assert!(resource.desktop().is_none());
}
