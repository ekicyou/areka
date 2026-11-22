//! 口の束の各欄が呼ばれる順を、記録する偽物の組で確かめるテスト。

use std::cell::{Cell, RefCell};
use std::sync::mpsc;

use areka_ghost::GhostRuntime;
use areka_input::user_break::drain_no_user_break_signals;
use areka_kanade::{ChangeOrigin, KanadeNotice};
use areka_runtime::reload_args::{DescriptNotes, ReloadTarget};
use areka_runtime::source_text_test_support::{code_only, index_of, squeeze};
use areka_runtime::spine::ScriptedShioriBackend;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::system::{IntoSystem, System};
use bevy_ecs::world::World;
use sample_ghost_kit::SampleRoot;
use shiori_host32_host::ExitKind;
use wintf::AppExit;
use wintf::ecs::Input;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;
use wintf::ecs::widget::bitmap_source::systems::drain_task_pool_commands;

use super::*;
use crate::ghost_session::{open_ghost_windows, register_systems};
use crate::ghost_switch::{
    GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch,
};
use crate::ghost_switch_test_support::{FakeShiori, RigSample, SwitchRig};
use crate::reload::{RefusedReason, ReloadOrigin, ReloadRequest, ReloadVerdict, request_reload};
use crate::shell_balloon_switch::{
    SkinKind, SkinOrigin, SkinRequest, SkinSpec, SkinVerdict, request_skin_switch,
};

