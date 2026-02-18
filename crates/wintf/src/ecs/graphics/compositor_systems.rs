//! 合成描画 ECS システム
//!
//! - `compositor_init_system`: ウィンドウエンティティへの `WindowD3D11Compositor` 自動作成・管理
//! - `composite_render_system`: 全エンティティの `GraphicsCommandList` を z-order + transform + opacity で合成描画

use super::compositor::WindowD3D11Compositor;
use crate::com::ulw::{present_layered_window, transfer_to_hbitmap};
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore, HasGraphicsResources, Visual};
use crate::ecs::layout::GlobalArrangement;
use crate::ecs::window::{WindowHandle, WindowPos};
// Note: WindowHandle は composite_render_system では不要（window_offset に GlobalArrangement を使用）
// だが ulw_present_system で使用するため import は維持
use bevy_ecs::hierarchy::Children;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace, warn};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::Matrix3x2;

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
            Changed<WindowPos>,
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
    /// ULW ビットマップはウィンドウクライアント領域の (0,0) から始まるため、
    /// GlobalArrangement のスクリーン座標からウィンドウ位置分を差し引く補正オフセット。
    window_offset: (f32, f32),
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
    let mut adjusted_transform = ga.transform;
    adjusted_transform.M31 -= ctx.window_offset.0;
    adjusted_transform.M32 -= ctx.window_offset.1;
    unsafe { ctx.dc.SetTransform(&adjusted_transform) };

    debug!(
        entity = ?entity,
        has_cmd = cmd_opt.is_some(),
        adj_tx = adjusted_transform.M31,
        adj_ty = adjusted_transform.M32,
        ga_bounds = ?(ga.bounds.left, ga.bounds.top, ga.bounds.right, ga.bounds.bottom),
        opacity = local_opacity,
        "[render_subtree] adjusted transform"
    );

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

/// 全エンティティの GraphicsCommandList を z-order + transform + opacity で
/// per-window 合成ビットマップに描画する。
pub fn composite_render_system(
    core: Res<GraphicsCore>,
    mut compositor_query: Query<(
        Entity,
        &mut WindowD3D11Compositor,
        &Children,
        &GlobalArrangement,
    )>,
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
) {
    let Some(dc) = core.device_context() else {
        return;
    };

    for (window_entity, mut compositor, window_children, window_ga) in compositor_query.iter_mut() {
        if !compositor.is_valid() {
            continue;
        }

        // Req 2.8: ダーティ判定（初回フレームまたは Changed<T> 検出）
        // Phase 2: Added<WindowD3D11Compositor> の別 Query を廃止し、
        // Mut::is_added() で初回フレームを検出（Query 競合回避）
        let is_added = compositor.is_added();
        let is_dirty = is_window_dirty(
            window_entity,
            window_children,
            &changed_query,
            &children_query,
            is_added,
        );
        trace!(
            entity = ?window_entity,
            is_added = is_added,
            is_dirty = is_dirty,
            size = ?compositor.cached_size(),
            "[composite_render_system] dirty check"
        );
        if !is_dirty {
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
        // ULW 補正: 合成ビットマップは (0,0) 起点で描画する。
        // GlobalArrangement.bounds はスクリーン物理ピクセル座標なので、
        // bounds.left/top をウィンドウ原点として使いビットマップ内ローカル座標に変換する。
        //
        // 重要: transform.M31/M32 はスケール込みの値 (offset * scale) であり bounds.left と
        // 一致しないため、window_offset には bounds.left/top を使用する。
        let window_offset = (window_ga.bounds.left, window_ga.bounds.top);
        let child_count = window_children.iter().count();
        debug!(
            entity = ?window_entity,
            child_count = child_count,
            window_offset_x = window_offset.0,
            window_offset_y = window_offset.1,
            cached_size = ?compositor.cached_size(),
            ga_bounds_left = window_ga.bounds.left,
            ga_bounds_top = window_ga.bounds.top,
            ga_bounds_right = window_ga.bounds.right,
            ga_bounds_bottom = window_ga.bounds.bottom,
            "[composite_render_system] rendering subtree (ULW offset compensation)"
        );
        let ctx = CompositeContext {
            dc,
            accumulated_opacity: 1.0,
            window_offset,
        };
        // ウィンドウエンティティをルートとして再帰走査
        // render_subtree はウィンドウ自身（背景がある場合）を描画した後、
        // 子エンティティへ再帰する。Children が entity_query の Optional なので
        // 子が無い場合も安全に処理される。
        render_subtree(&ctx, window_entity, &entity_query);

        // DEBUG: ULW ビットマップ外周に赤枠（2px）を描画
        // レイアウト由来 vs 描画由来の切り分け用
        {
            let (w, h) = compositor.cached_size();
            let fw = w as f32;
            let fh = h as f32;
            const BORDER: f32 = 2.0;
            let red = D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            };
            unsafe {
                dc.SetTransform(&Matrix3x2::identity());
                if let Ok(brush) = dc.CreateSolidColorBrush(&red, None) {
                    // 上辺
                    dc.FillRectangle(
                        &D2D_RECT_F {
                            left: 0.0,
                            top: 0.0,
                            right: fw,
                            bottom: BORDER,
                        },
                        &brush,
                    );
                    // 下辺
                    dc.FillRectangle(
                        &D2D_RECT_F {
                            left: 0.0,
                            top: fh - BORDER,
                            right: fw,
                            bottom: fh,
                        },
                        &brush,
                    );
                    // 左辺
                    dc.FillRectangle(
                        &D2D_RECT_F {
                            left: 0.0,
                            top: BORDER,
                            right: BORDER,
                            bottom: fh - BORDER,
                        },
                        &brush,
                    );
                    // 右辺
                    dc.FillRectangle(
                        &D2D_RECT_F {
                            left: fw - BORDER,
                            top: BORDER,
                            right: fw,
                            bottom: fh - BORDER,
                        },
                        &brush,
                    );
                }
            }
        }

        // 5. EndDraw（ターゲット復元は _target_guard の Drop で自動実行）
        let end_result = unsafe { dc.EndDraw(None, None) };

        // ワールド変換をリセット（次フレームのdrawシステムに非単位変換が漏れるのを防止）
        unsafe { dc.SetTransform(&Matrix3x2::identity()) };

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

        // DIB ピクセルダンプ（コンテンツ位置 + 非ゼロピクセルスキャン）
        if let Some(dib_bits) = compositor.dib_bits() {
            let (w, h) = compositor.cached_size();
            let stride = w as usize * 4;
            let total_bytes = stride * h as usize;
            if w > 0 && h > 0 {
                let buf: &[u8] = unsafe { std::slice::from_raw_parts(dib_bits, total_bytes) };
                // (15, 15) のピクセル — コンテンツ領域内のはず
                let sample_x = 15usize.min(w as usize - 1);
                let sample_y = 15usize.min(h as usize - 1);
                let px_offset = sample_y * stride + sample_x * 4;
                let px_15_15 = &buf[px_offset..px_offset + 4];
                // (100, 100) のピクセル
                let sx2 = 100usize.min(w as usize - 1);
                let sy2 = 100usize.min(h as usize - 1);
                let px2_offset = sy2 * stride + sx2 * 4;
                let px_100_100 = &buf[px2_offset..px2_offset + 4];
                // 非ゼロピクセルの最初の出現位置を探す
                let first_nonzero = buf.chunks(4).position(|c| c.iter().any(|&b| b != 0));
                let nonzero_count = buf.chunks(4).filter(|c| c.iter().any(|&b| b != 0)).count();
                let total_pixels = (w as usize) * (h as usize);
                trace!(
                    entity = ?window_entity,
                    size = ?(w, h),
                    px_15_15 = ?px_15_15,
                    px_100_100 = ?px_100_100,
                    first_nonzero_pixel_idx = ?first_nonzero,
                    nonzero_count,
                    total_pixels,
                    "[composite_render_system] DIB pixel dump [B,G,R,A per pixel]"
                );
            }
        }

        trace!(
            entity = ?window_entity,
            "[composite_render_system] Composition complete, dirty=true"
        );
    }
}

