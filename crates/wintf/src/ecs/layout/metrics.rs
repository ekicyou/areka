use bevy_ecs::prelude::*;
use tracing::warn;

// 共通型モジュールから re-export
pub use crate::ecs::types::{Offset, Size};

/// テキストレイアウトの物理サイズ
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct TextLayoutMetrics {
    pub width: f32,  // Physical width (pixels)
    pub height: f32, // Physical height (pixels)
}

/// レイアウトスケール（DPIスケール、ViewBox等）
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct LayoutScale {
    pub x: f32,
    pub y: f32,
}

impl LayoutScale {
    /// スケールのバリデーション（ゼロ値チェック）
    ///
    /// ゼロ値が検出された場合は警告ログを出力します。
    /// これはmayレベルの推奨事項であり、実行時エラーとはしません。
    pub fn validate(&self) {
        if self.x == 0.0 {
            warn!("LayoutScale.x is zero, which may cause layout issues");
        }
        if self.y == 0.0 {
            warn!("LayoutScale.y is zero, which may cause layout issues");
        }
    }
}

impl Default for LayoutScale {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}
