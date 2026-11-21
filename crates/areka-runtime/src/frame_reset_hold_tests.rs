//! 並べ直しの預かりの檻。
//!
//! 並べ直しは、窓のある全部のスコープの表示が決まるまで預かり、本当の大きさで一度だけ当てる。
//! 最初の面の前に当てると仮の大きさで解き、再スナップが下端の中央を保って縮めるので先頭の窓が
//! 作業領域の右端から離れ、起動の確定がもう一度並べ直す。

use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

use areka_emo_present::NeverShown;
use wintf::ecs::Point;

use crate::move_cue::{PositionCue, parse_move_directive};

use super::RuntimeWiring;

use super::reset_hold_test_support::SettledSource;
use super::run_move_drain_with;
use super::shared_test_support::wiring;
use super::test_support::{
    FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, capture_logs, placement_basis,
    pos_of, size_of,
};

/// 仮の大きさ（窓を作ったときの大きさ）より狭い、最初の面の本当の大きさ。
const REAL: [(u32, u32); 2] = [(330, 687), (230, 357)];
const WORK_AREA_RIGHT: i32 = 2574;
const FINALIZE_MOVED: &str = "chain_finalize: 実表示寸で連鎖を再解決";

struct Run {
    harness: FrameHarness,
    round1_logs: Vec<String>,
    finalize_logs: Vec<String>,
    before_finalize: [Option<Point>; 2],
}

impl Run {
    fn x(&self, scope: usize) -> i32 {
        pos_of(&self.harness.world, self.harness.char_window(scope))
            .expect("位置がある")
            .x
    }

    fn width(&self, scope: usize) -> i32 {
        size_of(&self.harness.world, self.harness.char_window(scope))
            .expect("大きさがある")
            .width
    }

    fn positions(&self) -> [Option<Point>; 2] {
        [0, 1].map(|s| pos_of(&self.harness.world, self.harness.char_window(s)))
    }
}

/// 巡 1: どちらも未表示で並べ直しを流す。巡 2: 両方が本当の大きさで表示し、再スナップ →
/// 位置の指令の相 → 起動の確定の順に回す。
fn boot_with_early_reset() -> Run {
    let mut harness = FrameHarness::new();
    harness.set_monitor_table_for_dpi(96);
    harness
        .world
        .insert_resource(placement_basis(BTreeMap::new(), 2));
    let (tx, wiring) = wiring();
    harness.advance_frame();
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");

    let round1_logs = capture_logs(|| {
        run_move_drain_with(&wiring, &SettledSource::new(), &mut harness.world);
    });

    harness.advance_frame();
    let real = PerTargetSizes::new([(0, Some(REAL[0])), (1, Some(REAL[1]))]);
    let shown = SettledSource::new().shown(0, REAL[0]).shown(1, REAL[1]);
    harness.run_resnap(&real);
    run_move_drain_with(&wiring, &shown, &mut harness.world);
    let before_finalize = [0, 1].map(|s| pos_of(&harness.world, harness.char_window(s)));
    let finalize_logs = capture_logs(|| harness.run_chain_finalize(&real));

    Run {
        harness,
        round1_logs,
        finalize_logs,
        before_finalize,
    }
}

/// 先頭の窓の右端が作業領域の右端にそろう。
#[test]
fn an_early_reset_aligns_the_leading_window_to_the_work_area_edge() {
    let run = boot_with_early_reset();
    assert_eq!(
        run.width(0),
        REAL[0].0 as i32,
        "前提: 本当の大きさで表示した"
    );
    assert_eq!(
        run.x(0) + run.width(0),
        WORK_AREA_RIGHT,
        "先頭の窓の右端が作業領域の右端から離れた"
    );
}

/// 起動の確定は窓を動かさない（並べ直しが一度で当たる）。
#[test]
fn an_early_reset_leaves_nothing_for_the_boot_finalize() {
    let run = boot_with_early_reset();
    let moved: Vec<&String> = run
        .finalize_logs
        .iter()
        .filter(|l| l.contains(FINALIZE_MOVED))
        .collect();
    assert!(
        moved.is_empty() && run.positions() == run.before_finalize,
        "起動の確定が窓を動かした: {moved:?}"
    );
}

/// 窓どうしの隙間は 0。
#[test]
fn an_early_reset_leaves_no_gap_between_windows() {
    let run = boot_with_early_reset();
    assert_eq!(
        run.x(0) - (run.x(1) + run.width(1)),
        0,
        "窓どうしに隙間が残った"
    );
}

/// 巡 1 に、待つスコープ 0 と 1 を挙げた預かりの記録が 1 行。
#[test]
fn an_early_reset_is_held_with_one_record_naming_both_scopes() {
    let run = boot_with_early_reset();
    let held: Vec<&String> = run
        .round1_logs
        .iter()
        .filter(|l| l.contains("reset_window_pos_held"))
        .collect();
    assert!(
        held.len() == 1 && held[0].contains("waiting=[0, 1]"),
        "預かりの記録が 1 行でない／待つスコープが 0 と 1 でない: {:?}",
        run.round1_logs
    );
}

