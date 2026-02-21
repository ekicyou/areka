//! WindowPos ↔ Arrangement 同期システム

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, trace};

use super::super::metrics::Offset;
use super::super::{Arrangement, GlobalArrangement};
use crate::ecs::graphics::format_entity_name;
use crate::ecs::window::{Window, WindowPos};

/// GlobalArrangement の変更を WindowPos に反映する ECS システム。
///
/// bounds.left/top → WindowPos.position（truncate to i32）
/// bounds の幅/高さ → WindowPos.size（ceil to i32）
pub fn window_pos_sync_system(
    mut query: Query<
        (Entity, &GlobalArrangement, &mut WindowPos, Option<&Name>),
        (With<Window>, Changed<GlobalArrangement>),
    >,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    use windows::Win32::Foundation::{POINT, SIZE};

    for (entity, global_arr, mut window_pos, name) in query.iter_mut() {
        let entity_name = format_entity_name(entity, name);
        let width = global_arr.bounds.right - global_arr.bounds.left;
        let height = global_arr.bounds.bottom - global_arr.bounds.top;

        debug!(
            frame = frame_count.0,
            entity = %entity_name,
            bounds_left = global_arr.bounds.left,
            bounds_top = global_arr.bounds.top,
            bounds_right = global_arr.bounds.right,
            bounds_bottom = global_arr.bounds.bottom,
            width = width,
            height = height,
            "[window_pos_sync_system] Processing entity"
        );

        if width <= 0.0 || height <= 0.0 {
            debug!(
                frame = frame_count.0,
                entity = %entity_name,
                "[window_pos_sync_system] Skipping: invalid bounds"
            );
            continue;
        }

        let new_position = POINT {
            x: global_arr.bounds.left as i32,
            y: global_arr.bounds.top as i32,
        };
        let new_size = SIZE {
            cx: width.ceil() as i32,
            cy: height.ceil() as i32,
        };

        let position_changed = window_pos.position != Some(new_position);
        let size_changed = window_pos.size != Some(new_size);

        if position_changed || size_changed {
            debug!(
                frame = frame_count.0,
                entity = %entity_name,
                old_pos = ?window_pos.position,
                old_size = ?window_pos.size,
                new_xy = format_args!("({},{})", new_position.x, new_position.y),
                new_size = format_args!("{}x{}", new_size.cx, new_size.cy),
                ga_bounds = format_args!("({:.0},{:.0})-({:.0},{:.0})", global_arr.bounds.left, global_arr.bounds.top, global_arr.bounds.right, global_arr.bounds.bottom),
                "[window_pos_sync] Updating WindowPos from GA"
            );
            window_pos.position = Some(new_position);
            window_pos.size = Some(new_size);
        } else {
            trace!(
                frame = frame_count.0,
                entity = %entity_name,
                "[window_pos_sync_system] No change needed"
            );
        }
    }
}

/// WindowPos.position の変更を Window の Arrangement.offset に反映
///
/// WM_WINDOWPOSCHANGED で更新された WindowPos.position（クライアント領域のスクリーン座標）を
/// Window の Arrangement.offset に反映する。
///
/// これにより GlobalArrangement.bounds が正しいスクリーン座標を持つようになり、
/// hit_test が正しく動作する。
///
/// `Changed<WindowPos>` フィルタにより、WindowPos が変更されたエンティティのみ処理する。
/// 変更がない場合は何もしない。
///
/// # 座標系
/// Window エンティティは LayoutRoot (scale=1.0) の子であり、
/// `GlobalArrangement.bounds.left = parent.bounds.left + offset × parent_scale = offset` となる。
/// したがって `Arrangement.offset` は物理ピクセル単位であり、
/// `WindowPos.position`（物理px）をそのまま offset に設定する（DPI除算不要）。
///
/// ルートエンティティ（親なし）の場合は `bounds.left = offset × DPI_scale` となるが、
/// 実アプリでは Window は常に LayoutRoot の子である。
pub fn sync_window_arrangement_from_window_pos(
    mut query: Query<
        (Entity, &WindowPos, &mut Arrangement, Option<&Name>),
        (With<Window>, Changed<WindowPos>),
    >,
) {
    use crate::ecs::graphics::format_entity_name;

    for (entity, window_pos, mut arrangement, name) in query.iter_mut() {
        let Some(position) = window_pos.position else {
            continue;
        };

        // CW_USEDEFAULT の場合はスキップ（ウィンドウ作成時の初期値）
        if position.x == windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT {
            continue;
        }

        // WindowPos.position は物理ピクセル座標
        // Window は LayoutRoot (scale=1.0) の子なので offset = position（DPI除算不要）
        let new_offset = Offset {
            x: position.x as f32,
            y: position.y as f32,
        };

        // 変更があった場合のみ更新
        if arrangement.offset != new_offset {
            let entity_name = format_entity_name(entity, name);
            debug!(
                entity = %entity_name,
                old = format_args!("({:.0},{:.0})", arrangement.offset.x, arrangement.offset.y),
                new = format_args!("({:.0},{:.0})", new_offset.x, new_offset.y),
                "[sync_window_arr] offset changed"
            );
            arrangement.offset = new_offset;
        }
    }
}
