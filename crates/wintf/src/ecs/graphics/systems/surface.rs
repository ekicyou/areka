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

/// 描き直しの印の係が読む部品
type MarkDirtyData = (
    &'static mut SurfaceGraphicsDirty,
    Ref<'static, SurfaceGraphics>,
    Option<Ref<'static, GraphicsCommandList>>,
    Option<&'static GlobalArrangement>,
);

/// 描き直しの印の係が見る変化（中身・描画面・配置）
type MarkDirtyFilter = Or<(
    Changed<GraphicsCommandList>,
    Changed<SurfaceGraphics>,
    Changed<GlobalArrangement>,
)>;

/// 描画面を作る係が読む部品
type SurfaceCreationData = (
    Entity,
    &'static VisualGraphics,
    &'static GlobalArrangement,
    &'static mut SurfaceGraphics,
    &'static mut SurfaceGraphicsDirty,
    Option<&'static Name>,
);

/// 描画面を作る係が見る変化（配置・中身・見た目の作り直し）
type SurfaceCreationFilter = (
    With<GraphicsCommandList>,
    Or<(
        Changed<GlobalArrangement>,
        Changed<GraphicsCommandList>,
        Changed<VisualGraphics>,
    )>,
);

/// 変更検知システム：描画内容に変更があった場合、SurfaceGraphicsDirtyを更新する
///
/// Phase 4以降の自己描画方式では、各EntityがSurfaceを持ち、自分自身のみを描画するため、
/// 親をたどる必要はない。変更があったEntity自身のSurfaceGraphicsDirtyを更新する。
///
/// 検知対象:
/// - GraphicsCommandList: 描画コマンドの変更
/// - SurfaceGraphics: Surfaceの再作成（Added含む）
/// - GlobalArrangement: スケール成分の変更（DPIスケール対応）。位置だけの変化では描き直さない
///   （描く中身は拡大率にしか依らない。大きさの変化は描画面の作り直しが `Changed<SurfaceGraphics>` で拾う）
///
/// Changed<SurfaceGraphicsDirty>を検出することで、render_surfaceが描画を実行する。
/// 印を立てない部品は `SurfaceGraphicsDirty` を書き換えない（変化の印を立てない）。
pub fn mark_dirty_surfaces(
    mut changed_query: Query<MarkDirtyData, MarkDirtyFilter>,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    for (mut dirty, surface, cmd_list, arrangement) in changed_query.iter_mut() {
        let content_changed = surface.is_changed() || cmd_list.is_some_and(|c| c.is_changed());
        let scale = arrangement.map(GlobalArrangement::scale);
        if !content_changed && scale.is_none_or(|s| s == dirty.drawn_scale) {
            continue;
        }
        dirty.requested_frame = frame_count.0 as u64;
        if let Some(scale) = scale {
            dirty.drawn_scale = scale;
        }
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
/// Note: SurfaceGraphicsは Visual の on_add（と後から窓へつなぐ係）で事前に空で配置されているため、
/// ここでは直接更新（set_surface）する。commands.insert() は使用しない。
///
/// 見た目が作り直された部品（`Changed<VisualGraphics>`）も対象にする。立ち直りでは描画面も
/// 空になっているので、大きさが同じでも作り直して新しい見た目に筆を付ける。
///
/// 本番では装置と WUC の資源が使える回だけ回す（`wuc_ready`・使えない回に印を食べない）。
/// 装置か WUC の資源が使えなければ戻るのは、使う人が係を直に登録したときの守り。
pub fn deferred_surface_creation_system(
    graphics: Option<Res<GraphicsCore>>,
    wuc_resource: Option<Res<WucGraphicsResource>>,
    // SurfaceGraphicsは事前配置されている前提
    mut query: Query<SurfaceCreationData, SurfaceCreationFilter>,
    mut stats: ResMut<SurfaceCreationStats>,
) {
    if !graphics.is_some_and(|g| g.is_valid()) {
        return;
    }

    let (compositor, graphics_device) = match wuc_resource.as_ref() {
        Some(r) => match (r.compositor(), r.graphics_device()) {
            (Some(c), Some(gd)) => (c, gd),
            _ => return,
        },
        None => return,
    };

    for (entity, visual_graphics, global_arrangement, mut surface_graphics, mut dirty, name) in
        query.iter_mut()
    {
        // GlobalArrangementからサイズを計算
        let Some((width, height)) =
            calculate_surface_size_from_global_arrangement(global_arrangement)
        else {
            // Req 5.1: スキップ理由ログ
            trace!(
                entity = %format_entity_name(entity, name),
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
        // 名前は作る・作り直す部品の分だけ作る（位置が動くたびに全部の部品が通る道）
        let entity_name = format_entity_name(entity, name);

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
/// SurfaceGraphicsコンポーネント自体は残し、内容をclear()する。
///
/// 命令の列がまだ無い部品だけを片づける（`Without<GraphicsCommandList>`）。立ち直りで外した
/// 命令の列が同じ回に付き直り、作り直したばかりの描画面を空にしないため。
///
/// # Requirements
/// - Req 1.3: CommandList削除時にSurface解放
/// - Req 1.4: 専用クリーンアップシステム
///
/// Note: commands.remove()ではなくclear()を使用し、
/// コンポーネントの存在自体は維持する（事前配置パターン）。
pub fn cleanup_surface_on_commandlist_removed(
    mut removed: RemovedComponents<GraphicsCommandList>,
    mut query: Query<
        (Entity, &VisualGraphics, &mut SurfaceGraphics, Option<&Name>),
        Without<GraphicsCommandList>,
    >,
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
            // 係には Err を返す先が無いので、失敗は記録だけ。
            if let Some(visual) = visual_graphics.visual()
                && let Ok(sprite) = visual.cast::<SpriteVisual>()
                && let Err(e) = sprite.SetBrush(None::<&CompositionBrush>)
            {
                error!(
                    entity = %entity_name,
                    error = ?e,
                    "[cleanup_surface_on_commandlist_removed] SetBrush(None) FAILED"
                );
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
