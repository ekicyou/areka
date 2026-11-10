use bevy_ecs::prelude::*;

// 共通型モジュールから re-export
pub use crate::ecs::types::{Offset, Size};

/// テキストレイアウトの物理サイズ
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct TextLayoutMetrics {
    pub width: f32,  // Physical width (pixels)
    pub height: f32, // Physical height (pixels)
}

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
