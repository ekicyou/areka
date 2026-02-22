//! Shared helpers for visual tests

use windows::core::Result;
use wintf::ecs::GraphicsCore;

/// テスト用の GraphicsCore を作成するヘルパー関数
pub fn setup_graphics() -> Result<GraphicsCore> {
    GraphicsCore::new()
}
