// =============================================================================
// 終わる相（`ghost_quit_system`）の据え付けの決定論テスト
//
// 毎フレームの相（`frame_system`）の登録先のテストは `areka-runtime` の
// `frame_schedule_tests.rs` が持つ。ここは本番の据え付けの関数 `wire_kanade_stop` を
// 空の World の `Schedules` へ直に呼び、終わる相が `Update` に載って停止通知 1 件で終了を
// 指示することを見る（GPU も窓も要らない）。
// =============================================================================

use std::sync::mpsc;

use areka_kanade::{KanadeNotice, KanadeStopCause, KanadeStopped};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::Schedules;
use wintf::ecs::Update;

use crate::app_exit::{ExitOrigin, FirstExit};
use crate::session::ghost_quit::wire_kanade_stop;

/// 毎フレームの相を登録しない World（順序の相手が居ない形）でも、`wire_kanade_stop` の登録は
/// bevy に受け入れられ、停止通知 1 件 → `Update` 1 回で終了が指示され最初の出所が残る。
///
/// 順序の相手（`frame_system`）が schedule に無い登録の形は bevy の挙動に依る 1 点なので、
/// 版が変わってこの形が拒まれたら（schedule の組み立てで落ちたら）ここが赤くなる。
/// 受け口の据え付けも `wire_kanade_stop` に任せるので、据えなくなれば終了が指示されず赤くなる。
#[test]
fn ghost_quit_system_without_the_frame_system_quits_on_one_notice() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(wintf::AppExit::new());
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    wire_kanade_stop(&mut world, rx);
    assert_eq!(
        world
            .resource::<Schedules>()
            .get(Update)
            .expect("登録後は `Update` の段が在る")
            .systems_len(),
        1,
        "`Update` に載るのは終了相 1 本だけ（毎フレームの相は載せない）"
    );

    tx.send(KanadeNotice::Stopped(KanadeStopped {
        cause: KanadeStopCause::Quit,
        handoff: None,
    }))
    .expect("停止通知を投函できる");
    world.run_schedule(Update);

    assert!(
        world
            .get_non_send::<wintf::AppExit>()
            .expect("受け口を挿してある")
            .is_requested(),
        "停止通知 1 件 → `Update` 1 回で終了が指示される"
    );
    assert_eq!(
        world.get_resource::<FirstExit>().map(|f| f.0.clone()),
        Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit)),
        "最初の出所が残る"
    );
}
