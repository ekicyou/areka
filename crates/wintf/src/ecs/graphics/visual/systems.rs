//! visual 層の配線の係。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use tracing::error;

use super::{
    OwnSurface, Visual, WorldVisual, has_window_ancestor, insert_graphics_parts, wants_own_surface,
};
use crate::ecs::graphics::GraphicsCommandList;
use crate::ecs::graphics::components::{SurfaceGraphics, SurfaceGraphicsDirty, VisualGraphics};

/// 持ち主か節かを決める部品の有無。
type RuleData = (
    Has<GraphicsCommandList>,
    Has<OwnSurface>,
    Has<WorldVisual>,
    Has<VisualGraphics>,
);

/// 窓の下の `Visual` に、規則 [`wants_own_surface`] の答えどおり描画面の 3 部品を入れる・外す。
/// 見る変化は親の変化（子孫も・上限つき）・節の部品（`WorldVisual`）の追加・目印と命令の列の出入り。
/// `WorldVisual` は装置の失いでも外れないので、失いの回に節が持ち主へ化けることはない。
/// `PreRenderSurface` の先頭で回し、入れた `VisualGraphics` は同じ回に作られる。
pub fn sync_surface_owner_parts(
    mut commands: Commands,
    added: Query<
        Entity,
        (
            With<Visual>,
            Or<(
                Added<WorldVisual>,
                Added<OwnSurface>,
                Added<GraphicsCommandList>,
            )>,
        ),
    >,
    mut removed_own: RemovedComponents<OwnSurface>,
    mut removed_list: RemovedComponents<GraphicsCommandList>,
    reparented: Query<Entity, Changed<ChildOf>>,
    parents: Query<&ChildOf>,
    windows: Query<(), With<crate::ecs::window::Window>>,
    children: Query<&Children>,
    visuals: Query<RuleData, With<Visual>>,
    pending: Query<
        (),
        (
            With<Visual>,
            Without<VisualGraphics>,
            Or<(
                Without<WorldVisual>,
                With<OwnSurface>,
                With<GraphicsCommandList>,
            )>,
        ),
    >,
) {
    let mut targets: Vec<Entity> = added.iter().collect();
    targets.extend(removed_own.read());
    targets.extend(removed_list.read());
    // 親の変化で要るのは入れることだけ。ふだんは待つものが無いので子孫を歩かない。
    let walk = !pending.is_empty();
    for entity in reparented.iter().filter(|_| walk) {
        // 子孫の走査も祖先と同じ上限で止める。
        let mut stack = vec![(entity, 0usize)];
        while let Some((e, depth)) = stack.pop() {
            targets.push(e);
            let Ok(kids) = children.get(e) else { continue };
            if depth >= crate::ecs::common::MAX_ANCESTOR_DEPTH {
                error!(
                    root = ?entity,
                    limit = crate::ecs::common::MAX_ANCESTOR_DEPTH,
                    "[sync_surface_owner_parts] 子孫の走査が上限を越えたので止める"
                );
                continue;
            }
            stack.extend(kids.iter().map(|k| (k, depth + 1)));
        }
    }
    for e in targets {
        let Ok((has_list, has_own, is_node, has_parts)) = visuals.get(e) else {
            continue; // 消えた実体・Visual でない実体
        };
        if wants_own_surface(has_list, has_own, is_node) {
            let joined = has_window_ancestor(
                e,
                |w| windows.contains(w),
                |c| parents.get(c).ok().map(|p| p.parent()),
            );
            if joined && !has_parts {
                insert_graphics_parts(&mut commands.entity(e));
            }
        } else if has_parts {
            commands
                .entity(e)
                .remove::<(VisualGraphics, SurfaceGraphics, SurfaceGraphicsDirty)>();
        }
    }
}
