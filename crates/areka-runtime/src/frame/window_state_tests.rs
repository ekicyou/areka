// =============================================================================
// 常に手前の状態と取り出しの相の決定論テスト（状態の遷移・窓への印・積む順・台本の再生・
// 起動の文脈が無いときの記録の欄の代わりの値）。
// 正本に居ない窓を飛ばす記録・変化／変化なし／raise の記録の欄・捨てた記録は本体の
// `runtime_tests/frame/window_state_tests.rs` が持つ。
//
// 帯の指令の列は中身を読む口を持たないので、期待の列を同じ口（`push`）で組み、
// `Debug` の写しを比べる（列の中身＝指令の順と entity の列）。
// =============================================================================

use std::sync::mpsc::Sender;

use bevy_ecs::world::World;
use log_capture_kit::capture;

use super::super::test_support::spawn_resnap_windows;
use super::*;

use super::test_support::{
    OFF, ON, all_windows, marked, queue_in, queue_text, state_with_channel, world_with_windows,
};
use WindowStateDirective::Raise;

/// 帯の指令の列を空にする（確定段の帯の系が巡ごとに空にする代わり）。
fn clear_queue(world: &mut World) {
    world.insert_resource(WindowBandQueue::default());
}

// ---------------------------------------------------------------------------
// 遷移の判断（純関数）：6 つの組
// ---------------------------------------------------------------------------

#[test]
fn decide_transition_covers_all_six_pairs() {
    assert_eq!(decide_transition(false, ON), Transition::TurnOn);
    assert_eq!(
        decide_transition(true, ON),
        Transition::Unchanged { on: true }
    );
    assert_eq!(decide_transition(true, OFF), Transition::TurnOff);
    assert_eq!(
        decide_transition(false, OFF),
        Transition::Unchanged { on: false }
    );
    assert_eq!(
        decide_transition(false, Raise),
        Transition::Raise { on: false }
    );
    assert_eq!(
        decide_transition(true, Raise),
        Transition::Raise { on: true }
    );
}

/// 既定の状態はオフ・受信端なし。
#[test]
fn default_state_is_off_without_receiver() {
    let state = StayOnTopState::default();
    assert!(!state.on);
    assert!(state.rx.is_none());
}

// ---------------------------------------------------------------------------
// 取り出しの相
// ---------------------------------------------------------------------------

/// 受信端が無い（テストの結線）ときは何もしない。
#[test]
fn without_receiver_the_phase_does_nothing() {
    let mut state = StayOnTopState::default();
    let (mut world, all) = world_with_windows();
    run_window_state_phase(&mut state, &mut world);
    assert!(!state.on);
    assert!(queue_in(&world).is_none());
    assert!(all.iter().all(|e| !marked(&world, *e)));
}

/// 正本が無い間は取り出さず、生えた巡で到着順に当てる（オン→`\v`→オフ）。
#[test]
fn holds_while_registry_absent_then_applies_in_arrival_order() {
    let (mut state, tx) = state_with_channel();
    let mut world = World::new();
    tx.send(ON).unwrap();
    tx.send(Raise).unwrap();
    tx.send(OFF).unwrap();

    run_window_state_phase(&mut state, &mut world);
    assert!(!state.on, "正本が無い間は状態を動かさない");
    assert!(queue_in(&world).is_none(), "正本が無い間は列へ積まない");

    // 送り直さずに窓を生やす＝最初に送った 3 件がチャネルに残っていれば当たる。
    let gw = spawn_resnap_windows(&mut world);
    let all = all_windows(&gw);
    run_window_state_phase(&mut state, &mut world);

    assert!(!state.on);
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[
            (BandOp::Enter, &all),
            (BandOp::Raise, &all),
            (BandOp::Leave, &all),
        ])
    );
    assert!(all.iter().all(|e| !marked(&world, *e)));
}

/// オフ→オンで全スコープのキャラクターとバルーンに印が付き、列に `Enter` が 1 件。
/// 帯の指令の列の資源が無ければ入れてから積む。
#[test]
fn turning_on_marks_every_ghost_window_and_queues_enter() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    assert_eq!(all.len(), 4, "2 スコープのキャラとバルーン");
    assert!(queue_in(&world).is_none());
    tx.send(ON).unwrap();
    run_window_state_phase(&mut state, &mut world);

    assert!(state.on);
    assert!(all.iter().all(|e| marked(&world, *e)));
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Enter, &all)])
    );
}

/// オン→オフで印が外れ、列に `Leave` が 1 件。
#[test]
fn turning_off_unmarks_every_ghost_window_and_queues_leave() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    tx.send(ON).unwrap();
    run_window_state_phase(&mut state, &mut world);
    clear_queue(&mut world);

    tx.send(OFF).unwrap();
    run_window_state_phase(&mut state, &mut world);

    assert!(!state.on);
    assert!(all.iter().all(|e| !marked(&world, *e)));
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Leave, &all)])
    );
}

/// 重ね（オンにオン・オフにオフ）では列が空で印も動かない。
#[test]
fn repeated_directives_queue_nothing() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    clear_queue(&mut world);

    tx.send(OFF).unwrap();
    run_window_state_phase(&mut state, &mut world);
    assert!(!state.on);
    assert_eq!(queue_in(&world).unwrap(), queue_text(&[]));
    assert!(all.iter().all(|e| !marked(&world, *e)));

    tx.send(ON).unwrap();
    run_window_state_phase(&mut state, &mut world);
    clear_queue(&mut world);
    tx.send(ON).unwrap();
    run_window_state_phase(&mut state, &mut world);
    assert!(state.on);
    assert_eq!(queue_in(&world).unwrap(), queue_text(&[]));
    assert!(all.iter().all(|e| marked(&world, *e)));
}

