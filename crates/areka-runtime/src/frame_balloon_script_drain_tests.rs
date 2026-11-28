//! 台本のバルーンの基準点の指令が move drain から当て口を通ってバルーンの位置まで届く檻。
//!
//! 着地の前に届いた指令は仮の大きさで置かれ、窓の大きさが着地した巡の move drain の頭で
//! 着地した大きさに解き直される。巡は `frame_system` の窓寸の照合・再スナップ・move drain の順で回す
//! （本番の並びそのものは `frame_move_order_tests.rs` が `frame.rs` の本文で押さえる）。

use std::collections::BTreeMap;

use areka_placement::reset_balloon_pos::{BalloonResetTrigger, reset_balloon_positions};
use areka_placement::script_balloon::{
    OffsetAxis, ScriptAlign, ScriptBalloonUnsettled, ScriptOffset,
};
use wintf::ecs::Point;

use crate::move_cue::PositionCue;

use super::move_hold::ShownRatioSource;
use super::reset_hold_test_support::SettledSource;
use super::shared_test_support::{steady_harness, wiring};
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, all_shown,
    placement_basis, pos_of, size_of,
};
use super::{RuntimeWiring, frame_system, run_move_drain_with};

/// スコープ 0 の最初の面の本当の大きさ（窓を作ったときの 434×687 より狭い）。
const REAL_0: (u32, u32) = (330, 687);

const SHIFT: Point = Point { x: 40, y: 30 };

fn right_of_body() -> PositionCue {
    PositionCue::SetBalloonAlign {
        scope: 0,
        align: ScriptAlign::Right,
    }
}

fn shifted_toward_body() -> PositionCue {
    PositionCue::SetBalloonOffset {
        scope: 0,
        offset: ScriptOffset {
            x: OffsetAxis::Absolute(SHIFT.x),
            y: OffsetAxis::Absolute(SHIFT.y),
        },
    }
}

fn script_harness() -> FrameHarness {
    let mut harness = steady_harness();
    harness
        .world
        .insert_resource(placement_basis(BTreeMap::new(), 2));
    // 檻の窓は記憶から戻したずらしで作られる。標準の位置から始め、利用者のずらしを持ち込まない。
    reset_balloon_positions(&mut harness.world, BalloonResetTrigger::Script);
    let _reset = harness.drain_writes();
    harness
}

/// 右に出すバルーンの台本の位置（本体の右隣から、正の x は本体の側）を、今のキャラ窓から求める。
fn script_position(harness: &FrameHarness) -> Point {
    let char_window = harness.char_window(0);
    let at = pos_of(&harness.world, char_window).expect("キャラ窓の位置がある");
    let size = size_of(&harness.world, char_window).expect("キャラ窓の大きさがある");
    Point {
        x: at.x + size.width - SHIFT.x,
        y: at.y + SHIFT.y,
    }
}

fn balloon_at(harness: &FrameHarness) -> Point {
    pos_of(&harness.world, harness.balloon_window(0)).expect("バルーンの位置がある")
}

fn unsettled(harness: &FrameHarness) -> bool {
    harness
        .world
        .entity(harness.char_window(0))
        .contains::<ScriptBalloonUnsettled>()
}

fn send_script(tx: &std::sync::mpsc::Sender<PositionCue>) {
    for cue in [right_of_body(), shifted_toward_body()] {
        tx.send(cue).expect("送出は成功する");
    }
}

/// `frame_system` の窓寸の照合・再スナップ・move drain を本番の順で 1 巡ぶん回す。
fn pass<S: ShownRatioSource>(
    harness: &mut FrameHarness,
    wiring: &RuntimeWiring,
    sizes: &PerTargetSizes,
    shown: &S,
) {
    harness.advance_frame();
    harness.run_reconcile(&mut FakeReports::default());
    harness.run_resnap(sizes);
    run_move_drain_with(wiring, shown, &mut harness.world);
}

fn spawn_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, Some(SPAWN_SIZE_1))])
}

