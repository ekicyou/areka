use super::{Visual, VisualDrawContent, VisualTransform, VisualVersion};
use crate::ecs::common::MAX_ANCESTOR_DEPTH;
use crate::ecs::graphics::components::VisualGraphics;
use crate::ecs::graphics::systems::format_entity_name;
use crate::ecs::layout::Arrangement;
use crate::numerics::*;
use bevy_ecs::prelude::*;
use std::collections::HashSet;
use tracing::{error, warn};
use windows_numerics::Matrix3x2;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WorldVisual {
    /// local → world の累積アフィン。
    pub world_mat: Matrix3x2,

    /// world_mat が軸に沿うか（`Matrix3x2Ext::axis_aligned`・90 度の倍数も沿う）。
    /// `PushAxisAlignedClip` 可否 / `Aabb::transform` fast path のキャッシュ。
    pub axis_aligned: bool,

    /// 自コンテンツの world AABB（`DrawContent.local_aabb * world_mat`）。
    /// Draw 時の実効クリップ `top_clip & content_world_aabb` に使用。
    pub content_world_aabb: Aabb,

    /// 自 + 子孫の world AABB union（サブツリー・カリング／スクロール集計用）。
    pub subtree_world_aabb: Aabb,
}

impl WorldVisual {
    /// 未計算状態の初期値。DFS 前のプレースホルダとして使う。
    pub const PENDING: WorldVisual = WorldVisual {
        world_mat: Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: 0.0,
            M32: 0.0,
        },
        axis_aligned: true,
        content_world_aabb: Aabb::EMPTY,
        subtree_world_aabb: Aabb::EMPTY,
    };

    /// world_mat から軸平行フラグを判定して構築する。
    /// content / subtree AABB は呼び出し側（DFS）で確定させる。
    #[inline]
    pub fn from_world_mat(world_mat: Matrix3x2) -> Self {
        Self {
            world_mat,
            axis_aligned: world_mat.axis_aligned(),
            content_world_aabb: Aabb::EMPTY,
            subtree_world_aabb: Aabb::EMPTY,
        }
    }
}

impl Default for WorldVisual {
    #[inline]
    fn default() -> Self {
        Self::PENDING
    }
}

/// `fill_world_visuals` が 1 つの `Visual` から読み書きする部品。
/// `WorldVisual` を持たないもの・`VisualGraphics` を持つものは描画面の持ち主（節でない）。
pub(crate) type FillData = (
    Ref<'static, Visual>,
    Option<Ref<'static, Arrangement>>,
    Option<&'static VisualTransform>,
    Option<Ref<'static, VisualDrawContent>>,
    Option<&'static mut WorldVisual>,
    Option<&'static mut VisualVersion>,
    Option<Ref<'static, Children>>,
    Has<VisualGraphics>,
    Option<&'static Name>,
);

/// 木の作りの誤り（`fill_world_visuals` が `error!` を残すもの）の種類。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum TreeIssue {
    /// 節の下の描画面の持ち主（鍵はその持ち主）。
    OwnerUnderNode,
    /// 深さの上限越え・親の輪（鍵は根）。
    TooDeep,
}

/// 毎回の検査で同じ誤りの `error!` を毎フレーム出さないための覚え。続く間は 1 回だけ記録し、
/// 消えた回の後にまた現れたらもう 1 回記録する。回の終わりに [`Self::end_round`] を呼ぶ。
/// 係の `Local` に置くので公開（中身は crate の外から触れない）。
#[derive(Default)]
pub struct ReportedIssues {
    prev: HashSet<(Entity, TreeIssue)>,
    seen: HashSet<(Entity, TreeIssue)>,
}

impl ReportedIssues {
    /// この回に見た誤りを覚え、前の回に無かった初めてのものなら `true`（記録する）。
    fn first_seen(&mut self, e: Entity, issue: TreeIssue) -> bool {
        self.seen.insert((e, issue)) && !self.prev.contains(&(e, issue))
    }

    /// 回の終わり。この回に見なかった誤りは消えたものとして忘れる。
    pub(crate) fn end_round(&mut self) {
        self.prev = std::mem::take(&mut self.seen);
    }
}

