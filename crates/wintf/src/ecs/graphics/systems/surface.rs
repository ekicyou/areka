use super::init::{calculate_surface_size_from_global_arrangement, format_entity_name};
use crate::com::dcomp::{DCompositionDeviceExt, DCompositionVisualExt};
use crate::ecs::graphics::{
    DCompGraphicsResource, GraphicsCommandList, GraphicsCore, SurfaceCreationStats,
    SurfaceGraphics, SurfaceGraphicsDirty, VisualGraphics,
};
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace, warn};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dxgi::Common::*;

/// Surfaceを作成してVisualに設定する
fn create_surface_for_visual(
    dcomp_resource: &DCompGraphicsResource,
    visual: &VisualGraphics,
    width: u32,
    height: u32,
) -> windows::core::Result<SurfaceGraphics> {
    if !dcomp_resource.is_valid() {
        return Err(windows::core::Error::from(E_FAIL));
    }

    // 1. IDCompositionSurface作成
    let dcomp = dcomp_resource
        .dcomp()
        .ok_or(windows::core::Error::from(E_FAIL))?;
    let surface = dcomp.create_surface(
        width,
        height,
        DXGI_FORMAT_B8G8R8A8_UNORM,
        DXGI_ALPHA_MODE_PREMULTIPLIED,
    )?;

    // 2. VisualにSurfaceを設定
    let visual_ref = visual.visual().ok_or(windows::core::Error::from(E_FAIL))?;
    visual_ref.set_content(&surface)?;

    Ok(SurfaceGraphics::new(surface, (width, height)))
}

// ========== Layout-to-Graphics Synchronization Systems ==========

/// Arrangement変更時にDirectComposition Surfaceを作成/リサイズ
/// ArrangementをSingle Source of Truthとして直接参照
///
/// # Deprecated
/// この関数は `deferred_surface_creation_system` に置き換えられました。
/// `deferred_surface_creation_system` は `GlobalArrangement.bounds` から
/// 物理ピクセルサイズを計算し、GraphicsCommandListの存在を条件として
/// Surface生成を行います。
///
/// Req 2.1: sync_surface_from_arrangement廃止
#[deprecated(
    since = "0.1.0",
    note = "Use deferred_surface_creation_system instead. This function will be removed in a future version."
)]
#[allow(dead_code)]
pub fn sync_surface_from_arrangement(
    graphics: Option<Res<GraphicsCore>>,
    dcomp_resource: Option<Res<DCompGraphicsResource>>,
    mut query: Query<
        (
            Entity,
            &VisualGraphics,
            &Arrangement,
            Option<&mut SurfaceGraphics>,
            Option<&Name>,
        ),
        Changed<Arrangement>,
    >,
    mut commands: Commands,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    let Some(graphics) = graphics else {
        return;
    };

    if !graphics.is_valid() {
        return;
    }

    let Some(ref dcomp_res) = dcomp_resource else {
        return;
    };

    let mut _processed_count = 0;

    for (entity, visual_graphics, arrangement, surface, name) in query.iter_mut() {
        let entity_name = format_entity_name(entity, name);
        if !visual_graphics.is_valid() {
            warn!(
                frame = frame_count.0,
                entity = %entity_name,
                "[sync_surface_from_arrangement] Entity has invalid VisualGraphics, skipping"
            );
            continue;
        }

        let width = arrangement.size.width as u32;
        let height = arrangement.size.height as u32;

        // サイズが0の場合はスキップ（まだレイアウトされていない）
        if width == 0 || height == 0 {
            trace!(
                frame = frame_count.0,
                entity = %entity_name,
                width = width,
                height = height,
                "[sync_surface_from_arrangement] Entity has zero size, skipping"
            );
            continue;
        }

        _processed_count += 1;
        trace!(
            frame = frame_count.0,
            entity = %entity_name,
            width = width,
            height = height,
            has_surface = surface.is_some(),
            "[sync_surface_from_arrangement] Processing entity"
        );

        match surface {
            Some(mut surf) => {
                // サイズ不一致の場合のみ再作成
                if surf.size != (width, height) {
                    trace!(
                        frame = frame_count.0,
                        entity = %entity_name,
                        old_size = ?surf.size,
                        new_width = width,
                        new_height = height,
                        "[sync_surface_from_arrangement] Entity resizing, calling SetContent"
                    );
                    match create_surface_for_visual(dcomp_res, visual_graphics, width, height) {
                        Ok(new_surface) => {
                            trace!(
                                frame = frame_count.0,
                                entity = %entity_name,
                                "[sync_surface_from_arrangement] Entity Surface resized successfully"
                            );
                            commands.entity(entity).insert(new_surface);
                            // SurfaceGraphicsDirtyも挿入して描画をトリガー
                            commands
                                .entity(entity)
                                .insert(SurfaceGraphicsDirty::default());
                        }
                        Err(e) => {
                            error!(error = ?e, "[sync_surface_from_arrangement] Error resizing surface");
                            surf.invalidate();
                        }
                    }
                }
            }
            None => {
                // Surfaceがまだ作成されていない場合は作成
                trace!(
                    frame = frame_count.0,
                    entity = %entity_name,
                    width = width,
                    height = height,
                    "[sync_surface_from_arrangement] Entity creating new Surface, calling SetContent"
                );
                match create_surface_for_visual(dcomp_res, visual_graphics, width, height) {
                    Ok(new_surface) => {
                        trace!(
                            frame = frame_count.0,
                            entity = %entity_name,
                            "[sync_surface_from_arrangement] Entity Surface created successfully"
                        );
                        commands.entity(entity).insert(new_surface);
                        // SurfaceGraphicsDirtyも挿入して描画をトリガー
                        commands
                            .entity(entity)
                            .insert(SurfaceGraphicsDirty::default());
                    }
                    Err(e) => {
                        error!(error = ?e, "[sync_surface_from_arrangement] Error creating new surface");
                    }
                }
            }
        }
    }
}

