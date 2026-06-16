//! 合成描画パイプライン ECS システム
//!
//! - `composite_render_system`: 全エンティティの `GraphicsCommandList` を z-order + transform + opacity で合成描画
//! - `ulw_present_system`: 合成済みビットマップを UpdateLayeredWindow で各ウィンドウに転送

mod guards;
mod traverse;

use guards::DcTargetGuard;
use traverse::{is_window_dirty, render_subtree, CompositeContext};

use crate::com::ulw::{present_layered_window, transfer_to_hbitmap};
use crate::ecs::graphics::compositor::WindowD3D11Compositor;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore, Visual};
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use crate::ecs::window::WindowHandle;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace, warn};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows_numerics::Matrix3x2;

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
        &Arrangement,
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
            Changed<Arrangement>,
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
                        &crate::ecs::Rect {
                            left: 0.0,
                            top: 0.0,
                            right: fw,
                            bottom: BORDER,
                        }
                        .into(),
                        &brush,
                    );
                    // 下辺
                    dc.FillRectangle(
                        &crate::ecs::Rect {
                            left: 0.0,
                            top: fh - BORDER,
                            right: fw,
                            bottom: fh,
                        }
                        .into(),
                        &brush,
                    );
                    // 左辺
                    dc.FillRectangle(
                        &crate::ecs::Rect {
                            left: 0.0,
                            top: BORDER,
                            right: BORDER,
                            bottom: fh - BORDER,
                        }
                        .into(),
                        &brush,
                    );
                    // 右辺
                    dc.FillRectangle(
                        &crate::ecs::Rect {
                            left: fw - BORDER,
                            top: BORDER,
                            right: fw,
                            bottom: fh - BORDER,
                        }
                        .into(),
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
            // NOTE(W3a-V): デバイスロスト時はここに D2DERR_RECREATE_TARGET が返るが、
            // 現状はログ出力のみで、プロダクションコードに GraphicsCore::invalidate() を
            // 呼ぶ経路が存在しない（テスト・example のみ）。このため復旧機構
            // （init_graphics_core → invalidate_dependent_components →
            // compositor_init_system 再作成）は発火せず、ULW ウィンドウは最終提示
            // フレームのまま恒久的に固まる（可用性の縮退）。HRESULT 検査による
            // デバイスロスト検出の追加はエラー処理の挙動変更のため proposals.md P40 に記録。
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
                // SAFETY: dib_bits は cached_size == (w, h) で CreateDIBSection した
                // 32bpp top-down DIB の先頭を指し、確保サイズはちょうど w * h * 4 バイト
                // （32bpp のため stride = w * 4 でパディングなし）。cached_size と DIB は
                // new()/resize() で常に同時に設定されるため total_bytes は確保サイズと一致し、
                // 範囲外読み出しは発生しない。乗算は CreateDIBSection が同じ積の確保に
                // 成功している（実メモリに収まる）ため usize でオーバーフローしない。
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
        // 不変条件: is_valid() == true ⇒ 直近の new()/resize() が成功 ⇒ 同寸法の
        // D2D CreateBitmap（最大ビットマップサイズ ≦ 16384）と CreateDIBSection が
        // 成功済み ⇒ w/h は i32::MAX を超えない。よって下の `as i32` は損失しない。
        debug_assert!(
            w <= i32::MAX as u32 && h <= i32::MAX as u32,
            "cached_size must fit in i32 (guaranteed by successful resource creation)"
        );
        let size: windows::Win32::Foundation::SIZE = crate::ecs::SizeI {
            width: w as i32,
            height: h as i32,
        }
        .into();

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
