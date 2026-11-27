use super::promotion::Phase;
use super::{
    AutoPromotion, MixMode, OwnSurface, Visual, VisualDrawContent, VisualGroup, VisualTransform,
    VisualVersion,
};
use crate::ecs::Size;
use crate::ecs::common::MAX_ANCESTOR_DEPTH;
use crate::ecs::graphics::systems::format_entity_name;
use crate::ecs::graphics::{ClipShape, GraphicsCommandList};
use crate::ecs::layout::Arrangement;
use crate::numerics::*;
use bevy_ecs::prelude::*;
use std::collections::HashSet;
use tracing::{error, warn};
use windows_numerics::Matrix3x2;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(AutoPromotion)]
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
/// 節の種類＝`WorldVisual` を持ち、`GraphicsCommandList` と `OwnSurface` を持たない（自動で上げた持ち主も
/// 節の種類）。ほかは手動の描画面の持ち主。新しい欄は先頭に置く（後ろから数える `..` の形を崩さない）。
pub(crate) type FillData = (
    Option<Ref<'static, VisualGroup>>,
    Has<GraphicsCommandList>,
    Has<OwnSurface>,
    Option<&'static AutoPromotion>,
    Ref<'static, Visual>,
    Option<Ref<'static, Arrangement>>,
    Option<Ref<'static, VisualTransform>>,
    Option<Ref<'static, VisualDrawContent>>,
    Option<&'static mut WorldVisual>,
    Option<&'static mut VisualVersion>,
    Option<Ref<'static, Children>>,
    Option<&'static Name>,
);

/// 木の作りの誤り（`fill_world_visuals` が `error!`／`warn!` を残すもの）の種類。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum TreeIssue {
    /// 深さの上限越え・親の輪（鍵は根）。
    TooDeep,
    /// 欄へ直に書かれた不透明度の NaN（鍵はその節・`warn!`・1.0 として描く）。
    NanOpacity,
    /// 節の下の手動の持ち主に効かせられない形（鍵はその持ち主・`warn!`・1 回の判断が残す）。中身は種類の語。
    Unhonored(&'static str),
}

/// 毎回の検査で同じ誤りの記録を毎フレーム出さないための覚え。続く間は 1 回だけ記録し、
/// 消えた回の後にまた現れたらもう 1 回記録する。回の終わりに [`Self::end_round`] を呼ぶ。
/// 係の `Local` に置くので公開（中身は crate の外から触れない）。
#[derive(Default)]
pub struct ReportedIssues {
    prev: HashSet<(Entity, TreeIssue)>,
    seen: HashSet<(Entity, TreeIssue)>,
}

impl ReportedIssues {
    /// この回に見た誤りを覚え、前の回に無かった初めてのものなら `true`（記録する）。
    pub(crate) fn first_seen(&mut self, e: Entity, issue: TreeIssue) -> bool {
        self.seen.insert((e, issue)) && !self.prev.contains(&(e, issue))
    }

    /// 回の終わり。この回に見なかった誤りは消えたものとして忘れる。
    pub(crate) fn end_round(&mut self) {
        self.prev = std::mem::take(&mut self.seen);
    }
}

/// 手動の持ち主 1 つの前順の走査の列（係の `Local` に置き使い回す）。節・自動で上げた持ち主・
/// 節の下の手動の持ち主を 1 本に並べる。係の `Local` に置くので公開（中身は crate の外から触れない）。
#[derive(Default, Debug)]
pub struct TreeScan {
    pub(crate) entries: Vec<ScanEntry>,
}

impl TreeScan {
    /// 見える節の `(実体, 深さ)`（前順）。手動の持ち主の項と見えない節は除く（画面は 1 回の判断の分け方を使う）。
    pub(crate) fn visible_nodes(&self) -> impl Iterator<Item = (Entity, usize)> + '_ {
        self.entries
            .iter()
            .filter(|e| e.is_visible_node())
            .map(|e| (e.entity, e.depth))
    }
}

