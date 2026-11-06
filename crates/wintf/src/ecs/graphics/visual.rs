//! 論理Visualコンポーネント
//!
//! サイズ情報はArrangementから取得する（Single Source of Truth）

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use tracing::{error, warn};

use super::clip::ClipShape;
use super::components::{SurfaceGraphics, SurfaceGraphicsDirty, VisualGraphics};

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
/// [`insert_graphics_parts_on_window_join`] が共用する。
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

/// 窓の外で作った部品の木を後から窓の下へ付けたとき、根と子孫のうち見た目の部品を
/// まだ持たない `Visual` に、フックと同じ 3 つを入れる（4 番）。
/// `PreRenderSurface` の先頭で回し、入れた `VisualGraphics` は同じ回に作られる。
pub fn insert_graphics_parts_on_window_join(
    mut commands: Commands,
    changed: Query<Entity, Changed<ChildOf>>,
    parents: Query<&ChildOf>,
    windows: Query<(), With<crate::ecs::window::Window>>,
    children: Query<&Children>,
    pending: Query<(), (With<Visual>, Without<VisualGraphics>)>,
) {
    // ふだんは窓に属する Visual が全部持っているので、待つものが無ければ何もしない。
    if pending.is_empty() {
        return;
    }
    for entity in &changed {
        let joined = has_window_ancestor(
            entity,
            |e| windows.contains(e),
            |e| parents.get(e).ok().map(|c| c.parent()),
        );
        if !joined {
            continue;
        }
        // 子孫の走査も祖先と同じ上限で止める。
        let mut stack = vec![(entity, 0usize)];
        while let Some((e, depth)) = stack.pop() {
            if pending.contains(e) {
                insert_graphics_parts(&mut commands.entity(e));
            }
            let Ok(kids) = children.get(e) else { continue };
            if depth >= crate::ecs::common::MAX_ANCESTOR_DEPTH {
                error!(
                    root = ?entity,
                    limit = crate::ecs::common::MAX_ANCESTOR_DEPTH,
                    "[insert_graphics_parts_on_window_join] 子孫の走査が上限を越えたので止める"
                );
                continue;
            }
            stack.extend(kids.iter().map(|k| (k, depth + 1)));
        }
    }
}

/// Visualコンポーネントが追加されたときに呼ばれるフック
/// - Arrangementを自動挿入（既に存在する場合はスキップ）
/// - BrushInheritマーカーを自動挿入（継承解決用）
/// - owner Window が存在する場合のみ VisualGraphics, SurfaceGraphics, SurfaceGraphicsDirty を挿入
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

    // コマンドを発行（チェック完了後に一度だけ commands を借用する）
    let mut cmds = world.commands();
    let mut entity_cmds = cmds.entity(entity);

    // Arrangementがまだ存在しない場合のみ挿入
    // Arrangementのon_addフックがGlobalArrangementとArrangementTreeChangedを自動挿入する
    if needs_arrangement {
        entity_cmds.insert(crate::ecs::layout::Arrangement::default());
    }

    // owner Window が存在する場合のみ graphics コンポーネントを挿入する
    if has_owner_window {
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
