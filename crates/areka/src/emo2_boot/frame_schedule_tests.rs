// =============================================================================
// T-N10（areka-P0-present-gpu-transform-scale task 4.2・要件 2.4／6.5）
// 毎フレームの相（`emo2_frame_system`）の**登録先**の決定論テスト
//
// 裁定（2026-09-12・設計ディスカッション 議題 1「描画の確定は Draw 前に終わらせる」）で
// 登録先は 1 巡の末尾の段（`FrameFinalize`）から上流の `Update` へ移った。相がそこで挿した
// 描画命令と配置を、同じ巡の伝播（`PostLayout`）・面の生成（`PreRenderSurface`）・描画
// （`RenderSurface`）が拾う——末尾の段に載せると絵の着地が次の巡へずれ、文字だけが 1 コマ
// 先に出る（要件 2.4）。
//
// この移動は挙動の檻に映らない。相の本体は `Emo2Wiring` が挿さっていない World では丸ごと
// 無操作であり、実窓を持たない檻から見ると「絵が 1 コマ遅れる」は原理的に観測できない。
// ゆえにここは 2 方向から押さえる。
//
// 1. 本番の登録の**字面**（`mod.rs` 手順 6 の 1 行）。登録先を旧に戻すとここが赤くなる。
// 2. その字面と同じ登録を実際に `Schedules` へ行い、当の相が `Update` に載り末尾の段には
//    載らないという**構造**（GPU も窓も要らない）。対照として、登録の前は `Update` に無い。
//
// 2 が本番の `wire_emo2_boot` を呼ばないのは、あの関数が実ゴーストの資産・UI アクター・
// SHIORI の子プロセスを要して決定論にならないためである。ゆえに登録の 1 行を写し取り、
// 写しが本番と食い違わないことを 1 の字面が見張る（写しの原本は [`BOOT_REGISTRATION`]）。
//
// 字面の走査は必ず**説明文を落とした本文**へ当てる。素の全文には本ファイルの説明の語も
// 相手ファイルの doc の語も入るので、当てる先を間違えると検査が恒真になる（各檻の末尾に
// 両方向の対照を置いてある）。
// =============================================================================

use std::sync::mpsc;

use areka_kanade::{KanadeStopCause, KanadeStopped};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, IntoSystemSet, ScheduleLabel, Schedules};
use wintf::ecs::{FrameFinalize, Update, update_typewriters};

use super::frame::{KanadeStopRx, emo2_frame_system, ghost_quit_system};
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::placement::spawn::wire_zorder_pair;

// ---------------------------------------------------------------- 道具立て

/// 本番（`wire_emo2_boot` 手順 6）の登録の字面。下の [`register_like_the_boot`] はこの写しであり、
/// 両者が食い違っていないことを [`t_n10_the_boot_registers_the_frame_system_into_update`] が見張る。
const BOOT_REGISTRATION: &str = "add_systems(Update, emo2_frame_system.after(update_typewriters));";

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 空白の連なりを 1 つに詰める（改行や字下げの入り方で檻が壊れないようにする）。
fn squeeze(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 指定の段に当の相が載っているか。
///
/// 判定は system の**型の集合**（bevy が関数 system ごとに自動で作る集合）が段のグラフに
/// 在るかどうかで行う。表示名の文字列は `bevy_ecs` の `debug` 機能が消えると
/// 「機能が無効」の定型文に化けるため、檻の判定材料にはしない。
fn schedule_holds_the_frame_system(world: &World, label: impl ScheduleLabel) -> bool {
    world
        .get_resource::<Schedules>()
        .and_then(|schedules| schedules.get(label))
        .is_some_and(|schedule| {
            schedule
                .graph()
                .system_sets
                .contains(emo2_frame_system.into_system_set())
        })
}

/// 本番の手順 6 と同じ登録を行う（[`BOOT_REGISTRATION`] の写し）。
fn register_like_the_boot(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, emo2_frame_system.after(update_typewriters));
}

/// 本番と同じ順序で段を組んだ World——鎖の適用系（末尾の段の 3 本）が**先に**載る。
///
/// 本番も `open_startup_window`（`wire_zorder_pair`）→ `wire_emo2_boot`（相の登録）の順である。
/// 末尾の段を空のままにすると「載らない」の主張が「段そのものが無い」で通ってしまうので、
/// ここで実際に 3 本を載せておく。
fn world_with_the_chain_apply_wired() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    wire_zorder_pair(&mut world);
    world
}

// ---------------------------------------------------------------------------
// ⑴ 本番の登録の字面——登録先を旧に戻すとここが赤くなる
// ---------------------------------------------------------------------------

