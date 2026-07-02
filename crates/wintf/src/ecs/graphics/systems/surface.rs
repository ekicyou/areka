use super::init::{calculate_surface_size_from_global_arrangement, format_entity_name};
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::ecs::graphics::{
    GraphicsCommandList, GraphicsCore, SurfaceCreationStats, SurfaceGraphics, SurfaceGraphicsDirty,
    VisualGraphics,
};
use crate::ecs::layout::GlobalArrangement;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace, warn};
use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use windows::UI::Composition::{CompositionBrush, SpriteVisual};
use windows::core::Interface;

// ========== Layout-to-Graphics Synchronization Systems ==========

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
    wuc_resource: Option<Res<WucGraphicsResource>>,
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

    let (compositor, graphics_device) = match wuc_resource.as_ref() {
        Some(r) => match (r.compositor(), r.graphics_device()) {
            (Some(c), Some(gd)) => (c, gd),
            _ => return,
        },
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

        // Surface作成（B8G8R8A8・Premultiplied・要件 6.4。DComp の
        // DXGI_FORMAT_B8G8R8A8_UNORM + DXGI_ALPHA_MODE_PREMULTIPLIED と等価）。
        let surface_res = graphics_device.CreateDrawingSurface(
            Size {
                Width: width as f32,
                Height: height as f32,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        );

        match surface_res {
            Ok(surface) => {
                // WUC: surface を SurfaceBrush で束ね、SpriteVisual.SetBrush で適用する
                // （DComp の visual.SetContent(surface) 一段に対し、brush が一段挟まる・要件 6.3）。
                let brush = match compositor.CreateSurfaceBrushWithSurface(&surface) {
                    Ok(b) => b,
                    Err(e) => {
                        error!(
                            entity = %entity_name,
                            error = ?e,
                            "[deferred_surface_creation] CreateSurfaceBrushWithSurface FAILED"
                        );
                        continue;
                    }
                };

                // VisualにBrushを設定（SpriteVisual へ cast して SetBrush）
                if let Some(visual) = visual_graphics.visual() {
                    trace!(
                        entity = %entity_name,
                        "[deferred_surface_creation] SetBrush calling"
                    );
                    match visual.cast::<SpriteVisual>() {
                        Ok(sprite) => {
                            // WUC 固有: SpriteVisual は自身の Size 内にのみ brush を描画する
                            // （DComp の SetContent(surface) は surface の自然サイズで描画するため
                            // Visual サイズ設定が不要だった）。live パイプラインは Visual に Size を
                            // 設定しないため、surface と同一の物理サイズを SpriteVisual に設定して
                            // 空描画を防ぎ、DComp の SetContent と等価な描画範囲を保つ（要件 6.3/8.2）。
                            if let Err(e) = sprite.SetSize(windows_numerics::Vector2 {
                                X: width as f32,
                                Y: height as f32,
                            }) {
                                error!(
                                    entity = %entity_name,
                                    error = ?e,
                                    "[deferred_surface_creation] SetSize FAILED"
                                );
                            }
                            match sprite.SetBrush(&brush) {
                                Ok(_) => trace!(
                                    entity = %entity_name,
                                    "[deferred_surface_creation] SetBrush SUCCESS"
                                ),
                                Err(e) => error!(
                                    entity = %entity_name,
                                    error = ?e,
                                    "[deferred_surface_creation] SetBrush FAILED"
                                ),
                            }
                        }
                        Err(e) => error!(
                            entity = %entity_name,
                            error = ?e,
                            "[deferred_surface_creation] cast to SpriteVisual FAILED"
                        ),
                    }
                } else {
                    warn!(
                        entity = %entity_name,
                        "[deferred_surface_creation] NO VISUAL! SetBrush skipped"
                    );
                }

                // 直接更新（commands.insert()ではなく）。surface と brush を保持（brush 寿命固定）。
                surface_graphics.set_surface(surface, brush, (width, height));
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

            // Visual の Brush をクリア（Req 1.3）。DComp の SetContent(None) に対応。
            if let Some(visual) = visual_graphics.visual() {
                if let Ok(sprite) = visual.cast::<SpriteVisual>() {
                    // None を設定して Brush を解除
                    let _ = sprite.SetBrush(None::<&CompositionBrush>);
                }
            }

            // SurfaceGraphics（surface + brush）をクリア（コンポーネント自体は残す）
            surface_graphics.clear();

            stats.record_deleted();

            debug!(
                entity = %entity_name,
                "[cleanup_surface_on_commandlist_removed] SurfaceGraphics cleared"
            );
        }
    }
}