// ==========================================================================
// ulw_present_system
// ==========================================================================

/// 合成済みビットマップを UpdateLayeredWindow で各ウィンドウに転送する。
///
/// `CommitComposition` ステージで実行され、`composite_render_system` が設定した
/// dirty フラグを消費する。dirty=false のウィンドウはスキップし、
/// ULW 成功後に dirty=false を設定する。失敗時は warn ログ + 次フレーム再試行。
pub fn ulw_present_system(mut query: Query<(&WindowHandle, &mut WindowD3D11Compositor)>) {
    for (window_handle, mut compositor) in query.iter_mut() {
        let is_valid = compositor.is_valid();
        let is_dirty = compositor.is_dirty();
        // リソース未初期化またはダーティでなければスキップ
        if !is_valid || !is_dirty {
            trace!(
                "[ulw_present_system] skip: valid={}, dirty={}",
                is_valid, is_dirty
            );
            continue;
        }

        let hwnd = window_handle.hwnd;
        let (w, h) = compositor.cached_size();
        let size = SIZE {
            cx: w as i32,
            cy: h as i32,
        };

        trace!(
            hwnd = ?hwnd,
            size_w = w,
            size_h = h,
            "[ulw_present_system] calling UpdateLayeredWindow"
        );

        // MemoryDC 取得（HBITMAP が SelectObject 済み）
        let Some(hdc) = compositor.memory_dc() else {
            warn!("[ulw_present_system] memory_dc() returned None");
            continue;
        };

        match present_layered_window(hwnd, hdc, &size) {
            Ok(()) => {
                compositor.set_dirty(false);
                trace!("[ulw_present_system] UpdateLayeredWindow succeeded, dirty=false");
            }
            Err(e) => {
                warn!(
                    "[ulw_present_system] UpdateLayeredWindow failed: {e:?}, retrying next frame"
                );
                // dirty フラグは true のまま → 次フレームで再試行
            }
        }
    }
}
