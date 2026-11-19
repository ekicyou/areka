//! 判定に入れるキャラ窓の矩形の列（balloon-flip-side-rules・設計 OtherChars・要件 2.6／2.8／2.9）と、
//! 起動の並べ直しの後に全スコープのバルーンを判定し直す系（設計 RejudgeSystem・balloon-flip-side-rules 要件 2.7／2.10／4.4）。
//!
//! 「表示している」は、キャラ窓の子の描画面の `Visual.is_visible` のどれかが真かで読む（設計 D3・前提 A2）。
//! ほかの子が窓に付くようになると黙って「表示している」に倒れるので、そのときはこの読み方を確かめ直す。

use bevy_ecs::prelude::*;
use tracing::warn;
use wintf::ecs::{FrameFinalize, Point, Visual};

use super::{
    BALLOON_LIMIT_RUNTIME_CONTEXT, BALLOON_LIMIT_UNRESOLVED_TAG, BalloonFollowTrigger,
    BalloonLimit, PlacementRoute, follow_balloon, window_rect_of,
};
use crate::chain_finalize::ChainFinalized;
use crate::resolver::RectPx;
use crate::spawn::GhostWindows;

/// 判定に入れるキャラ窓の矩形の列（スコープの昇順）。矩形が決まらない窓と表示していない窓は入れない（balloon-flip-side-rules 要件 2.6）。
///
/// 自分のスコープも外さない（判定の核が数えない）。`GhostWindows` が無ければ空の列で、記録はしない。
pub(super) fn shown_char_rects(world: &World) -> Vec<(usize, RectPx)> {
    let Some(gw) = world.get_resource::<GhostWindows>() else {
        return Vec::new();
    };
    gw.scopes()
        .filter_map(|scope| {
            let window = gw.char_window(scope)?;
            let rect = window_rect_of(world, window)?;
            is_shown(world, window).then_some((scope, rect))
        })
        .collect()
}

/// 箱の文字の面の名前（`areka-emo-text` の `surface_window_child.rs` が付ける）。
const TEXT_BOX_SURFACE_NAME: &str = "emo-text-box-surface";

/// キャラ窓の子の描画面のどれかが可視か（子が無い・全部不可視なら偽）。
///
/// 箱の文字の面は数えない。`Visual::default()`（可視）のまま付き、`\s[-1]` の後も次の箱の同期まで
/// 残るので、数えると隠した直後の 1 フレームのあいだ隠れた窓が「表示している」に倒れる。
fn is_shown(world: &World, window: Entity) -> bool {
    world.get::<Children>(window).is_some_and(|children| {
        children.iter().any(|c| {
            world
                .get::<Name>(c)
                .is_none_or(|n| n.as_str() != TEXT_BOX_SURFACE_NAME)
                && world.get::<Visual>(c).is_some_and(|v| v.is_visible)
        })
    })
}

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

/// 全スコープの `BalloonLimit(true)` のバルーンを今のキャラ窓の位置で判定し直す（balloon-flip-side-rules 要件 2.10）。
///
/// 呼び手＝上の系・`reset_window_positions`・`realign_chain_once_with`（窓を書き終えた後）。
/// 随伴の引き金は並べ直しの書き込みと同じ `MoveCue`（新しい語を作らない・balloon-flip-side-rules 要件 4.2）。
pub(crate) fn rejudge_all_balloons(world: &mut World) {
    let Some(gw) = world.get_resource::<GhostWindows>() else {
        warn!(
            context = BALLOON_LIMIT_RUNTIME_CONTEXT,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} GhostWindows が無く判定し直すスコープを決められない → 判定し直さない"
        );
        return;
    };
    // `limit,0` のバルーンは並べ替えの判定に入らない（可視性の守りを当てない随伴で守りの寄せを書き戻さない）。
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
