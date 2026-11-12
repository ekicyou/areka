//! metrics.rs 系コンポーネントのテスト
//!
//! `LayoutScale` の Default を確かめる。
//!
//! 対象: crates/wintf/src/ecs/layout/metrics.rs

use wintf::ecs::layout::LayoutScale;

/// LayoutScale::default() は恒等スケール (1.0, 1.0) を返す
#[test]
fn test_layout_scale_default_is_identity() {
    let scale = LayoutScale::default();
    assert_eq!(scale.x, 1.0, "default scale.x should be 1.0");
    assert_eq!(scale.y, 1.0, "default scale.y should be 1.0");
}
