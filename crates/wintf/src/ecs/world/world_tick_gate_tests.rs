//! tick の門を実際の駆動へ組み込んだ部分の決定論テスト（task 3.3・要件 3.2/3.4/3.7/
//! 4.4/4.5/6.2/6.3）。
//!
//! 判断そのもの（純関数 `should_run`）は兄弟の `tick_gate_tests.rs` が全組合せで
//! 固定している。ここで見るのはその**手前と後始末**——旗を読んで倒し、数を数え、
//! 省略のときに何を進めないか、である。
//!
//! 実時間の閾値は 1 つも使わない（要件 6.5）。旗は原則として引数で注入し、プロセス
//! 共有の旗を触るのは 1 本だけで、その 1 本は錠 [`TICK_WAKE_TEST_LOCK`] を取ってから
//! 触る（同じ錠を `runtime/tick_bridge.rs` の兄弟テストも取る）。共有の旗は他のテスト
//! からも立ちうるので、共有経由の検査は「回る」側しか主張しない。

use super::*;

use crate::ecs::graphics::FrameTime;
use crate::ecs::window::transition_diag::TickStart;
use std::time::Instant;

use super::tick_diag::SCHEDULE_LABELS;
use super::tick_gate::{RunReason, TICK_GATE_WARMUP_FRAMES, TICK_HEARTBEAT_FRAMES, TickDecision};
use super::tick_wake::{PRESENT, WakeSnapshot};

// ------------------------------------------------------------------ 小道具

/// 旗も期限も無い読み取り結果（省略へ倒れる唯一の入力）。
fn no_wake() -> WakeSnapshot {
    WakeSnapshot {
        bits: 0,
        deadline_due: false,
    }
}

/// 起動直後の全走ぶんだけ判定を進める（回さずに数だけ進める）。
///
/// 起動直後は門が有効でも無条件に回るので、省略を観測するにはまずここを抜ける必要が
/// ある。抜けた直後の「前回回した tick からの回数」は 0 である（全部回ったため）。
fn fast_forward_warmup(ecs: &mut EcsWorld) {
    for i in 0..TICK_GATE_WARMUP_FRAMES {
        assert_eq!(
            ecs.decide_tick_with(no_wake()),
            TickDecision::Run(RunReason::Warmup),
            "起動直後の {i} 回目は無条件で回すべき"
        );
    }
}

/// 13 本のスケジュールが実行順に自分の名前を積む記録資源。
#[derive(Resource, Default)]
struct TickOrderLog(Vec<&'static str>);

/// 13 本すべてへ記録プローブを 1 本ずつ差す。
fn install_order_probes(ecs: &mut EcsWorld) {
    macro_rules! probe {
        ($label:expr, $name:literal) => {
            ecs.add_systems($label, |mut log: ResMut<TickOrderLog>| {
                log.0.push($name);
            });
        };
    }
    probe!(Input, "Input");
    probe!(Update, "Update");
    probe!(PreLayout, "PreLayout");
    probe!(Layout, "Layout");
    probe!(PostLayout, "PostLayout");
    probe!(UISetup, "UISetup");
    probe!(GraphicsSetup, "GraphicsSetup");
    probe!(Draw, "Draw");
    probe!(PreRenderSurface, "PreRenderSurface");
    probe!(RenderSurface, "RenderSurface");
    probe!(Composition, "Composition");
    probe!(CommitComposition, "CommitComposition");
    probe!(FrameFinalize, "FrameFinalize");
}

/// 画面更新 1 回ぶんを本番と同じ形で回す（`tick_one_frame` の中身と同じ分岐）。
fn one_frame(ecs: &mut EcsWorld, snapshot: WakeSnapshot) -> TickDecision {
    let decision = ecs.decide_tick_with(snapshot);
    if decision.is_run() {
        ecs.try_tick_world();
    } else {
        ecs.note_skipped_tick();
    }
    decision
}

// ------------------------------------------------------- 既定は「門を切る」

/// (要件 3.2) 門の既定は無効で、判定は何回呼んでも「回す」に倒れる。
///
/// 周 1 の A/B までは既定 OFF＝門を入れる前と同じ挙動、が仕様である（設計 C16）。
#[test]
fn tick_gate_is_disabled_by_default_and_every_decision_runs() {
    let mut ecs = EcsWorld::new();
    assert!(
        !ecs.tick_gate_enabled(),
        "門の既定は無効（周 1 の A/B で採否を決めるまで）"
    );

    // 起動直後を抜け、心拍の間隔も跨いでなお「門が無効」の理由で回り続ける。
    for i in 0..(TICK_GATE_WARMUP_FRAMES + TICK_HEARTBEAT_FRAMES + 5) {
        assert_eq!(
            ecs.decide_tick_with(no_wake()),
            TickDecision::Run(RunReason::Disabled),
            "門が無効なら {i} 回目も判断せず回すべき"
        );
    }
}

/// (要件 3.2) 門の有効は起動時に切り替えられ、読み出せる。
#[test]
fn set_tick_gate_toggles_and_reads_back() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    assert!(ecs.tick_gate_enabled());
    ecs.set_tick_gate(false);
    assert!(!ecs.tick_gate_enabled());
}

