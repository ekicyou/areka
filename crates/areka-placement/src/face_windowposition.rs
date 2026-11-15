//! バルーンの面の `windowposition` を配置の材料へ入れ、標準の位置にあるバルーンだけを新しい面の
//! 標準の位置へ置き直す（balloon-scroll-arrows 要件 1.7・design「FaceWindowPosition」）。
//!
//! 標準の位置は、シェルの切り替えの入れ直し（[`super::reseed::apply_shell_descript`]）・
//! バルーン位置初期化と同じ [`resolve_standard_scopes`] で解き、同じ [`reinstall_balloon`] で
//! 据え直す。窓の位置の記憶は書かない（面は台本の一時の状態）。

use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};
use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::world::World;
use tracing::{info, warn};

use super::reseed::{StandardScope, reinstall_balloon, resolve_standard_scopes};
use super::reset_pos::DefaultPlacementBasis;
use super::resolver::OffsetBase;
use super::spawn::{BalloonLimit, GhostWindows};

/// 利用者がこのスコープのバルーンを標準の位置から動かした印（キャラ窓の entity に付ける）。
///
/// 付ける: ドラッグで追随の基準を据え直したとき（追随の `reestablish_balloon_follow`）・記憶から
/// 戻したずらしを使ったとき（[`mark_by_base`]）。外す: 標準の位置へ据え直したとき（[`reinstall_balloon`]）。
/// 値の比べで決めないのは、追随の換算と解き直しの丸めが食い違うから。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BalloonUserMoved;

/// 利用者のドラッグで追随の基準を据え直したキャラ窓 `c` に印を付ける（追随の持ち物が呼ぶ 1 行）。
pub(crate) fn mark_user_moved(world: &mut World, c: Entity) {
    world.entity_mut(c).insert(BalloonUserMoved);
}

/// 据えたずらしの基準 `base` で印を付け外しする。未係留の基準は記憶から戻したずらし
/// （`persist::merge_scope` の保存値の腕だけが作る・配置式の解は必ず係留済み）なので付け、
/// 係留済みの標準の基準なら外す。
pub(crate) fn mark_by_base(world: &mut World, c: Entity, base: OffsetBase) {
    if base.dpi.is_none() {
        mark_user_moved(world, c);
    } else {
        world.entity_mut(c).remove::<BalloonUserMoved>();
    }
}

/// `scope` の面の `windowposition` を材料（[`DefaultPlacementBasis`]）へ入れ、まだ動かしていない
/// バルーンを新しい面の標準の位置へ置き直す。置き直したら `true`。
///
/// - 値が前と同じなら何もしない（記録も無し）。
/// - 利用者が動かした印（[`BalloonUserMoved`]）の無いスコープだけを置き直す。ドラッグした・
///   記憶から戻したバルーンは位置を保ち、材料と画面の内に保つ指定（`BalloonLimit`）だけを替える。
/// - 材料・窓の一式が無ければ `warn!(event = "balloon_face_windowposition_skipped")` を出して
///   何もしない。変化 1 回につき `info!(event = "balloon_face_windowposition")` を 1 行。
pub fn apply_face_windowposition(
    world: &mut World,
    scope: usize,
    wp: (WindowPosition, WindowPositionRaw),
) -> bool {
    let skipped = |reason: &str| {
        warn!(
            event = "balloon_face_windowposition_skipped",
            scope, reason, "面の windowposition を配置へ入れる材料が無いので何もしない"
        );
        false
    };
    let Some(basis) = world.get_resource::<DefaultPlacementBasis>() else {
        return skipped("no_default_basis");
    };
    if basis.balloon.windowpositions.get(&scope) == Some(&wp) {
        return false;
    }
    let Some(windows) = world.get_resource::<GhostWindows>().cloned() else {
        return skipped("no_ghost_windows");
    };
    let mut new = basis.clone();
    new.balloon.windowpositions.insert(scope, wp);
    world.insert_resource(new);

    let basis = world.resource::<DefaultPlacementBasis>();
    let standard = resolve_standard_scopes(world, &windows, &basis.source, &basis.balloon)
        .into_iter()
        .find(|s| s.running.input.scope == scope);
    let moved = match standard {
        Some(StandardScope { running, placement }) => {
            let untouched = !world
                .entity(running.char_window)
                .contains::<BalloonUserMoved>();
            if untouched {
                reinstall_balloon(world, &running, &placement);
            } else {
                // 動かしたバルーンは置き直さないが、画面の内に保つ指定は新しい面の値にする。
                world
                    .entity_mut(running.balloon_window)
                    .insert(BalloonLimit(placement.balloon_limit));
            }
            untouched
        }
        // 解けなかった理由は resolve_standard_scopes が warn で残している。
        None => false,
    };
    info!(
        event = "balloon_face_windowposition",
        scope, moved, "面の windowposition を配置の材料へ入れた"
    );
    moved
}

#[cfg(test)]
#[path = "face_windowposition_tests.rs"]
mod tests;
