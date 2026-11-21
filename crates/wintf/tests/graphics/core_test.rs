//! GraphicsCore初期化のテスト
//!
//! このテストは以下を検証します:
//! - 複数回の初期化が可能であること

use wintf::ecs::GraphicsCore;

#[test]
fn test_graphics_core_multiple_creation() {
    let result1 = GraphicsCore::new();
    assert!(result1.is_ok(), "1回目の作成が成功すること");

    let result2 = GraphicsCore::new();
    assert!(result2.is_ok(), "2回目の作成が成功すること");
}

#[test]
fn test_graphics_core_debug_output() {
    let graphics = GraphicsCore::new().expect("GraphicsCore作成成功");

    let debug_output = format!("{:?}", graphics);

    assert!(
        debug_output.contains("GraphicsCore"),
        "Debug出力にGraphicsCoreが含まれること"
    );
}

#[test]
fn test_graphics_core_send_sync() {
    // コンパイル時に Send + Sync を確かめる

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<GraphicsCore>();
    assert_sync::<GraphicsCore>();
}