/// `\v` は状態も印も変えず、列に `Raise` を積む（オフでもオンでも）。
#[test]
fn raise_keeps_state_and_marks() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();

    tx.send(Raise).unwrap();
    run_window_state_phase(&mut state, &mut world);
    assert!(!state.on);
    assert!(all.iter().all(|e| !marked(&world, *e)));
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Raise, &all)])
    );

    tx.send(ON).unwrap();
    run_window_state_phase(&mut state, &mut world);
    clear_queue(&mut world);
    tx.send(Raise).unwrap();
    run_window_state_phase(&mut state, &mut world);
    assert!(state.on);
    assert!(all.iter().all(|e| marked(&world, *e)));
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Raise, &all)])
    );
}

/// 正本の外の entity（別のゴースト・メニューなど）には印が付かず、列にも載らない。
#[test]
fn entities_outside_registry_are_untouched() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    let outsider = world.spawn_empty().id();
    tx.send(ON).unwrap();
    run_window_state_phase(&mut state, &mut world);

    assert!(!marked(&world, outsider));
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Enter, &all)])
    );
}

// ---------------------------------------------------------------------------
// 台本の再生から状態まで
// ---------------------------------------------------------------------------

/// 台本を本番と同じ解析と組み立てで cue にし、受け口を登録した再生器を返す（`end` は台本の終わり）。
fn player_for(
    script: &str,
    tx: Sender<WindowStateDirective>,
) -> (dola::cue::CuePlayer, dola::TimePoint) {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(
        &instructions,
        &areka_sakura::sysvar::SystemVarSnapshot::default(),
    );
    let sheet = compiled
        .sheet
        .with_absolute_start_time(dola::TimePoint::ZERO);
    let mut player = dola::cue::CuePlayer::from_sheet(&sheet).expect("起点を押した台本");
    player.register_sink(Box::new(crate::window_state_cue::WindowStateCueSink::new(
        tx,
    )));
    (player, sheet.absolute_end_time().expect("起点を押した台本"))
}

/// 台本を最後まで再生してから取り出しの相を 1 回回す。
fn play_to_end(
    script: &str,
    state: &mut StayOnTopState,
    tx: &Sender<WindowStateDirective>,
    world: &mut World,
) {
    let (mut player, end) = player_for(script, tx.clone());
    player.tick(end);
    run_window_state_phase(state, world);
}

/// 台本のオン・`\v`・オフが受け口から線を通って状態を動かす。
#[test]
fn script_playback_turns_on_raises_and_turns_off() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();

    play_to_end(
        r"\0\![set,windowstate,stayontop]\e",
        &mut state,
        &tx,
        &mut world,
    );
    assert!(state.on, "オンの指令で状態がオン");
    assert!(all.iter().all(|e| marked(&world, *e)));
    clear_queue(&mut world);

    play_to_end(r"\0あ\v\e", &mut state, &tx, &mut world);
    assert!(state.on, "`\\v` は状態を変えない");
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Raise, &all)]),
        "`\\v` が手前へ出すとして届く"
    );
    clear_queue(&mut world);

    play_to_end(
        r"\0\![set,windowstate,!stayontop]\e",
        &mut state,
        &tx,
        &mut world,
    );
    assert!(!state.on, "オフの指令で状態がオフ");
    assert!(all.iter().all(|e| !marked(&world, *e)));
}

/// 指令の所へ来る前に中断した台本では状態が変わらない。止めずに最後まで流した
/// 同じ台本がオンにすることを対照に置く（待ちが実際に指令を後ろへずらしている）。
#[test]
fn script_stopped_before_the_directive_leaves_the_state() {
    const SCRIPT: &str = r"\0あ\_w[1000]\![set,windowstate,stayontop]\e";

    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    let (mut player, _) = player_for(SCRIPT, tx.clone());
    player.tick(dola::TimePoint::ZERO);
    // 中断は本番（`drive.rs`）と同じく player を捨てる形。
    drop(player);
    run_window_state_phase(&mut state, &mut world);
    assert!(!state.on, "中断した台本で状態が動いた");
    assert!(queue_in(&world).is_none());
    assert!(all.iter().all(|e| !marked(&world, *e)));

    // 対照: 中断しなければ同じ台本でオンになる。
    play_to_end(SCRIPT, &mut state, &tx, &mut world);
    assert!(
        state.on,
        "対照の前提が崩れている（最後まで流してもオンにならない）"
    );
}

/// 起動の文脈が無ければゴーストの欄は `-`、オフの `\v` の帯は `normal`。
#[test]
fn log_ghost_falls_back_to_dash_and_raise_reports_normal_band() {
    let (mut state, tx) = state_with_channel();
    let (mut world, _) = world_with_windows();
    tx.send(Raise).unwrap();
    let ((), events) = capture(|| run_window_state_phase(&mut state, &mut world));
    let events: Vec<_> = events
        .iter()
        .filter(|e| e.message().starts_with("[window_state]"))
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].field_str("ghost"), Some("-"));
    assert_eq!(events[0].field_str("band"), Some("normal"));
}
