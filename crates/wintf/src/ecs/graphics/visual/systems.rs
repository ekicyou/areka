//! visual 層の配線の係。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use tracing::{error, trace, warn};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

use super::draw::builder::{NodeData, build};
use super::world_visual::{FillData, fill_world_visuals};
use super::{
    DrawTree, OwnSurface, Visual, VisualDrawContent, VisualTransform, WorldVisual,
    has_window_ancestor, insert_graphics_parts, wants_own_surface,
};
use crate::ecs::graphics::components::{SurfaceGraphics, SurfaceGraphicsDirty, VisualGraphics};
use crate::ecs::graphics::systems::format_entity_name;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::Arrangement;
use crate::numerics::Aabb;

/// 持ち主か節かを決める部品の有無。
type RuleData = (
    Has<GraphicsCommandList>,
    Has<OwnSurface>,
    Has<WorldVisual>,
    Has<VisualGraphics>,
);

/// 窓の下の `Visual` に、規則 [`wants_own_surface`] の答えどおり描画面の 3 部品を入れる・外す
/// （外すときは根の部品 `DrawTree` も一緒に外す・`DrawTree` は根にだけ在る）。
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
            commands.entity(e).remove::<(
                VisualGraphics,
                SurfaceGraphics,
                SurfaceGraphicsDirty,
                DrawTree,
            )>();
        }
    }
}

/// 根の木を組み直すか: 木の中に変化があった・どこかで節の部品が外れた（どの根か分からないので
/// 全部の根）・装置が変わった（図形と命令の列が古い）・根の部品がまだ無い、のどれか。
pub(crate) fn needs_rebuild(
    changed: bool,
    removed_any: bool,
    core_changed: bool,
    has_tree: bool,
) -> bool {
    changed || removed_any || core_changed || !has_tree
}

/// どの根か分からない木の出入り（節の部品の外れ・節の昇格／降格）。どれか 1 件でも在れば全部の根を組み直す。
/// 昇格（節に `VisualGraphics` が入る）は節の値も根の `Children` も変えないので、`Added` で拾う。
/// 降格は `OwnSurface` の外れに限らず `GraphicsCommandList` の外れでも起きるので、`VisualGraphics` の外れで拾う。
#[derive(bevy_ecs::system::SystemParam)]
pub struct TreeEdits<'w, 's> {
    removed_content: RemovedComponents<'w, 's, VisualDrawContent>,
    removed_transform: RemovedComponents<'w, 's, VisualTransform>,
    removed_parent: RemovedComponents<'w, 's, ChildOf>,
    removed_own: RemovedComponents<'w, 's, OwnSurface>,
    demoted: RemovedComponents<'w, 's, VisualGraphics>,
    promoted: Query<'w, 's, (), (Added<VisualGraphics>, With<WorldVisual>)>,
}

impl TreeEdits<'_, '_> {
    /// 出入りが在ったか。読み手は毎回読み切る（短絡させない）。
    pub fn any(&mut self) -> bool {
        self.removed_content.read().count()
            + self.removed_transform.read().count()
            + self.removed_parent.read().count()
            + self.removed_own.read().count()
            + self.demoted.read().count()
            > 0
            || !self.promoted.is_empty()
    }
}

