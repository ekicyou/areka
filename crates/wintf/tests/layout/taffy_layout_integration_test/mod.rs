//! taffy layout 統合テスト
//!
//! 元の `taffy_layout_integration_test.rs` を凝集度ごとにサブモジュールへ分割（挙動非破壊）。
//! - `wrapper_types`: TaffyStyle / TaffyComputedLayout ラッパー型テスト（テスト1.1-1.8）
//! - `box_components`: 高レベルレイアウトコンポーネントテスト（テスト2.1-2.5）
//! - `unit_integration`: ユニット・統合テスト（テスト7.1-7.9）

use bevy_ecs::prelude::*;
use taffy::prelude::*;
use wintf::ecs::layout::taffy::{TaffyComputedLayout, TaffyLayoutResource, TaffyStyle};
use wintf::ecs::layout::{
    Arrangement, BoxMargin, BoxPadding, BoxSize, BoxStyle, Dimension, FlexContainer, FlexItem,
    GlobalArrangement, LayoutRoot, LengthPercentage, LengthPercentageAuto, Rect,
};

mod box_components;
mod unit_integration;
mod wrapper_types;
