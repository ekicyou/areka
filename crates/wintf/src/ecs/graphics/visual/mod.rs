//! 論理Visualコンポーネント
//!
//! サイズ情報はArrangementから取得する（Single Source of Truth）

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use tracing::warn;

use super::clip::ClipShape;
use super::components::{SurfaceGraphics, SurfaceGraphicsDirty, VisualGraphics};

mod draw;
mod draw_content;
mod draw_tree;
mod systems;
mod transform;
mod version;
mod world_visual;

#[cfg(test)]
pub(crate) mod draw_content_test_support;
#[cfg(test)]
mod draw_content_tests;
#[cfg(test)]
mod systems_tests;
#[cfg(test)]
mod visual_tests;
#[cfg(test)]
mod world_visual_tests;

pub use draw::command::*;
pub(crate) use draw::emit;
pub use draw_content::*;
pub use draw_tree::*;
pub use systems::*;
pub use transform::*;
pub use version::*;
pub use world_visual::*;

/// 論理的なVisualコンポーネント
/// サイズ情報はArrangementから取得する（Single Source of Truth）
///
/// # 可視・切り抜き・不透明度
/// 可視（`is_visible`）・切り抜き（`clip`）・不透明度（`opacity`）は、描画面の有無に依らず
/// `Visual` の 1 つの性質。描画面の持ち主では WUC（`visual_property_sync_system`・
/// `clip_sync_system`）が、描画面を持たない節では D2D（描画の木を描く係）が実現する。
/// - 任意の図形の切り抜き（`ClipShape::Geometry`）は節だけが効かせる。持ち主では `warn!` を
///   1 度残して切り抜かない。
/// - 節の不透明度は保留（1.0 以外は `warn!` を 1 度残して 1.0 で描く）。
///
/// # 描画面の持ち主か節か
/// 新しい描き方の部品（`VisualDrawContent`・`VisualTransform`）を持つ `Visual` は既定で
/// 描画面を持たない節になる（一度節になれば、装置の失いで中身が外れても節のまま）。
/// `GraphicsCommandList` を持つ `Visual` と何も持たない `Visual` は描画面の持ち主。
/// 節を持ち主へ戻す（手動で昇格する）には目印 [`OwnSurface`] を付ける。昇格した持ち主の
/// `VisualDrawContent` は自分の描画面の木の先頭の 1 件として子より先に描かれる。持ち主の
/// `VisualTransform` は WUC では効かない（入った・変わった回に `warn!` を 1 度残して無視する）。
///
/// # ライフタイムイベント
/// - `on_add`: `Arrangement::default()`を自動挿入
///   - これにより`Arrangement`の`on_add`が連鎖的に`GlobalArrangement`と`ArrangementTreeChanged`を挿入
///
/// # メモリ戦略
/// - **Table戦略**: すべてのエンティティに存在する根幹コンポーネント
/// - on_addフックで他の必須コンポーネントを連鎖的に生成
#[derive(Component, Debug, Clone, PartialEq)]
#[component(on_add = on_visual_add)]
pub struct Visual {
    pub is_visible: bool,
    pub opacity: f32,
    pub transform_origin: crate::ecs::PointF,
    pub clip: Option<ClipShape>,
}

/// 描画面を持つ目印（手動の昇格の口）。在れば新しい描き方の部品の有無に依らず、
/// その `Visual` は描画面の持ち主（WUC の Visual と描画面を持ち、その下の木の根）になる。
/// 一度節になった `Visual` を持ち主へ戻すのもこの目印で行う。自分の `VisualDrawContent` は
/// 描画面に描かれ続ける（子の節が無くても）。`VisualTransform` は無視される（`warn!` が 1 度）。
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OwnSurface;

/// 描画面を持つか（持ち主）、持たないか（節）の規則。1 か所だけに置く。
/// `is_node` は `WorldVisual` を持つか（新しい描き方の部品が `require` で必ず連れてくる・
/// 装置の失いでも外れない）。
pub(crate) fn wants_own_surface(
    has_command_list: bool,
    has_own_surface: bool,
    is_node: bool,
) -> bool {
    has_command_list || has_own_surface || !is_node
}

