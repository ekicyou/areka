//! 切替の「降ろした」と「起こす」の間の待ちと、降ろす失敗の記録の決定論テスト。

use areka_kanade::{ChangeHandoff, ChangeOrigin};
use areka_runtime::spine::{ScriptedShioriBackend, ScriptedShioriBackendBuilder};
use log_capture_kit::capture;
use temp_path_kit::TempPath;
use wintf::AppExit;

use super::between::BetweenHold;
use super::boot_event_tests::running_a_on;
use super::fallback_tests::{root_with, switching_world};
use super::*;
use crate::body_systems::NO_BODY;
use crate::boot_resolve::boot_resolve_tests::count_event;
use crate::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::session_end::test_support::levels_of;

/// 上書きの口で待ちを立てる本体。
const HOLDING: BodySystems = BodySystems {
    overwrite_between: hold_between,
    ..NO_BODY
};

fn scripted(on_boot: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || standard_script(on_boot)))
}

/// 切替で来たときに答える台本。
fn switched_in(on_boot: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        standard_script(on_boot).get("OnGhostChanged", Ok(None))
    }))
}

/// 材料を預かって待っているか。
fn holding(world: &World) -> bool {
    world
        .get_non_send::<BetweenHold>()
        .is_some_and(|hold| hold.0.is_some())
}

/// B への切替を頼み、待ちに入るまで回す（受理・待ちに入った・B を起こした回数）。
fn switch_to_b_until_held(rig: &mut SwitchRig) -> (SwitchVerdict, bool, usize) {
    let verdict = request_ghost_switch(
        &mut rig.world,
        SwitchRequest {
            ghost: GhostSpec::Folder("B".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event: None,
        },
    );
    let held = rig.pump_talking_until(|rig| rig.exit_requested() || holding(&rig.world));
    (verdict, held, rig.calls("B").len())
}

/// 上書きの口が待ちを立てると、全窓を閉じた後で起こさずに戻り、段は送り出しのまま材料を預かる。
#[test]
fn switch_to_waits_after_closing_windows_when_the_hook_holds() {
    let tmp = TempPath::new("ghost-switch-between-hold");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");
    world.insert_resource(HOLDING);

    let ((), events) = capture(|| switch_to(&mut world, ChangeHandoff { script: None }));

    assert_eq!(
        (
            holding(&world),
            count_event(&events, "windows_closed_for_restart"),
            count_event(&events, "ghost_switch_no_context"),
            count_event(&events, "ghost_switch_booted"),
            world.get_non_send::<SwitchInFlight>().map(|f| f.stage),
            world.non_send::<AppExit>().is_requested(),
        ),
        (true, 1, 0, 0, Some(SwitchStage::SendOff), false),
        "(待ち・窓を閉じた・起こす試み・起こした・段・終了の指示): {events:?}"
    );
}

/// 待たせた切替は `resume_between` で切替先を起こし、定常に着いて予約と待ちが下りる。
#[test]
fn resume_between_boots_the_held_target() {
    let mut rig = running_a_on(
        HOLDING,
        vec![("A", scripted(r"\0A\e")), ("B", switched_in(r"\0B\e"))],
    );

    let held = switch_to_b_until_held(&mut rig);
    let (booted, events) = capture(|| {
        resume_between(&mut rig.world);
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("B").len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let after = (
        rig.exit_requested(),
        rig.world.get_non_send::<BetweenHold>().is_some(),
    );
    assert!(rig.shutdown());

    assert_eq!(
        (
            held,
            booted,
            after,
            count_event(&events, "ghost_switch_no_hold")
        ),
        ((SwitchVerdict::Accepted, true, 0), true, (false, false), 0),
        "((受理・待ち・待ちの間に B を起こした回数)・B が定常・(終了・待ちの残り)・待ち無しの記録): {events:?}"
    );
}

/// 上書きの口が待ちを立てなければ、前に残った印があってもその場で起こしに行き、印は残らない。
/// 待ちの無い `resume_between` は warn を 1 件残すだけ。
#[test]
fn switch_to_boots_at_once_without_a_hold_from_the_hook() {
    let tmp = TempPath::new("ghost-switch-between-no-hold");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");
    hold_between(&mut world);

    let ((), events) = capture(|| switch_to(&mut world, ChangeHandoff { script: None }));
    let ((), resumed) = capture(|| resume_between(&mut world));

    assert_eq!(
        (
            world.get_non_send::<BetweenHold>().is_some(),
            count_event(&events, "ghost_switch_no_context"),
            levels_of(&resumed, "ghost_switch_no_hold"),
            count_event(&resumed, "ghost_switch_no_context"),
        ),
        (false, 1, vec![tracing::Level::WARN], 0),
        "(印の残り・起こす試み・待ち無しの記録・続きで起こす試み): {events:?} / {resumed:?}"
    );
}

/// 待たせた切替先を起こせなければ既定ゴーストへ戻して定常に着く。
#[test]
fn resume_between_falls_back_to_default_when_the_target_fails() {
    let mut rig = running_a_on(
        HOLDING,
        vec![
            ("A", scripted(r"\0A\e")),
            ("B", FakeShiori::WiringFail),
            (DEFAULT_GHOST_FOLDER, scripted(r"\0emo2\e")),
        ],
    );

    let held = switch_to_b_until_held(&mut rig);
    let (fell_back, events) = capture(|| {
        resume_between(&mut rig.world);
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls(DEFAULT_GHOST_FOLDER).len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let after = (
        rig.exit_requested(),
        rig.world.get_non_send::<BetweenHold>().is_some(),
    );
    assert!(rig.shutdown());

    assert_eq!(
        (
            (held.0, held.1),
            fell_back,
            after,
            count_event(&events, "ghost_switch_boot_failed"),
        ),
        ((SwitchVerdict::Accepted, true), true, (false, false), 1),
        "((受理・待ち)・既定が定常・(終了・待ちの残り)・起動の失敗): {events:?}"
    );
}

/// `unload` の答えを持たない台本（SHIORI のアクターが倒れ、降ろす `shutdown` が失敗する）。
fn script_without_unload() -> ScriptedShioriBackendBuilder {
    ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(r"\0A\e".to_owned())))
        .notify("basewareversion", Ok(()))
        .notify("OnClose", Ok(()))
}

/// 降ろす `shutdown` が失敗したら「降ろした」の info は出さず、失敗の error だけが残る。
#[test]
fn take_down_reports_down_only_when_shutdown_succeeds() {
    let mut rig = running_a_on(
        NO_BODY,
        vec![("A", FakeShiori::Scripted(Box::new(script_without_unload)))],
    );

    let ((), events) = capture(|| take_down(&mut rig.world));

    assert_eq!(
        (
            count_event(&events, "ghost_switch_down_failed"),
            count_event(&events, "ghost_switch_down_ms"),
        ),
        (1, 0),
        "(降ろす失敗・降ろした): {events:?}"
    );
}
