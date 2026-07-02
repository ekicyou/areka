//! GraphicsCore アクセサのギャップテスト (W3b-T)
//!
//! 既存テスト（core_test / core_ecs_test / reinit_unit_test）は生成・invalidate・
//! is_valid を検証済みだが、全アクセサの Some/None 遷移は未固定だったため追加する。
//! 対象: crates/wintf/src/ecs/graphics/core.rs
//!
//! Note: 旧 DCompGraphicsResource Debug テストは DComp 定義撤去（task 3.2）で削除。
//! WUC 版の状態遷移は src の wuc_resource.rs 単体テストが担う。

use wintf::ecs::GraphicsCore;

/// new() 直後は全アクセサが Some を返す
#[test]
fn all_accessors_some_after_new() {
    let graphics = GraphicsCore::new().expect("GraphicsCore creation");
    assert!(graphics.is_valid());
    assert!(graphics.d2d_factory().is_some());
    assert!(graphics.d2d_device().is_some());
    assert!(graphics.dwrite_factory().is_some());
    assert!(graphics.device_context().is_some());
    assert!(graphics.d3d().is_some());
    assert!(graphics.dxgi().is_some());
}

/// invalidate() 後は全アクセサが None を返す（デバイスロスト時の安全性）
#[test]
fn all_accessors_none_after_invalidate() {
    let mut graphics = GraphicsCore::new().expect("GraphicsCore creation");
    graphics.invalidate();
    assert!(!graphics.is_valid());
    assert!(graphics.d2d_factory().is_none());
    assert!(graphics.d2d_device().is_none());
    assert!(graphics.dwrite_factory().is_none());
    assert!(graphics.device_context().is_none());
    assert!(graphics.d3d().is_none());
    assert!(graphics.dxgi().is_none());
}