/// 本番の結線は毎フレームの相を `Update` へ、上流の `Update` 鎖の最後の系より後に載せる
/// （裁定 2026-09-12・要件 2.4）。
///
/// 旧い形（末尾の段への登録・`.before` による段内順序指定）が残っていないことも同時に見る。
/// 片側だけでは、新旧が並んで書かれた状態を緑のまま通してしまう。
#[test]
fn t_n10_the_boot_registers_the_frame_system_into_update() {
    let raw = include_str!("mod.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains(BOOT_REGISTRATION),
        "毎フレームの相の登録が `Update`＋上流の鎖の後という形になっていない（絵の着地が次の巡へずれる・要件 2.4）: {BOOT_REGISTRATION}"
    );
    assert!(
        !squeezed.contains("add_systems(FrameFinalize, emo2_frame_system"),
        "旧い登録先（1 巡の末尾の段）が残っている"
    );
    assert!(
        !squeezed.contains("emo2_frame_system.before(apply_zorder_chain)"),
        "旧い段内の順序指定（`.before(apply_zorder_chain)`）が残っている（段をまたぐ順序は指定できない）"
    );
    // 写し（`register_like_the_boot`）が原本の字面と食い違っていないこと——原本だけ更新して
    // 写しを忘れると構造の檻が古い写しに対して緑のまま残るため、本ファイル自身も同じ字面を含む。
    assert!(
        squeeze(&code_only(include_str!("frame_schedule_tests.rs"))).contains(BOOT_REGISTRATION),
        "構造の檻の写し（register_like_the_boot）が BOOT_REGISTRATION と食い違っている"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("pub fn wire_emo2_boot("),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("完成済み 5 トラックのエンジン"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("完成済み 5 トラックのエンジン"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 構造——写した登録は `Update` に載り、末尾の段には載らない
// ---------------------------------------------------------------------------

/// 本番と同じ登録を行うと、当の相は `Update` に載り、鎖の適用系が居る末尾の段には載らない。
///
/// 末尾の段は空ではない（適用系ほか 3 本が先に載っている）ので、「載らない」の主張は
/// 「段そのものが無い」で成り立っているのではない。
#[test]
fn t_n10_the_registration_lands_the_frame_system_in_update_only() {
    let mut world = world_with_the_chain_apply_wired();

    register_like_the_boot(&mut world);

    assert!(
        schedule_holds_the_frame_system(&world, Update),
        "本番と同じ登録をしたのに相が `Update` に載っていない"
    );
    assert!(
        !schedule_holds_the_frame_system(&world, FrameFinalize),
        "相が 1 巡の末尾の段にも載っている（絵の着地が次の巡へずれる経路が残っている）"
    );

    let schedules = world.resource::<Schedules>();
    assert_eq!(
        schedules
            .get(Update)
            .expect("登録後は `Update` の段が在る")
            .systems_len(),
        1,
        "`Update` へ載るのは当の相ちょうど 1 本（順序指定の相手まで載ると相順の前提が変わる）"
    );
    assert_eq!(
        schedules
            .get(FrameFinalize)
            .expect("末尾の段は結線済み（確立系・ペア維持系・鎖の適用系）")
            .systems_len(),
        3,
        "末尾の段が空になっている＝「相が載らない」の主張が空虚（対照が成立していない）"
    );
}

/// 対照——登録の前は `Update` に何も無い。
///
/// これが無いと上の檻は「何をしても載っていると言う」形と区別が付かない。
#[test]
fn t_n10_before_the_registration_the_update_stage_holds_nothing() {
    let world = world_with_the_chain_apply_wired();

    assert!(
        !schedule_holds_the_frame_system(&world, Update),
        "登録の前から相が `Update` に載っている（判定が恒真）"
    );
    assert!(
        !world.resource::<Schedules>().contains(Update),
        "前提: 鎖の結線は `Update` の段を作らない（この World は wintf の既定システムを持たない）"
    );
}

// ---------------------------------------------------------------------------
// ⑶ 終了相の据え付け（areka-P0-shiori-fault-notice 3.2・要件 6.4・設計検証 論点 3）
// ---------------------------------------------------------------------------

/// 本番（`wire_kanade_stop`）の終了相の登録の字面。下のテストはこの写しを素の World で行う。
const QUIT_REGISTRATION: &str = "add_systems(Update, ghost_quit_system.before(emo2_frame_system));";

/// 毎フレームの相を登録しない World（＝LogSink 側の起動と同じ形）でも、終了相の登録は
/// bevy に受け入れられ、停止通知 1 件 → `Update` 1 回で終了が指示され最初の出所が残る。
///
/// 順序の相手（`emo2_frame_system`）が schedule に無い登録の形は bevy の挙動に依る 1 点なので、
/// 版が変わってこの形が拒まれたら（schedule の組み立てで落ちたら）ここが赤くなる。
/// 本番の登録から行ごと消す・順序指定を外すと字面の確認が赤くなる。
#[test]
fn ghost_quit_system_without_the_frame_system_quits_on_one_notice() {
    assert!(
        squeeze(&code_only(include_str!("mod.rs"))).contains(QUIT_REGISTRATION),
        "終了相の登録が本番の据え付けに無い、または順序指定が変わっている: {QUIT_REGISTRATION}"
    );

    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(wintf::AppExit::new());
    let (tx, rx) = mpsc::channel::<KanadeStopped>();
    world.insert_non_send(KanadeStopRx(rx));
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, ghost_quit_system.before(emo2_frame_system));
    assert_eq!(
        world
            .resource::<Schedules>()
            .get(Update)
            .expect("登録後は `Update` の段が在る")
            .systems_len(),
        1,
        "`Update` に載るのは終了相 1 本だけ（毎フレームの相は載せない）"
    );

    tx.send(KanadeStopped {
        cause: KanadeStopCause::Quit,
    })
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