/// ChildOf チェーンを辿ってオーナー Window が存在するかを判定するヘルパー
///
/// - エンティティ自身が `Window` の場合は即座に `true`
/// - `ChildOf` を辿って祖先に `Window` があれば `true`
/// - 見つからない場合（orphan Visual）は `false`
/// - 祖先の走査は上限つき（`ancestors`）。上限を越えたら「見つからない」として `false`
pub fn owner_window_exists(world: &DeferredWorld, entity: Entity) -> bool {
    use crate::ecs::window::Window;
    // 親の輪（A→B→A）があっても上限で止まる（on_visual_add から同期的に呼ばれる）。
    has_window_ancestor(
        entity,
        |e| world.get::<Window>(e).is_some(),
        |e| world.get::<ChildOf>(e).map(|c| c.parent()),
    )
}

/// 窓に属するかの規則（自分が窓か、上限つきの祖先に窓が在るか）。フックと
/// [`sync_surface_owner_parts`] が共用する。
fn has_window_ancestor(
    entity: Entity,
    is_window: impl Fn(Entity) -> bool,
    parent_of: impl FnMut(Entity) -> Option<Entity>,
) -> bool {
    is_window(entity) || crate::ecs::common::ancestors(entity, parent_of).any(is_window)
}

/// 窓に属する部品へ入れる見た目の部品 3 つ。既に在るものは入れない。
fn insert_graphics_parts(entity_cmds: &mut EntityCommands) {
    entity_cmds.insert_if_new((
        VisualGraphics::default(),
        SurfaceGraphics::default(),
        SurfaceGraphicsDirty::default(),
    ));
}

/// Visualコンポーネントが追加されたときに呼ばれるフック
/// - Arrangementを自動挿入（既に存在する場合はスキップ）
/// - BrushInheritマーカーを自動挿入（継承解決用）
/// - owner Window が存在し [`wants_own_surface`] が真の場合のみ VisualGraphics, SurfaceGraphics, SurfaceGraphicsDirty を挿入
fn on_visual_add(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;

    // 先に全てのチェックを行う（借用の問題を回避）
    let needs_arrangement = world
        .get::<crate::ecs::layout::Arrangement>(entity)
        .is_none();
    let needs_brush_inherit = world
        .get::<crate::ecs::widget::BrushInherit>(entity)
        .is_none();

    // owner Window 存在判定: 祖先に Window があるかを確認
    let has_owner_window = owner_window_exists(&world, entity);
    // bevy はバンドルを全部書いてからフックを呼ぶので、同じバンドルの部品が見える。
    let wants_surface = wants_own_surface(
        world.get::<super::GraphicsCommandList>(entity).is_some(),
        world.get::<OwnSurface>(entity).is_some(),
        world.get::<WorldVisual>(entity).is_some(),
    );

    // コマンドを発行（チェック完了後に一度だけ commands を借用する）
    let mut cmds = world.commands();
    let mut entity_cmds = cmds.entity(entity);

    // Arrangementがまだ存在しない場合のみ挿入
    // Arrangementのon_addフックがGlobalArrangementとArrangementTreeChangedを自動挿入する
    if needs_arrangement {
        entity_cmds.insert(crate::ecs::layout::Arrangement::default());
    }

    // owner Window が存在し、規則が描画面の持ち主と答える場合のみ graphics コンポーネントを挿入する
    if has_owner_window && wants_surface {
        insert_graphics_parts(&mut entity_cmds);
    }

    // BrushInheritマーカーを挿入（継承解決システムで処理される）
    // Note: Brushesコンポーネントは挿入しない（オプショナル設計）
    if needs_brush_inherit {
        entity_cmds.insert(crate::ecs::widget::BrushInherit);
    }
}

impl Default for Visual {
    fn default() -> Self {
        Self {
            is_visible: true,
            opacity: 1.0,
            transform_origin: crate::ecs::PointF::default(),
            clip: None,
        }
    }
}

impl Visual {
    /// opacity を 0.0〜1.0 にクランプして設定する。
    /// 範囲外の値は warn ログを出力後にクランプされる。
    pub fn set_opacity(&mut self, value: f32) {
        if value < 0.0 || value > 1.0 {
            warn!(
                value = value,
                "Opacity value is outside valid range [0.0, 1.0]"
            );
        }
        self.opacity = value.clamp(0.0, 1.0);
    }

    /// クランプ済み opacity 値を返す (0.0〜1.0)。
    pub fn clamped_opacity(&self) -> f32 {
        self.opacity.clamp(0.0, 1.0)
    }

    /// is_visible を設定する。
    pub fn set_visible(&mut self, visible: bool) {
        self.is_visible = visible;
    }

    /// クリップ形状を設定する。`None` でクリップ解除。
    pub fn set_clip(&mut self, clip: Option<ClipShape>) {
        self.clip = clip;
    }
}
