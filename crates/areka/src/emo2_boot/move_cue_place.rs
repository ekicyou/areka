//! `\![move]` の UI スレッドでの適用（design.md「MovePlace」・要件 1.9・2.5・2.7〜2.12・
//! 3.1〜3.3・3.5〜3.7・4.7・9.1）。
//!
//! 基準を World から解いて [`BaseFrame`] にし、純粋な計算（[`compute_move_target`]）で吸い付きの
//! 前の位置を求め、対象の窓の [`Anchored`] で吸い付きの射影（[`project_anchor`]）に通してから
//! [`move_window_to`] で書く。位置の記憶・`Anchored`・既定の位置・バルーンのずらしは変えない
//! （書く口は `move_window_to` だけ・バルーンは今のずらしで付いて動く）。
//!
//! どの失敗も panic せずトークを止めない: 動かせないときは `move_skipped`、基準を置き換えたときは
//! `move_frame_substituted`、`Anchored` が無いときは `move_anchor_missing` の `warn!` を残す。

use bevy_ecs::prelude::World;
use tracing::{info, warn};
use wintf::ecs::WindowPos;
use wintf::ecs::window::monitor::Monitor;

use areka_emo_compose::ScaleRatio;

use super::geometry::{BaseFrame, FrameNote, compute_move_target};
use super::{BaseposResolver, CanonDefaultBasepos, MoveBase, MoveDirective};
use crate::placement::chain_finalize::ScriptPlaced;
use crate::placement::follow::{
    Anchored, MonitorSnapshot, follow_sticky_move, log_sticky_ignored, move_window_to,
    project_anchor, work_area_for_window,
};
use crate::placement::primary_monitor;
use crate::placement::resolver::{PointPx, RectPx, SizePx};
use crate::placement::spawn::GhostWindows;

/// 窓の今の矩形（位置と正の大きさが揃うときだけ）。
fn window_rect(world: &World, scope: u32) -> Option<RectPx> {
    let gw = world.get_resource::<GhostWindows>()?;
    let wp = world.get::<WindowPos>(gw.char_window(scope as usize)?)?;
    let (p, s) = (wp.position?, wp.size?);
    (s.width > 0 && s.height > 0).then(|| RectPx {
        left: p.x,
        top: p.y,
        right: p.x.saturating_add(s.width),
        bottom: p.y.saturating_add(s.height),
    })
}

/// 窓の矩形の基準位置（今の既定＝幅÷2・下端）を左上からのずれで返す。
fn canon_basepos(rect: RectPx) -> PointPx {
    CanonDefaultBasepos.basepos(wintf::ecs::SizeI::new(
        rect.right - rect.left,
        rect.bottom - rect.top,
    ))
}

fn window_frame(rect: RectPx) -> BaseFrame {
    BaseFrame::Window {
        rect,
        basepos: canon_basepos(rect),
    }
}

/// `screen`: 動かす窓の中心のモニタ（外れたら最近傍）の作業領域（要件 3.1・3.2）。
fn screen_frame(world: &World, target: RectPx) -> Result<BaseFrame, &'static str> {
    world
        .get_resource::<MonitorSnapshot>()
        .and_then(|snap| work_area_for_window(snap, target))
        .map(BaseFrame::Area)
        .ok_or("no_work_area")
}

/// 基準の枠を解く。動かせないときは `move_skipped` の理由を返す。
fn resolve_frame(
    world: &mut World,
    directive: &MoveDirective,
    target: RectPx,
    notes: &mut Vec<FrameNote>,
) -> Result<BaseFrame, &'static str> {
    match directive.base {
        MoveBase::Scope(n) => match window_rect(world, n) {
            Some(rect) => Ok(window_frame(rect)),
            None => {
                // 基準の窓が無い → screen へ倒す（要件 3.7）。
                notes.push(FrameNote::MissingBaseScope { scope: n });
                screen_frame(world, target)
            }
        },
        MoveBase::Me => Ok(window_frame(target)),
        MoveBase::Screen => screen_frame(world, target),
        MoveBase::PrimaryScreen => {
            let monitors: Vec<Monitor> = world.query::<&Monitor>().iter(world).cloned().collect();
            let wa = primary_monitor(&monitors)
                .ok_or("no_primary_monitor")?
                .work_area;
            Ok(BaseFrame::Area(RectPx {
                left: wa.left,
                top: wa.top,
                right: wa.right,
                bottom: wa.bottom,
            }))
        }
        MoveBase::Global => Ok(BaseFrame::Origin),
    }
}

