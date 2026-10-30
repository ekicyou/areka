//! 面の `move` のずれを窓の位置へ当てる口（seriko-pattern-calls「MoveShiftPlacement」・
//! 要件 7.1・7.3・7.5・7.8）。
//!
//! 提示が覚えたずれ（作者基準）に倍率をかけ、元の位置（今の位置 − 当てた分）へ足してから、
//! 今の置き方の決まり（吸い付き・作業領域の中への寄せ・見える範囲の守り）を後から当てる。
//! 書くのは位置だけで、窓の寸は変えない（7.5）。補わずに一度で当てる（7.3）。
//! 呼び手は毎フレームのずれの相（`emo2_boot/frame/move_shift.rs`）。

use areka_emo_compose::ScaleRatio;
use bevy_ecs::prelude::*;
use tracing::warn;
use wintf::ecs::WindowPos;

use super::super::balloon_limit::clamp_rect_to_work_area;
use super::super::windowposition::scale_signed;
use super::{
    Anchor, Anchored, BalloonFollow, BalloonFollowTrigger, MonitorSnapshot, PlacementRoute,
    PointPx, SizePx, apply_visibility_guard, follow_balloon, move_window_with_route,
    project_anchor, rect_at, work_area_for_window,
};

const ZERO: PointPx = PointPx { x: 0, y: 0 };

/// 窓に当てたずれの記録（キャラ窓とバルーン窓の entity が持つ・無ければずれ 0）。
///
/// `applied` はキャラ窓では「実際に書いた位置 − 元の位置」（揃えや寄せで縮んだ分も正しく
/// 引ける）、バルーン窓では倍率をかけたずれそのもの。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoveShiftApplied {
    want: (i64, i64),
    ratio: ScaleRatio,
    applied: PointPx,
}

/// 作者基準のずれに倍率をかける（`\![move]` と同じ `scale_signed`・`i32` へ飽和）。
fn scaled(want: (i64, i64), k: ScaleRatio) -> PointPx {
    let sat = |v: i64| i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX });
    PointPx {
        x: scale_signed(sat(want.0), k),
        y: scale_signed(sat(want.1), k),
    }
}

/// バルーン窓に当てたずれ（バルーンの追いの提案が足す分）。
pub(super) fn balloon_shift(world: &World, balloon: Entity) -> PointPx {
    world
        .get::<MoveShiftApplied>(balloon)
        .map_or(ZERO, |r| r.applied)
}

/// 保存の側が引くための元の位置（`pos` − 当てた分・記録が無ければ `pos` のまま）。
///
/// 記憶にずれを焼き付けない（起動のたびに積もらない・要件 7.8）。
pub(crate) fn origin_position(world: &World, window: Entity, pos: PointPx) -> PointPx {
    let applied = world
        .get::<MoveShiftApplied>(window)
        .map_or(ZERO, |r| r.applied);
    PointPx {
        x: pos.x.saturating_sub(applied.x),
        y: pos.y.saturating_sub(applied.y),
    }
}

/// 記録の (ずれ, 倍率) がこの値と同じか（毎フレームの相が書かずに済ますかの判定）。
///
/// 記録が無い窓はずれ 0 を当てたのと同じに見る（まだ一度も動かしていない窓へ書かない）。
pub(crate) fn shift_is_current(
    world: &World,
    window: Entity,
    want: (i64, i64),
    k: ScaleRatio,
) -> bool {
    world
        .get::<MoveShiftApplied>(window)
        .map_or(want == (0, 0), |r| r.want == want && r.ratio == k)
}

/// 窓を絶対の位置へ書いた書き手（位置の初期化）が、書いた位置を新しい元の位置にする。
///
/// 記録ごと外すので、毎フレームの相は今のずれをその上へ当て直す（ずれは元の位置からの相対・
/// 書いた位置から当てた分を引いて元の位置を見誤らない）。
pub(crate) fn rebase_shift(world: &mut World, window: Entity) {
    if let Ok(mut e) = world.get_entity_mut(window) {
        e.remove::<MoveShiftApplied>();
    }
}

/// 利用者のドラッグの終わりで、その窓の当てた分を 0 にする（置いた所が新しい元の位置）。
///
/// `want`・`ratio` は残すので、毎フレームの相は同じずれを当て直さない。
pub(super) fn reset_applied(world: &mut World, window: Entity) {
    if let Some(mut r) = world.get_mut::<MoveShiftApplied>(window) {
        r.applied = ZERO;
    }
}

