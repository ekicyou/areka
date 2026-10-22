//! 窓の組の連れ動き（areka-P0-window-link-directives・設計「sticky_follow」）。
//!
//! - [`partner_target`] は組の相手の新しい位置（純・4.2・5.2）
//! - [`wintf_moves_window`]／[`sync_drag_writer`] はドラッグで窓を書くのが wintf か areka かの
//!   決め方と、その付け直し（2.4・設計の決め 12）。`DragConfig.move_window` を書くのは
//!   窓を作るとき（`spawn.rs`）を除いて [`sync_drag_writer`] だけ
//! - [`CharDragSession`] はキャラクターの窓のドラッグの控え。始め（[`begin_char_drag`]）に
//!   ドラッグした窓と組の相手の始めの位置・始めの向き・書き手を控え、途中
//!   （[`follow_partners_on_drag`]）と終わり（[`finish_char_drag`]）は「始めからの量」で相手を
//!   動かす（設計の決め 4）。wintf の `DraggingState` は読まない

use bevy_ecs::prelude::*;
use tracing::{info, warn};
use wintf::ecs::drag::DragConfig;
use wintf::ecs::{Point, WindowPos};

use super::super::persist::saved_form_anchor;
use super::super::sticky_group_ledger::StickyGroupLedger;
use super::{
    Anchor, Anchored, BalloonFollowTrigger, CharWindowMarker, MonitorSnapshot, PlacementRoute,
    PointPx, SizePx, char_pos_entries, char_pos_to_origin_x, enqueue_window_set_pos,
    follow_balloon, persist_entries, project_anchor, resize_window_to,
};

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

/// 今のキャラクターの窓のドラッグの控え（1 つだけ・wintf も同時に 1 つしかドラッグしない）。
#[derive(Resource, Debug, Clone)]
pub(crate) struct CharDragSession {
    dragged: Entity,
    /// ドラッグした窓の始めの位置（`OnDragStart` のときの `WindowPos`・量の起点）。
    dragged_start: PointPx,
    /// このドラッグで wintf が窓を動かすか（wintf が `move_window` を控えたのと同じ回に読む）。
    wintf_moves: bool,
    /// ドラッグを始めたときの向き（wintf が `move_window` を控えたのと同じ回に読む）。
    anchor: Anchor,
    /// 組の相手（窓の無い相手は入れない）とその始めの位置。
    partners: Vec<PartnerStart>,
}

#[derive(Debug, Clone, Copy)]
struct PartnerStart {
    scope: usize,
    window: Entity,
    start: PointPx,
}

fn position_px(world: &World, window: Entity) -> Option<PointPx> {
    world
        .get::<WindowPos>(window)
        .and_then(|wp| wp.position)
        .map(|p| PointPx { x: p.x, y: p.y })
}

/// 窓の大きさ。読めなければ 0（`project_anchor` が恒等へ縮退する）。
fn size_px(world: &World, window: Entity) -> SizePx {
    world
        .get::<WindowPos>(window)
        .and_then(|wp| wp.size)
        .map(|s| SizePx {
            w: s.width,
            h: s.height,
        })
        .unwrap_or(SizePx { w: 0, h: 0 })
}

fn scope_of(world: &World, window: Entity) -> Option<usize> {
    world.get::<CharWindowMarker>(window).map(|m| m.scope)
}

/// 帳簿での組の相手のスコープ（帳簿が無い・組に無ければ空）。
fn ledger_partners(world: &World, scope: usize) -> Vec<u32> {
    match (
        world.get_resource::<StickyGroupLedger>(),
        u32::try_from(scope),
    ) {
        (Some(ledger), Ok(scope)) => ledger.partners_of(scope),
        _ => Vec::new(),
    }
}

/// ドラッグの始めを控える（2.4・3.6・設計の決め 4）。組に無い窓でも向きと書き手は控える。
pub(crate) fn begin_char_drag(world: &mut World, dragged: Entity) {
    let (Some(anchor), Some(dragged_start)) = (
        world.get::<Anchored>(dragged).map(|a| a.0),
        position_px(world, dragged),
    ) else {
        warn!(
            entity = ?dragged,
            "sticky: ドラッグした窓の向きか位置が読めないので控えを作らない"
        );
        // 前のドラッグの控えを使い回さない。
        world.remove_resource::<CharDragSession>();
        return;
    };
    let wintf_moves = world
        .get::<DragConfig>(dragged)
        .is_some_and(|d| d.move_window);
    let wanted = scope_of(world, dragged)
        .map(|scope| ledger_partners(world, scope))
        .unwrap_or_default();
    let mut partners = Vec::new();
    for want in wanted {
        let found = world
            .query::<(Entity, &CharWindowMarker)>()
            .iter(world)
            .find(|(_, m)| u32::try_from(m.scope) == Ok(want))
            .map(|(e, m)| (e, m.scope));
        match found.and_then(|(window, scope)| {
            position_px(world, window).map(|start| PartnerStart {
                scope,
                window,
                start,
            })
        }) {
            Some(partner) => partners.push(partner),
            None => warn!(
                event = "sticky_member_absent",
                scope = want,
                when = "drag",
                "sticky: 組に窓の無いスコープがある（連れ動きから外す）"
            ),
        }
    }
    world.insert_resource(CharDragSession {
        dragged,
        dragged_start,
        wintf_moves,
        anchor,
        partners,
    });
}

