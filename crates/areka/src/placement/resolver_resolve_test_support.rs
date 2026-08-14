//! `resolve_placement` の檻が共有するヘルパー（テスト専用）。
//!
//! 元は `resolver_resolve_tests.rs` の冒頭に置かれていたが、同ファイルが 1,000 行規約を
//! 超えたため主題別 4 ファイルへ分割するにあたり、共有部分をここへ集約した。
//! 中身は移設前と逐語同一で、可視性を `pub(super)` へ広げただけである。
//!
//! 面別の DPI ヘルパー（`DPIS`/`px`/`work_area`）は隣の `resolver_test_support.rs` が持つ。

use super::test_support::px;
use super::*;
use crate::placement::config::{Alignment, BalloonSide, PlacementConfig, ScopeConfig};

/// 左上が原点でない work area（クランプ境界の left/top 依存を暴く）。
pub(super) fn offset_work_area(dpi: i32) -> RectPx {
    RectPx {
        left: px(64, dpi),
        top: px(40, dpi),
        right: px(64 + 1920, dpi),
        bottom: px(40 + 1080, dpi),
    }
}

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

pub(super) fn cfg_of(scopes: Vec<(usize, ScopeConfig)>) -> PlacementConfig {
    PlacementConfig {
        scopes: scopes.into_iter().collect(),
        zorder_raw: None,
        sticky_window_raw: None,
        shell_dpi_raw: None,
    }
}

pub(super) fn input(scope: usize, w: i32, h: i32) -> ScopeInput {
    ScopeInput {
        scope,
        char_size: SizePx { w, h },
        balloon_size: SizePx { w: w / 2, h: h / 2 },
    }
}