/// キャラ窓へずれを当てる。書いたら `true`。
///
/// 窓・位置・寸・作業領域のどれかが無ければ `warn!` のうえ `false`（記録は変えないので、
/// 次のフレームで当て直す）。
pub(crate) fn apply_char_shift(
    world: &mut World,
    window: Entity,
    want: (i64, i64),
    k: ScaleRatio,
) -> bool {
    let skipped = |reason: &str| {
        warn!(
            event = "move_shift_skipped",
            entity = ?window,
            reason,
            "面の move のずれを当てる材料が無いので当てない（次のフレームで当て直す）"
        );
        false
    };
    let Some((pos, size)) = world
        .get::<WindowPos>(window)
        .and_then(|wp| Some((wp.position?, wp.size?)))
        .filter(|(_, s)| s.width > 0 && s.height > 0)
    else {
        return skipped("no_window_position");
    };
    let size = SizePx {
        w: size.width,
        h: size.height,
    };
    let applied = world
        .get::<MoveShiftApplied>(window)
        .map_or(ZERO, |r| r.applied);
    let base = PointPx {
        x: pos.x.saturating_sub(applied.x),
        y: pos.y.saturating_sub(applied.y),
    };
    let shift = scaled(want, k);
    let raw = PointPx {
        x: base.x.saturating_add(shift.x),
        y: base.y.saturating_add(shift.y),
    };

    let snapshot = world.get_resource::<MonitorSnapshot>();
    let Some(area) = snapshot.and_then(|s| work_area_for_window(s, rect_at(raw, size))) else {
        return skipped("no_work_area");
    };
    // 置き方の決まりを後から当てる（7.8）: 吸い付き → 作業領域の中への寄せ → 見える範囲の守り。
    let anchor = world.get::<Anchored>(window).map_or(Anchor::Free, |a| a.0);
    let anchored = project_anchor(anchor, raw, size, snapshot);
    let clamped = clamp_rect_to_work_area(anchored, size, area);
    let old_rect = Some(rect_at(PointPx { x: pos.x, y: pos.y }, size));
    // 守りの入力は寄せた後の位置（`raw` だと自分の寄せを勝手に飛んだ窓＝OffscreenPull と見誤る）。
    let to = apply_visibility_guard(
        window,
        PlacementRoute::PatternMove,
        snapshot,
        old_rect,
        clamped,
        clamped,
        size,
    );

    if !move_window_with_route(world, window, to.x, to.y, PlacementRoute::PatternMove) {
        return skipped("write_refused");
    }
    let written = world
        .get::<WindowPos>(window)
        .and_then(|wp| wp.position)
        .map_or(to, |p| PointPx { x: p.x, y: p.y });
    world.entity_mut(window).insert(MoveShiftApplied {
        want,
        ratio: k,
        applied: PointPx {
            x: written.x.saturating_sub(base.x),
            y: written.y.saturating_sub(base.y),
        },
    });
    true
}

/// バルーン窓へずれを当てる（記録してから、そのキャラ窓の今の位置で追い直す）。書いたら `true`。
///
/// バルーン窓・その持ち主のキャラ窓・キャラ窓の位置のどれかが無ければ `warn!` のうえ `false`。
pub(crate) fn apply_balloon_shift(
    world: &mut World,
    balloon: Entity,
    want: (i64, i64),
    k: ScaleRatio,
) -> bool {
    let skipped = |reason: &str| {
        warn!(
            event = "move_shift_skipped",
            entity = ?balloon,
            reason,
            "バルーンの面の move のずれを当てる材料が無いので当てない（次のフレームで当て直す）"
        );
        false
    };
    if world.get_entity(balloon).is_err() {
        return skipped("no_balloon_window");
    }
    let owner = world
        .query::<(Entity, &BalloonFollow, &WindowPos)>()
        .iter(world)
        .find(|(_, f, _)| f.balloon == balloon)
        .map(|(e, _, wp)| (e, wp.position));
    let Some((char_window, Some(pos))) = owner else {
        return skipped("no_char_window_position");
    };
    world.entity_mut(balloon).insert(MoveShiftApplied {
        want,
        ratio: k,
        applied: scaled(want, k),
    });
    follow_balloon(
        world,
        char_window,
        pos,
        BalloonFollowTrigger::Placement(PlacementRoute::PatternMove),
    );
    true
}
