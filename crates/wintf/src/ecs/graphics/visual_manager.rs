use crate::ecs::graphics::{GraphicsCore, Visual, VisualGraphics, WindowGraphics};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace};
use windows::UI::Composition::Compositor;
use windows::core::Interface;

use super::format_entity_name;
use super::wuc_resource::WucGraphicsResource;

/// デフォルトのVisualコンポーネントをEntityに挿入する (R3)
///
/// ウィジェットの on_add フックから呼び出すことを想定したヘルパー関数。
/// Entity が存在しない場合は何もしない。
///
/// # Arguments
/// * `world` - ECS World への可変参照
/// * `entity` - Visual を挿入する対象の Entity
pub fn insert_visual(world: &mut World, entity: Entity) {
    insert_visual_with(world, entity, Visual::default());
}

/// カスタム Visual コンポーネントを Entity に挿入する (R3)
///
/// ウィジェットの on_add フックから呼び出すことを想定したヘルパー関数。
/// Entity が存在しない場合は何もしない。
///
/// # Arguments
/// * `world` - ECS World への可変参照
/// * `entity` - Visual を挿入する対象の Entity
/// * `visual` - 挿入する Visual コンポーネント
pub fn insert_visual_with(world: &mut World, entity: Entity, visual: Visual) {
    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
        entity_mut.insert(visual);
    }
}

/// Visualリソースのみを作成する（Surfaceは作成しない）
///
/// Phase 6リファクタリング: Visual作成をPreLayoutに移動し、
/// Surface作成はDrawスケジュールでCommandList存在時に遅延実行する。
///
/// Note: SurfaceGraphicsとSurfaceGraphicsDirtyはVisual.on_addで事前配置されている
fn create_visual_only(entity: Entity, vg: &mut VisualGraphics, compositor: &Compositor) {
    // SpriteVisual は ContainerVisual を継承し、brush（surface 束ね）も子（階層）も持てるため
    // 全 Visual を SpriteVisual で統一する。基底 WUC `Visual` へ格納する。
    let visual_res = compositor
        .CreateSpriteVisual()
        .and_then(|sprite| sprite.cast::<windows::UI::Composition::Visual>());
    match visual_res {
        Ok(v) => {
            // VisualGraphicsを直接更新（既にVisual.on_addで配置済み）
            *vg = VisualGraphics::new(v.clone());

            // SurfaceGraphicsとSurfaceGraphicsDirtyはVisual.on_addで事前配置済み
            // 明示的なset_changed()は不要（VisualGraphicsの更新でChanged検知される）

            debug!(
                entity = ?entity,
                "Visual created (VisualGraphics initialized)"
            );
        }
        Err(e) => {
            error!(error = ?e, "Failed to create visual");
        }
    }
}

/// Visualコンポーネントに基づいてGPUリソースを管理するシステム
///
/// Phase 6: Visualのみを作成し、Surfaceは作成しない。
/// SurfaceはDrawスケジュールでCommandList存在時に遅延作成される。
///
/// Changed: Added<Visual> から Changed<VisualGraphics> + !is_valid() パターンに移行
/// Visual.on_add で VisualGraphics::default() が挿入され、このシステムがGPUリソースを作成
pub fn visual_resource_management_system(
    // NOTE: Commands は Phase 6（Surface 作成の Draw スケジュール移行）以降未使用。
    // パラメータ削除はシステムのアクセスセット変更にあたるため P45 として記録（P39 と同系統）。
    _commands: Commands,
    graphics: Res<GraphicsCore>,
    wuc_resource: Option<Res<WucGraphicsResource>>,
    mut query: Query<
        (Entity, &Visual, &mut VisualGraphics, Option<&Name>),
        Changed<VisualGraphics>,
    >,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    if !graphics.is_valid() {
        return;
    }

    let compositor = match wuc_resource.as_ref().and_then(|r| r.compositor()) {
        Some(c) => c,
        None => return,
    };

    for (entity, _visual, mut vg, name) in query.iter_mut() {
        // VisualGraphicsが無効な場合のみGPUリソースを作成
        if !vg.is_valid() {
            let entity_name = format_entity_name(entity, name);
            trace!(
                frame = frame_count.0,
                entity = %entity_name,
                "VisualGraphics initialization starting (Changed + !is_valid)"
            );
            create_visual_only(entity, &mut vg, compositor);
            trace!(
                frame = frame_count.0,
                entity = %entity_name,
                "VisualGraphics initialization completed"
            );
        }
    }
}

/// WindowGraphicsとVisualGraphicsを紐付けるシステム
///
/// WindowGraphicsを持つエンティティにVisualGraphicsが追加された場合、
/// そのVisualをウィンドウのルートVisualとして設定する。
pub fn window_visual_integration_system(
    query: Query<
        (Entity, &WindowGraphics, &VisualGraphics, Option<&Name>),
        Or<(Changed<WindowGraphics>, Changed<VisualGraphics>)>,
    >,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    for (entity, window_graphics, visual_graphics, name) in query.iter() {
        if let Some(target) = window_graphics.get_target() {
            if let Some(visual) = visual_graphics.visual() {
                let entity_name = format_entity_name(entity, name);
                trace!(
                    frame = frame_count.0,
                    entity = %entity_name,
                    "SetRoot executing"
                );
                // target / visual は直前の二重 Some ガードで有効性を確認済み。
                // DesktopWindowTarget::SetRoot は WUC の安全メソッド（unsafe 不要）で、
                // 失敗は Result で報告される。
                // NOTE(W3b-V): 失敗の黙殺（`let _ =`）は意図的 — SetRoot が失敗するのは
                // 実質デバイスロスト時のみで、復旧は invalidate 系統が担う設計
                // （ただし検出経路の不在は P40 を参照）。
                let _ = target.SetRoot(visual);
            }
        }
    }
}
