//! 今の定義の唯一の書き手 [`select_balloon_face`] と、drain の後の面の変化の拾い [`after_drain`]
//! （balloon-scroll-arrows 要件 1.7・8.6・design「BalloonFaceSelector」）。
//!
//! 装着（面 0）・drain の後の面の切り替え・バルーンの切り替え（新しい束）が同じ書き手を通る。
//! 書き手は文字の層の作り直しを呼ばない——同じフレームの後段の文字層スケール相が
//! `balloon_models[scope]` を読み、唯一の権威 `refresh_actor_binding` が比べて作り直す
//! （drain → 書き手 → 文字層スケール相 → 文字の相）。

use std::collections::HashMap;

use areka_emo_present::TargetId;
use areka_emo_text::scroll_marker::StopReason;
use areka_sakura::ActorKey;
use bevy_ecs::world::World;
use tracing::{debug, error};

use super::{Emo2Wiring, balloon_target};
use crate::placement::face_windowposition::apply_face_windowposition;

/// 書き手を呼んだ契機（記録の欄）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::emo2_boot) enum FaceCause {
    Attach,
    FaceChange,
    BalloonSwitch,
}

/// scope の今の定義を束の面 `face` の定義へ差し替える（今の定義の唯一の書き手）。
///
/// (1) その面の矢印の絵と行の中の印の絵（script-small-tags 要件 5.1・5.2）を文字の層へ入れ、
/// (2) 面が前と違えば手動の位置を捨て（要件 8.6）、
/// (3) 面の `windowposition` を配置へ届け（要件 1.7）、(4) `balloon_models[scope]` を面の
/// モデルにし、(5) 面の番号を覚える。束が無い scope は `error!` で何もしない。
pub(in crate::emo2_boot) fn select_balloon_face(
    wiring: &mut Emo2Wiring,
    world: &mut World,
    scope: u32,
    face: u32,
    cause: FaceCause,
) {
    let Some(definitions) = wiring.balloon_definitions.get(&scope) else {
        error!(
            scope,
            face,
            ?cause,
            "balloon_face: 定義の束が無いので今の定義を差し替えない"
        );
        return;
    };
    let definition = definitions.for_face(face).clone();
    let previous = wiring.balloon_faces.insert(scope, face);
    let face_changed = previous.is_some_and(|p| p != face);
    let actor = ActorKey::from(scope.to_string());
    {
        let mut runtime = wiring.runtime.borrow_mut();
        runtime.set_balloon_markers(actor.clone(), definition.arrows);
        runtime.set_inline_marker(actor.clone(), definitions.inline_marker_for_face(face));
        if face_changed {
            runtime.forget_manual_of(&actor, StopReason::FaceChanged);
        }
    }
    let model = definition.model;
    apply_face_windowposition(
        world,
        scope as usize,
        (model.windowposition(), model.windowposition_raw().clone()),
    );
    wiring.balloon_models.insert(scope, model);
    debug!(
        scope,
        ?previous,
        face,
        ?cause,
        "balloon_face: 今の定義を差し替えた"
    );
}

/// drain がこのフレームで `ShowSurface` を適用した target のうち、balloon の target で presenter の
/// 今の面が覚えた面と違う scope だけ書き手を通す（動く絵のコマは面の番号が同じなので通さない）。
pub(super) fn after_drain(wiring: &mut Emo2Wiring, world: &mut World, shown: &[TargetId]) {
    let changed = changed_faces(shown, &wiring.balloon_faces, |target| {
        wiring.presenter.current_surface_id(target)
    });
    for (scope, face) in changed {
        select_balloon_face(wiring, world, scope, face, FaceCause::FaceChange);
    }
}

/// 面が替わった `(scope, 新しい面)` を scope の昇順で返す（[`after_drain`] の判断・純粋）。
fn changed_faces(
    shown: &[TargetId],
    faces: &HashMap<u32, u32>,
    current: impl Fn(TargetId) -> Option<u32>,
) -> Vec<(u32, u32)> {
    let mut changed: Vec<(u32, u32)> = faces
        .iter()
        .filter(|(scope, _)| shown.contains(&balloon_target(**scope)))
        .filter_map(|(&scope, &face)| {
            let now = current(balloon_target(scope))?;
            (now != face).then_some((scope, now))
        })
        .collect();
    changed.sort_unstable();
    changed
}

#[cfg(test)]
#[path = "balloon_face_tests.rs"]
mod tests;
