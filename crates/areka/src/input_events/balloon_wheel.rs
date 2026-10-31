//! 普通のバルーンの窓のホイールの系（balloon-scroll-arrows 要件 6・design「BalloonWheel」）。
//!
//! キャラ窓の見張り（`char_watch`）と同じ `Input` の段・`dispatch_pointer_events` の前で、
//! `PointerState` の持ち主から `BalloonWindowMarker` の窓を引き、縦の回転をそのスコープの
//! バルーンの場所の手動の送り（`wheel_at`）へ渡す。`OnMouseWheel` は送らず、横の回転は読まない
//! （要件 6.7）。OS の「1 回にスクロールする行数」は回転を受けた時だけ読む（設定の変わり目は
//! OS が知らせないので、受けた時に引く）。

use std::rc::Rc;

use areka_emo_text::place::PlaceKey;
use areka_emo_text::scroll_marker::WheelLines;
use areka_sakura::ActorKey;
use bevy_ecs::prelude::*;
use wintf::ecs::Input;
use wintf::ecs::find_owner_window;
use wintf::ecs::pointer::{PointerState, dispatch_pointer_events};

use crate::emo2_boot::frame::Emo2Wiring;
use crate::placement::spawn::BalloonWindowMarker;

/// `SPI_GETWHEELSCROLLLINES` が「1 画面ずつ」を表す値（`WHEEL_PAGESCROLL`・windows クレートに無い）。
const WHEEL_PAGESCROLL: u32 = u32::MAX;

/// 読めなかったときの行数（OS の既定）。
const DEFAULT_WHEEL_LINES: u32 = 3;

/// バルーンの窓のホイールの系を `Input` の段の `dispatch_pointer_events` の前へ 1 回だけ登録する
/// （呼び手は `ghost_session::register_systems`・プロセスに 1 回）。
pub(crate) fn register_balloon_wheel(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, balloon_wheel.before(dispatch_pointer_events));
}

/// 毎 tick の系（排他）。
pub(crate) fn balloon_wheel(world: &mut World) {
    balloon_wheel_with(world, os_wheel_lines);
}

/// 本体。`read_lines` は縦の回転を受けた tick に 1 回だけ呼ぶ（テストは数えて差し替える）。
pub(super) fn balloon_wheel_with(world: &mut World, read_lines: impl FnOnce() -> WheelLines) {
    // 持ち主（1 窓に高々 1 つ）の縦の回転をバルーン窓のスコープへ写す。横は読まない（要件 6.7）。
    let owners: Vec<(Entity, i16)> = world
        .query::<(Entity, &PointerState)>()
        .iter(world)
        .filter(|(_, s)| s.wheel.vertical != 0)
        .map(|(e, s)| (e, s.wheel.vertical))
        .collect();
    let rotations: Vec<(usize, i32)> = owners
        .into_iter()
        .filter_map(|(owner, vertical)| {
            let window = find_owner_window(world, owner)?;
            let scope = world.get::<BalloonWindowMarker>(window)?.scope;
            Some((scope, i32::from(vertical)))
        })
        .collect();
    if rotations.is_empty() {
        return;
    }
    let Some(runtime) = world
        .get_non_send::<Emo2Wiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "balloon_wheel_no_emo2",
            "Emo2Wiring 不在（boot 前／失敗）: バルーンのホイールを no-op 縮退"
        );
        return;
    };
    let per_notch = read_lines();
    let Ok(mut runtime) = runtime.try_borrow_mut() else {
        tracing::error!(
            event = "balloon_runtime_borrow_failed",
            "文字層ランタイムを借用できずバルーンのホイールを送れない → 本 tick は見送る"
        );
        return;
    };
    for (scope, delta) in rotations {
        let actor = ActorKey::from(scope.to_string());
        runtime.wheel_at(&PlaceKey::balloon(&actor), delta, per_notch);
    }
}

/// OS の「1 回にスクロールする行数」（`WHEEL_PAGESCROLL` は 1 画面ずつ）。
pub(super) fn os_wheel_lines() -> WheelLines {
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETWHEELSCROLLLINES, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };
    let mut lines: u32 = DEFAULT_WHEEL_LINES;
    // SAFETY: SPI_GETWHEELSCROLLLINES は pvparam へ u32 を 1 つ書くだけ。
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETWHEELSCROLLLINES,
            0,
            Some((&mut lines as *mut u32).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    if let Err(e) = read {
        tracing::warn!(
            event = "wheel_lines_read_failed",
            error = %e,
            "OS のホイールの行数を読めない → 既定の {DEFAULT_WHEEL_LINES} 行で送る"
        );
        return WheelLines::Lines(DEFAULT_WHEEL_LINES);
    }
    if lines == WHEEL_PAGESCROLL {
        WheelLines::Page
    } else {
        WheelLines::Lines(lines)
    }
}

#[cfg(test)]
#[path = "balloon_wheel_tests.rs"]
mod tests;