thread_local! {
    static LOG: RefCell<Vec<(&'static str, String)>> = const { RefCell::new(Vec::new()) };
    static PROBE: Cell<fn(&World) -> String> = const { Cell::new(|_| String::new()) };
    static UPDATE_BUSY: Cell<bool> = const { Cell::new(false) };
    static INSTALL_BUSY: Cell<bool> = const { Cell::new(false) };
}

fn record(slot: &'static str, world: &World) {
    let mark = PROBE.with(|probe| probe.get()(world));
    LOG.with(|log| log.borrow_mut().push((slot, mark)));
}

/// 呼ばれた欄の名前と、そのときの World の印（`PROBE` が読む）を記録から取り出す。
fn take_log() -> Vec<(&'static str, String)> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

/// 各欄が名前と World の印をスレッドの記録へ足す偽物の組。忙しさの答えは `UPDATE_BUSY`・`INSTALL_BUSY`。
pub(crate) const RECORDING: BodySystems = BodySystems {
    menu_poll: |world| record("menu_poll", world),
    after_input: |world| record("after_input", world),
    after_char_pointer: |world| record("after_char_pointer", world),
    after_balloon_pointer: |world| record("after_balloon_pointer", world),
    after_runtime_wired: |world: &mut World, _: &GhostRuntime| record("after_runtime_wired", world),
    on_switch_end: |world, arrived| {
        record(
            if arrived {
                "on_switch_end(true)"
            } else {
                "on_switch_end(false)"
            },
            world,
        )
    },
    on_steady: |world| record("on_steady", world),
    overwrite_between: |world| record("overwrite_between", world),
    update_busy: |world| {
        record("update_busy", world);
        UPDATE_BUSY.with(Cell::get)
    },
    install_busy: |world| {
        record("install_busy", world);
        INSTALL_BUSY.with(Cell::get)
    },
    cue_sinks: |world| {
        record("cue_sinks", world);
        (NO_BODY.cue_sinks)(world)
    },
};

/// 同じスレッドで前に走ったテストの印の読み手・記録・忙しさの答えを戻す。
fn reset() {
    PROBE.with(|probe| probe.set(|_| String::new()));
    take_log();
    UPDATE_BUSY.with(|busy| busy.set(false));
    INSTALL_BUSY.with(|busy| busy.set(false));
}

fn slots(log: &[(&'static str, String)]) -> Vec<&'static str> {
    log.iter().map(|(slot, _)| *slot).collect()
}

/// MCP からの読み直しは更新 → インストールの順に問い、更新が忙しければインストールを問わない。
/// シェル・バルーンの切替は更新だけを問う。
#[test]
fn busy_slots_are_asked_update_first() {
    reset();
    let mut world = World::new();
    world.insert_resource(RECORDING);
    let reload = |world: &mut World| {
        request_reload(
            world,
            ReloadRequest {
                target: ReloadTarget::Ghost,
                origin: ReloadOrigin::Mcp,
                notes: DescriptNotes::default(),
            },
        )
    };
    let skin = |world: &mut World| {
        request_skin_switch(
            world,
            SkinRequest {
                kind: SkinKind::Shell,
                target: SkinSpec::Name("planted".to_owned()),
                origin: SkinOrigin::Script { raise_event: false },
            },
        )
    };

    UPDATE_BUSY.with(|busy| busy.set(true));
    let reload_busy = (reload(&mut world), slots(&take_log()));
    let skin_busy = (skin(&mut world), slots(&take_log()));
    UPDATE_BUSY.with(|busy| busy.set(false));
    let reload_idle = (reload(&mut world), slots(&take_log()));
    let skin_idle = (skin(&mut world), slots(&take_log()));

    assert_eq!(
        (reload_busy, reload_idle),
        (
            (
                ReloadVerdict::Refused(RefusedReason::Updating),
                vec!["update_busy"]
            ),
            (
                ReloadVerdict::Refused(RefusedReason::NoContext),
                vec!["update_busy", "install_busy"]
            ),
        ),
        "読み直しの (判定, 問われた欄) が 更新が忙しい・暇 の順で違う"
    );
    assert_eq!(
        (skin_busy, skin_idle),
        (
            (SkinVerdict::Updating, vec!["update_busy"]),
            (SkinVerdict::NoContext, vec!["update_busy"]),
        ),
        "シェル・バルーンの切替の (判定, 問われた欄) が 更新が忙しい・暇 の順で違う"
    );
}

/// `Input` の段の系の数と、`user_break` の汲み出しが載っているか（系の名は bevy の `debug` の機能が
/// 無いと読めないので、数と型で代える）。
fn input_marks(world: &World) -> String {
    let Some(schedule) = world.resource::<Schedules>().get(Input) else {
        return "0 false".to_owned();
    };
    let user_break = IntoSystem::into_system(drain_no_user_break_signals).system_type();
    let has_user_break = schedule
        .graph()
        .systems
        .iter()
        .any(|(_, system, _)| system.system_type() == user_break);
    format!("{} {has_user_break}", schedule.systems_len())
}

/// `balloon` の離脱から `balloon_wheel` までの入力の系が `Input` の段に足す系の数。
fn input_systems_between_menu_poll_and_after_input() -> usize {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    areka_input::balloon::register_balloon_leave_system(&mut world);
    areka_input::shell_box_handler::register_box_leave_system(&mut world);
    areka_input::choice_drain::register_choice_drain(&mut world);
    areka_input::char_watch::register_char_watch(&mut world);
    areka_input::balloon_wheel::register_balloon_wheel(&mut world);
    world
        .resource::<Schedules>()
        .get(Input)
        .map_or(0, |s| s.systems_len())
}

/// `menu_poll` は `user_break` の汲み出しと `balloon` の離脱の間、`after_input` は入力の系の後で、
/// 1 回ずつこの順に呼ばれる。
#[test]
fn menu_poll_and_after_input_sit_in_the_registration_order() {
    reset();
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(AppExit::new());
    let (_kanade_stop_tx, kanade_stop_rx) = mpsc::channel::<KanadeNotice>();
    PROBE.with(|probe| probe.set(input_marks));

    register_systems(&mut world, kanade_stop_rx, RECORDING);

    let log = take_log();
    assert_eq!(
        slots(&log),
        vec!["menu_poll", "after_input"],
        "登録の中で呼ばれた欄が 1 回ずつこの順でない"
    );
    let parse = |mark: &str| {
        let (len, user_break) = mark.split_once(' ').expect("印は「数 真偽」");
        (len.parse::<usize>().expect("系の数"), user_break == "true")
    };
    let ((menu_len, menu_user_break), (after_len, _)) = (parse(&log[0].1), parse(&log[1].1));
    assert_eq!(
        (menu_user_break, after_len - menu_len),
        (true, input_systems_between_menu_poll_and_after_input()),
        "(menu_poll のとき user_break の汲み出しが在る, 2 欄の間に載った系の数) が違う"
    );
}

/// 窓を作るクロージャの中で、キャラ窓の差し込み → `after_char_pointer` → バルーン窓の差し込み →
/// `after_balloon_pointer` の順に並ぶ。
#[test]
fn window_slots_sit_between_the_input_attachments() {
    reset();
    let code = squeeze(&code_only(include_str!("ghost_session.rs")));
    let start = index_of(
        &code,
        "let spawn: BoxedCommand = Box::new(move |world: &mut World| {",
    );
    let closure = &code[start..start + index_of(&code[start..], "Ok(PreparedWindows {")];
    let at = [
        "areka_input::attach_char_pointer_handlers(world);",
        "(BodySystems::of(world).after_char_pointer)(world);",
        "areka_input::balloon::attach_balloon_pointer_handlers(world);",
        "(BodySystems::of(world).after_balloon_pointer)(world);",
    ]
    .map(|needle| index_of(closure, needle));
    assert!(
        at.is_sorted(),
        "窓を作るクロージャの中の差し込みと口の順が違う（位置: {at:?}）"
    );
}

fn count(slot: &str) -> usize {
    LOG.with(|log| log.borrow().iter().filter(|(s, _)| *s == slot).count())
}

/// 台詞を返さない台本（起動記録を植えたゴースト用・どの起動の事象も 204）。
fn silent() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnGhostChanged", Ok(None))
            .get("OnBoot", Ok(None))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
    }))
}

/// A を窓ごと起こして定常にし、B へ切り替えて B の窓と定常まで回す。窓を開くたびに
/// `after_char_pointer` → `after_balloon_pointer`、起こすたびに `cue_sinks` → `after_runtime_wired`、
/// メニューからの切替要求は `update_busy` を 1 回読み、`overwrite_between` は B の `cue_sinks` より前、B の定常で `on_switch_end(true)` → `on_steady`。
#[test]
fn body_slots_follow_the_boot_and_switch_order() {
    reset();
    let sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let mut rig = SwitchRig::with(
        RigSample::of(sample),
        RECORDING,
        vec![("A", silent()), ("B", silent())],
    );
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain_task_pool_commands);
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");

    let cfg_a = rig.cfg("A");
    let a_opened = open_ghost_windows(&mut rig.world, &cfg_a).is_ok();
    rig.boot("A");
    let a_ready =
        rig.pump_input_until(|_| count("on_steady") == 1 && count("after_balloon_pointer") == 1);
    let verdict = request_ghost_switch(
        &mut rig.world,
        SwitchRequest {
            ghost: GhostSpec::Folder("B".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Manual,
            boot_event: None,
        },
    );
    let b_ready = rig.pump_input_until(|rig| {
        count("on_steady") == 2
            && count("after_balloon_pointer") == 2
            && rig.world.get_non_send::<SwitchInFlight>().is_none()
    });
    let log = slots(&take_log());
    let shutdown_ok = rig.shutdown();

    let window = |slot: &&str| slot.starts_with("after_") && slot.ends_with("_pointer");
    let positions = |slot: &str| -> Vec<usize> {
        log.iter()
            .enumerate()
            .filter(|(_, s)| **s == slot)
            .map(|(at, _)| at)
            .collect()
    };
    let (wired, char_pointer) = (
        positions("after_runtime_wired"),
        positions("after_char_pointer"),
    );
    // B の窓の閉包と B の定常の知らせはどちらが先に着くかが決まらないので、全体の並びでなく
    // 窓の欄とそれ以外の欄を別々に比べ、窓の欄の位置は起こした回の結線の後かで見る。
    let windows_after_wiring = [
        wired.first(),
        char_pointer.first(),
        positions("overwrite_between").first(),
        wired.get(1),
        char_pointer.get(1),
    ]
    .into_iter()
    .collect::<Option<Vec<_>>>()
    .is_some_and(|at| at.is_sorted());
    assert_eq!(
        (
            (a_opened, a_ready, verdict, b_ready, shutdown_ok),
            log.iter()
                .copied()
                .filter(|s| !window(s))
                .collect::<Vec<_>>(),
            log.iter().copied().filter(window).collect::<Vec<_>>(),
            windows_after_wiring,
        ),
        (
            (true, true, SwitchVerdict::Accepted, true, true),
            vec![
                "menu_poll",
                "after_input",
                "cue_sinks",
                "after_runtime_wired",
                "on_steady",
                "update_busy",
                "overwrite_between",
                "cue_sinks",
                "after_runtime_wired",
                "on_switch_end(true)",
                "on_steady",
            ],
            vec![
                "after_char_pointer",
                "after_balloon_pointer",
                "after_char_pointer",
                "after_balloon_pointer",
            ],
            true,
        ),
        "((A の窓の準備, A の窓と定常, 受理, B の窓と定常, 降ろせた), 窓の外の欄, 窓の欄, 窓が結線の後): {log:?}"
    );
}
