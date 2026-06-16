use super::init::format_entity_name;
use crate::com::d2d::D2D1DeviceContextExt;
use crate::com::dcomp::{DCompositionDeviceExt, DCompositionSurfaceExt};
use crate::ecs::graphics::{
    DCompGraphicsResource, GraphicsCommandList, GraphicsCore, SurfaceGraphics, SurfaceGraphicsDirty,
};
use crate::ecs::layout::GlobalArrangement;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::error;

// ========== 描画システム ==========

/// Surfaceへの自己描画（Phase 4: 自己描画方式）
///
/// 各Entityが自分のSurfaceに自分のGraphicsCommandListのみを描画する。
/// draw_recursive方式を廃止し、DirectCompositionのVisual階層で合成を行う。
///
/// - Changed<SurfaceGraphicsDirty>を持つEntityが対象（マーカー方式から移行）
/// - 自分のGraphicsCommandListのみを描画（子は描画しない）
/// - 子の描画は各子が自分のSurfaceで行う
/// - GlobalArrangementのスケール成分を適用（DPIスケール対応）
pub fn render_surface(
    surfaces: Query<
        (
            Entity,
            &SurfaceGraphics,
            &GlobalArrangement,
            Option<&GraphicsCommandList>,
            Option<&Name>,
        ),
        Changed<SurfaceGraphicsDirty>,
    >,
    _graphics_core: Option<Res<GraphicsCore>>,
    _frame_count: Res<crate::ecs::world::FrameCount>,
) {
    use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
    use windows::Win32::Graphics::Direct2D::Common::D2D1_COMPOSITE_MODE_SOURCE_OVER;
    use windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_LINEAR;

    for (entity, surface, global_arrangement, cmd_list_opt, name) in surfaces.iter() {
        let entity_name = format_entity_name(entity, name);
        // 正常パスのログは抑制（毎フレーム出力されるため）
        // eprintln!("[Frame {}] [render_surface] === Self-rendering Entity={} ===", _frame_count.0, entity_name);

        if !surface.is_valid() {
            // Surface未初期化時のみログ出力
            // eprintln!("[render_surface] Surface invalid for Entity={}, skipping", entity_name);
            continue;
        }

        // Surface描画開始
        let surface_ref = match surface.surface() {
            Some(s) => s,
            None => continue,
        };

        let (dc, offset) = match surface_ref.begin_draw(None) {
            Ok(result) => {
                // 正常パスのログは抑制
                // eprintln!("[render_surface] BeginDraw succeeded for Entity={}, offset=({}, {})", entity_name, result.1.x, result.1.y);
                result
            }
            Err(err) => {
                error!(
                    entity = %entity_name,
                    error = ?err,
                    hresult = format_args!("0x{:08X}", err.code().0),
                    "BeginDraw failed"
                );
                continue;
            }
        };

        // BeginDrawのoffsetとGlobalArrangementのスケールを適用
        // - offset: Surface内の描画開始位置（物理ピクセル単位）- DirectCompositionがSurface Atlasを使用している場合に非ゼロ
        // - scale: GlobalArrangementから抽出したスケール成分（DPIスケール含む）
        // - Visual.SetOffsetX/Yで位置は設定済みなので、ここでは描画開始位置とスケールのみ
        unsafe {
            // GlobalArrangementからスケール成分のみを抽出
            let scale_x = global_arrangement.scale_x();
            let scale_y = global_arrangement.scale_y();

            // 変換順序: まずスケール、次にoffset平行移動
            // これによりスケール後の座標系でoffset位置に描画される
            let transform = windows_numerics::Matrix3x2 {
                M11: scale_x,
                M12: 0.0,
                M21: 0.0,
                M22: scale_y,
                M31: offset.x as f32,
                M32: offset.y as f32,
            };

            dc.SetTransform(&transform);

            // 正常パスのログは抑制
            // eprintln!("[render_surface] Transform for Entity={}: scale=({}, {}), offset=({}, {})", entity_name, scale_x, scale_y, offset.x, offset.y);
        }

        // 透明色クリア
        dc.clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        // 自己描画: 自分のGraphicsCommandListのみを描画
        // 子の描画は行わない（各子が自分のSurfaceで行う）
        if let Some(cmd_list) = cmd_list_opt {
            if let Some(command_list) = cmd_list.command_list() {
                // 正常パスのログは抑制
                // eprintln!("[render_surface] Drawing own CommandList for Entity={}", entity_name);
                unsafe {
                    dc.DrawImage(
                        command_list,
                        None,
                        None,
                        D2D1_INTERPOLATION_MODE_LINEAR,
                        D2D1_COMPOSITE_MODE_SOURCE_OVER,
                    );
                }
            }
        }

        // EndDraw
        if let Err(e) = surface_ref.end_draw() {
            error!(error = ?e, "EndDraw failed");
        }

        // Note: Changed<SurfaceGraphicsDirty> は自動リセットされるため、
        // マーカー削除は不要（旧SurfaceUpdateRequestedパターンから移行済み）
    }
}

/// DirectCompositionのすべての変更を確定する
pub fn commit_composition(
    dcomp_resource: Option<Res<DCompGraphicsResource>>,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    let Some(dcomp_resource) = dcomp_resource else {
        // DCompGraphicsResource 未初期化（DComp ウィンドウなし）: 正常スキップ
        return;
    };

    if !dcomp_resource.is_valid() {
        // DComp デバイス無効化中: 次フレームで再初期化予定
        return;
    }

    let dcomp = match dcomp_resource.dcomp() {
        Some(d) => d,
        None => {
            return;
        }
    };

    match dcomp.commit() {
        Ok(()) => {
            // 正常時はログ出力を抑制（毎フレーム出力されるため）
            // デバッグ時はコメントを外して有効化
            // eprintln!("[Frame {}] [commit_composition] Commit成功", frame_count.0);
        }
        Err(e) => {
            error!(
                frame = frame_count.0,
                error = ?e,
                hresult = format!("0x{:08X}", e.code().0),
                "[commit_composition] Commit failed"
            );
        }
    }
}
