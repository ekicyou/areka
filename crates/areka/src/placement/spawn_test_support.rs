use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::WindowHandle;

use super::GhostWindowMarker;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;

// -------------------------------------------------------------------------
// テストヘルパ（bare World・emo2 相当 2 スコープ。値は resolver 出力を模した
// 合成値で、96 の倍数を避けて隠れた dpi 再スケールがあれば一致が崩れる檻とする。
// 恒等式 balloon_offset ≡ balloon_pos − char_pos を満たすように構築する）
// -------------------------------------------------------------------------

/// scope0/scope1 の 2 スコープぶんの解決済み配置（emo2 相当の形）。
pub(super) fn two_scope_placements() -> Vec<ScopePlacement> {
    vec![
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 733 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 708 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            anchor: Anchor::Bottom, // emo2＝alignmenttodesktop,bottom
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1063 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1044 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            anchor: Anchor::Bottom, // emo2＝alignmenttodesktop,bottom
        },
    ]
}

pub(super) fn titles() -> GhostTitles {
    GhostTitles::from_scope_titles([(0, "むらさき".to_string()), (1, "エモ".to_string())])
}

/// 全 GhostWindowMarker 窓 entity を収集する。
pub(super) fn ghost_window_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .collect()
}

/// 偽 HWND を持つ `WindowHandle`（4.2 と同じ fake WindowHandle パターン）。
pub(super) fn fake_window_handle(raw: isize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut core::ffi::c_void),
        instance: HINSTANCE::default(),
    }
}
