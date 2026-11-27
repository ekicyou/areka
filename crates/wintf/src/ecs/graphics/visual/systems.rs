//! visual 層の配線の係。

use std::collections::HashSet;

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use tracing::{error, trace};
use windows::Win32::Foundation::E_UNEXPECTED;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::{ID2D1Factory, ID2D1Geometry};

use super::draw::builder::{NodeData, TreeFrame, build};
use super::promotion::Phase;
use super::promotion_round::{PromotionData, RoundContext, demote_unseen, run_round};
use super::world_visual::{FillData, ReportedIssues, TreeScan, fill_world_visuals};
use super::{
    AutoPromotion, DrawCommand, DrawTree, OwnSurface, SurfacePlacement, Visual, VisualDrawContent,
    VisualGroup, VisualTransform, WorldVisual, has_window_ancestor, insert_graphics_parts,
    wants_own_surface,
};
use crate::ecs::graphics::components::{SurfaceGraphics, SurfaceGraphicsDirty, VisualGraphics};
use crate::ecs::graphics::systems::{
    calculate_surface_size_from_global_arrangement, format_entity_name,
};
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::ecs::graphics::{FrameTime, GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::{Arrangement, GlobalArrangement};
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
/// 見る変化は親の変化（子孫も・上限つき）・節の部品（`WorldVisual`）の追加・目印と命令の列の出入り・
/// 上げ下げの相の変化（`AutoPromotion` は相が変わった回だけ変化の印が立つ）。
/// 目印か命令の列が入った節は上げ下げの状態を初期化し、用意中か上げた形で持っていた `DrawTree` を
/// 外す（手動の持ち主の根として組み直させる）。
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
    manual: Query<
        Entity,
        (
            With<Visual>,
            Or<(Added<OwnSurface>, Added<GraphicsCommandList>)>,
        ),
    >,
    // 相の変化の拾い（0）と状態の初期化・相の読み（1）は同じ部品を読み書きするので分ける。
    mut promotions: ParamSet<(
        Query<Entity, (With<Visual>, Changed<AutoPromotion>)>,
        Query<&mut AutoPromotion>,
    )>,
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
    targets.extend(promotions.p0().iter());
    let mut states = promotions.p1();
    for e in manual.iter() {
        let Ok(mut a) = states.get_mut(e) else {
            continue;
        };
        if a.rule.phase() == Phase::Node {
            // 相は変わらない: 履歴だけ消し、変化の印を立てない。
            a.bypass_change_detection().rule.reset();
        } else {
            a.rule.reset();
            commands.entity(e).remove::<DrawTree>();
        }
        let a = a.bypass_change_detection();
        a.skip_memo = None;
        a.built = false;
    }
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
        let auto_owner = promotions
            .p1()
            .get(e)
            .is_ok_and(|a| a.rule.phase() != Phase::Node);
        if wants_own_surface(has_list, has_own, is_node, auto_owner) {
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
    removed_group: RemovedComponents<'w, 's, VisualGroup>,
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
            + self.removed_group.read().count()
            + self.removed_parent.read().count()
            + self.removed_own.read().count()
            + self.demoted.read().count()
            > 0
            || !self.promoted.is_empty()
    }
}

/// 根として回す手動の描画面の持ち主の部品（自動で上げた持ち主の部分の列もここから書く）。
type RootData = (
    Entity,
    Ref<'static, Arrangement>,
    Option<Ref<'static, Children>>,
    Option<&'static mut DrawTree>,
    Option<&'static Name>,
    Option<Ref<'static, VisualDrawContent>>,
    Option<&'static GlobalArrangement>,
);

/// 節の種類（自動で上げた持ち主も含む・手動の持ち主を除く）。
type NodeKind = (
    With<WorldVisual>,
    Without<OwnSurface>,
    Without<GraphicsCommandList>,
);

