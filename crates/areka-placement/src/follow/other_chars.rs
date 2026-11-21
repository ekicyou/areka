//! 起動の並べ直しの後に全スコープのバルーンを判定し直す系。

use bevy_ecs::prelude::*;
use tracing::warn;
use wintf::ecs::{FrameFinalize, Point};

use super::{
    BALLOON_LIMIT_RUNTIME_CONTEXT, BALLOON_LIMIT_UNRESOLVED_TAG, BalloonFollowTrigger,
    BalloonLimit, PlacementRoute, follow_balloon, window_rect_of,
};
use crate::chain_finalize::ChainFinalized;
use crate::spawn::GhostWindows;

/// 判定し直しの系をプロセスに 1 回だけ載せる（`ghost_session::register_systems` から）。
pub fn register_balloon_rejudge(world: &mut World) {
    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        rejudge_balloons_after_chain_finalize.run_if(resource_added::<ChainFinalized>),
    );
}

/// 起動の並べ直しの後の判定し直しの系（`ChainFinalized` の挿入で 1 度・中身は [`rejudge_all_balloons`]）。
fn rejudge_balloons_after_chain_finalize(world: &mut World) {
    rejudge_all_balloons(world);
}

/// 全スコープの `BalloonLimit(true)` のバルーンを今のキャラ窓の位置で判定し直す。
///
/// 呼び手＝上の系・`reset_window_positions`・`realign_chain_once_with`（窓を書き終えた後）。
/// 随伴の引き金は並べ直しの書き込みと同じ `MoveCue`。
pub(crate) fn rejudge_all_balloons(world: &mut World) {
    let Some(gw) = world.get_resource::<GhostWindows>() else {
        warn!(
            context = BALLOON_LIMIT_RUNTIME_CONTEXT,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} GhostWindows が無く判定し直すスコープを決められない → 判定し直さない"
        );
        return;
    };
    // `limit,0` のバルーンは対象に入れない（可視性の守りを当てない随伴で守りの寄せを書き戻さないため）。
    let targets: Vec<(usize, Entity)> = gw
        .scopes()
        .filter(|&scope| {
            gw.balloon_window(scope)
                .and_then(|b| world.get::<BalloonLimit>(b))
                .is_some_and(|l| l.0)
        })
        .filter_map(|scope| Some((scope, gw.char_window(scope)?)))
        .collect();
    for (scope, char_window) in targets {
        let Some(rect) = window_rect_of(world, char_window) else {
            warn!(
                scope,
                context = BALLOON_LIMIT_RUNTIME_CONTEXT,
                "{BALLOON_LIMIT_UNRESOLVED_TAG} キャラ窓の位置が未確定のため判定し直せない → このスコープを飛ばす"
            );
            continue;
        };
        let pos = Point {
            x: rect.left,
            y: rect.top,
        };
        follow_balloon(
            world,
            char_window,
            pos,
            BalloonFollowTrigger::Placement(PlacementRoute::MoveCue),
        );
    }
}