// ------------------------------------------------------------ 数の数え方

/// (要件 3.2) 起動からの回数は省略した回も含めて進み、前回回した tick からの回数は
/// 回したときだけ 0 に戻る。
#[test]
fn counters_advance_on_every_frame_and_reset_only_on_run() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    fast_forward_warmup(&mut ecs);

    let (since_boot, since_run) = ecs.tick_gate_counters();
    assert_eq!(
        since_boot, TICK_GATE_WARMUP_FRAMES,
        "起動直後の全走ぶんだけ起動からの回数が進んでいるべき"
    );
    assert_eq!(since_run, 0, "全部回ったので前回実行からの回数は 0");

    // 省略が 3 回続くと、起動からの回数も前回実行からの回数も 3 進む。
    for _ in 0..3 {
        assert_eq!(ecs.decide_tick_with(no_wake()), TickDecision::Skip);
    }
    assert_eq!(
        ecs.tick_gate_counters(),
        (TICK_GATE_WARMUP_FRAMES + 3, 3),
        "省略でも起動からの回数は進み、前回実行からの回数も積み上がる"
    );

    // 旗が立てば回り、前回実行からの回数だけが 0 に戻る。
    assert!(
        ecs.decide_tick_with(WakeSnapshot {
            bits: PRESENT.bits(),
            deadline_due: false,
        })
        .is_run()
    );
    assert_eq!(
        ecs.tick_gate_counters(),
        (TICK_GATE_WARMUP_FRAMES + 4, 0),
        "回した回で前回実行からの回数は 0 へ戻る"
    );
}

/// (要件 3.2) 起動直後を抜けたあと、旗も期限も無ければ省略になる。
#[test]
fn gate_skips_after_warmup_when_nothing_asks_for_a_run() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    fast_forward_warmup(&mut ecs);

    assert_eq!(
        ecs.decide_tick_with(no_wake()),
        TickDecision::Skip,
        "起動直後を抜けて旗も期限も無ければ省略"
    );
}

/// (要件 3.2) 旗も期限も無いまま [`TICK_HEARTBEAT_FRAMES`] 回省略したら、次は心拍で回る。
///
/// 心拍は「前回**回した** tick からの回数」で測るので、省略が 30 回続いた次
/// （＝入力の `frames_since_run` が 30 に達した回）が心拍である。
#[test]
fn heartbeat_runs_after_the_configured_number_of_skips() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    fast_forward_warmup(&mut ecs);

    for i in 0..TICK_HEARTBEAT_FRAMES {
        assert_eq!(
            ecs.decide_tick_with(no_wake()),
            TickDecision::Skip,
            "{i} 回目はまだ心拍に届かない"
        );
    }
    assert_eq!(
        ecs.decide_tick_with(no_wake()),
        TickDecision::Run(RunReason::Heartbeat),
        "心拍の間隔に達したら理由が無くても 1 回回す"
    );
    assert!(
        ecs.last_run_was_heartbeat(),
        "心拍で回した印が立つ（観測行の heartbeat= に載る）"
    );

    // 回したら印は倒れる——門を通さない呼び出しが前回の印を引き継がない。
    ecs.try_tick_world();
    assert!(
        !ecs.last_run_was_heartbeat(),
        "回し終えたら心拍の印は倒れているべき"
    );
}