/// 走査の列の 1 項。
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScanEntry {
    pub entity: Entity,
    /// 根の子が 1。
    pub depth: usize,
    /// 親の節の添字（根の直下は `None`）。
    pub parent: Option<usize>,
    /// 部分木の終わり（前順で連続・排他）。
    pub end: usize,
    pub kind: ScanKind,
    /// 節は自分と祖先の節が見える。手動の持ち主は祖先の節が見える（自分の可視は WUC が写す）。
    pub visible: bool,
    /// 節は埋めた値（見えない節は部品へ書かない）。手動の持ち主は置き場の行列×自分の局所の変形と、
    /// 自分の配置の矩形から作る（親の節の部分木の外接矩形には足さない）。
    pub world: WorldVisual,
    /// この回の更新（見える節の版の上がりか、その節自身の `VisualTransform`・`Arrangement` の変化）。
    pub updated: bool,
    /// 自分の切り抜きの種類。
    pub own_clip: ClipClass,
    /// 自分の件の混ぜ方が `SOURCE_OVER` か（中身が無ければ真）。
    pub source_over: bool,
    /// 親の持ち主から自分の親までの節の積み重ね。
    pub ancestors: AncestorAcc,
}

impl ScanEntry {
    pub(crate) fn is_visible_node(&self) -> bool {
        self.visible && matches!(self.kind, ScanKind::Node { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ScanKind {
    Node {
        phase: Phase,
    },
    /// 節の下か根の直下の手動の持ち主。`parent_world` は親の節の世界の行列（根の直下は恒等）。
    Owner {
        parent_world: Matrix3x2,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipClass {
    None,
    Rect,
    Rounded,
    Geometry,
}

impl ClipClass {
    fn of(clip: Option<&ClipShape>) -> Self {
        match clip {
            None => Self::None,
            Some(ClipShape::Rectangle) => Self::Rect,
            Some(
                ClipShape::RoundedRectangle { .. } | ClipShape::RoundedRectangleIndividual { .. },
            ) => Self::Rounded,
            Some(ClipShape::Geometry(_)) => Self::Geometry,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AncestorAcc {
    /// 祖先の節の `clamped_opacity` の積。
    pub opacity: f32,
    /// 祖先に不透明度 1 未満か角丸・任意の図形の切り抜きかまとめの節が在る。
    pub layer: bool,
    /// 祖先の矩形の切り抜きの交わり（親の持ち主の DIP・軸に沿うものだけ）。
    pub rect_clip: Option<Aabb>,
    /// 軸に沿わない矩形の切り抜きの祖先が在る。
    pub rect_clip_rotated: bool,
    /// 祖先に `SOURCE_OVER` でない混ぜ方の中身が在る。
    pub blend: bool,
}

impl AncestorAcc {
    /// 根の直下（祖先の節が無い）。
    pub(crate) const ROOT: Self = Self {
        opacity: 1.0,
        layer: false,
        rect_clip: None,
        rect_clip_rotated: false,
        blend: false,
    };

    /// 節 `visual`（配置の大きさ `size`・埋めた `world`）の子へ渡す積み重ね。
    fn under(
        self,
        visual: &Visual,
        size: Size,
        world: &WorldVisual,
        source_over: bool,
        group: bool,
    ) -> Self {
        let opacity = visual.clamped_opacity();
        let mut acc = Self {
            opacity: self.opacity * opacity,
            layer: self.layer
                || group
                || opacity < 1.0
                || matches!(
                    ClipClass::of(visual.clip.as_ref()),
                    ClipClass::Rounded | ClipClass::Geometry
                ),
            blend: self.blend || !source_over,
            ..self
        };
        if let Some(ClipShape::Rectangle) = visual.clip {
            if world.axis_aligned {
                let r = Aabb::new(0.0, 0.0, size.width, size.height).transform(world.world_mat);
                acc.rect_clip = Some(acc.rect_clip.map_or(r, |c| c.intersect(r)));
            } else {
                acc.rect_clip_rotated = true;
            }
        }
        acc
    }
}

/// 根 `root`（手動の描画面の持ち主）の下を前順に 1 度歩き、節の `WorldVisual` を根からの DIP で埋め、
/// 走査の列 `scan` を記す。
///
/// - 局所の変形は `VisualTransform × 平行移動(Arrangement.offset)`（無ければ恒等）・
///   `world = local * parent`（行ベクトル）。根は恒等。
/// - 前順（`Children` の並び順）に訪れる。描く列は 1 回の判断（`run_round`）がこの列から分ける。
/// - 見えない節と不透明度（`clamped_opacity`）が 0 の節は、自分も子孫も `visible = false` の項になり
///   （`WorldVisual` は前の値のまま）、版を上げる・不透明度の NaN の検査・深さの検査だけを行う。
/// - 欄へ直に書かれた不透明度の NaN は 1.0 として訪れ、`warn!` を `reported` で続く間 1 回だけ残す。
/// - 子が手動の持ち主なら、その部分木は歩かず、置き場の材料（親の節の世界の行列）を持つ項を 1 つ
///   入れて止まる（子孫の節はその持ち主自身の歩きが受け持つ・記録しない）。自動で上げた持ち主は
///   節の種類なので通り抜ける。
/// - 深さが `MAX_ANCESTOR_DEPTH` を越えた節は訪れず `error!` を 1 行残す（親の輪もここで止まる）。
///   `reported` で、誤りが続く間は 1 回だけにする（止めるは毎回行う）。
/// - `Visual`・描く中身・まとめ（`VisualGroup`）が変わった節（見えなくなった節も）は `VisualVersion` を 1 つ上げる。
///   `VisualTransform` は見ない＝変形だけでは上げない。`Visual` の変化には `opacity`
///   （描き直しになる）・`transform_origin`（版が余分に上がるだけ）も入る。
/// - 値が変わった節だけ書く。戻り値「変化あり」は、版の上がり・行列などの変化・節の入り
///   （`WorldVisual` が付いた）・親子の変化（`Children`・`root_children_changed`）・根の配置の変化
///   （`root_arrangement_changed`）・切り抜きを持つ節の `Arrangement` の変化。後の 2 つは版を上げない。
///   節の出は `update_draw_trees` が `RemovedComponents` で拾う。
#[allow(clippy::too_many_arguments)]
pub(crate) fn fill_world_visuals(
    root: Entity,
    root_name: Option<&Name>,
    nodes: &mut Query<FillData>,
    root_arrangement_changed: bool,
    children_of_root: &[Entity],
    root_children_changed: bool,
    scan: &mut TreeScan,
    reported: &mut ReportedIssues,
) -> bool {
    let entries = &mut scan.entries;
    entries.clear();
    let mut changed = root_arrangement_changed || root_children_changed;
    let mut bumped: Vec<Entity> = Vec::new();
    // (実体, 深さ, 親の項の添字, 見えない部分木の中か, 祖先の積み重ね)。逆に積んで pop が並び順になるようにする。
    let mut stack: Vec<(Entity, usize, Option<usize>, bool, AncestorAcc)> = children_of_root
        .iter()
        .rev()
        .map(|&c| (c, 1, None, false, AncestorAcc::ROOT))
        .collect();
    while let Some((e, depth, parent, hidden, acc)) = stack.pop() {
        let Ok((
            group,
            has_list,
            has_own,
            auto,
            visual,
            arrangement,
            transform,
            content,
            wv,
            _,
            kids,
            name,
        )) = nodes.get(e)
        else {
            continue; // 消えた実体・Visual でない実体
        };
        let parent_world = parent.map_or(Matrix3x2::identity(), |p| entries[p].world.world_mat);
        let local = transform
            .as_ref()
            .map_or(Matrix3x2::identity(), |t| t.to_affine())
            * arrangement
                .as_ref()
                .map_or(Matrix3x2::identity(), |a| a.offset.into());
        let size = arrangement.as_ref().map_or_else(Size::default, |a| a.size);
        let own_clip = ClipClass::of(visual.clip.as_ref());
        // `SOURCE_OVER` でないまとめは 1 枚を切り抜きまで広げて重ねるので、そうした件と同じ扱い。
        let source_over = content
            .as_ref()
            .is_none_or(|c| c.mix_mode == MixMode::default())
            && group.as_ref().is_none_or(|g| g.mix == MixMode::default());
        let idx = entries.len();
        if wv.is_none() || has_list || has_own {
            let mut w = WorldVisual::from_world_mat(local * parent_world);
            w.content_world_aabb =
                Aabb::new(0.0, 0.0, size.width, size.height).transform(w.world_mat);
            w.subtree_world_aabb = w.content_world_aabb;
            entries.push(ScanEntry {
                entity: e,
                depth,
                parent,
                end: idx + 1,
                kind: ScanKind::Owner { parent_world },
                visible: !hidden,
                world: w,
                updated: false,
                own_clip,
                source_over,
                ancestors: acc,
            });
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
        let bump = visual.is_changed()
            || content.as_ref().is_some_and(|c| c.is_changed())
            || group.as_ref().is_some_and(|g| g.is_changed());
        if bump {
            bumped.push(e);
        }
        if visual.opacity.is_nan() && reported.first_seen(e, TreeIssue::NanOpacity) {
            warn!(
                entity = %format_entity_name(e, name),
                "[fill_world_visuals] 節の不透明度が NaN なので、1.0 として描く"
            );
        }
        // 見えない部分木も版だけは追う（隠れている間の変化を、また見えた回に失わない）。
        let visible = !hidden && visual.is_visible && visual.clamped_opacity() != 0.0;
        if visible {
            changed |= kids.as_ref().is_some_and(|k| k.is_changed())
                || (visual.clip.is_some() && arrangement.as_ref().is_some_and(|a| a.is_changed()));
        }
        let mut w = WorldVisual::from_world_mat(local * parent_world);
        w.content_world_aabb = content
            .as_ref()
            .map_or(Aabb::EMPTY, |c| c.local_aabb.transform(w.world_mat));
        w.subtree_world_aabb = w.content_world_aabb;
        // 祖先の動きで世界の行列だけが変わった回は数えない。
        let updated = visible
            && (bump
                || transform.as_ref().is_some_and(|t| t.is_changed())
                || arrangement.as_ref().is_some_and(|a| a.is_changed()));
        let phase = auto.map_or(Phase::Node, |a| a.rule.phase());
        let child_acc = acc.under(&visual, size, &w, source_over, group.is_some());
        entries.push(ScanEntry {
            entity: e,
            depth,
            parent,
            end: idx + 1,
            kind: ScanKind::Node { phase },
            visible,
            world: w,
            updated,
            own_clip,
            source_over,
            ancestors: acc,
        });
        if let Some(kids) = kids {
            stack.extend(
                kids.iter()
                    .rev()
                    .map(|k| (k, depth + 1, Some(idx), !visible, child_acc)),
            );
        }
    }
    // 前順では子が親より後ろに在るので、逆にたどれば子の和と部分木の終わりが先に揃う。
    // 外接矩形の和は見える節だけ（手動の持ち主と見えない節は足さない）。
    for i in (0..entries.len()).rev() {
        let Some(p) = entries[i].parent else { continue };
        entries[p].end = entries[p].end.max(entries[i].end);
        if entries[i].is_visible_node() {
            entries[p].world.subtree_world_aabb = entries[p]
                .world
                .subtree_world_aabb
                .union(entries[i].world.subtree_world_aabb);
        }
    }
    for entry in entries.iter().filter(|e| e.is_visible_node()) {
        if let Ok((.., Some(mut wv), _, _, _)) = nodes.get_mut(entry.entity) {
            changed |= wv.is_added();
            if *wv != entry.world {
                *wv = entry.world;
                changed = true;
            }
        }
    }
    changed |= !bumped.is_empty();
    for e in bumped {
        if let Ok((.., Some(mut version), _, _)) = nodes.get_mut(e) {
            version.bump();
        }
    }
    changed
}