fn real_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(REAL_0)), (1, Some(SPAWN_SIZE_1))])
}

#[test]
fn a_directive_drained_once_is_written_to_the_balloon_position() {
    let mut harness = script_harness();
    let (tx, wiring) = wiring();
    send_script(&tx);

    run_move_drain_with(&wiring, &all_shown, &mut harness.world);

    assert_eq!(balloon_at(&harness), script_position(&harness));
    assert!(!unsettled(&harness), "着地したスコープに印が残った");
}

#[test]
fn a_directive_before_landing_is_resolved_again_at_the_landed_size_in_the_landing_pass() {
    let mut harness = script_harness();
    let (tx, wiring) = wiring();
    let shown = SettledSource::new().shown(0, REAL_0);
    send_script(&tx);

    pass(&mut harness, &wiring, &spawn_sizes(), &shown);
    assert_eq!(
        balloon_at(&harness),
        script_position(&harness),
        "着地の前の巡に仮の大きさの台本の位置へ置かれない"
    );
    assert!(unsettled(&harness), "前提: 着地の前の当てに印が付く");

    pass(&mut harness, &wiring, &spawn_sizes(), &shown);
    assert!(unsettled(&harness), "着地していない巡に印が外れた");

    pass(&mut harness, &wiring, &real_sizes(), &shown);
    assert_eq!(
        size_of(&harness.world, harness.char_window(0)).map(|s| (s.width, s.height)),
        Some((REAL_0.0 as i32, REAL_0.1 as i32)),
        "前提: この巡に窓の大きさが着地した"
    );
    assert_eq!(
        balloon_at(&harness),
        script_position(&harness),
        "着地した巡に着地した大きさで解き直されない"
    );
    assert!(!unsettled(&harness), "着地した巡に印が残った");
}

/// 起動の最初の台本の指令と台詞が同じ巡に届く。台詞の面がその巡に表示し、窓寸の照合もその巡に済む。
#[test]
fn the_first_directive_and_dialogue_in_one_pass_land_the_balloon_in_that_pass() {
    let mut harness = script_harness();
    let (tx, wiring) = wiring();
    send_script(&tx);

    pass(
        &mut harness,
        &wiring,
        &real_sizes(),
        &SettledSource::new().shown(0, REAL_0),
    );

    assert_eq!(balloon_at(&harness), script_position(&harness));
    assert!(!unsettled(&harness), "着地した巡に印が残った");
}

/// 同じ形で、窓寸の照合だけが次の巡へ回る。見えた巡は仮の大きさの台本の位置、照合の巡に着地した大きさの位置。
#[test]
fn the_first_directive_and_dialogue_land_when_the_size_reconciliation_moves_to_the_next_pass() {
    let mut harness = script_harness();
    let (tx, wiring) = wiring();
    let shown = SettledSource::new().shown(0, REAL_0);
    send_script(&tx);

    pass(&mut harness, &wiring, &spawn_sizes(), &shown);
    let provisional = script_position(&harness);
    assert_eq!(
        balloon_at(&harness),
        provisional,
        "見えた巡に台本の位置に無い"
    );

    pass(&mut harness, &wiring, &real_sizes(), &shown);

    let landed = script_position(&harness);
    assert_ne!(landed, provisional, "前提: 着地で台本の位置が変わる");
    assert_eq!(
        balloon_at(&harness),
        landed,
        "照合の巡に着地した大きさで置かれない"
    );
}

/// 本番の `frame_system` の 1 回で、届いた指令がバルーンの位置まで書かれる（表示の層は未装着で、着地の前として当たる）。
#[test]
fn the_production_frame_system_writes_a_directive_in_the_pass_it_arrives() {
    let mut harness = script_harness();
    let (tx, wiring) = wiring();
    harness.world.insert_non_send(wiring);
    send_script(&tx);

    frame_system(&mut harness.world);

    assert_eq!(balloon_at(&harness), script_position(&harness));
    assert!(
        unsettled(&harness),
        "未装着の表示の層は着地を答えない（着地の前の当て）"
    );
}
