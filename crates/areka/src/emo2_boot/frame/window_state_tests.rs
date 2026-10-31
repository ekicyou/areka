// =============================================================================
// 常に手前の状態と取り出しの相の決定論テスト（stay-on-top-directives task 3.1・
// 要件 1.1〜1.3／2.1／2.7〜2.9／3.1／3.2／3.4／6.1／6.2／6.7／7.2）
//
// 帯の指令の列は中身を読む口を持たないので、期待の列を同じ口（`push`）で組み、
// `Debug` の写しを比べる（列の中身＝指令の順と entity の列）。
// =============================================================================

use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};

use super::super::Emo2Wiring;
use super::super::test_support::spawn_resnap_windows;
use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};

use WindowStateDirective::{Raise, StayOnTop as Set};

const ON: WindowStateDirective = Set { on: true };
const OFF: WindowStateDirective = Set { on: false };

/// 受信端を渡した状態と送信端。
fn state_with_channel() -> (StayOnTopState, Sender<WindowStateDirective>) {
    let (tx, rx) = channel();
    let state = StayOnTopState {
        rx: Some(rx),
        on: false,
    };
    (state, tx)
}

/// 2 スコープの窓を持つ World と、ゴーストの窓の全部の entity（スコープ順・キャラ→バルーン）。
fn world_with_windows() -> (World, Vec<Entity>) {
    let mut world = World::new();
    let gw = spawn_resnap_windows(&mut world);
    let all = gw
        .scopes()
        .flat_map(|s| [gw.char_window(s).unwrap(), gw.balloon_window(s).unwrap()])
        .collect();
    (world, all)
}

/// 今のゴーストのフォルダを `pasta` とする起動の文脈。
fn boot_context() -> BootContext {
    let root = BasewareRoot::new(PathBuf::from("skins"));
    BootContext {
        roots: areka_ghost::BasewareRoots::single(root.dir().to_path_buf()),
        roots_origin: crate::roots::RootsOrigin::EnvVar,
        app_profile_dir: PathBuf::from("skins/profile"),
        helper_exe: PathBuf::from("skins/helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("pasta"),
                balloon_root: root.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir("pasta"),
                folder: Some("pasta".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    }
}

/// 期待の帯の指令の列（`Debug` の写し）。
fn queue_text(ops: &[(BandOp, &[Entity])]) -> String {
    let mut queue = WindowBandQueue::default();
    for (op, windows) in ops {
        queue.push(*op, windows.to_vec());
    }
    format!("{queue:?}")
}

/// World の帯の指令の列（`Debug` の写し・資源が無ければ `None`）。
fn queue_in(world: &World) -> Option<String> {
    world
        .get_resource::<WindowBandQueue>()
        .map(|queue| format!("{queue:?}"))
}

fn marked(world: &World, entity: Entity) -> bool {
    world.get::<StayOnTop>(entity).is_some()
}

/// 帯の指令の列を空にする（確定段の帯の系が巡ごとに空にする代わり）。
fn clear_queue(world: &mut World) {
    world.insert_resource(WindowBandQueue::default());
}

fn window_state_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.message().starts_with("[window_state]"))
        .collect()
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

/// 既定の状態はオフ・受信端なし（要件 3.4）。
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
    let all: Vec<Entity> = gw
        .scopes()
        .flat_map(|s| [gw.char_window(s).unwrap(), gw.balloon_window(s).unwrap()])
        .collect();
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

/// オフ→オンで全スコープのキャラクターとバルーンに印が付き、列に `Enter` が 1 件（要件 2.1）。
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

/// オン→オフで印が外れ、列に `Leave` が 1 件（要件 3.1）。
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

/// 重ね（オンにオン・オフにオフ）では列が空で印も動かない（要件 2.8・3.2）。
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

/// `\v` は状態も印も変えず、列に `Raise` を積む（オフでもオンでも・要件 1.1・2.9）。
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

/// 正本に載っているが World に居ない窓は飛ばして記録し、残りの窓には印を付けて積む。
#[test]
fn despawned_registry_window_is_skipped_with_a_debug_record() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    world.insert_resource(boot_context());
    // 印の除去の掛け金は正本からスコープごと落とすので、古い写しを据え直して
    // 「正本に載るが World に居ない」形を作る（対の片割れだけが先に消えた間と同じ）。
    let stale = world.resource::<GhostWindows>().clone();
    world.despawn(all[1]);
    world.insert_resource(stale);
    let rest = [all[0], all[2], all[3]];
    tx.send(ON).unwrap();

    let ((), events) = capture(|| run_window_state_phase(&mut state, &mut world));

    let skipped: Vec<_> = window_state_events(&events)
        .into_iter()
        .filter(|e| e.field("dropped").is_some())
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].level, tracing::Level::DEBUG);
    assert_eq!(skipped[0].field_str("ghost"), Some("pasta"));
    assert_eq!(skipped[0].field("dropped"), Some("1"));
    assert!(rest.iter().all(|e| marked(&world, *e)));
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Enter, &rest)])
    );
}

/// 正本の外の entity（別のゴースト・メニューなど）には印が付かず、列にも載らない（要件 2.7）。
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
// 記録の欄（要件 6.1・6.2・6.7）
// ---------------------------------------------------------------------------

