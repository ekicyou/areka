//! 判定に入れるキャラ窓の矩形の列（balloon-flip-side-rules・設計 OtherChars・要件 2.6／2.8／2.9）。
//!
//! 「表示している」は、キャラ窓の子の描画面の `Visual.is_visible` のどれかが真かで読む（設計 D3・前提 A2）。
//! ほかの子が窓に付くようになると黙って「表示している」に倒れるので、そのときはこの読み方を確かめ直す。

use bevy_ecs::prelude::*;
use wintf::ecs::Visual;

use super::window_rect_of;
use crate::resolver::RectPx;
use crate::spawn::GhostWindows;

/// 判定に入れるキャラ窓の矩形の列（スコープの昇順）。矩形が決まらない窓と表示していない窓は入れない（要件 2.6）。
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

/// キャラ窓の子の描画面のどれかが可視か（子が無い・全部不可視なら偽）。
fn is_shown(world: &World, window: Entity) -> bool {
    world.get::<Children>(window).is_some_and(|children| {
        children
            .iter()
            .any(|c| world.get::<Visual>(c).is_some_and(|v| v.is_visible))
    })
}
