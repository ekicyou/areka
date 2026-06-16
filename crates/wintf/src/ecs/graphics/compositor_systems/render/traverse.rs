//! 合成描画ツリーの再帰走査とダーティ判定
//!
//! - `CompositeContext`: 親→子へ伝搬する描画コンテキスト
//! - `draw_with_opacity`: opacity を適用した GraphicsCommandList 描画
//! - `render_subtree`: エンティティとサブツリーの再帰合成描画
//! - `is_window_dirty`: ウィンドウサブツリー内の変更検出

use super::guards::ClipGuard;
use crate::ecs::graphics::{GraphicsCommandList, Visual};
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::Matrix3x2;

/// 合成描画ツリー走査時に親→子へ伝搬する描画コンテキスト
pub(super) struct CompositeContext<'a> {
    pub(super) dc: &'a ID2D1DeviceContext,
    pub(super) accumulated_opacity: f32,
    /// ULW ビットマップはウィンドウクライアント領域の (0,0) から始まるため、
    /// GlobalArrangement のスクリーン座標からウィンドウ位置分を差し引く補正オフセット。
    pub(super) window_offset: (f32, f32),
}

/// opacity を適用して GraphicsCommandList を描画する。
///
/// opacity == 1.0 の場合は Effect を介さず直接描画で最適化。
unsafe fn draw_with_opacity(
    dc: &ID2D1DeviceContext,
    command_list: &ID2D1CommandList,
    opacity: f32,
) -> windows::core::Result<()> {
    if (opacity - 1.0).abs() < f32::EPSILON {
        // opacity == 1.0: 直接描画（Effect 不要）
        unsafe {
            dc.DrawImage(
                command_list,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            );
        }
    } else {
        // opacity < 1.0: ColorMatrix Effect で alpha 乗算
        let effect = unsafe { dc.CreateEffect(&CLSID_D2D1ColorMatrix)? };
        unsafe { effect.SetInput(0, command_list, true) };

        // D2D_MATRIX_5X4_F: alpha チャネルに opacity を乗算
        // [3][3] = opacity、他は単位行列
        let mut matrix: D2D_MATRIX_5X4_F = unsafe { std::mem::zeroed() };
        matrix.Anonymous.Anonymous._11 = 1.0;
        matrix.Anonymous.Anonymous._22 = 1.0;
        matrix.Anonymous.Anonymous._33 = 1.0;
        matrix.Anonymous.Anonymous._44 = opacity;

        // D2D1_COLORMATRIX_PROP_COLOR_MATRIX = 0
        // SAFETY: matrix はスタック上の有効な D2D_MATRIX_5X4_F であり、
        // from_raw_parts の長さはちょうど size_of::<D2D_MATRIX_5X4_F>() バイト
        // （参照元の領域を超えない）。SetValue はバイト列を読み取りコピーするのみで
        // スライスを保持しない。
        unsafe {
            effect.SetValue(
                0,
                D2D1_PROPERTY_TYPE_UNKNOWN,
                std::slice::from_raw_parts(
                    &matrix as *const D2D_MATRIX_5X4_F as *const u8,
                    std::mem::size_of::<D2D_MATRIX_5X4_F>(),
                ),
            )?;
        }

        let output = unsafe { effect.GetOutput()? };
        unsafe {
            dc.DrawImage(
                &output,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            );
        }
    }
    Ok(())
}

