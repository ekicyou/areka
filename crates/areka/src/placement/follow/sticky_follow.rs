//! 窓の組の連れ動き（areka-P0-window-link-directives・設計「sticky_follow」）。
//!
//! - [`partner_target`] は組の相手の新しい位置（純・4.2・5.2）
//! - [`wintf_moves_window`]／[`sync_drag_writer`] はドラッグで窓を書くのが wintf か areka かの
//!   決め方と、その付け直し（2.4・設計の決め 12）。`DragConfig.move_window` を書くのは
//!   窓を作るとき（`spawn.rs`）を除いて [`sync_drag_writer`] だけ

use bevy_ecs::prelude::*;
use wintf::ecs::drag::DragConfig;

use super::super::sticky_group_ledger::StickyGroupLedger;
use super::{Anchor, Anchored, CharWindowMarker, MonitorSnapshot, PointPx, SizePx, project_anchor};

/// 相手の新しい位置（純）: 始めの位置＋量を、相手の向きで射影する（4.2・5.2）。
///
/// 吸着した軸の量は捨ててから [`project_anchor`] で辺へ付ける。モニタの写しが無く射影が
/// 恒等に縮退しても、吸着した軸は始めの位置のまま動かない。
pub fn partner_target(
    start: PointPx,
    delta: PointPx,
    anchor: Anchor,
    size: SizePx,
    snapshot: Option<&MonitorSnapshot>,
) -> PointPx {
    let (dx, dy) = match anchor {
        Anchor::Top | Anchor::Bottom => (delta.x, 0),
        Anchor::Left | Anchor::Right => (0, delta.y),
        Anchor::Free => (delta.x, delta.y),
    };
    let raw = PointPx {
        x: start.x.saturating_add(dx),
        y: start.y.saturating_add(dy),
    };
    project_anchor(anchor, raw, size, snapshot)
}

/// このキャラクターの窓のドラッグで、wintf に窓を動かさせるか（純）。吸着なしで、しかも
/// 組に入っていない窓だけが真。組に入った窓は吸着なしでも areka が書く（相手と同じ回に書いて
/// ずらさないため・設計の決め 12）。
pub fn wintf_moves_window(anchor: Anchor, grouped: bool) -> bool {
    anchor.is_free() && !grouped
}

/// すべてのキャラクターの窓の `DragConfig.move_window` を、今の `Anchored` と帳簿から付け直す。
///
/// 帳簿が無い（窓を作る閉包がまだ据えていない）ときは、どの窓も組なしとして付ける。
pub(crate) fn sync_drag_writer(world: &mut World) {
    let grouped: Vec<u32> = world
        .get_resource::<StickyGroupLedger>()
        .map(|ledger| {
            ledger
                .groups()
                .iter()
                .flat_map(|g| g.members.iter().copied())
                .collect()
        })
        .unwrap_or_default();
    let mut query = world.query::<(&CharWindowMarker, &Anchored, &mut DragConfig)>();
    for (marker, anchored, mut drag) in query.iter_mut(world) {
        let in_group = u32::try_from(marker.scope).is_ok_and(|s| grouped.contains(&s));
        drag.move_window = wintf_moves_window(anchored.0, in_group);
    }
}