/// 並べ直しの材料を置いた、起動の確定の後の足場（窓は作ったときの大きさ）。
fn settled_rig() -> (FrameHarness, Sender<PositionCue>, RuntimeWiring) {
    let mut harness = FrameHarness::new();
    harness.set_monitor_table_for_dpi(96);
    harness
        .world
        .insert_resource(placement_basis(BTreeMap::new(), 2));
    let (tx, wiring) = wiring();
    harness.advance_frame();
    (harness, tx, wiring)
}

fn both_shown() -> SettledSource {
    SettledSource::new()
        .shown(0, SPAWN_SIZE_0)
        .shown(1, SPAWN_SIZE_1)
}

fn only_scope0_shown() -> SettledSource {
    SettledSource::new().shown(0, SPAWN_SIZE_0)
}

fn partner_hidden() -> SettledSource {
    only_scope0_shown().unshown(1, NeverShown::Hidden)
}

/// 原点を基準に横だけ `x` へ置く。
fn to_global(scope: u32, x: i32) -> PositionCue {
    let tokens = vec![format!("--X={x}"), "--base=global".to_string()];
    PositionCue::Move(parse_move_directive(scope, &tokens).directive)
}

fn x_of(harness: &FrameHarness, scope: usize) -> i32 {
    pos_of(&harness.world, harness.char_window(scope))
        .expect("位置がある")
        .x
}

fn events<'a>(logs: &'a [String], event: &str) -> Vec<&'a String> {
    let ev = format!("event=\"{event}\"");
    logs.iter().filter(|l| l.contains(&ev)).collect()
}

fn drain(harness: &mut FrameHarness, wiring: &RuntimeWiring, shown: &SettledSource) -> Vec<String> {
    capture_logs(|| run_move_drain_with(wiring, shown, &mut harness.world))
}

/// 表示の決まっていない窓への預かった移動は、後の並べ直しで捨てられ、並べ直しは預かられる。相方が
/// 表示した巡に並べ直しが当たり、捨てた移動は当たらない。
#[test]
fn a_reset_discards_the_held_move_and_is_held_itself() {
    let (mut harness, tx, wiring) = settled_rig();
    tx.send(to_global(1, 10)).expect("送出は成功する");
    drain(&mut harness, &wiring, &only_scope0_shown());

    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");
    let held_round = drain(&mut harness, &wiring, &only_scope0_shown());
    assert!(
        events(&held_round, "move_hold_superseded")
            .iter()
            .any(|l| l.contains("scope=1")),
        "捨てた記録が無い: {held_round:?}"
    );
    let held = events(&held_round, "reset_window_pos_held");
    assert!(
        held.len() == 1 && held[0].contains("waiting=[1]"),
        "並べ直しが相方を待って預かられていない: {held_round:?}"
    );
    assert!(
        events(&held_round, "reset_window_pos").is_empty(),
        "並べ直しが表示の前に当たった: {held_round:?}"
    );

    let shown_round = drain(&mut harness, &wiring, &both_shown());
    assert_eq!(
        events(&shown_round, "reset_window_pos").len(),
        2,
        "表示の巡に並べ直しが当たらない: {shown_round:?}"
    );
    assert_ne!(x_of(&harness, 1), 10, "捨てた移動が当たった");
    drain(&mut harness, &wiring, &both_shown());
    assert!(wiring.move_held.borrow().is_empty(), "預かりが残った");
    assert_ne!(x_of(&harness, 1), 10, "捨てた移動が後で当たった");
}

/// 並べ直しを預かる間に届いた移動は追い越さずに預かられ、表示の巡に並べ直しの後で届いた順に当たる。
#[test]
fn a_move_behind_a_held_reset_applies_after_it_in_arrival_order() {
    let (mut harness, tx, wiring) = settled_rig();
    let before0 = x_of(&harness, 0);
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");
    tx.send(to_global(0, 30)).expect("送出は成功する");
    let held_round = drain(&mut harness, &wiring, &only_scope0_shown());
    assert!(
        !events(&held_round, "move_hold_behind").is_empty(),
        "後の移動が追い越さずに預かられた記録が無い: {held_round:?}"
    );
    assert_eq!(x_of(&harness, 0), before0, "後の移動が並べ直しを追い越した");

    tx.send(to_global(0, 50)).expect("送出は成功する");
    let shown_round = drain(&mut harness, &wiring, &both_shown());
    assert_eq!(
        events(&shown_round, "reset_window_pos").len(),
        2,
        "表示の巡に並べ直しが当たらない: {shown_round:?}"
    );
    assert_eq!(x_of(&harness, 0), 50, "並べ直しの後に届いた順で当たらない");
    assert!(wiring.move_held.borrow().is_empty(), "預かりが残った");
}