/// 変更検知システム：描画内容に変更があった場合、SurfaceGraphicsDirtyを更新する
///
/// Phase 4以降の自己描画方式では、各EntityがSurfaceを持ち、自分自身のみを描画するため、
/// 親をたどる必要はない。変更があったEntity自身のSurfaceGraphicsDirtyを更新する。
///
/// 検知対象:
/// - GraphicsCommandList: 描画コマンドの変更
/// - SurfaceGraphics: Surfaceの再作成（Added含む）
/// - GlobalArrangement: スケール成分の変更（DPIスケール対応）
///
/// Changed<SurfaceGraphicsDirty>を検出することで、render_surfaceが描画を実行する。
/// SurfaceUpdateRequestedマーカーは廃止され、フレーム番号更新方式に置き換えられた。
pub fn mark_dirty_surfaces(
    mut changed_query: Query<
        (Entity, &mut SurfaceGraphicsDirty, Option<&Name>),
        (
            Or<(
                Changed<GraphicsCommandList>,
                Changed<SurfaceGraphics>,
                Added<SurfaceGraphics>,
                Changed<GlobalArrangement>,
            )>,
            With<SurfaceGraphics>,
        ),
    >,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    let mut count = 0;
    for (entity, mut dirty, name) in changed_query.iter_mut() {
        dirty.requested_frame = frame_count.0 as u64;
        count += 1;
        // 正常パスのログは抑制（毎フレーム出力されるため）
        let _entity_name = format_entity_name(entity, name);
        // eprintln!("[Frame {}] [mark_dirty_surfaces] Entity={} marked dirty", frame_count.0, _entity_name);
    }
    if count > 0 {
        // 正常パスのログは抑制（毎フレーム出力されるため）
        // eprintln!("[Frame {}] [mark_dirty_surfaces] Total {} entities marked dirty", frame_count.0, count);
    }
}

