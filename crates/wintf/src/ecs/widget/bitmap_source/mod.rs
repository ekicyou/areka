//! BitmapSource ウィジェットモジュール
//!
//! WIC画像読み込みとD2D描画を提供する。
//! 画像の解読は裏の作業で 1 回だけ行い、メモリの画像として World へ渡す。
//! UI スレッド（描画・αのマスク）はその写しを読むだけで、解読はしない。

mod alpha_mask;
mod bitmap_source;
mod resource;
pub mod systems;
mod task_pool;
mod wic_core;

pub use alpha_mask::AlphaMask;
pub use bitmap_source::BitmapSource;
pub use resource::{BitmapSourceGraphics, BitmapSourceResource};
pub use systems::{draw_bitmap_sources, generate_alpha_mask_system};
pub use task_pool::{BoxedCommand, CommandSender, WintfTaskPool};
pub use wic_core::WicCore;

#[cfg(test)]
mod tests;
