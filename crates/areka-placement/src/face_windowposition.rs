//! バルーンの面の `windowposition` を配置の材料へ入れ、標準の位置にあるバルーンだけを新しい面の
//! 標準の位置へ置き直す（balloon-scroll-arrows 要件 1.7・design「FaceWindowPosition」）。
//!
//! 標準の位置は、シェルの切り替えの入れ直し（[`super::reseed::apply_shell_descript`]）・
//! バルーン位置初期化と同じ [`resolve_standard_scopes`] で解き、同じ [`reinstall_balloon`] で
//! 据え直す。窓の位置の記憶は書かない（面は台本の一時の状態）。

use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};
use bevy_ecs::world::World;
use tracing::{info, warn};

use super::follow::BalloonFollow;
use super::reseed::{StandardScope, reinstall_balloon, resolve_standard_scopes};
use super::reset_pos::DefaultPlacementBasis;
use super::spawn::GhostWindows;

/// `scope` の面の `windowposition` を材料（[`DefaultPlacementBasis`]）へ入れ、まだ動かしていない
/// バルーンを新しい面の標準の位置へ置き直す。置き直したら `true`。
///
/// - 値が前と同じなら何もしない（記録も無し）。
/// - 今のずらしが前の材料の標準のずらしと等しいスコープだけを置き直す。動かしてある
///   （ドラッグ・台本・記憶）バルーンは位置を保ち、材料だけを替える。
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
    let Some(old) = world.get_resource::<DefaultPlacementBasis>().cloned() else {
        return skipped("no_default_basis");
    };
    if old.balloon.windowpositions.get(&scope) == Some(&wp) {
        return false;
    }
    let Some(windows) = world.get_resource::<GhostWindows>().cloned() else {
        return skipped("no_ghost_windows");
    };
    let mut new = old.clone();
    new.balloon.windowpositions.insert(scope, wp);
    world.insert_resource(new.clone());

    let standard_of = |world: &World, basis: &DefaultPlacementBasis| {
        resolve_standard_scopes(world, &windows, &basis.source, &basis.balloon)
            .into_iter()
            .find(|s| s.running.input.scope == scope)
    };
    let before = standard_of(world, &old);
    let after = standard_of(world, &new);
    let moved = match (before, after) {
        (Some(before), Some(StandardScope { running, placement })) => {
            let c = running.char_window;
            let untouched = world
                .get::<BalloonFollow>(c)
                .is_some_and(|f| f.offset() == before.placement.balloon_offset_base.offset);
            if untouched {
                reinstall_balloon(
                    world,
                    &running,
                    placement.balloon_offset_base,
                    placement.balloon_keyword_base,
                );
            }
            untouched
        }
        // 解けなかった理由は resolve_standard_scopes が warn で残している。
        _ => false,
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