/// エンティティとそのサブツリーを再帰的に合成描画する。
pub(super) fn render_subtree(
    ctx: &CompositeContext,
    entity: Entity,
    query: &Query<(
        &Arrangement,
        &GlobalArrangement,
        Option<&GraphicsCommandList>,
        &Visual,
        Option<&Children>,
    )>,
) {
    let Ok((arrangement, ga, cmd_opt, visual, children_opt)) = query.get(entity) else {
        return;
    };

    // Req 2.3: is_visible == false → サブツリーごとスキップ
    if !visual.is_visible {
        trace!(entity = ?entity, "[render_subtree] SKIP: not visible");
        return;
    }

    // Req 2.4: opacity 累積計算
    let local_opacity = (ctx.accumulated_opacity * visual.clamped_opacity()).clamp(0.0, 1.0);

    // Req 2.6: opacity == 0.0 → サブツリーごとスキップ
    if local_opacity == 0.0 {
        trace!(entity = ?entity, "[render_subtree] SKIP: opacity=0");
        return;
    }

    trace!(
        entity = ?entity,
        opacity = local_opacity,
        has_cmd = cmd_opt.is_some(),
        transform = ?ga.transform,
        "[render_subtree] drawing entity"
    );

    // Req 2.2: SetTransform
    // ULW 補正: GlobalArrangement はスクリーン座標だが、合成ビットマップは
    // ウィンドウクライアント領域の (0,0) 起点なので、ウィンドウ位置分を差し引く。
    //
    // 重要: transform.M31/M32 は Arrangement → Matrix3x2 変換（translation * scale）に
    // よりオフセットにスケールが乗算されるため、DPIスケール ≠ 1.0 の場合
    // bounds.left/top と一致しない。bounds は scaled_offset = offset * parent_scale で
    // 正しく物理座標を保持するため、位置には bounds を使用しスケールには
    // transform.M11/M22 を使用する（bounds-based transform reconstruction）。
    let adjusted_transform = Matrix3x2 {
        M11: ga.transform.M11,
        M12: ga.transform.M12,
        M21: ga.transform.M21,
        M22: ga.transform.M22,
        M31: ga.bounds.left - ctx.window_offset.0,
        M32: ga.bounds.top - ctx.window_offset.1,
    };
    unsafe { ctx.dc.SetTransform(&adjusted_transform) };

    debug!(
        entity = ?entity,
        has_cmd = cmd_opt.is_some(),
        adj_tx = adjusted_transform.M31,
        adj_ty = adjusted_transform.M32,
        scale_x = adjusted_transform.M11,
        scale_y = adjusted_transform.M22,
        ga_bounds = ?(ga.bounds.left, ga.bounds.top, ga.bounds.right, ga.bounds.bottom),
        opacity = local_opacity,
        "[render_subtree] adjusted transform (bounds-based)"
    );

    // クリップ適用（SetTransform 後、draw_with_opacity 前）
    // ClipGuard の RAII により、スコープ終了時に自動で Pop が呼ばれる。
    // ULW モードではローカル座標 (0,0)-(w,h) を使用（DPI は SetTransform に含まれる）。
    let _clip_guard = if let Some(clip_shape) = &visual.clip {
        let (w, h) = (arrangement.size.width, arrangement.size.height);
        if w > 0.0 && h > 0.0 {
            match unsafe { ClipGuard::push(ctx.dc, clip_shape, w, h) } {
                Ok(guard) => {
                    trace!(
                        entity = ?entity,
                        clip = ?clip_shape,
                        size = ?(w, h),
                        "[render_subtree] clip pushed"
                    );
                    Some(guard)
                }
                Err(e) => {
                    error!(
                        entity = ?entity,
                        error = ?e,
                        "[render_subtree] ClipGuard::push failed, continuing without clip"
                    );
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Req 2.5: opacity 適用描画
    if let Some(cmd) = cmd_opt {
        if let Some(command_list) = cmd.command_list() {
            trace!(entity = ?entity, "[render_subtree] draw_with_opacity");
            if let Err(e) = unsafe { draw_with_opacity(ctx.dc, command_list, local_opacity) } {
                error!(
                    entity = ?entity,
                    error = ?e,
                    "[composite_render_system] draw_with_opacity failed"
                );
                // 当該エンティティの描画をスキップ（子への再帰は継続）
            }
        } else {
            trace!(entity = ?entity, "[render_subtree] command_list is None (closed?)");
        }
    }

    // 子エンティティへ再帰（accumulated_opacity + window_offset を伝搬）
    let child_ctx = CompositeContext {
        dc: ctx.dc,
        accumulated_opacity: local_opacity,
        window_offset: ctx.window_offset,
    };
    if let Some(children) = children_opt {
        for child in children.iter() {
            render_subtree(&child_ctx, child, query);
        }
    }
}

/// ウィンドウのサブツリー内に変更があるか検出する。
pub(super) fn is_window_dirty(
    window_entity: Entity,
    window_children: &Children,
    changed_query: &Query<
        Entity,
        Or<(
            Changed<GraphicsCommandList>,
            Changed<GlobalArrangement>,
            Changed<Visual>,
            Changed<Arrangement>,
        )>,
    >,
    children_query: &Query<&Children>,
    is_compositor_added: bool,
) -> bool {
    // 初回フレーム検出（Mut::is_added() で判定）
    if is_compositor_added {
        return true;
    }

    // サブツリー全体を走査して Changed<T> を検出
    fn check_subtree(
        entity: Entity,
        changed_query: &Query<
            Entity,
            Or<(
                Changed<GraphicsCommandList>,
                Changed<GlobalArrangement>,
                Changed<Visual>,
                Changed<Arrangement>,
            )>,
        >,
        children_query: &Query<&Children>,
    ) -> bool {
        if changed_query.contains(entity) {
            return true;
        }
        if let Ok(children) = children_query.get(entity) {
            for child in children.iter() {
                if check_subtree(child, changed_query, children_query) {
                    return true;
                }
            }
        }
        false
    }

    // ウィンドウエンティティ自体の変更もチェック
    if changed_query.contains(window_entity) {
        return true;
    }

    for child in window_children.iter() {
        if check_subtree(child, changed_query, children_query) {
            return true;
        }
    }

    false
}
