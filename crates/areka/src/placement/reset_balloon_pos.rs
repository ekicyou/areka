//! バルーン位置初期化（`\![execute,resetballoonpos]`・メニュー・近道の 3 つの入口が呼ぶ 1 つの
//! 関数・areka-P0-window-link-directives 要件 7.1〜7.4・9.5・design「reset_balloon_pos」）。
//!
//! 標準の位置は、シェルの切り替えが部品を入れ直すのと同じ [`resolve_standard_scopes`] で解く
//! （記憶は重ねない）。材料（[`DefaultPlacementBasis`]）は起動とシェルの切り替えが同じ形で置き、
//! 窓の今の DPI で解くので、同じシェル・同じ拡大率なら同じ位置になる（7.4）。
//! キャラクターの窓・`Anchored`・`AnchorSource`・窓の組の帳簿には触らない（7.2）。

use bevy_ecs::world::World;
use tracing::{info, warn};
use wintf::ecs::WindowPos;

use super::diag::PlacementRoute;
use super::follow::{BalloonFollow, BalloonFollowTrigger, follow_balloon};
use super::persist::{balloon_offset_entries, persist_entries};
use super::reseed::{StandardScope, resolve_standard_scopes};
use super::reset_pos::DefaultPlacementBasis;
use super::spawn::{BalloonKeywordBase, GhostWindows};

/// 初期化のきっかけ（要件 9.5 の記録の欄）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonResetTrigger {
    Script,
    Menu,
    Shortcut,
}

impl BalloonResetTrigger {
    fn as_str(self) -> &'static str {
        match self {
            Self::Script => "script",
            Self::Menu => "menu",
            Self::Shortcut => "shortcut",
        }
    }
}

/// すべてのバルーンを各キャラクターの窓に対する標準の位置へ戻し、記憶へ書く。1 つでも戻せたら真。
///
/// 材料（[`DefaultPlacementBasis`]・[`GhostWindows`]）が無ければ
/// `warn!(event = "balloon_pos_reset_skipped", reason)` を出して何もしない。標準の位置を解けなかった
/// スコープは同じ語の `warn!`（scope・reason）で外し、残りを戻す。
pub fn reset_balloon_positions(world: &mut World, trigger: BalloonResetTrigger) -> bool {
    let skipped = |reason: &str| {
        warn!(
            event = "balloon_pos_reset_skipped",
            reason,
            trigger = trigger.as_str(),
            "バルーン位置初期化の材料が無いので何もしない"
        );
        false
    };
    let Some(basis) = world.get_resource::<DefaultPlacementBasis>().cloned() else {
        return skipped("no_default_basis");
    };
    let Some(windows) = world.get_resource::<GhostWindows>().cloned() else {
        return skipped("no_ghost_windows");
    };

    let resolved = resolve_standard_scopes(world, &windows, &basis.source, &basis.balloon);
    for scope in windows.scopes() {
        if !resolved.iter().any(|s| s.running.input.scope == scope) {
            warn!(
                event = "balloon_pos_reset_skipped",
                scope,
                reason = "not_resolved",
                trigger = trigger.as_str(),
                "標準の位置を解けないスコープのバルーンは戻さない"
            );
        }
    }

    let mut reset = false;
    for StandardScope { running, placement } in resolved {
        let scope = running.input.scope;
        let c = running.char_window;
        let base = placement.balloon_offset_base;
        world
            .entity_mut(c)
            .insert(BalloonFollow::new(running.balloon_window, base));
        match placement.balloon_keyword_base {
            Some((mode, adjust)) => {
                world
                    .entity_mut(c)
                    .insert(BalloonKeywordBase { mode, adjust });
            }
            None => {
                world.entity_mut(c).remove::<BalloonKeywordBase>();
            }
        }
        // シェルの切り替えでバルーンを置き直すときと同じ明示の配置（見張りなし）。
        follow_balloon(
            world,
            c,
            running.char_pos,
            BalloonFollowTrigger::Placement(PlacementRoute::Restore),
        );
        // 記憶へ標準のずらしを書く（次の起動も標準の位置に出る・7.3・裁定 8）。
        persist_entries(world, balloon_offset_entries(scope as u32, base.offset));
        let to = world
            .get::<WindowPos>(running.balloon_window)
            .and_then(|wp| wp.position);
        info!(
            event = "balloon_pos_reset",
            scope,
            to_x = to.map(|p| p.x),
            to_y = to.map(|p| p.y),
            offset_x = base.offset.x,
            offset_y = base.offset.y,
            trigger = trigger.as_str(),
            "バルーンを標準の位置へ戻した"
        );
        reset = true;
    }
    reset
}

#[cfg(test)]
#[path = "reset_balloon_pos_tests.rs"]
mod tests;