/// 手動の描画面の持ち主（根）ごとに、節の `WorldVisual`・`VisualVersion` と走査の列を埋め、1 回の判断
/// （`run_round`・時刻 `FrameTime` が無い回は相を進めない）で描く列を分け、組み直す回だけ親の列を
/// 根の部品 [`DrawTree`] へ書く・入れる。直下に節が無く自分の `VisualDrawContent` も無い根からは外す。
/// 根の自分の中身は列の先頭の 1 件になり、変わった回に根の版を上げて組み直す。
///
/// 自動で上げた持ち主（節の種類）は根として回さず、親の根の回で部分の列を組む（根を組み直す回と、
/// まだ `DrawTree` を持たない回）。組んだ部分の列は全部の根を回した後にまとめて書き（根の問い合わせを
/// 回している間は別の実体を可変で借りられない）、用意中の部分を書いた回に描いた印を立てる。
/// 全部の根を回した後、前の回に控えた自動の持ち主のうちどの走査の列にも現れなかった実体を見えないとして
/// 戻し（全部の節を歩かない）、どの根の回でも書かなかった置き場を外す。
///
/// 装置が無効な回も埋めることと判断はする（版の上がりと行列を捨てない・装置は要らない）が、組まない
/// （図形は工場から作る）。立ち直りの回は `Res<GraphicsCore>` の変化で全部の根を組み直す。
/// 節の書き込み（`FillData`）・読み取り（`NodeData`）・判断（`PromotionData`）は同じ部品を取るので
/// `ParamSet` で分ける（B0001）。
/// `PreRenderSurface` の見た目の管理の後・描画面の作成の前に回す（入れた根の部品は同じ回に拾われる）。
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_draw_trees(
    mut commands: Commands,
    graphics: Option<Res<GraphicsCore>>,
    wuc: Option<Res<WucGraphicsResource>>,
    frame_time: Option<Res<FrameTime>>,
    mut roots: Query<RootData, With<VisualGraphics>>,
    node_kinds: Query<(), NodeKind>,
    with_tree: Query<(), With<DrawTree>>,
    placed: Query<Entity, With<SurfacePlacement>>,
    mut set: ParamSet<(Query<FillData>, Query<NodeData>, Query<PromotionData>)>,
    mut edits: TreeEdits,
    mut scan: Local<TreeScan>,
    mut reported: Local<ReportedIssues>,
    mut autos: Local<HashSet<Entity>>,
) {
    let removed_any = edits.any();
    let factory = graphics.as_ref().and_then(|g| g.d2d_factory());
    let core_changed = graphics.as_ref().is_some_and(|g| g.is_changed());
    let device_ready = graphics.as_ref().is_some_and(|g| g.is_valid())
        && wuc.as_ref().is_some_and(|w| w.is_valid());
    let now = frame_time.map(|t| t.0);
    let mut written = HashSet::new();
    let mut seen = HashSet::new();
    let mut segment_trees: Vec<(Entity, Vec<DrawCommand>, ID2D1Geometry)> = Vec::new();
    for (root, arrangement, kids, tree, name, own, global) in roots.iter_mut() {
        if node_kinds.contains(root) {
            continue; // 自動で上げた持ち主は親の根の回が組む
        }
        let own_changed = own.as_ref().is_some_and(|c| c.is_changed());
        if own_changed && let Ok((.., Some(mut version), _, _)) = set.p0().get_mut(root) {
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
            &mut scan,
            &mut reported,
        ) || own_changed;
        let ctx = RoundContext {
            now,
            device_ready,
            scale: global.map_or(1.0, |g| g.scale_x()),
            host_px: global
                .and_then(calculate_surface_size_from_global_arrangement)
                .unwrap_or((0, 0)),
        };
        let plan = run_round(
            &ctx,
            &scan,
            &mut set.p2(),
            &mut commands,
            &mut written,
            &mut reported,
        );
        seen.extend(plan.autos.iter().copied());
        let Some(factory) = factory else { continue };
        let rebuild = needs_rebuild(
            changed || plan.moved,
            removed_any,
            core_changed,
            tree.is_some(),
        );
        let mut segments = plan
            .segments
            .iter()
            .filter(|seg| rebuild || !with_tree.contains(seg.owner))
            .peekable();
        if !rebuild && segments.peek().is_none() {
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
        for seg in segments {
            let (w, h) = seg.origin.size_px;
            let nodes = set.p1();
            let owner_name = nodes.get(seg.owner).ok().and_then(|d| d.5);
            let mut out = Vec::new();
            build(
                &mut out,
                factory,
                &unit_rect,
                Aabb::new(0.0, 0.0, w as f32, h as f32),
                &nodes,
                &TreeFrame::Segment {
                    owner: seg.owner,
                    owner_name,
                    origin_px: seg.origin.px,
                    scale: ctx.scale,
                },
                &seg.visited,
            );
            segment_trees.push((seg.owner, out, unit_rect.clone()));
        }
        if !rebuild {
            continue;
        }
        let viewport = Aabb::new(0.0, 0.0, arrangement.size.width, arrangement.size.height);
        let mut out = Vec::new();
        build(
            &mut out,
            factory,
            &unit_rect,
            viewport,
            &set.p1(),
            &TreeFrame::Root {
                root,
                root_name: name,
            },
            &plan.host_visited,
        );
        trace!(
            root = %format_entity_name(root, name),
            nodes = plan.host_visited.len(),
            commands = out.len(),
            "[update_draw_trees] 木を組み直した"
        );
        write_tree(&mut commands, root, tree, out, unit_rect, false);
    }
    for (owner, out, unit_rect) in segment_trees {
        let Ok((.., tree, _, _, _)) = roots.get_mut(owner) else {
            continue; // 3 部品が外れた
        };
        write_tree(&mut commands, owner, tree, out, unit_rect, true);
        if let Ok((Some(mut auto), ..)) = set.p2().get_mut(owner)
            && auto.rule.phase() == Phase::Preparing
        {
            auto.bypass_change_detection().built = true;
        }
    }
    if let Some(now) = now {
        demote_unseen(now, autos.difference(&seen).copied(), &mut set.p2());
        *autos = seen;
    } else {
        autos.extend(seen); // 時刻が無い回は戻さず、時刻のある回まで控える
    }
    for e in placed.iter().filter(|e| !written.contains(e)) {
        commands.entity(e).remove::<SurfacePlacement>();
    }
    reported.end_round();
}

/// 組んだ列を根の部品へ書く（無ければ入れる）。`device_px` は部分の列（物理ピクセル）か。根の木は
/// いつも偽で書くので、部分の列だった木が手動の持ち主の木になっても古い真が残らない。
fn write_tree(
    commands: &mut Commands,
    e: Entity,
    tree: Option<Mut<DrawTree>>,
    out: Vec<DrawCommand>,
    unit_rect: ID2D1Geometry,
    device_px: bool,
) {
    match tree {
        Some(mut tree) => {
            tree.commands = out;
            tree.unit_rect = Some(unit_rect);
            tree.device_px = device_px;
        }
        None => {
            commands.entity(e).insert(DrawTree {
                commands: out,
                unit_rect: Some(unit_rect),
                device_px,
            });
        }
    }
}

/// 根 `root` の下の見える節を、描く係と同じ順（`fill_world_visuals` → `build`）で 1 本の描く列にする
/// （画面の外へ描く口が使う・自動の上げ下げはしない）。視野は根の `Arrangement.size`。
pub(crate) fn offscreen_tree_commands(
    world: &mut World,
    root: Entity,
    factory: &ID2D1Factory,
) -> windows::core::Result<Vec<DrawCommand>> {
    let param_error = |e: bevy_ecs::system::SystemParamValidationError| {
        windows::core::Error::new(E_UNEXPECTED, e.to_string())
    };
    let top = world
        .get::<Children>(root)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    let name = world.get::<Name>(root).cloned();
    let mut scan = TreeScan::default();
    let mut fill: SystemState<Query<FillData>> = SystemState::new(world);
    fill_world_visuals(
        root,
        name.as_ref(),
        &mut fill.get_mut(world).map_err(param_error)?,
        false,
        &top,
        false,
        &mut scan,
        &mut ReportedIssues::default(),
    );
    let visited: Vec<_> = scan.visible_nodes().collect();
    let size = world
        .get::<Arrangement>(root)
        .map(|a| a.size)
        .unwrap_or_default();
    // SAFETY: Windows API 境界。
    let unit_rect: ID2D1Geometry = unsafe {
        factory.CreateRectangleGeometry(&D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 1.0,
            bottom: 1.0,
        })
    }?
    .into();
    let mut nodes: SystemState<Query<NodeData>> = SystemState::new(world);
    let mut out = Vec::new();
    build(
        &mut out,
        factory,
        &unit_rect,
        Aabb::new(0.0, 0.0, size.width, size.height),
        &nodes.get(world).map_err(param_error)?,
        &TreeFrame::Root {
            root,
            root_name: name.as_ref(),
        },
        &visited,
    );
    Ok(out)
}
