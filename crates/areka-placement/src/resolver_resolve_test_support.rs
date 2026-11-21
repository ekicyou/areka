//! `resolve_placement` の檻が共有するヘルパー（テスト専用）。
//!
//! 面別の DPI ヘルパー（`DPIS`/`px`/`work_area`/`offset_work_area`）は隣の
//! `resolver_test_support.rs` が持つ（`offset_work_area` はバルーンの収めのテストも使う）。
//! `cfg_of` は配置のテストが共に使う `placement_shared_test_support.rs` のものを、
//! `offset_work_area` は隣のものを出し直す。

pub(super) use super::test_support::offset_work_area;
use super::*;
use crate::config::{Alignment, BalloonSide, ScopeConfig};
pub(super) use crate::shared_test_support::cfg_of;

pub(super) fn scope_cfg(
    alignment: Alignment,
    default_x: Option<i32>,
    default_y: Option<i32>,
) -> ScopeConfig {
    ScopeConfig {
        alignment,
        default_x,
        default_y,
        balloon_alignment: BalloonSide::Left,
        balloon_offset: None,
        ..ScopeConfig::default()
    }
}

pub(super) fn input(scope: usize, w: i32, h: i32) -> ScopeInput {
    ScopeInput {
        scope,
        char_size: SizePx { w, h },
        balloon_size: SizePx { w: w / 2, h: h / 2 },
    }
}
