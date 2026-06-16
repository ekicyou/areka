use super::*;
use crate::ecs::types::Rect;
use windows_numerics::Matrix3x2;

mod alpha_mask;
mod entity_ex;
mod opacity;
mod tree_ex;

/// テスト用ヘルパー: 指定した bounds を持つ GlobalArrangement を作成
fn make_global_arrangement(left: f32, top: f32, right: f32, bottom: f32) -> GlobalArrangement {
    GlobalArrangement {
        transform: Matrix3x2::translation(left, top),
        bounds: Rect {
            left,
            top,
            right,
            bottom,
        },
    }
}
