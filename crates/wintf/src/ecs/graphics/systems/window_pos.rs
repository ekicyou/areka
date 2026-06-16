use super::init::format_entity_name;
use crate::ecs::drag::WindowDragging;
use crate::ecs::graphics::DCompGraphicsResource;
use crate::ecs::graphics::GraphicsCore;
use crate::ecs::graphics::compositor::WindowD3D11Compositor;
use crate::ecs::widget::bitmap_source::BitmapSourceGraphics;
use crate::ecs::window::{SetWindowPosCommand, Window, WindowHandle, WindowPos};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, trace, warn};
use windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE;

/// WindowPos変更時にSetWindowPosコマンドをキューに追加
///
/// クライアント領域座標をウィンドウ全体座標に変換してからコマンドを生成する。
/// echo 時は `WM_WINDOWPOSCHANGED` ハンドラが `bypass_change_detection()` で更新するため
/// `Changed<WindowPos>` が発火せず、本システムのトリガー自体が発火しない。
///
/// ドラッグ中のウィンドウ（`WindowDragging` 付き）はサイズ変更のみ発行し、
/// `SWP_NOMOVE` フラグで位置を固定する。これにより DPI 変更時の
/// ウィンドウリサイズがドラッグ中でも正しく反映される。
pub fn apply_window_pos_changes(
    query: Query<
        (
            Entity,
            &WindowHandle,
            &WindowPos,
            Option<&Name>,
            Has<WindowDragging>,
        ),
        (Changed<WindowPos>, With<Window>),
    >,
) {
    for (entity, window_handle, window_pos, name, is_dragging) in query.iter() {
        let entity_name = format_entity_name(entity, name);

        let position = window_pos.position.unwrap_or_default();
        let size = window_pos.size.unwrap_or_default();

        // CW_USEDEFAULTが設定されている場合はスキップ（ウィンドウ作成時の初期値）
        // 座標変換をスキップし、ウィンドウ作成時の初期配置を優先
        if position.x == windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT
            || size.width == windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT
        {
            trace!(
                entity = %entity_name,
                x = position.x,
                y = position.y,
                width = size.width,
                height = size.height,
                "[apply_window_pos_changes] CW_USEDEFAULT detected, skipping"
            );
            continue;
        }

        // クライアント領域座標をウィンドウ全体座標に変換
        let (x, y, width, height) = match window_pos.to_window_coords(window_handle) {
            Ok(coords) => coords,
            Err(e) => {
                // 変換失敗時はフォールバック：元の座標を使用
                warn!(
                    entity = %entity_name,
                    error = %e,
                    "[apply_window_pos_changes] Failed to transform window coordinates. Using original values."
                );
                (position.x, position.y, size.width, size.height)
            }
        };

        // SetWindowPosコマンドを生成してキューに追加
        // 直接SetWindowPosを呼び出さない（World借用競合防止）
        let mut flags = window_pos.build_flags_for_system();
        let hwnd_insert_after = window_pos.get_hwnd_insert_after();

        // ドラッグ中は SWP_NOMOVE を強制: サイズのみ変更
        if is_dragging {
            flags |= SWP_NOMOVE;
        }

        debug!(
            entity = %entity_name,
            client_xy = format_args!("({},{})", position.x, position.y),
            client_size = format_args!("{}x{}", size.width, size.height),
            win_xy = format_args!("({},{})", x, y),
            win_size = format_args!("{}x{}", width, height),
            is_dragging = is_dragging,
            "[apply_window_pos] Enqueue SetWindowPos"
        );

        let cmd = SetWindowPosCommand::new(
            window_handle.hwnd,
            x,
            y,
            width,
            height,
            flags,
            hwnd_insert_after,
        );
        SetWindowPosCommand::enqueue(cmd);

        debug!(
            entity = %entity_name,
            x = position.x,
            y = position.y,
            width = size.width,
            height = size.height,
            "[apply_window_pos_changes] Command enqueued"
        );
    }
}

/// 依存コンポーネント無効化
/// Phase 2: DComp Query（WindowGraphics, VisualGraphics, SurfaceGraphics）を除去し、
/// WindowD3D11Compositorを追加。BitmapSourceGraphicsは維持（DComp非依存）。
///
/// NOTE(W3b-V): Phase 2 の除去により、本システムは DComp 系コンポーネント
/// （WindowGraphics / VisualGraphics / SurfaceGraphics）を無効化しない。一方で
/// 再初期化側（init_window_graphics / visual_resource_management_system）は
/// `!is_valid()` を再作成の前提条件とするため、デバイスロスト時にこれらは旧デバイス
/// 由来の COM ポインタを保持したまま「有効」と判定され続け、検出（P40）が実装されても
/// DComp パイプラインは復旧しない（tests/graphics/window_pos_systems_test.rs の
/// stale 特性化テストで現行挙動を固定。無効化対象の再追加は挙動変更のため
/// P40 の設計判断・セル断片 W3b-V の分析を参照）。
pub fn invalidate_dependent_components(
    graphics: Option<Res<GraphicsCore>>,
    dcomp_resource: Option<ResMut<DCompGraphicsResource>>,
    mut compositor_query: Query<&mut WindowD3D11Compositor>,
    mut bitmap_source_query: Query<&mut BitmapSourceGraphics>,
) {
    if let Some(gc) = graphics {
        if !gc.is_valid() {
            warn!(
                "[invalidate_dependent_components] GraphicsCore invalid - invalidating all dependent components"
            );

            // DCompGraphicsResource の無効化
            if let Some(mut dcr) = dcomp_resource {
                dcr.invalidate();
            }

            for mut comp in compositor_query.iter_mut() {
                comp.invalidate();
            }
            for mut bsg in bitmap_source_query.iter_mut() {
                bsg.invalidate();
            }
        }
    }
}
