use super::init::format_entity_name;
use crate::ecs::graphics::DCompGraphicsResource;
use crate::ecs::graphics::GraphicsCore;
use crate::ecs::graphics::compositor::WindowD3D11Compositor;
use crate::ecs::widget::bitmap_source::BitmapSourceGraphics;
use crate::ecs::window::{SetWindowPosCommand, Window, WindowHandle, WindowPos};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, trace, warn};

/// WindowPos変更時にSetWindowPosコマンドをキューに追加
///
/// クライアント領域座標をウィンドウ全体座標に変換してからコマンドを生成する。
/// echo 時は `WM_WINDOWPOSCHANGED` ハンドラが `bypass_change_detection()` で更新するため
/// `Changed<WindowPos>` が発火せず、本システムのトリガー自体が発火しない。
pub fn apply_window_pos_changes(
    mut query: Query<
        (Entity, &WindowHandle, &WindowPos, Option<&Name>),
        (Changed<WindowPos>, With<Window>),
    >,
) {
    for (entity, window_handle, window_pos, name) in query.iter_mut() {
        let entity_name = format_entity_name(entity, name);

        // エコーバックチェック
        let position = window_pos.position.unwrap_or_default();
        let size = window_pos.size.unwrap_or_default();

        // CW_USEDEFAULTが設定されている場合はスキップ（ウィンドウ作成時の初期値）
        // 座標変換をスキップし、ウィンドウ作成時の初期配置を優先
        if position.x == windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT
            || size.cx == windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT
        {
            trace!(
                entity = %entity_name,
                x = position.x,
                y = position.y,
                cx = size.cx,
                cy = size.cy,
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
                (position.x, position.y, size.cx, size.cy)
            }
        };

        // SetWindowPosコマンドを生成してキューに追加
        // 直接SetWindowPosを呼び出さない（World借用競合防止）
        let flags = window_pos.build_flags_for_system();
        let hwnd_insert_after = window_pos.get_hwnd_insert_after();

        debug!(
            entity = %entity_name,
            client_xy = format_args!("({},{})", position.x, position.y),
            client_size = format_args!("{}x{}", size.cx, size.cy),
            win_xy = format_args!("({},{})", x, y),
            win_size = format_args!("{}x{}", width, height),
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
            cx = size.cx,
            cy = size.cy,
            "[apply_window_pos_changes] Command enqueued"
        );
    }
}

/// 依存コンポーネント無効化
/// Phase 2: DComp Query（WindowGraphics, VisualGraphics, SurfaceGraphics）を除去し、
/// WindowD3D11Compositorを追加。BitmapSourceGraphicsは維持（DComp非依存）。
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
