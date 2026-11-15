use crate::follow::OffsetBase;
use bevy_ecs::prelude::*;

use super::GhostWindowMarker;
use crate::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::source::GhostTitles;

// -------------------------------------------------------------------------
// テストヘルパ（bare World・emo2 相当 2 スコープ。値は resolver 出力を模した
// 合成値で、96 の倍数を避けて隠れた dpi 再スケールがあれば一致が崩れる檻とする。
// 恒等式 balloon_offset ≡ balloon_pos − char_pos を満たすように構築する）。
// ずらしの基準は未係留（`OffsetBase::unpinned`）なので、記憶から戻した配置と同じく
// 生やした各キャラ窓に利用者が動かした印（`BalloonUserMoved`）が付く。
// -------------------------------------------------------------------------

/// scope0/scope1 の 2 スコープぶんの解決済み配置（emo2 相当の形）。
pub fn two_scope_placements() -> Vec<ScopePlacement> {
    vec![
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 733 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 708 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            balloon_offset_base: OffsetBase::unpinned(PointPx { x: -412, y: -25 }),
            // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
            balloon_limit: true,
            anchor: Anchor::Bottom, // emo2＝alignmenttodesktop,bottom
            balloon_keyword_base: None,
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1063 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1044 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            balloon_offset_base: OffsetBase::unpinned(PointPx { x: 285, y: -19 }),
            // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
            balloon_limit: true,
            anchor: Anchor::Bottom, // emo2＝alignmenttodesktop,bottom
            balloon_keyword_base: None,
        },
    ]
}

pub fn titles() -> GhostTitles {
    GhostTitles::from_scope_titles([(0, "むらさき".to_string()), (1, "エモ".to_string())])
}

/// 全 GhostWindowMarker 窓 entity を収集する。
pub fn ghost_window_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .collect()
}

/// 自プロセス所有の非表示トップレベル窓を作る（wintf 側の確立系テストと同一レシピ）。
///
/// owner 関係はトップレベル窓の間でのみ意味を持つため `WS_POPUP`（親なし）で作る。
/// 重なり管理の結線テスト（ペア・鎖）の 2 ファイルが共に使う。
#[cfg(test)]
pub(super) fn create_test_window(title: windows::core::PCWSTR) -> windows::Win32::Foundation::HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
    };
    use windows::core::w;
    // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ウィンドウを生成する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            title,
            WINDOW_STYLE(WS_POPUP.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a hidden test window")
    }
}

/// 生成したテスト窓を破棄する（後始末）。
#[cfg(test)]
pub(super) fn destroy_test_window(hwnd: windows::Win32::Foundation::HWND) {
    // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する。
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
    }
}

/// ハンドル付きの窓 entity を作る。
#[cfg(test)]
pub(super) fn spawn_window(world: &mut World, hwnd: windows::Win32::Foundation::HWND) -> Entity {
    world.spawn(wintf::ecs::WindowHandle { hwnd }).id()
}
