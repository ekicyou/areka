use super::init::format_entity_name;
use crate::com::d2d::D2D1DeviceContextExt;
use crate::com::wuc::DrawingSurfaceInteropExt;
use crate::ecs::graphics::visual::emit;
use crate::ecs::graphics::{
    DrawTree, GraphicsCommandList, GraphicsCore, SurfaceGraphics, SurfaceGraphicsDirty,
};
use crate::ecs::layout::GlobalArrangement;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::error;
use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext6;
use windows::Win32::System::WinRT::Composition::ICompositionDrawingSurfaceInterop;
use windows::core::Interface;

// ========== 描画システム ==========

/// 描画の係が読む部品
type RenderSurfaceData = (
    Entity,
    &'static SurfaceGraphics,
    &'static GlobalArrangement,
    Option<&'static GraphicsCommandList>,
    Option<&'static DrawTree>,
    Option<&'static Name>,
);

/// Surfaceへの自己描画（Phase 4: 自己描画方式）
///
/// 各Entityが自分のSurfaceに自分のGraphicsCommandListを描画し、根の木（DrawTree）が在れば
/// その直後に木の列を 1 度再生する（木が無ければ命令の列だけ＝今までと同じ画素）。
/// 子の描画は各子が自分のSurfaceで行い、WUC の見た目（Visual）の木で合成する。
///
/// - Changed<SurfaceGraphicsDirty>を持つEntityが対象
/// - GlobalArrangementのスケール成分を適用（DPIスケール対応）
/// - 装置が無い・無効なら何も描かずに戻る（描画面は作り直しで全部作り直される）
/// - `GraphicsCore` を可変で触るのは装置の失いのときだけ（ふだんの描画で変化の印を立てない）
/// - 失いでない BeginDraw の失敗は、その面の描き直しの印を命令で立て直し、次の回で描き直す
pub fn render_surface(
    surfaces: Query<RenderSurfaceData, Changed<SurfaceGraphicsDirty>>,
    graphics_core: Option<ResMut<GraphicsCore>>,
    mut commands: Commands,
) {
    use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
    use windows::Win32::Graphics::Direct2D::Common::D2D1_COMPOSITE_MODE_SOURCE_OVER;
    use windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_LINEAR;

    let Some(mut graphics_core) = graphics_core else {
        return;
    };
    if !graphics_core.is_valid() {
        return;
    }

    for (entity, surface, global_arrangement, cmd_list_opt, tree, name) in surfaces.iter() {
        let Some(surface_ref) = surface.surface() else {
            continue;
        };

        // WUC: CompositionDrawingSurface を ICompositionDrawingSurfaceInterop へ cast して
        // BeginDraw（必ず None＝全面。Some(部分矩形) は WUC で E_INVALIDARG になる・task 1.2 実測）。
        let surface_interop = match surface_ref.cast::<ICompositionDrawingSurfaceInterop>() {
            Ok(si) => si,
            Err(err) => {
                error!(
                    entity = %format_entity_name(entity, name),
                    error = ?err,
                    "cast to ICompositionDrawingSurfaceInterop failed"
                );
                continue;
            }
        };

        let (dc, offset) = match surface_interop.begin_draw(None) {
            Ok(result) => result,
            Err(err) => {
                // 失いの記録と無効化は note_draw_failure が残す。残りの描画面も同じ装置なので描かない。
                // 変化の印は失いのときだけ立てる（失いでない失敗で下流に控えを捨てさせない）。
                if graphics_core
                    .bypass_change_detection()
                    .note_draw_failure(&err)
                {
                    graphics_core.set_changed();
                    return;
                }
                error!(
                    entity = %format_entity_name(entity, name),
                    error = ?err,
                    hresult = format_args!("0x{:08X}", err.code().0),
                    "BeginDraw failed"
                );
                // 次の回で描き直す（41 番）。係が自分で立てた変化の印は次の回の自分に見えないので
                // 命令で立てる。部品ごと消えていれば描き直しも要らない。
                commands
                    .entity(entity)
                    .queue_silenced(|mut e: EntityWorldMut| {
                        if let Some(mut dirty) = e.get_mut::<SurfaceGraphicsDirty>() {
                            dirty.set_changed();
                        }
                    });
                crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::GRAPHICS);
                continue;
            }
        };

        // 変換順序: まずスケール（GlobalArrangement のスケール成分・DPI 含む）、次に BeginDraw の
        // offset（Surface Atlas 内の描画開始位置・物理ピクセル）。位置は Visual のオフセットで設定済み。
        let transform = windows_numerics::Matrix3x2 {
            M11: global_arrangement.scale_x(),
            M12: 0.0,
            M21: 0.0,
            M22: global_arrangement.scale_y(),
            M31: offset.x as f32,
            M32: offset.y as f32,
        };
        unsafe {
            dc.SetTransform(&transform);
        }

        // 透明色クリア
        dc.clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        // 自己描画: 自分のGraphicsCommandListのみを描画
        if let Some(command_list) = cmd_list_opt.and_then(|c| c.command_list()) {
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
        // 木の節は根の後に同じ描画面へ（変換は上の拡大＋offset を根に）。失敗は EndDraw に出る。
        if let Some(tree) = tree {
            let dc6 = dc.cast::<ID2D1DeviceContext6>().ok();
            emit(&dc, dc6.as_ref(), transform, &tree.commands);
        }

        if let Err(e) = surface_interop.end_draw() {
            error!(entity = %format_entity_name(entity, name), error = ?e, "EndDraw failed");
        }
    }
}

// Note: DComp の `commit_composition` システムは WUC 移行で削除された（要件 7.1）。
// WUC は DispatcherQueue 経由で変更が暗黙反映されるため明示的な commit は不要。
// `CommitComposition` schedule 自体は空スケジュール（no-op）として残置される
// （ULW present system は wintf-ulw-removal で撤去済み）。