/// `\![move]` の [`MoveDirective`] を UI スレッドで窓へ書く。書いたら `true`。
///
/// 動かせなかったときは `warn!` のうえ `false`（トークは止めない）。`k` は対象のスコープの
/// シェルに今かかっている拡大率で、台本の X・Y（作者基準）を物理 px へ写すのにだけ使う。
/// 書けたら、組の相手を実際に動いた量だけ連れ動かす（`ignore_sticky_window` が真なら連れ動かさず
/// 記録だけ・areka-P0-window-link-directives 要件 5.1・5.3）。書けなければ連れ動かさない。
/// `Anchored` は読むだけで書かない。
pub fn apply_move_directive(world: &mut World, directive: &MoveDirective, k: ScaleRatio) -> bool {
    let scope = directive.scope;
    let skipped = |reason: &str| {
        warn!(
            event = "move_skipped",
            scope, reason, "\\![move] の材料が無いので動かさない"
        );
        false
    };

    let Some(gw) = world.get_resource::<GhostWindows>() else {
        return skipped("no_ghost_windows");
    };
    let Some(target) = gw.char_window(scope as usize) else {
        return skipped("no_target_window");
    };
    let Some(target_rect) = window_rect(world, scope) else {
        return skipped("no_target_position");
    };

    let mut notes = Vec::new();
    let frame = match resolve_frame(world, directive, target_rect, &mut notes) {
        Ok(frame) => frame,
        Err(reason) => return skipped(reason),
    };

    let computed = compute_move_target(
        &frame,
        target_rect,
        canon_basepos(target_rect),
        directive,
        k,
    );
    notes.extend(computed.notes);
    for note in &notes {
        warn!(
            event = "move_frame_substituted",
            scope,
            ?note,
            "\\![move] の基準を置き換えて動かす"
        );
    }

    // 吸い付きの当て直し（要件 2.10・2.11）。`Anchored` が無ければ当て直さない（Free と同じ）。
    let script = computed.position;
    let anchor = world.get::<Anchored>(target).map(|a| a.0);
    let to = match anchor {
        Some(anchor) => project_anchor(
            anchor,
            script,
            SizePx {
                w: target_rect.right - target_rect.left,
                h: target_rect.bottom - target_rect.top,
            },
            world.get_resource::<MonitorSnapshot>(),
        ),
        None => {
            warn!(
                event = "move_anchor_missing",
                scope, "\\![move] の窓に Anchored が無いので吸い付きを当て直さずに書く"
            );
            script
        }
    };

    if !move_window_to(world, target, to.x, to.y) {
        // 窓の書込口が受けなかった（窓ハンドルがまだ無い等）。
        return skipped("write_refused");
    }
    // 起動時の確定が台本の置いた窓を引き戻さないための印（x が既定のままでも・scg 7.3）。
    world.entity_mut(target).insert(ScriptPlaced);
    let base_rect = match frame {
        BaseFrame::Window { rect, .. } | BaseFrame::Area(rect) => rect,
        BaseFrame::Origin => RectPx {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
    };
    info!(
        event = "move_applied",
        scope,
        base = ?directive.base,
        base_offset = ?computed.base_offset_used,
        move_offset = ?directive.move_offset,
        ?anchor,
        base_left = base_rect.left,
        base_top = base_rect.top,
        base_right = base_rect.right,
        base_bottom = base_rect.bottom,
        from_x = target_rect.left,
        from_y = target_rect.top,
        script_x = script.x,
        script_y = script.y,
        to_x = to.x,
        to_y = to.y,
        "\\![move] で窓を動かした"
    );
    // 組の相手は、動かした窓が実際に動いた量だけ連れ動かす（要件 5.1）。
    let delta = PointPx {
        x: to.x.saturating_sub(target_rect.left),
        y: to.y.saturating_sub(target_rect.top),
    };
    if directive.ignore_sticky_window {
        log_sticky_ignored(world, scope);
    } else {
        follow_sticky_move(world, scope, delta);
    }
    true
}

#[cfg(test)]
#[path = "move_cue_place_tests.rs"]
mod tests;
