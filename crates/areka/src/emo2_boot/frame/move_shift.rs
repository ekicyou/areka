//! 面の `move` のずれの相（seriko-pattern-calls design「MoveShiftPlacement」・要件 3.8・7.1・7.3・7.6）。
//!
//! 移動の相（[`run_move_drain_phase`](super::run_move_drain_phase)）の直後に毎フレーム走る。
//! 各スコープのキャラ窓とバルーン窓について、提示が覚えたずれと当てた拡大率を読み、
//! 窓に当てた記録と食い違うときだけ置き方の口（`placement::follow::apply_char_shift`・
//! `apply_balloon_shift`）で当てる。
//!
//! - 拡大率がまだ無い（最初の面を表示していない）窓は何もしない（表示したフレームで当てる）。
//! - (ずれ, 拡大率) が記録と同じなら書かない。
//! - 窓の書き込みを見送る間（拡大率と表の整合待ち）は書かず、次のフレームへ回す
//!   （記録を変えないので、門が開いたフレームでそのまま当たる）。

use areka_emo_compose::ScaleRatio;
use areka_emo_present::{EmoPresenter, TargetId};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use crate::placement::dpi_sync::{self, HoldSite};
use crate::placement::follow::{apply_balloon_shift, apply_char_shift, shift_is_current};
use crate::placement::spawn::GhostWindows;

use super::super::target_map::{balloon_target, shell_target};

/// 提示のずれと当てた拡大率の読み口（本番は表示層・檻は偽物）。
pub(super) trait MoveShiftSource {
    /// 対象が覚えたずれ（作者基準 px・未登録は (0, 0)）。
    fn move_shift(&self, target: TargetId) -> (i64, i64);
    /// 対象に今かかっている拡大率（まだ面を出していなければ `None`）。
    fn applied_ratio(&self, target: TargetId) -> Option<ScaleRatio>;
}

impl MoveShiftSource for EmoPresenter {
    fn move_shift(&self, target: TargetId) -> (i64, i64) {
        EmoPresenter::move_shift(self, target)
    }
    fn applied_ratio(&self, target: TargetId) -> Option<ScaleRatio> {
        EmoPresenter::applied_ratio(self, target)
    }
}

/// ずれの相（本番の入口）。
pub(super) fn run_move_shift_phase(presenter: &EmoPresenter, world: &mut World) {
    run_move_shift_with(presenter, world);
}

/// [`run_move_shift_phase`] の本体（読み口越しの形・本番もここへ委譲する）。
pub(super) fn run_move_shift_with<S: MoveShiftSource + ?Sized>(source: &S, world: &mut World) {
    let Some(gw) = world.get_resource::<GhostWindows>() else {
        return;
    };
    let mut windows: Vec<(Entity, TargetId, bool)> = Vec::new();
    for scope in gw.scopes() {
        let s = scope as u32;
        if let Some(w) = gw.char_window(scope) {
            windows.push((w, shell_target(s), false));
        }
        if let Some(b) = gw.balloon_window(scope) {
            windows.push((b, balloon_target(s), true));
        }
    }
    for (window, target, is_balloon) in windows {
        // 破棄済みの窓（終了処理の途中）は当てる先が無い。正常系ゆえ何も残さない。
        if world.get_entity(window).is_err() {
            continue;
        }
        let Some(k) = source.applied_ratio(target) else {
            continue;
        };
        let want = source.move_shift(target);
        if shift_is_current(world, window, want, k)
            || dpi_sync::defers_window_write(world, window, HoldSite::PatternMove)
        {
            continue;
        }
        // 当てられなかったとき（材料が無い）は口の中で warn! 済みで、記録が変わらないので次で当て直す。
        if is_balloon {
            apply_balloon_shift(world, window, want, k);
        } else {
            apply_char_shift(world, window, want, k);
        }
    }
}

#[cfg(test)]
#[path = "move_shift_tests.rs"]
mod tests;