/// 遅延Surface作成システム (最適化版)
///
/// GraphicsCommandListが存在するEntityに対してSurfaceを条件付きで作成する。
/// - 対象: VisualGraphics + GraphicsCommandList を持つEntity
/// - サイズ: GlobalArrangement.boundsから物理ピクセルサイズを計算
/// - 条件: サイズが有効（幅・高さが1以上）な場合のみ作成
/// - サイズ変更: 既存Surfaceとサイズ不一致の場合は再作成
///
/// # Requirements
/// - Req 1.1: CommandList追加時にSurface作成
/// - Req 1.2: CommandListなしならスキップ（クエリ条件で実現）
/// - Req 2.2: deferred_surface_creation唯一化
/// - Req 2.3: トリガーをCommandList存在のみに
/// - Req 3.1: GlobalArrangement.boundsから計算
/// - Req 3.2: スケール適用後のサイズ（物理ピクセル）
/// - Req 3.3: サイズ0の場合はスキップ
/// - Req 3.4: サイズ変更時にSurface再作成
/// - Req 5.1: スキップ理由ログ
/// - Req 5.2: 作成ログ（物理サイズ）
///
/// Note: SurfaceGraphicsは visual_resource_management_system で事前に空で配置されているため、
/// ここでは直接更新（set_surface）する。commands.insert() は使用しない。
pub fn deferred_surface_creation_system(
    graphics: Res<GraphicsCore>,
    dcomp_resource: Option<Res<DCompGraphicsResource>>,
    // 統合クエリ: SurfaceGraphicsを持ち、GlobalArrangementまたはGraphicsCommandListが変更されたEntity
    // SurfaceGraphicsは事前配置されている前提
    mut query: Query<
        (
            Entity,
            &VisualGraphics,
            &GraphicsCommandList,
            &GlobalArrangement,
            &mut SurfaceGraphics,
            &mut SurfaceGraphicsDirty,
            Option<&Name>,
        ),
        Or<(Changed<GlobalArrangement>, Changed<GraphicsCommandList>)>,
    >,
    mut stats: ResMut<SurfaceCreationStats>,
) {
    if !graphics.is_valid() {
        return;
    }

    let dcomp = match dcomp_resource.as_ref().and_then(|r| r.dcomp()) {
        Some(d) => d,
        None => return,
    };

    for (
        entity,
        visual_graphics,
        _cmd_list,
        global_arrangement,
        mut surface_graphics,
        mut dirty,
        name,
    ) in query.iter_mut()
    {
        let entity_name = format_entity_name(entity, name);

        // GlobalArrangementからサイズを計算
        let Some((width, height)) =
            calculate_surface_size_from_global_arrangement(global_arrangement)
        else {
            // Req 5.1: スキップ理由ログ
            trace!(
                entity = %entity_name,
                bounds = ?global_arrangement.bounds,
                "[deferred_surface_creation] Entity skipped: invalid size from GlobalArrangement"
            );
            stats.record_skipped();
            continue;
        };

        // サイズが同じなら何もしない（既にSurfaceが有効な場合）
        if surface_graphics.is_valid() && surface_graphics.size == (width, height) {
            continue;
        }

        // 新規作成かリサイズかを判定
        let is_new = !surface_graphics.is_valid();

        if is_new {
            debug!(
                entity = %entity_name,
                width = width,
                height = height,
                "[deferred_surface_creation] Creating Surface"
            );
        } else {
            debug!(
                entity = %entity_name,
                old_size = ?surface_graphics.size,
                new_width = width,
                new_height = height,
                "[deferred_surface_creation] Resizing Surface"
            );
        }

        // Surface作成
        let surface_res = dcomp.create_surface(
            width,
            height,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        );

        match surface_res {
            Ok(surface) => {
                // VisualにSurfaceを設定
                if let Some(visual) = visual_graphics.visual() {
                    trace!(
                        entity = %entity_name,
                        "[deferred_surface_creation] SetContent calling"
                    );
                    unsafe {
                        match visual.SetContent(&surface) {
                            Ok(_) => trace!(
                                entity = %entity_name,
                                "[deferred_surface_creation] SetContent SUCCESS"
                            ),
                            Err(e) => error!(
                                entity = %entity_name,
                                error = ?e,
                                "[deferred_surface_creation] SetContent FAILED"
                            ),
                        }
                    }
                } else {
                    warn!(
                        entity = %entity_name,
                        "[deferred_surface_creation] NO VISUAL! SetContent skipped"
                    );
                }

                // 直接更新（commands.insert()ではなく）
                surface_graphics.set_surface(surface, (width, height));
                // SurfaceGraphicsDirtyのChangedをトリガー
                dirty.requested_frame = dirty.requested_frame.wrapping_add(1);

                if is_new {
                    stats.record_created();
                    debug!(
                        entity = %entity_name,
                        "[deferred_surface_creation] Surface created successfully"
                    );
                } else {
                    stats.record_resized();
                    debug!(
                        entity = %entity_name,
                        "[deferred_surface_creation] Surface resized successfully"
                    );
                }
            }
            Err(e) => {
                error!(
                    entity = %entity_name,
                    error = ?e,
                    "[deferred_surface_creation] Failed to create/resize surface"
                );
            }
        }
    }
}

/// GraphicsCommandList削除時のSurface解放システム
///
/// GraphicsCommandListが削除されたEntityからSurfaceGraphicsをクリアする。
/// VisualGraphicsはVisual階層を維持するため削除しない。
/// SurfaceGraphicsコンポーネント自体は残し、内容をinvalidate()する。
///
/// # Requirements
/// - Req 1.3: CommandList削除時にSurface解放
/// - Req 1.4: 専用クリーンアップシステム
///
/// Note: commands.remove()ではなくinvalidate()を使用し、
/// コンポーネントの存在自体は維持する（事前配置パターン）。
pub fn cleanup_surface_on_commandlist_removed(
    mut removed: RemovedComponents<GraphicsCommandList>,
    mut query: Query<(Entity, &VisualGraphics, &mut SurfaceGraphics, Option<&Name>)>,
    mut stats: ResMut<SurfaceCreationStats>,
) {
    for entity in removed.read() {
        // SurfaceGraphicsを持つEntityのみ処理
        if let Ok((entity, visual_graphics, mut surface_graphics, name)) = query.get_mut(entity) {
            // 既にinvalidな場合はスキップ
            if !surface_graphics.is_valid() {
                continue;
            }

            let entity_name = format_entity_name(entity, name);

            debug!(
                entity = %entity_name,
                "[cleanup_surface_on_commandlist_removed] Clearing SurfaceGraphics"
            );

            // VisualのContentをクリア（Req 1.3）
            if let Some(visual) = visual_graphics.visual() {
                unsafe {
                    // nullptrを設定してSurfaceを解除
                    let _ = visual.SetContent(None);
                }
            }

            // SurfaceGraphicsをクリア（コンポーネント自体は残す）
            surface_graphics.clear();

            stats.record_deleted();

            debug!(
                entity = %entity_name,
                "[cleanup_surface_on_commandlist_removed] SurfaceGraphics cleared"
            );
        }
    }
}
