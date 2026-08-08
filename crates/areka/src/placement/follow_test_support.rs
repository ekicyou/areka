use std::time::Instant;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::SizeI;
use wintf::ecs::drag::{DragEndEvent, DragEvent, DraggingState};
use wintf::ecs::{Point, WindowHandle, WindowPos};

use super::MonitorSnapshot;
use crate::placement::resolver::RectPx;

// -------------------------------------------------------------------------
// テストヘルパ（偽装境界: 実 HWND なしの headless World で決定論検証する。
// SetWindowPosCommand は TLS キューへの enqueue のみで flush しないため、
// 偽 HWND に対する実 SetWindowPos は一切呼ばれない——wintf 自身の
// window_pos_systems_test と同じ流儀）
// -------------------------------------------------------------------------

/// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム）。
pub(super) fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// position 初期値付きの WindowPos。
pub(super) fn window_pos_at(x: i32, y: i32) -> WindowPos {
    WindowPos {
        position: Some(Point { x, y }),
        ..Default::default()
    }
}

/// entity の WindowPos.position を読む（未設定は panic で検出）。
pub(super) fn position_of(world: &World, entity: Entity) -> Point {
    world
        .get::<WindowPos>(entity)
        .expect("WindowPos があるはず")
        .position
        .expect("position があるはず")
}

pub(super) fn drag_event(target: Entity) -> DragEvent {
    DragEvent {
        target,
        start_position: Point::new(0, 0),
        position: Point::new(10, 10),
        is_primary: true,
        timestamp: Instant::now(),
    }
}

pub(super) fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
    RectPx {
        left,
        top,
        right,
        bottom,
    }
}

/// 単一モニタの合成 snapshot（物理 px・96 の倍数を避けた下端で再スケール檻）。
pub(super) fn single_monitor_snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1920, 1043)],
    }
}

/// 全 4 辺が 96 の非倍数の単一モニタ snapshot（各アンカー辺再計算の再スケール檻）。
/// left=53・top=37・right=1877・bottom=1043（いずれも 96 で割り切れない・非零原点）。
pub(super) fn odd_edge_snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(53, 37, 1877, 1043)],
    }
}

// -------------------------------------------------------------------------
// bottom 吸着ドラッグ（task 8.2R・4.7・DD15 v2: 単一ライター）
//
// BottomSnap キャラ窓は DragConfig{move_window:false}＝wndproc は窓を動かさず、
// on_char_drag が DraggingState（dispatch_drag_events が挿入）＋DragEvent の
// カーソル座標から生ドラッグ座標を復元し、ポリシー適用済み座標を一度だけ書く。
// headless では DraggingState を注入して実 flow を模し、handler を直接呼ぶ。
// -------------------------------------------------------------------------

/// DraggingState（dispatch_drag_events 挿入の模擬）。wintf の実セマンティクス:
/// `initial_inset`＝ドラッグ開始時の**窓位置**（dispatch.rs が initial_window_pos
/// を転記）・`drag_start_pos`＝開始カーソル（スクリーン物理 px）。
pub(super) fn dragging_state(initial_window: (i32, i32), drag_start: (i32, i32)) -> DraggingState {
    DraggingState {
        drag_start_pos: Point::new(drag_start.0, drag_start.1),
        initial_inset: (initial_window.0 as f32, initial_window.1 as f32),
    }
}

/// カーソル座標付き DragEvent（start_position は DraggingState と同値・実 flow 準拠）。
pub(super) fn drag_event_at(target: Entity, start: (i32, i32), cursor: (i32, i32)) -> DragEvent {
    DragEvent {
        target,
        start_position: Point::new(start.0, start.1),
        position: Point::new(cursor.0, cursor.1),
        is_primary: true,
        timestamp: Instant::now(),
    }
}

/// 最終カーソル座標付き DragEndEvent。
pub(super) fn drag_end_event_at(target: Entity, cursor: (i32, i32)) -> DragEndEvent {
    DragEndEvent {
        target,
        position: Point::new(cursor.0, cursor.1),
        cancelled: false,
        is_primary: true,
        timestamp: Instant::now(),
    }
}

/// position＋size 付きの WindowPos（spawn の `window_pos` と同型）。
pub(super) fn window_pos_sized(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
    WindowPos {
        position: Some(Point { x, y }),
        size: Some(SizeI::new(w, h)),
        ..Default::default()
    }
}
