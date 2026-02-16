//! 合成描画 ECS システム
//!
//! - `compositor_init_system`: ウィンドウエンティティへの `WindowD3D11Compositor` 自動作成・管理
//! - `composite_render_system`: 全エンティティの `GraphicsCommandList` を z-order + transform + opacity で合成描画

use super::compositor::WindowD3D11Compositor;
use crate::com::ulw::transfer_to_hbitmap;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore, HasGraphicsResources, Visual};
use crate::ecs::layout::GlobalArrangement;
use crate::ecs::window::{WindowHandle, WindowPos};
use bevy_ecs::hierarchy::Children;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;

use super::systems::format_entity_name;

// ==========================================================================
// compositor_init_system
// ==========================================================================

/// HWND を持つウィンドウエンティティに `WindowD3D11Compositor` を自動作成・管理する。
///
/// - 新規ウィンドウ検出（`Without<WindowD3D11Compositor>`）→ 作成
/// - デバイスロスト復旧（`Changed<HasGraphicsResources>` + `!is_valid()`）→ 再作成
/// - リサイズ検出（`cached_size` vs `WindowPos.size`）→ `resize()`
pub fn compositor_init_system(
    core: Res<GraphicsCore>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &WindowHandle,
            &WindowPos,
            &HasGraphicsResources,
            Option<&mut WindowD3D11Compositor>,
            Option<&Name>,
        ),
        Or<(
            Without<WindowD3D11Compositor>,
            Changed<HasGraphicsResources>,
        )>,
    >,
) {
    // GraphicsCore が無効なら早期リターン
    let Some(dc) = core.device_context() else {
        return;
    };

    for (entity, _handle, window_pos, _res, compositor_opt, name) in query.iter_mut() {
        let entity_name = format_entity_name(entity, name);

        // WindowPos.size が None または幅/高さ 0 の場合はスキップ
        let Some(size) = window_pos.size else {
            continue;
        };
        let w = size.cx as u32;
        let h = size.cy as u32;
        if w == 0 || h == 0 {
            continue;
        }

        match compositor_opt {
            None => {
                // 新規ウィンドウ: WindowD3D11Compositor 作成
                debug!(
                    entity = %entity_name,
                    width = w,
                    height = h,
                    "[compositor_init_system] Creating WindowD3D11Compositor"
                );
                match WindowD3D11Compositor::new(dc, w, h) {
                    Ok(compositor) => {
                        debug!(
                            entity = %entity_name,
                            "[compositor_init_system] WindowD3D11Compositor created"
                        );
                        commands.entity(entity).insert(compositor);
                    }
                    Err(e) => {
                        error!(
                            entity = %entity_name,
                            error = ?e,
                            "[compositor_init_system] Error creating WindowD3D11Compositor"
                        );
                    }
                }
            }
            Some(mut compositor) => {
                if !compositor.is_valid() {
                    // デバイスロスト復旧: 再作成
                    debug!(
                        entity = %entity_name,
                        "[compositor_init_system] Re-creating WindowD3D11Compositor (device lost)"
                    );
                    let old_generation = compositor.generation();
                    match WindowD3D11Compositor::new(dc, w, h) {
                        Ok(mut new_compositor) => {
                            // 旧 generation を引き継ぎインクリメント
                            let target_gen = old_generation.wrapping_add(1);
                            while new_compositor.generation() < target_gen {
                                new_compositor.increment_generation();
                            }
                            *compositor = new_compositor;
                            debug!(
                                entity = %entity_name,
                                old_generation = old_generation,
                                new_generation = compositor.generation(),
                                "[compositor_init_system] WindowD3D11Compositor re-created"
                            );
                        }
                        Err(e) => {
                            error!(
                                entity = %entity_name,
                                error = ?e,
                                "[compositor_init_system] Re-creation failed, invalidating"
                            );
                            compositor.invalidate();
                        }
                    }
                } else if compositor.cached_size() != (w, h) {
                    // リサイズ検出: resize()
                    debug!(
                        entity = %entity_name,
                        old_size = ?compositor.cached_size(),
                        new_size = ?(w, h),
                        "[compositor_init_system] Resizing WindowD3D11Compositor"
                    );
                    if let Err(e) = compositor.resize(dc, w, h) {
                        error!(
                            entity = %entity_name,
                            error = ?e,
                            "[compositor_init_system] Resize failed, keeping old size"
                        );
                    }
                }
            }
        }
    }
}

// ==========================================================================
// composite_render_system
// ==========================================================================

/// ID2D1DeviceContext のターゲット切替を RAII パターンで管理し、
/// スコープ終了時に自動復元する。
struct DcTargetGuard<'a> {
    dc: &'a ID2D1DeviceContext,
    prev_target: Option<ID2D1Image>,
}

impl<'a> DcTargetGuard<'a> {
    /// DC のターゲットを new_target に切り替え、RAII ガードを返す。
    unsafe fn new(dc: &'a ID2D1DeviceContext, new_target: &ID2D1Bitmap1) -> Self {
        let prev_target = unsafe { dc.GetTarget().ok() };
        unsafe { dc.SetTarget(new_target) };
        Self { dc, prev_target }
    }
}

impl Drop for DcTargetGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.dc.SetTarget(self.prev_target.as_ref());
        }
    }
}