/// ドラッグ中の窓の縛りの向き（控えがあれば始めの向き・無ければ `Anchored`・2.4）。
pub(crate) fn drag_anchor(world: &World, window: Entity) -> Option<Anchor> {
    match world.get_resource::<CharDragSession>() {
        Some(session) if session.dragged == window => Some(session.anchor),
        _ => world.get::<Anchored>(window).map(|a| a.0),
    }
}

/// 今のドラッグで wintf が窓を動かすか（`begin_char_drag` で控えた値。控えが無ければ今の
/// `DragConfig.move_window`）。
pub(crate) fn drag_wintf_moves(world: &World, window: Entity) -> bool {
    match world.get_resource::<CharDragSession>() {
        Some(session) if session.dragged == window => session.wintf_moves,
        _ => world
            .get::<DragConfig>(window)
            .is_some_and(|d| d.move_window),
    }
}

/// `dragged` の控えを返す。無い・別の窓のものなら、組の相手がいるときだけ記録する（4.1）。
fn session_for(world: &World, dragged: Entity) -> Option<CharDragSession> {
    let reason = match world.get_resource::<CharDragSession>() {
        Some(session) if session.dragged == dragged => return Some(session.clone()),
        Some(_) => "other_window",
        None => "no_session",
    };
    if let Some(scope) = scope_of(world, dragged)
        && !ledger_partners(world, scope).is_empty()
    {
        warn!(
            event = "sticky_follow_skipped",
            scope, reason, "sticky: ドラッグの控えが無いので組の相手を動かさない"
        );
    }
    None
}

/// 相手を始めの位置＋量へ書き、バルーンを付いて動かす（経路なし＝ドラッグと同じ・4.1〜4.4）。
/// 書けた相手とその位置を返す。
fn move_partners(
    world: &mut World,
    session: &CharDragSession,
    delta: PointPx,
) -> Vec<(PartnerStart, PointPx)> {
    let mut moved = Vec::new();
    for partner in &session.partners {
        let Some(anchor) = world.get::<Anchored>(partner.window).map(|a| a.0) else {
            warn!(
                scope = partner.scope,
                entity = ?partner.window,
                "sticky: 組の相手の向きが読めないので動かさない"
            );
            continue;
        };
        let size = size_px(world, partner.window);
        let target = partner_target(
            partner.start,
            delta,
            anchor,
            size,
            world.get_resource::<MonitorSnapshot>(),
        );
        if enqueue_window_set_pos(world, partner.window, target.x, target.y, None, None) {
            let pos = Point {
                x: target.x,
                y: target.y,
            };
            follow_balloon(world, partner.window, pos, BalloonFollowTrigger::Drag);
            moved.push((*partner, target));
        }
    }
    moved
}

fn delta_from(start: PointPx, pos: PointPx) -> PointPx {
    PointPx {
        x: pos.x.saturating_sub(start.x),
        y: pos.y.saturating_sub(start.y),
    }
}

/// ドラッグの途中: 始めからの量で組の相手を動かす（4.1〜4.4）。1 回ごとは記録しない（9.3）。
pub(crate) fn follow_partners_on_drag(world: &mut World, dragged: Entity, pos: PointPx) {
    let Some(session) = session_for(world, dragged) else {
        return;
    };
    move_partners(world, &session, delta_from(session.dragged_start, pos));
}

/// ドラッグの終わり: 最終の量で相手を書き、相手の位置を記憶へ書き、控えを外す（4.5・9.3）。
/// ドラッグ中に台本が向きを変えていれば、今の向きで当て直す（2.4）。
pub(crate) fn finish_char_drag(world: &mut World, dragged: Entity, final_pos: PointPx) {
    let Some(session) = session_for(world, dragged) else {
        return;
    };
    let delta = delta_from(session.dragged_start, final_pos);
    for (partner, pos) in move_partners(world, &session, delta) {
        // 次の起動は descript の向きで読み戻すので、記憶の形もそれにそろえる（4.8）。
        let Some(form) = saved_form_anchor(world, partner.window) else {
            warn!(
                scope = partner.scope,
                "sticky: 組の相手の記憶の形が決まらないので位置を記憶に書かない"
            );
            continue;
        };
        let saved = char_pos_to_origin_x(form, pos, size_px(world, partner.window));
        persist_entries(world, char_pos_entries(partner.scope as u32, saved));
    }
    if !session.partners.is_empty()
        && let Some(scope) = scope_of(world, dragged)
    {
        let partners: Vec<usize> = session.partners.iter().map(|p| p.scope).collect();
        info!(
            event = "sticky_follow",
            trigger = "drag",
            scope,
            partners = ?partners,
            dx = delta.x,
            dy = delta.y,
            "sticky: 組の相手をドラッグに連れ動かした"
        );
    }
    world.remove_resource::<CharDragSession>();
    if world
        .get::<Anchored>(dragged)
        .is_some_and(|now| now.0 != session.anchor)
    {
        let size = size_px(world, dragged);
        resize_window_to(world, dragged, size, PlacementRoute::MoveCue);
    }
}