/// 描画面の持ち主（根）ごとに節の `WorldVisual`・`VisualVersion` を埋め、組み直す回だけ
/// 平らな列を作って根の部品 [`DrawTree`] へ書く・入れる。直下に節が無く自分の `VisualDrawContent` も
/// 無い根からは外す。根の自分の中身は列の先頭の 1 件になり、変わった回に根の版を上げて組み直す。
/// 持ち主の `VisualTransform` は WUC では効かないので、入った・変わった回に `warn!` を 1 行残して無視する。
///
/// 装置が無効な回も埋めることはする（版の上がりと行列を捨てない・装置は要らない）が、組まない
/// （図形は工場から作る）。立ち直りの回は `Res<GraphicsCore>` の変化で全部の根を組み直す。
/// 節の書き込み（`FillData`）と読み取り（`NodeData`）は同じ部品を取るので `ParamSet` で分ける（B0001）。
/// `PreRenderSurface` の見た目の管理の後・描画面の作成の前に回す（入れた根の部品は同じ回に拾われる）。
#[allow(clippy::type_complexity)]
pub fn update_draw_trees(
    mut commands: Commands,
    graphics: Option<Res<GraphicsCore>>,
    mut roots: Query<(
        Entity,
        Ref<Arrangement>,
        Option<Ref<Children>>,
        Option<&mut DrawTree>,
        Option<&Name>,
        Option<Ref<VisualDrawContent>>,
        Option<Ref<VisualTransform>>,
        Ref<VisualGraphics>,
    )>,
    node_kinds: Query<(), (With<WorldVisual>, Without<VisualGraphics>)>,
    mut set: ParamSet<(Query<FillData>, Query<NodeData>)>,
    mut edits: TreeEdits,
    mut visited: Local<Vec<(Entity, usize)>>,
) {
    let removed_any = edits.any();
    let factory = graphics.as_ref().and_then(|g| g.d2d_factory());
    let core_changed = graphics.as_ref().is_some_and(|g| g.is_changed());
    for (root, arrangement, kids, tree, name, own, transform, graphics_parts) in roots.iter_mut() {
        if transform.is_some_and(|t| t.is_changed() || graphics_parts.is_added()) {
            warn!(
                entity = %format_entity_name(root, name),
                "[update_draw_trees] 描画面の持ち主の VisualTransform は WUC では効かないので無視する"
            );
        }
        let own_changed = own.as_ref().is_some_and(|c| c.is_changed());
        if own_changed && let Ok((.., Some(mut version), _, _, _)) = set.p0().get_mut(root) {
            version.bump();
        }
        let top: &[Entity] = kids.as_deref().map_or(&[], |k| &**k);
        if own.is_none() && !top.iter().any(|&c| node_kinds.contains(c)) {
            if tree.is_some() {
                commands.entity(root).remove::<DrawTree>();
                trace!(
                    root = %format_entity_name(root, name),
                    "[update_draw_trees] 直下に節も自分の中身も無くなったので根の部品を外す"
                );
            }
            continue;
        }
        let changed = fill_world_visuals(
            root,
            name,
            &mut set.p0(),
            arrangement.is_changed(),
            top,
            kids.as_ref().is_some_and(|k| k.is_changed()),
            &mut visited,
        ) || own_changed;
        let Some(factory) = factory else { continue };
        if !needs_rebuild(changed, removed_any, core_changed, tree.is_some()) {
            continue;
        }
        let unit_rect = match tree.as_ref().and_then(|t| t.unit_rect.clone()) {
            Some(u) => u,
            None => match unsafe {
                factory.CreateRectangleGeometry(&D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: 1.0,
                    bottom: 1.0,
                })
            } {
                Ok(g) => g.into(),
                Err(e) => {
                    error!(
                        root = %format_entity_name(root, name),
                        error = ?e,
                        hresult = format_args!("0x{:08X}", e.code().0),
                        "[update_draw_trees] 単位矩形の図形を作れないので、この根を組まない"
                    );
                    continue;
                }
            },
        };
        let viewport = Aabb::new(0.0, 0.0, arrangement.size.width, arrangement.size.height);
        let mut out = Vec::new();
        build(
            &mut out,
            factory,
            &unit_rect,
            viewport,
            &set.p1(),
            root,
            name,
            &visited,
        );
        trace!(
            root = %format_entity_name(root, name),
            nodes = visited.len(),
            commands = out.len(),
            "[update_draw_trees] 木を組み直した"
        );
        match tree {
            Some(mut tree) => {
                tree.commands = out;
                tree.unit_rect = Some(unit_rect);
            }
            None => {
                commands.entity(root).insert(DrawTree {
                    commands: out,
                    unit_rect: Some(unit_rect),
                });
            }
        }
    }
}