/// 最初の面の前に隠した窓は待たない: 並べ直しはその巡に当たり、隠した窓ごとに 1 行の記録が残り、
/// 後の指令も当たる。
#[test]
fn a_hidden_partner_does_not_hold_the_reset() {
    let (mut harness, tx, wiring) = settled_rig();
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");
    tx.send(to_global(0, 30)).expect("送出は成功する");
    let logs = drain(&mut harness, &wiring, &partner_hidden());

    assert!(
        events(&logs, "reset_window_pos_held").is_empty(),
        "隠した窓を待って預かった: {logs:?}"
    );
    assert_eq!(
        events(&logs, "reset_window_pos").len(),
        2,
        "並べ直しがその巡に当たらない: {logs:?}"
    );
    let unshown = events(&logs, "reset_window_pos_unshown");
    assert!(
        unshown.len() == 1
            && unshown[0].contains("scope=1")
            && unshown[0].contains("reason=\"hidden\""),
        "隠した窓の記録が 1 行でない: {logs:?}"
    );
    assert_eq!(x_of(&harness, 0), 30, "後の指令が預かられた");

    tx.send(to_global(0, 40)).expect("送出は成功する");
    drain(&mut harness, &wiring, &partner_hidden());
    assert_eq!(x_of(&harness, 0), 40, "次の巡の指令が預かられた");
}

/// 預かっていた並べ直しは、相方が「まだ」から「隠した」に決まった巡に当たる。
#[test]
fn a_held_reset_applies_when_the_partner_settles_as_hidden() {
    let (mut harness, tx, wiring) = settled_rig();
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");
    let held_round = drain(&mut harness, &wiring, &only_scope0_shown());
    assert_eq!(
        events(&held_round, "reset_window_pos_held").len(),
        1,
        "前提: 並べ直しは預かられた: {held_round:?}"
    );

    let settled_round = drain(&mut harness, &wiring, &partner_hidden());
    assert_eq!(
        events(&settled_round, "reset_window_pos").len(),
        2,
        "隠した に決まった巡に並べ直しが当たらない: {settled_round:?}"
    );
    assert_eq!(
        events(&settled_round, "reset_window_pos_unshown").len(),
        1,
        "{settled_round:?}"
    );
    assert!(
        events(&settled_round, "reset_window_pos_held").is_empty(),
        "預かり続ける巡に記録が出た: {settled_round:?}"
    );
}

/// 全部が表示した定常では、並べ直しは届いた巡に当たり、新しい行は出ない。
#[test]
fn a_reset_in_the_steady_state_applies_on_arrival_without_new_records() {
    let (mut harness, tx, wiring) = settled_rig();
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");
    let logs = drain(&mut harness, &wiring, &both_shown());
    assert_eq!(
        events(&logs, "reset_window_pos").len(),
        2,
        "並べ直しが届いた巡に当たらない: {logs:?}"
    );
    assert!(
        events(&logs, "reset_window_pos_held").is_empty()
            && events(&logs, "reset_window_pos_unshown").is_empty(),
        "定常で新しい行が出た: {logs:?}"
    );
}

/// 表示の決まっていない窓への移動は預かり、その窓が「隠した」に決まった巡に預かりの列から出て、
/// 移動ごとに 1 行の記録を残して今の大きさで置かれる。
#[test]
fn a_move_to_a_partner_settling_as_hidden_is_placed_at_the_current_size() {
    let (mut harness, tx, wiring) = settled_rig();
    let before1 = x_of(&harness, 1);
    let size1 = size_of(&harness.world, harness.char_window(1));
    tx.send(to_global(1, 10)).expect("送出は成功する");
    tx.send(to_global(1, 20)).expect("送出は成功する");
    let held_round = drain(&mut harness, &wiring, &only_scope0_shown());
    assert!(
        events(&held_round, "move_ratio_unset").len() == 1 && wiring.move_held.borrow().len() == 2,
        "まだの間に預かられていない: {held_round:?}"
    );
    assert_eq!(x_of(&harness, 1), before1, "まだの間に当たった");

    let settled_round = drain(&mut harness, &wiring, &partner_hidden());
    let placed = events(&settled_round, "move_hold_unshown");
    assert!(
        placed.len() == 2
            && placed.iter().all(|l| l.contains("scope=1")
                && l.contains("waiting=1")
                && l.contains("reason=\"hidden\"")
                && l.contains("handling=\"placed\"")),
        "移動ごとに 1 行の記録が無い: {settled_round:?}"
    );
    assert!(wiring.move_held.borrow().is_empty(), "預かりが残った");
    assert_eq!(x_of(&harness, 1), 20, "隠した窓へ位置が置かれない");
    assert_eq!(
        size_of(&harness.world, harness.char_window(1)),
        size1,
        "今の大きさで置かれない"
    );
}
