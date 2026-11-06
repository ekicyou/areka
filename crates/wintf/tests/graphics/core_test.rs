//! GraphicsCore初期化のテスト
//!
//! このテストは以下を検証します:
//! - 複数回の初期化が可能であること

use wintf::ecs::GraphicsCore;

#[test]
fn test_graphics_core_multiple_creation() {
    // 複数回のGraphicsCore作成が可能であることを確認

    let result1 = GraphicsCore::new();
    assert!(result1.is_ok(), "1回目の作成が成功すること");

    let result2 = GraphicsCore::new();
    assert!(result2.is_ok(), "2回目の作成が成功すること");
}

#[test]
fn test_graphics_core_debug_output() {
    // GraphicsCoreのDebug出力が機能することを確認

    let graphics = GraphicsCore::new().expect("GraphicsCore作成成功");

    let debug_output = format!("{:?}", graphics);

    // Debug出力にGraphicsCoreという文字列が含まれることを確認
    assert!(
        debug_output.contains("GraphicsCore"),
        "Debug出力にGraphicsCoreが含まれること"
    );
}

#[test]
fn test_graphics_core_send_sync() {
    // GraphicsCoreがSend + Syncを実装していることを確認
    // （コンパイル時にチェックされる）

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<GraphicsCore>();
    assert_sync::<GraphicsCore>();
}
