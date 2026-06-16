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
use crate::ecs::window::CompositionMode;

/// 論理的なVisualコンポーネント
/// サイズ情報はArrangementから取得する（Single Source of Truth）
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

/// ChildOf チェーンを辿ってオーナー Window の CompositionMode を取得するヘルパー
///
/// - エンティティ自身が `Window` の場合は即座に返す
/// - `ChildOf` を辿って祖先の `Window` を探す
/// - 見つからない場合（orphan Visual）は `None`
pub fn find_owner_window_composition_mode(
    world: &DeferredWorld,
    entity: Entity,
) -> Option<CompositionMode> {
    // 自身が Window の場合
    if let Some(w) = world.get::<crate::ecs::window::Window>(entity) {
        return Some(w.composition_mode());
    }

    // ChildOf チェーンを辿る
    // NOTE(W3b-V): bevy の Relationship 管理は自己参照 ChildOf（A→A）を警告付きで
    // 除去するが、間接巡回（A→B→A）は構築可能であり、その場合このループは終了しない
    // （on_visual_add フックから同期的に呼ばれるため影響大）。通常の API 使用では
    // 巡回は生成されないため前提の明文化に留める（巡回ガードの追加は P48 に記録）。
    let mut current = entity;
    loop {
        let parent = world.get::<ChildOf>(current).map(|c| c.parent());
        match parent {
            Some(p) => {
                if let Some(w) = world.get::<crate::ecs::window::Window>(p) {
                    return Some(w.composition_mode());
                }
                current = p;
            }
            None => return None,
        }
    }
}

/// Visualコンポーネントが追加されたときに呼ばれるフック
/// - Arrangementを自動挿入（既に存在する場合はスキップ）
/// - BrushInheritマーカーを自動挿入（継承解決用）
/// - DComp モードの場合のみ VisualGraphics, SurfaceGraphics, SurfaceGraphicsDirty を挿入
fn on_visual_add(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;

    // 先に全てのチェックを行う（借用の問題を回避）
    let needs_arrangement = world
        .get::<crate::ecs::layout::Arrangement>(entity)
        .is_none();
    let needs_brush_inherit = world
        .get::<crate::ecs::widget::BrushInherit>(entity)
        .is_none();

    // DComp モード判定: 祖先 Window の CompositionMode を確認
    let is_dcomp_mode =
        find_owner_window_composition_mode(&world, entity) == Some(CompositionMode::DComp);

    // DComp モードの場合のみ DComp コンポーネントを挿入対象にする
    let needs_visual_graphics = is_dcomp_mode && world.get::<VisualGraphics>(entity).is_none();
    let needs_surface_graphics = is_dcomp_mode && world.get::<SurfaceGraphics>(entity).is_none();
    let needs_surface_dirty = is_dcomp_mode && world.get::<SurfaceGraphicsDirty>(entity).is_none();

    // コマンドを発行（チェック完了後に一度だけ commands を借用する）
    let mut cmds = world.commands();
    let mut entity_cmds = cmds.entity(entity);

    // Arrangementがまだ存在しない場合のみ挿入
    // Arrangementのon_addフックがGlobalArrangementとArrangementTreeChangedを自動挿入する
    if needs_arrangement {
        entity_cmds.insert(crate::ecs::layout::Arrangement::default());
    }

    if needs_visual_graphics {
        entity_cmds.insert(VisualGraphics::default());
    }
    if needs_surface_graphics {
        entity_cmds.insert(SurfaceGraphics::default());
    }
    if needs_surface_dirty {
        entity_cmds.insert(SurfaceGraphicsDirty::default());
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
    /// `Opacity::clamped()` の移行先。
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
