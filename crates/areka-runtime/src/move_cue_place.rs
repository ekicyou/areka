//! `\![move]` の UI スレッドでの適用。
//!
//! 基準を World から解いて [`BaseFrame`] にし、純粋な計算（[`compute_move_target`]）で吸い付きの
//! 前の位置を求め、対象の窓の [`Anchored`] で吸い付きの射影（[`project_anchor`]）に通してから
//! [`move_window_to`] で書く。位置の記憶・`Anchored`・既定の位置・バルーンのずらしは変えない
//! （バルーンは今のずらしで付いて動く）。ほかに、窓へ `ScriptPlaced` を付け、組の相手を連れ動かし、
//! 絶対の位置へ書いた軸の面の `move` のずれを当て直す。
//!
//! どの失敗も panic せずトークを止めない: 動かせないときは `move_skipped`、基準を置き換えたときは
//! `move_frame_substituted`、`Anchored` が無いときは `move_anchor_missing` の `warn!` を残す。

use bevy_ecs::prelude::World;
use tracing::{info, warn};
use wintf::ecs::WindowPos;
use wintf::ecs::window::monitor::Monitor;

use areka_emo_compose::ScaleRatio;

use super::geometry::{BaseFrame, FrameNote, canon_basepos, compute_move_target};
use super::{AxisSpec, MoveBase, MoveDirective};
use areka_placement::chain_finalize::ScriptPlaced;
use areka_placement::follow::{
    Anchored, MonitorSnapshot, follow_sticky_move, log_sticky_ignored, move_window_to,
    project_anchor, rebase_shift_axes, work_area_for_window,
};
use areka_placement::resolver::{PointPx, RectPx, SizePx};
use areka_placement::spawn::GhostWindows;
use areka_placement::{primary_monitor, work_area_px};

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

fn window_frame(rect: RectPx) -> BaseFrame {
    BaseFrame::Window {
        rect,
        basepos: canon_basepos(rect),
    }
}

/// `screen`: 動かす窓の中心のモニタ（外れたら最近傍）の作業領域。
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
                // 基準の窓が無い → screen へ倒す。
                notes.push(FrameNote::MissingBaseScope { scope: n });
                screen_frame(world, target)
            }
        },
        MoveBase::Me => Ok(window_frame(target)),
        MoveBase::Screen => screen_frame(world, target),
        MoveBase::PrimaryScreen => {
            let monitors: Vec<Monitor> = world.query::<&Monitor>().iter(world).cloned().collect();
            let primary = primary_monitor(&monitors).ok_or("no_primary_monitor")?;
            Ok(BaseFrame::Area(work_area_px(primary)))
        }
        MoveBase::Global => Ok(BaseFrame::Origin),
    }
}

fn log_substitutions(scope: u32, notes: &[FrameNote]) {
    for note in notes {
        warn!(
            event = "move_frame_substituted",
            scope,
            ?note,
            "\\![move] の基準を置き換えて動かす"
        );
    }
}

/// `\![move]` の [`MoveDirective`] を UI スレッドで窓へ書く。書いたら `true`。
///
/// 動かせなかったときは `warn!` のうえ `false`（トークは止めない）。`k` は対象のスコープの
/// シェルに今かかっている拡大率で、台本の X・Y（作者基準）を物理 px へ写すのにだけ使う。
/// 書いたら、組の相手を実際に動いた量だけ連れ動かす（`ignore_sticky_window` が真なら連れ動かさず
/// 記録だけ）。書けなければ連れ動かさない。
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
        Err(reason) => {
            log_substitutions(scope, &notes);
            return skipped(reason);
        }
    };

    let computed = compute_move_target(
        &frame,
        target_rect,
        canon_basepos(target_rect),
        directive,
        k,
    );
    notes.extend(computed.notes);
    log_substitutions(scope, &notes);

    // 吸い付きの当て直し。`Anchored` が無ければ当て直さない（Free と同じ）。
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
    // 起動時の確定が台本の置いた窓を引き戻さないための印（x が既定のままでも付ける）。
    world.entity_mut(target).insert(ScriptPlaced);
    // 絶対の位置へ書いた軸は、書いた位置が面の `move` のずれの新しい元の位置。
    let relative =
        matches!(directive.base, MoveBase::Me) || directive.base == MoveBase::Scope(scope);
    let absolute = |axis: AxisSpec| !relative && matches!(axis, AxisSpec::Px(_));
    rebase_shift_axes(world, target, absolute(directive.x), absolute(directive.y));
    // 連れ動きと記録は、ずれを当て直した後の位置で数える。
    let to = window_rect(world, scope).map_or(to, |r| PointPx {
        x: r.left,
        y: r.top,
    });
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
        // 拡大率の取り違えを後で測るための欄: 当てた拡大率と動かす窓の今の大きさ。
        k = ?k,
        target_w = target_rect.right - target_rect.left,
        target_h = target_rect.bottom - target_rect.top,
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
    // 組の相手は、動かした窓が実際に動いた量だけ連れ動かす。
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
