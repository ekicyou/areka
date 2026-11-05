use crate::ecs::graphics::{GraphicsCore, Visual, VisualGraphics, WindowGraphics};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace};
use windows::UI::Composition::Compositor;
use windows::core::Interface;

use super::format_entity_name;
use super::wuc_resource::WucGraphicsResource;

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
///
/// 本番では装置が使える回だけ回す（`graphics_core_ready`）。装置が無いか無効なら戻るのは、
/// 使う人が係を直に登録したときの守り。
pub fn visual_resource_management_system(
    graphics: Option<Res<GraphicsCore>>,
    wuc_resource: Option<Res<WucGraphicsResource>>,
    mut query: Query<
        (Entity, &Visual, &mut VisualGraphics, Option<&Name>),
        Changed<VisualGraphics>,
    >,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    if !graphics.is_some_and(|g| g.is_valid()) {
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
        if let (Some(target), Some(visual)) =
            (window_graphics.get_target(), visual_graphics.visual())
        {
            let entity_name = format_entity_name(entity, name);
            trace!(
                frame = frame_count.0,
                entity = %entity_name,
                "SetRoot executing"
            );
            // 係には Err を返す先が無いので記録だけ（立ち直りは装置の失いの係が担う）。
            if let Err(e) = target.SetRoot(visual) {
                error!(entity = %entity_name, error = ?e, "SetRoot failed");
            }
        }
    }
}
