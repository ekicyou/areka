//! 位置初期化の 2 つの近道（areka-P0-window-link-directives 要件 8.1〜8.4・design「shortcut」）。
//!
//! Ctrl＋Shift＋F12 は「シェル位置初期化」、Ctrl＋Shift＋F10 は「バルーン位置初期化」と同じ働きをする。
//! 受け手はゴーストの窓に wintf の `OnKeyDown` として差すので、ゴーストの窓がキーの入力先のときだけ
//! 効き、ほかのアプリでの同じキーは横取りしない（8.4・裁定 9）。全体の近道（`RegisterHotKey`）は使わない。

use bevy_ecs::prelude::*;
use wintf::ecs::window::{KeyDown, OnKeyDown};

use crate::menu::reset_balloon_pos_frame::reset_balloon_enabled;
use crate::menu::reset_pos_frame::{raise_reset_window_pos, reset_position_enabled};
use crate::placement::reset_balloon_pos::{BalloonResetTrigger, reset_balloon_positions};
use crate::placement::spawn::GhostWindowMarker;

/// 仮想キー F10（`VK_F10`）。
const VK_F10: u16 = 0x79;
/// 仮想キー F12（`VK_F12`）。
const VK_F12: u16 = 0x7B;

/// 位置初期化の近道。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionShortcut {
    /// Ctrl＋Shift＋F12（シェル位置初期化）。
    ResetShellPosition,
    /// Ctrl＋Shift＋F10（バルーン位置初期化）。
    ResetBalloonPosition,
}

impl PositionShortcut {
    fn as_str(self) -> &'static str {
        match self {
            Self::ResetShellPosition => "reset_shell_position",
            Self::ResetBalloonPosition => "reset_balloon_position",
        }
    }
}

/// Ctrl と Shift が押され Alt が押されていない・繰り返しでない F12／F10 だけを拾う（純）。
pub(crate) fn shortcut_of(key: KeyDown) -> Option<PositionShortcut> {
    if !key.ctrl || !key.shift || key.alt || key.repeat {
        return None;
    }
    match key.vk {
        VK_F12 => Some(PositionShortcut::ResetShellPosition),
        VK_F10 => Some(PositionShortcut::ResetBalloonPosition),
        _ => None,
    }
}

/// 窓に差す受け手（`OnKeyDown` の署名）。組み合わせが合えば、選べない場面でも真を返して押下を消費する。
pub(crate) fn on_ghost_key_down(world: &mut World, _entity: Entity, key: KeyDown) -> bool {
    let Some(shortcut) = shortcut_of(key) else {
        return false;
    };
    let selectable = match shortcut {
        PositionShortcut::ResetShellPosition => reset_position_enabled(world),
        PositionShortcut::ResetBalloonPosition => reset_balloon_enabled(world),
    };
    if !selectable {
        tracing::info!(
            event = "position_shortcut_ignored",
            shortcut = shortcut.as_str(),
            reason = "not_selectable",
            "[shortcut] 位置初期化の近道を受けたが、今は選べない: 何もしない"
        );
        return true;
    }
    match shortcut {
        // メニューと同じ往復（`OnResetWindowPos` の答えで並べ直すかを決める）（8.1）。
        PositionShortcut::ResetShellPosition => raise_reset_window_pos(world),
        PositionShortcut::ResetBalloonPosition => {
            reset_balloon_positions(world, BalloonResetTrigger::Shortcut);
        }
    }
    true
}

/// `GhostWindowMarker` を持つ全 entity（キャラクターの窓とバルーンの窓）へ受け手 [`on_ghost_key_down`]
/// を差す。`ghost_session::prepare_ghost_windows` の窓を作る閉包で、投げ込みの受け手の隣で呼ぶ。
pub(crate) fn attach_shortcut_receivers(world: &mut World) {
    let ghost_windows: Vec<Entity> = world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .collect();
    for e in ghost_windows {
        world.entity_mut(e).insert(OnKeyDown(on_ghost_key_down));
    }
}

#[cfg(test)]
#[path = "shortcut_tests.rs"]
mod tests;
