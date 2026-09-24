//! 起こし直しの単位（areka-P0-ghost-restart-unit）。
//!
//! ゴーストを 1 体起こすのに要る手順を、**プロセスに 1 回**のもの（系の登録）と
//! **ゴーストごと**のもの（状態の載せ替え）に分けて置く場所である。系はどれも状態
//! （`NonSend`／`Resource`）が無ければ無操作で戻る自己防御を持つので、登録を先に 1 度だけ
//! 済ませ、状態は後から入れ替えても系は新しい状態を見る。
//!
//! 今ここに在るのは登録の入口 [`register_systems`] だけで、呼び手は `fn main` の 1 か所
//! （`WinApp` を作った直後・起動窓の前）である。各結線（`wire_*`）は状態の挿入だけを行い、
//! 系を登録しない。

use std::sync::mpsc::Receiver;

use areka_kanade::KanadeStopped;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use wintf::ecs::FrameFinalize;

use crate::app_exit;
use crate::emo2_boot;
use crate::input_events;
use crate::menu;
use crate::placement;
use crate::readme;

/// 系の登録をプロセスに 1 回・1 か所から行う（要件 2.1・2.2・3.3）。
///
/// 載せる系と順序（段ごとに今日の挿入順を保つ）:
/// - `Update` ← 毎フレームの相 → 停止通知の受け口（受け口 `KanadeStopRx` はプロセスに 1 つ
///   なので [`emo2_boot::wire_kanade_stop`] を分割せずそのまま呼ぶ）
/// - `Input` ← 説明書 → 中断 → メニュー → バルーンの離脱 → 選択肢の送り
/// - `FrameFinalize` ← クリック透過 → OS の閉鎖要求 → 重なり順の対（状態がゴーストごとで
///   ないので `wire_zorder_pair` をそのまま呼ぶ）
///
/// 各系の並び（`before`／`after`・`chain`）は各登録関数が持つ。ここは順に呼ぶだけ。
///
/// 今日との差は 1 件: `Input` の 5 系は LogSink の起動でも登録される。5 系はどれも状態が
/// 無ければ `trace!` だけで戻るので、見え方は変わらない。
pub(crate) fn register_systems(world: &mut World, kanade_stop_rx: Receiver<KanadeStopped>) {
    emo2_boot::register_emo2_frame_system(world);
    emo2_boot::wire_kanade_stop(world, kanade_stop_rx);

    readme::register_readme_drain(world);
    input_events::user_break::register_user_break_drain(world);
    menu::register_menu_poll(world);
    input_events::balloon::register_balloon_leave_system(world);
    input_events::choice_drain::register_choice_drain(world);

    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        placement::spawn::register_ghost_windows_click_through,
    );
    world
        .resource_mut::<Schedules>()
        .add_systems(FrameFinalize, app_exit::attach_os_close_request);
    placement::spawn::wire_zorder_pair(world);
}
