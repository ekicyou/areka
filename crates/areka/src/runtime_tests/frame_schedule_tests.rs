// =============================================================================
// T-N10（areka-P0-present-gpu-transform-scale task 4.2・要件 2.4／6.5）
// 毎フレームの相（`frame_system`）の**登録先**の決定論テスト
//
// 裁定（2026-09-12・設計ディスカッション 議題 1「描画の確定は Draw 前に終わらせる」）で
// 登録先は 1 巡の末尾の段（`FrameFinalize`）から上流の `Update` へ移った。相がそこで挿した
// 描画命令と配置を、同じ巡の伝播（`PostLayout`）・面の生成（`PreRenderSurface`）・描画
// （`RenderSurface`）が拾う——末尾の段に載せると絵の着地が次の巡へずれ、文字だけが 1 コマ
// 先に出る（要件 2.4）。
//
// この移動は挙動の檻に映らない。相の本体は `RuntimeWiring` が挿さっていない World では丸ごと
// 無操作であり、実窓を持たない檻から見ると「絵が 1 コマ遅れる」は原理的に観測できない。
// ゆえにここは 2 方向から押さえる。
//
// 1. 本番の登録の**字面**（`mod.rs` 手順 6 の 1 行）。登録先を旧に戻すとここが赤くなる。
// 2. その字面と同じ登録を実際に `Schedules` へ行い、当の相が `Update` に載り末尾の段には
//    載らないという**構造**（GPU も窓も要らない）。対照として、登録の前は `Update` に無い。
//
// 2 が本番の `wire_runtime` を呼ばないのは、あの関数が実ゴーストの資産・UI アクター・
// SHIORI の子プロセスを要して決定論にならないためである。ゆえに登録の 1 行を写し取り、
// 写しが本番と食い違わないことを 1 の字面が見張る（写しの原本は [`BOOT_REGISTRATION`]）。
//
// 字面の走査は必ず**説明文を落とした本文**へ当てる。素の全文には本ファイルの説明の語も
// 相手ファイルの doc の語も入るので、当てる先を間違えると検査が恒真になる（各檻の末尾に
// 両方向の対照を置いてある）。
// =============================================================================

use std::sync::mpsc;

use areka_kanade::{KanadeNotice, KanadeStopCause, KanadeStopped};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use wintf::ecs::Update;

use super::source_text_test_support::{code_only, squeeze};
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::session::ghost_quit::{KanadeNoticeRx, ghost_quit_system};
use areka_runtime::frame::frame_system;

// ---------------------------------------------------------------------------
// ⑶ 終了相の据え付け（areka-P0-shiori-fault-notice 3.2・要件 6.4・設計検証 論点 3）
// ---------------------------------------------------------------------------

/// 本番（`wire_kanade_stop`）の終了相の登録の字面。下のテストはこの写しを素の World で行う。
const QUIT_REGISTRATION: &str = "add_systems(Update, ghost_quit_system.before(frame_system));";

/// 毎フレームの相を登録しない World（順序の相手が居ない形）でも、終了相の登録は
/// bevy に受け入れられ、停止通知 1 件 → `Update` 1 回で終了が指示され最初の出所が残る。
///
/// 順序の相手（`frame_system`）が schedule に無い登録の形は bevy の挙動に依る 1 点なので、
/// 版が変わってこの形が拒まれたら（schedule の組み立てで落ちたら）ここが赤くなる。
/// 本番の登録から行ごと消す・順序指定を外すと字面の確認が赤くなる。
#[test]
fn ghost_quit_system_without_the_frame_system_quits_on_one_notice() {
    assert!(
        squeeze(&code_only(include_str!("../session/ghost_quit.rs"))).contains(QUIT_REGISTRATION),
        "終了相の登録が本番の据え付けに無い、または順序指定が変わっている: {QUIT_REGISTRATION}"
    );

    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(wintf::AppExit::new());
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    world.insert_non_send(KanadeNoticeRx(rx));
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, ghost_quit_system.before(frame_system));
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