// --------------------------------------------------- 省略が遅れとして残らない

/// (要件 6.2) 省略 → 表示指令の旗 → 次の判定が回す。
#[test]
fn skip_then_present_flag_makes_the_next_decision_run() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    fast_forward_warmup(&mut ecs);

    assert_eq!(ecs.decide_tick_with(no_wake()), TickDecision::Skip);

    let decision = ecs.decide_tick_with(WakeSnapshot {
        bits: PRESENT.bits(),
        deadline_due: false,
    });
    assert_eq!(
        decision,
        TickDecision::Run(RunReason::Wake(PRESENT.bits())),
        "省略の直後に表示指令の旗が立てば、次の判定は回す（遅れは 1 画面更新まで）"
    );
}

/// (要件 6.2) プロセス共有の旗を通しても同じことが成り立つ。
///
/// 共有の旗は他のテストからも立ちうるので、主張するのは「回る」側だけである
/// （「省略になる」は共有経由では主張しない）。
#[test]
fn present_flag_through_the_shared_registry_makes_the_decision_run() {
    let _lock = TICK_WAKE_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    fast_forward_warmup(&mut ecs);

    // 溜まっている旗を一度倒してから、表示指令の旗だけを立てる。
    let _ = tick_wake::take(Instant::now());
    tick_wake::mark(PRESENT);

    let decision = ecs.decide_tick(Instant::now());
    match decision {
        TickDecision::Run(RunReason::Wake(bits)) => assert!(
            bits & PRESENT.bits() != 0,
            "立てた表示指令の旗が判断の理由に含まれるべき（bits={bits:#x}）"
        ),
        other => panic!("表示指令の旗が立っていれば回すべき: {other:?}"),
    }
}

// ------------------------------------------- 省略で何を進めないか（要件 3.2）

/// (要件 3.2/6.2) 省略の回は `FrameCount`／`FrameTime`／`TickStart` を進めない。
/// 省略のあとに回せば、フレーム番号はちょうど 1 だけ進む。
#[test]
fn skipped_frames_advance_neither_frame_count_nor_frame_time_nor_tick_start() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    fast_forward_warmup(&mut ecs);

    // 基準を作るために 1 回回す。
    ecs.try_tick_world();
    let frame = ecs.world().resource::<FrameCount>().0;
    let frame_time = ecs.world().resource::<FrameTime>().0;
    let tick_start = ecs.world().resource::<TickStart>().0;

    for i in 0..5 {
        assert_eq!(
            ecs.decide_tick_with(no_wake()),
            TickDecision::Skip,
            "{i} 回目は省略のはず"
        );
        ecs.note_skipped_tick();
    }

    assert_eq!(
        ecs.world().resource::<FrameCount>().0,
        frame,
        "省略の回でフレーム番号は進まない"
    );
    assert_eq!(
        ecs.world().resource::<FrameTime>().0,
        frame_time,
        "省略の回で表示時刻は進まない"
    );
    assert_eq!(
        ecs.world().resource::<TickStart>().0,
        tick_start,
        "省略の回で tick の起点時刻は進まない"
    );

    ecs.try_tick_world();
    assert_eq!(
        ecs.world().resource::<FrameCount>().0,
        frame + 1,
        "省略のあとに回せばフレーム番号はちょうど 1 進む"
    );
}

/// 変化検出の観測に使う印（値を書き換えると `Changed` が立つ）。
#[derive(Component)]
struct ChangeProbe(u32);