/// 合成描画ツリー走査時に親→子へ伝搬する描画コンテキスト
struct CompositeContext<'a> {
    dc: &'a ID2D1DeviceContext,
    accumulated_opacity: f32,
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
fn render_subtree(
    ctx: &CompositeContext,
    entity: Entity,
    query: &Query<(
        &GlobalArrangement,
        Option<&GraphicsCommandList>,
        &Visual,
        Option<&Children>,
    )>,
) {
    let Ok((ga, cmd_opt, visual, children_opt)) = query.get(entity) else {
        return;
    };

    // Req 2.3: is_visible == false → サブツリーごとスキップ
    if !visual.is_visible {
        return;
    }

    // Req 2.4: opacity 累積計算
    let local_opacity = (ctx.accumulated_opacity * visual.clamped_opacity()).clamp(0.0, 1.0);

    // Req 2.6: opacity == 0.0 → サブツリーごとスキップ
    if local_opacity == 0.0 {
        return;
    }

    // Req 2.2: SetTransform
    unsafe { ctx.dc.SetTransform(&ga.transform) };

    // Req 2.5: opacity 適用描画
    if let Some(cmd) = cmd_opt {
        if let Some(command_list) = cmd.command_list() {
            if let Err(e) = unsafe { draw_with_opacity(ctx.dc, command_list, local_opacity) } {
                error!(
                    entity = ?entity,
                    error = ?e,
                    "[composite_render_system] draw_with_opacity failed"
                );
                // 当該エンティティの描画をスキップ（子への再帰は継続）
            }
        }
    }

    // 子エンティティへ再帰（accumulated_opacity を伝搬）
    let child_ctx = CompositeContext {
        dc: ctx.dc,
        accumulated_opacity: local_opacity,
    };
    if let Some(children) = children_opt {
        for child in children.iter() {
            render_subtree(&child_ctx, child, query);
        }
    }
}

/// ウィンドウのサブツリー内に変更があるか検出する。
fn is_window_dirty(
    window_entity: Entity,
    window_children: &Children,
    changed_query: &Query<
        Entity,
        Or<(
            Changed<GraphicsCommandList>,
            Changed<GlobalArrangement>,
            Changed<Visual>,
        )>,
    >,
    children_query: &Query<&Children>,
    added_query: &Query<Entity, Added<WindowD3D11Compositor>>,
) -> bool {
    // 初回フレーム検出
    if added_query.contains(window_entity) {
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

    for child in window_children.iter() {
        if check_subtree(child, changed_query, children_query) {
            return true;
        }
    }

    false
}

/// 全エンティティの GraphicsCommandList を z-order + transform + opacity で
/// per-window 合成ビットマップに描画する。
pub fn composite_render_system(
    core: Res<GraphicsCore>,
    mut compositor_query: Query<(Entity, &mut WindowD3D11Compositor, &Children)>,
    entity_query: Query<(
        &GlobalArrangement,
        Option<&GraphicsCommandList>,
        &Visual,
        Option<&Children>,
    )>,
    changed_query: Query<
        Entity,
        Or<(
            Changed<GraphicsCommandList>,
            Changed<GlobalArrangement>,
            Changed<Visual>,
        )>,
    >,
    children_query: Query<&Children>,
    added_query: Query<Entity, Added<WindowD3D11Compositor>>,
) {
    let Some(dc) = core.device_context() else {
        return;
    };

    for (window_entity, mut compositor, window_children) in compositor_query.iter_mut() {
        if !compositor.is_valid() {
            continue;
        }

        // Req 2.8: ダーティ判定（初回フレームまたは Changed<T> 検出）
        if !is_window_dirty(
            window_entity,
            window_children,
            &changed_query,
            &children_query,
            &added_query,
        ) {
            continue;
        }

        // 2. DC ターゲット切替（RAII ガード）
        let comp_bmp = match compositor.composition_bitmap() {
            Some(bmp) => bmp.clone(),
            None => continue,
        };
        let _target_guard = unsafe { DcTargetGuard::new(dc, &comp_bmp) };

        // 3. BeginDraw → Clear(transparent)
        unsafe { dc.BeginDraw() };
        unsafe {
            dc.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
        }

        // 4. 再帰走査（Req 2.1: depth-first pre-order）
        let ctx = CompositeContext {
            dc,
            accumulated_opacity: 1.0,
        };
        for child in window_children.iter() {
            render_subtree(&ctx, child, &entity_query);
        }

        // 5. EndDraw（ターゲット復元は _target_guard の Drop で自動実行）
        let end_result = unsafe { dc.EndDraw(None, None) };
        if let Err(e) = end_result {
            error!(
                error = ?e,
                "[composite_render_system] EndDraw failed"
            );
            continue;
        }

        // 6. CopyFromBitmap（Req 2.7）
        if let Some(staging) = compositor.staging_bitmap() {
            let copy_result = unsafe { staging.CopyFromBitmap(None, &comp_bmp, None) };
            if let Err(e) = copy_result {
                error!(
                    error = ?e,
                    "[composite_render_system] CopyFromBitmap failed"
                );
                continue;
            }
        }

        // 7. transfer_to_hbitmap（Req 4.1-4.5）
        if let (Some(staging), Some(dib_bits)) =
            (compositor.staging_bitmap(), compositor.dib_bits())
        {
            let (w, h) = compositor.cached_size();
            if let Err(e) = unsafe { transfer_to_hbitmap(staging, dib_bits, w, h) } {
                error!(
                    error = ?e,
                    "[composite_render_system] transfer_to_hbitmap failed"
                );
                continue;
            }
        }

        // 8. dirty フラグ設定（Req 2.7、Phase 3 で消費）
        compositor.set_dirty(true);

        trace!("[composite_render_system] Composition complete for window");
    }
}