#[test]
fn log_fields_for_change_unchanged_and_raise() {
    let (mut state, tx) = state_with_channel();
    let (mut world, _) = world_with_windows();
    world.insert_resource(boot_context());
    tx.send(ON).unwrap();
    tx.send(ON).unwrap();
    tx.send(Raise).unwrap();
    tx.send(OFF).unwrap();

    let ((), events) = capture(|| run_window_state_phase(&mut state, &mut world));
    let events = window_state_events(&events);
    assert_eq!(events.len(), 4);

    let on = events[0];
    assert_eq!(on.level, tracing::Level::INFO);
    assert_eq!(on.field_str("ghost"), Some("pasta"));
    assert_eq!(on.field_str("directive"), Some("stayontop"));
    assert_eq!(on.field_str("before"), Some("off"));
    assert_eq!(on.field_str("after"), Some("on"));
    assert_eq!(on.field("windows"), Some("4"));

    let same = events[1];
    assert_eq!(same.level, tracing::Level::DEBUG);
    assert_eq!(same.field_str("ghost"), Some("pasta"));
    assert_eq!(same.field_str("directive"), Some("stayontop"));
    assert_eq!(same.field_str("state"), Some("on"));

    let raise = events[2];
    assert_eq!(raise.level, tracing::Level::DEBUG);
    assert_eq!(raise.field_str("ghost"), Some("pasta"));
    assert_eq!(raise.field_str("band"), Some("topmost"));
    assert_eq!(raise.field("windows"), Some("4"));

    let off = events[3];
    assert_eq!(off.level, tracing::Level::INFO);
    assert_eq!(off.field_str("directive"), Some("!stayontop"));
    assert_eq!(off.field_str("before"), Some("on"));
    assert_eq!(off.field_str("after"), Some("off"));
}

/// 起動の文脈が無ければゴーストの欄は `-`、オフの `\v` の帯は `normal`。
#[test]
fn log_ghost_falls_back_to_dash_and_raise_reports_normal_band() {
    let (mut state, tx) = state_with_channel();
    let (mut world, _) = world_with_windows();
    tx.send(Raise).unwrap();
    let ((), events) = capture(|| run_window_state_phase(&mut state, &mut world));
    let events = window_state_events(&events);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].field_str("ghost"), Some("-"));
    assert_eq!(events[0].field_str("band"), Some("normal"));
}

// ---------------------------------------------------------------------------
// オンのまま降ろした記録（task 3.2・要件 4.1／4.2／6.4）
// ---------------------------------------------------------------------------

/// 結線（状態 `on`）と起動の文脈を持つ World。
fn world_with_wiring(on: bool) -> World {
    let mut world = World::new();
    world.insert_resource(boot_context());
    let mut wiring = super::super::test_support::headless_wiring_with(
        channel().1,
        super::super::test_support::zero_clock(),
    );
    wiring.window_state.on = on;
    world.insert_non_send(wiring);
    world
}

/// オンなら理由ごとに 1 行（`info!`・ゴースト・理由）を残し、状態は動かさない。
#[test]
fn discard_log_reports_reason_while_on() {
    for (reason, text) in [
        (DiscardReason::Exit, "exit"),
        (DiscardReason::Switch, "switch"),
        (DiscardReason::Reload, "reload"),
    ] {
        let world = world_with_wiring(true);
        let ((), events) = capture(|| log_stay_on_top_discard(&world, reason));
        let events = window_state_events(&events);
        assert_eq!(events.len(), 1, "{reason:?}");
        assert_eq!(events[0].level, tracing::Level::INFO);
        assert_eq!(events[0].field_str("ghost"), Some("pasta"));
        assert_eq!(events[0].field_str("reason"), Some(text));
        assert!(world.get_non_send::<Emo2Wiring>().unwrap().window_state.on);
    }
}

// ---------------------------------------------------------------------------
// 台本の再生から状態まで（task 5.1・要件 1.1〜1.4）
// ---------------------------------------------------------------------------

/// 台本を本番と同じ解析と組み立てで cue にし、受け口を登録した再生器を返す（`end` は台本の終わり）。
fn player_for(script: &str, tx: Sender<WindowStateDirective>) -> (dola::cue::CuePlayer, f64) {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(
        &instructions,
        &areka_sakura::sysvar::SystemVarSnapshot::default(),
    );
    let mut player = dola::cue::CuePlayer::from_sheet(&compiled.sheet);
    player.register_sink(Box::new(
        crate::emo2_boot::window_state_cue::WindowStateCueSink::new(tx),
    ));
    (player, compiled.sheet.absolute_end_time())
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

/// 台本のオン・`\v`・オフが受け口から線を通って状態を動かす（要件 1.1〜1.3）。
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

/// 指令の所へ来る前に中断した台本では状態が変わらない（要件 1.4）。止めずに最後まで流した
/// 同じ台本がオンにすることを対照に置く（待ちが実際に指令を後ろへずらしている）。
#[test]
fn script_stopped_before_the_directive_leaves_the_state() {
    const SCRIPT: &str = r"\0あ\_w[1000]\![set,windowstate,stayontop]\e";

    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    let (mut player, end) = player_for(SCRIPT, tx.clone());
    player.tick(0.0);
    player.stop();
    player.tick(end);
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

/// オフのとき・結線が無いときは何も書かない。
#[test]
fn discard_log_is_silent_while_off_or_unwired() {
    let off = world_with_wiring(false);
    let ((), events) = capture(|| log_stay_on_top_discard(&off, DiscardReason::Exit));
    assert!(window_state_events(&events).is_empty());

    let unwired = World::new();
    let ((), events) = capture(|| log_stay_on_top_discard(&unwired, DiscardReason::Switch));
    assert!(window_state_events(&events).is_empty());
}