/// 根 `root`（描画面の持ち主）の下の節の `WorldVisual` を根からの DIP で埋める。
///
/// - 局所の変形は `VisualTransform × 平行移動(Arrangement.offset)`（無ければ恒等）・
///   `world = local * parent`（行ベクトル）。根は恒等。
/// - 前順（`Children` の並び順）に訪れ、`visited` に訪れた節と深さ（根の子が 1）を入れる。
///   平らにする係はこの順をそのまま使う。
/// - 見えない節は自分も子孫も訪れない（`WorldVisual` は前の値のまま）。ただし版を上げる・
///   不透明度の `warn!`・持ち主と深さの検査だけは見えない部分木の中でも行う。
/// - 子が描画面の持ち主なら訪れない。親が節なら `error!` を残して切る（根の直下は記録しない）。
/// - 深さが `MAX_ANCESTOR_DEPTH` を越えた節は訪れず `error!` を 1 行残す（親の輪もここで止まる）。
/// - この 2 つの `error!` は `reported` で、誤りが続く間は 1 回だけにする（切る・止めるは毎回行う）。
/// - `Visual` か描く中身が変わった節（見えなくなった節も）は `VisualVersion` を 1 つ上げる。
///   `VisualTransform` は見ない＝変形だけでは上げない。`Visual` の変化には `opacity`・
///   `transform_origin` も入る（版が余分に上がるだけ）。
/// - `Visual` が変わった回に `opacity != 1.0` なら `warn!` を 1 行（節の不透明度は 1.0 で描く）。
/// - 値が変わった節だけ書く。戻り値「変化あり」は、版の上がり・行列などの変化・節の入り
///   （`WorldVisual` が付いた）・親子の変化（`Children`・`root_children_changed`）・根の配置の変化
///   （`root_arrangement_changed`）・切り抜きを持つ節の `Arrangement` の変化。後の 2 つは版を上げない。
///   節の出は `update_draw_trees` が `RemovedComponents` で拾う。
pub(crate) fn fill_world_visuals(
    root: Entity,
    root_name: Option<&Name>,
    nodes: &mut Query<FillData>,
    root_arrangement_changed: bool,
    children_of_root: &[Entity],
    root_children_changed: bool,
    visited: &mut Vec<(Entity, usize)>,
    reported: &mut ReportedIssues,
) -> bool {
    visited.clear();
    let mut changed = root_arrangement_changed || root_children_changed;
    let mut bumped: Vec<Entity> = Vec::new();
    let mut worlds: Vec<WorldVisual> = Vec::new();
    let mut parents: Vec<Option<usize>> = Vec::new();
    // (実体, 深さ, 親の visited の添字, 見えない部分木の中か)。逆に積んで pop が並び順になるようにする。
    let mut stack: Vec<(Entity, usize, Option<usize>, bool)> = children_of_root
        .iter()
        .rev()
        .map(|&c| (c, 1, None, false))
        .collect();
    while let Some((e, depth, parent, hidden)) = stack.pop() {
        let Ok((visual, arrangement, transform, content, wv, _, kids, has_surface, name)) =
            nodes.get(e)
        else {
            continue; // 消えた実体・Visual でない実体
        };
        if has_surface || wv.is_none() {
            if (parent.is_some() || hidden) && reported.first_seen(e, TreeIssue::OwnerUnderNode) {
                error!(
                    entity = %format_entity_name(e, name),
                    "[fill_world_visuals] 節の下の描画面の持ち主は支えないので、その部分木を切る"
                );
            }
            continue;
        }
        if depth > MAX_ANCESTOR_DEPTH {
            if reported.first_seen(root, TreeIssue::TooDeep) {
                error!(
                    root = %format_entity_name(root, root_name),
                    limit = MAX_ANCESTOR_DEPTH,
                    "[fill_world_visuals] 木の深さが上限を越えたので、それより深い節を訪れない"
                );
            }
            continue;
        }
        if visual.is_changed() || content.as_ref().is_some_and(|c| c.is_changed()) {
            bumped.push(e);
        }
        if visual.is_changed() && visual.opacity != 1.0 {
            warn!(
                entity = %format_entity_name(e, name),
                opacity = visual.opacity,
                "[fill_world_visuals] 節の不透明度の層は wintf-d2d-composite-rules の範囲なので、1.0 で描く"
            );
        }
        if hidden || !visual.is_visible {
            // 見えない部分木も版だけは追う（隠れている間の変化を、また見えた回に失わない）。
            if let Some(kids) = kids {
                stack.extend(kids.iter().rev().map(|k| (k, depth + 1, None, true)));
            }
            continue;
        }
        changed |= kids.as_ref().is_some_and(|k| k.is_changed())
            || (visual.clip.is_some() && arrangement.as_ref().is_some_and(|a| a.is_changed()));
        let parent_mat = parent.map_or(Matrix3x2::identity(), |p| worlds[p].world_mat);
        let local = transform.map_or(Matrix3x2::identity(), |t| t.to_affine())
            * arrangement.map_or(Matrix3x2::identity(), |a| a.offset.into());
        let mut w = WorldVisual::from_world_mat(local * parent_mat);
        w.content_world_aabb = content.map_or(Aabb::EMPTY, |c| c.local_aabb.transform(w.world_mat));
        w.subtree_world_aabb = w.content_world_aabb;
        let idx = visited.len();
        visited.push((e, depth));
        worlds.push(w);
        parents.push(parent);
        if let Some(kids) = kids {
            stack.extend(kids.iter().rev().map(|k| (k, depth + 1, Some(idx), false)));
        }
    }
    // 前順では子が親より後ろに在るので、逆にたどれば子の和が先に揃う。
    for i in (0..worlds.len()).rev() {
        if let Some(p) = parents[i] {
            worlds[p].subtree_world_aabb = worlds[p]
                .subtree_world_aabb
                .union(worlds[i].subtree_world_aabb);
        }
    }
    for (&(e, _), w) in visited.iter().zip(&worlds) {
        if let Ok((.., Some(mut wv), _, _, _, _)) = nodes.get_mut(e) {
            changed |= wv.is_added();
            if *wv != *w {
                *wv = *w;
                changed = true;
            }
        }
    }
    changed |= !bumped.is_empty();
    for e in bumped {
        if let Ok((.., Some(mut version), _, _, _)) = nodes.get_mut(e) {
            version.bump();
        }
    }
    changed
}