/// `Changed<ChangeProbe>` に当たった entity を積む記録資源。
#[derive(Resource, Default)]
struct ChangedSeen(Vec<Entity>);

/// (要件 6.2) 省略を挟んでも変化検出は消えない——省略中に書き換えた値は、次に回した
/// tick で `Changed` として拾える。
///
/// 省略の回はどのスケジュールも走らないので bevy の変化の刻みが進まない。したがって
/// 「省略したせいで変化を取りこぼす」ことは原理的に起きない、というのがここの主張である。
#[test]
fn changes_made_during_skipped_frames_are_seen_by_the_next_run() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);

    ecs.world_mut().insert_resource(ChangedSeen::default());
    let entity = ecs.world_mut().spawn(ChangeProbe(0)).id();
    ecs.add_systems(
        Update,
        |mut seen: ResMut<ChangedSeen>, q: Query<Entity, Changed<ChangeProbe>>| {
            for e in q.iter() {
                seen.0.push(e);
            }
        },
    );

    fast_forward_warmup(&mut ecs);

    // 追加そのものも変化なので、まず 1 回回して記録を空にする（基準合わせ）。
    ecs.try_tick_world();
    ecs.world_mut().resource_mut::<ChangedSeen>().0.clear();

    // 省略の最中に値を書き換える。
    ecs.world_mut()
        .get_mut::<ChangeProbe>(entity)
        .expect("印を付けた entity は生きているはず")
        .0 = 1;

    for _ in 0..3 {
        assert_eq!(ecs.decide_tick_with(no_wake()), TickDecision::Skip);
        ecs.note_skipped_tick();
    }
    assert!(
        ecs.world().resource::<ChangedSeen>().0.is_empty(),
        "省略の回はスケジュールを走らせないので、まだ誰も拾っていない"
    );

    ecs.try_tick_world();
    assert_eq!(
        ecs.world().resource::<ChangedSeen>().0,
        vec![entity],
        "省略を挟んでも、次に回した tick で変化がちょうど 1 件拾えるべき"
    );
}

// ------------------------------------------- 省略経路でも 13 本の順序は不変

/// (要件 3.4/6.3) 省略と全走が交互に来ても、回した tick の 13 本は毎回同じ順序で走る。
#[test]
fn skipped_frames_do_not_disturb_the_thirteen_schedule_order() {
    let mut ecs = EcsWorld::new();
    ecs.set_tick_gate(true);
    ecs.world_mut().insert_resource(TickOrderLog::default());
    install_order_probes(&mut ecs);

    fast_forward_warmup(&mut ecs);

    // 省略 2 回 → 表示指令で全走 → 省略 1 回 → 明示の全走要求、という並び。
    let present = WakeSnapshot {
        bits: PRESENT.bits(),
        deadline_due: false,
    };
    let force = WakeSnapshot {
        bits: tick_wake::FORCE.bits(),
        deadline_due: false,
    };
    assert_eq!(one_frame(&mut ecs, no_wake()), TickDecision::Skip);
    assert_eq!(one_frame(&mut ecs, no_wake()), TickDecision::Skip);
    assert!(one_frame(&mut ecs, present).is_run());
    assert_eq!(one_frame(&mut ecs, no_wake()), TickDecision::Skip);
    assert!(one_frame(&mut ecs, force).is_run());

    let order = ecs.world().resource::<TickOrderLog>().0.clone();
    assert_eq!(
        order.len(),
        SCHEDULE_LABELS.len() * 2,
        "回ったのは 2 回だけ（省略の回は 1 本も走らない）"
    );
    assert_eq!(
        &order[..SCHEDULE_LABELS.len()],
        &SCHEDULE_LABELS[..],
        "1 回目の 13 本は固定順のはず"
    );
    assert_eq!(
        &order[SCHEDULE_LABELS.len()..],
        &SCHEDULE_LABELS[..],
        "省略を挟んだ 2 回目も同じ固定順のはず"
    );
}
