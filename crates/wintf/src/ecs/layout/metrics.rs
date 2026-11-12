// 共通型モジュールから re-export
pub use crate::ecs::types::{Offset, Size};

/// レイアウトスケール（DPIスケール、ViewBox等）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutScale {
    pub x: f32,
    pub y: f32,
}

impl Default for LayoutScale {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}
