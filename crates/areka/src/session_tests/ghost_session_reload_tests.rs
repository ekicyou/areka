//! 名前の差し替えの口の決定論テスト（本体の系を登記した World で起こす）。

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::AppExit;

use super::ghost_session_restart_tests::{boot_round, shutdown_bounded};
use crate::sample_test_support::acquire_emo2;
use areka_kanade::KanadeNotice;
use areka_parsers::package::GhostNames;
use areka_session::ghost_session::*;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use std::path::PathBuf;
use std::sync::mpsc;

/// 名前の差し替え: 実行系があれば委ねて真を返し、`names()` が新しい名前を読む。
/// 実行系が無ければ偽。束は結線ありの腕だけが持つ（テスト用の組み立ては `None`）。
#[test]
fn set_names_delegates_to_runtime_or_returns_false() {
    let mut names = GhostNames::default();
    names.name = Some("読み直した名前".to_owned());

    let mut bare = GhostSession::for_test(None, PathBuf::from("ghost_session_reload_tests/根"));
    let bare_result = (bare.set_names(names.clone()), bare.sink_sources().is_none());

    // SAFETY: 資産の焼き込み（WIC）に要る COM 初期化（既初期化の S_FALSE 等は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(AppExit::new());
    let (kanade_stop, kanade_stop_rx) = mpsc::channel::<KanadeNotice>();
    register_systems(&mut world, kanade_stop_rx, crate::BODY_SYSTEMS);
    let sample = acquire_emo2();
    let (mut session, shiori) = boot_round(&mut world, &sample, kanade_stop);
    let replaced = session.set_names(names.clone());
    let read_back = session.names().cloned();
    shutdown_bounded("名前を替えた単位を降ろす", session, &shiori);

    assert_eq!(
        (bare_result, replaced, read_back),
        ((false, true), true, Some(names)),
        "名前の差し替えが実行系へ委ねられていない（実行系なし (偽・束なし)・実行系あり (真・読み返した名前)）"
    );
}
