//! 既定の位置への並べ直し（areka-P0-move-canon 要件 8.6・8.8・8.9・9.2・design ResetPos）。
//!
//! `\![execute,resetwindowpos]` と右クリックのメニューの「シェル位置初期化」が呼ぶ。位置の記憶が
//! 無いときに起動で並べるのと同じ配置の規則（[`resolve_placement`]）を、**今表示している窓の
//! 大きさ**で 1 度解き、全スコープのキャラクターの窓をその位置へ書く。拡大率の値は読まない——
//! 今の大きさがすでに今の拡大率で描いた大きさなので、DPI でも将来の倍率でも構造的に追従する
//! （research §13 議題 1）。
//!
//! 書込は起動の連鎖の確定と同じ 3 手——[`move_window_to`]（`MoveCue`・バルーンは今のずらしで
//! 付いて動く）→ 既定の位置を自分で記す → [`persist_unremembered_char_positions`] で記憶へ書く。
//! `Anchored`・バルーンのずらし・吸い付きの設定は変えない（SSP 2.9.18 の実測 M9 と同じ）。

use bevy_ecs::prelude::*;
use tracing::{info, warn};
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{DPI, WindowPos};

use super::config::build_placement_config;
use super::follow::move_window_to;
use super::persist::persist_unremembered_char_positions;
use super::primary_monitor;
use super::reseed::BalloonPlacementInputs;
use super::resolver::{RectPx, ScopeInput, SizePx, resolve_placement};
use super::source::DescriptSource;
use super::spawn::GhostWindows;

/// 標準の位置の材料——窓の組の descript の値と走っているバルーンの配置の値
/// （design ResetPos State Management・areka-P0-window-link-directives 設計の決め 7）。
///
/// 窓の組ごとに 1 つ。起動の準備とシェルの切り替えの後始末が同じ形で置く（要件 7.4）。
/// 並べ直し（[`reset_window_positions`]）は descript の値から配置の設定を組んで読む。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct DefaultPlacementBasis {
    pub(crate) source: DescriptSource,
    pub(crate) balloon: BalloonPlacementInputs,
}

/// 全スコープのキャラクターの窓を既定の位置へ並べ直し、記憶へ書く。並べ直した窓が 1 つでも
/// あれば `true`。
///
/// 材料（[`DefaultPlacementBasis`]・[`GhostWindows`]・主モニタ）が無ければ
/// `warn!(event = "reset_window_pos_skipped")` を出して何もしない。窓の大きさが揃わない
/// スコープは同じ語の `warn!` で外し、残りで解く。
pub fn reset_window_positions(world: &mut World) -> bool {
    let skipped = |reason: &str| {
        warn!(
            event = "reset_window_pos_skipped",
            reason, "並べ直しの材料が無いので何もしない"
        );
        false
    };
    // キャラクターの窓の位置（P1〜P4）はバルーンの欄を読まないので、descript の値だけで組む。
    let Some(cfg) = world
        .get_resource::<DefaultPlacementBasis>()
        .map(|b| build_placement_config(&b.source.ghost_kv, &b.source.shell_kv))
    else {
        return skipped("no_default_basis");
    };
    let Some(ghost_windows) = world.get_resource::<GhostWindows>().cloned() else {
        return skipped("no_ghost_windows");
    };
    let monitors: Vec<Monitor> = world.query::<&Monitor>().iter(world).cloned().collect();
    let Some(primary) = primary_monitor(&monitors) else {
        return skipped("no_primary_monitor");
    };
    let wa = primary.work_area;
    let work_area = RectPx {
        left: wa.left,
        top: wa.top,
        right: wa.right,
        bottom: wa.bottom,
    };
    let dpi = u16::try_from(primary.dpi).unwrap_or(u16::MAX);
    let primary_dpi = DPI::from_dpi(dpi, dpi);

    let size_of = |entity: Option<Entity>| {
        let size = world.get::<WindowPos>(entity?)?.size?;
        Some(SizePx {
            w: size.width,
            h: size.height,
        })
    };
    let mut inputs = Vec::new();
    for scope in ghost_windows.scopes() {
        let char_size = size_of(ghost_windows.char_window(scope));
        let balloon_size = size_of(ghost_windows.balloon_window(scope));
        let (Some(char_size), Some(balloon_size)) = (char_size, balloon_size) else {
            warn!(
                event = "reset_window_pos_skipped",
                scope,
                reason = "no_window_size",
                "窓の大きさが揃わないスコープを外して並べ直す"
            );
            continue;
        };
        inputs.push(ScopeInput {
            scope,
            char_size,
            balloon_size,
        });
    }

    let mut moved = false;
    for p in resolve_placement(&cfg, work_area, &inputs, primary_dpi) {
        let Some(char_window) = ghost_windows.char_window(p.scope) else {
            continue;
        };
        let from = world.get::<WindowPos>(char_window).and_then(|w| w.position);
        if !move_window_to(world, char_window, p.char_pos.x, p.char_pos.y) {
            continue;
        }
        if let Some(mut gw) = world.get_resource_mut::<GhostWindows>() {
            gw.set_default_char_pos(p.scope, p.char_pos);
        }
        info!(
            event = "reset_window_pos",
            scope = p.scope,
            from_x = from.map(|f| f.x),
            from_y = from.map(|f| f.y),
            to_x = p.char_pos.x,
            to_y = p.char_pos.y,
            "キャラクターの窓を既定の位置へ並べ直した"
        );
        moved = true;
    }
    persist_unremembered_char_positions(world);
    moved
}

#[cfg(test)]
#[path = "reset_pos_tests.rs"]
mod reset_pos_tests;
