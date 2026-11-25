//! 起動中のゴーストへの上書きの 2 往復（降ろした → 背景のスレッドが展開 → 起こす → 定常で返す）の
//! 決定論テスト。`desk_overwrite_tests.rs` の子（土台・書庫・記録の道具を借りる）。
//!
//! 背景のスレッドの代わりにテストが窓口へ直接頼み、「降ろした」を受けてから自分で展開する（展開の段で
//! 止めて、窓口と切替の道筋の振る舞いを段ごとに見る）。窓口と切替の道筋は切替の土台の上の本物。

use std::sync::mpsc::Sender;
use std::thread;

use super::*;
use crate::install::destination::install_into;
use crate::install::worker::{DeskAsk, OrderGuard};
use areka_runtime::spine::{SPIN_WAIT, join_thread_bounded};
use areka_session::exit_wait::begin_close;
use areka_session::ghost_session::GhostSlot;

fn asks(rig: &SwitchRig) -> Sender<DeskAsk> {
    rig.world.non_send::<InstallDesk>().asks_tx.clone()
}

/// 上書きを頼み、窓口が答えるまで台詞を進めながら回す（届かなければ None）。
fn ask_until_answered(rig: &mut SwitchRig, archive: NarArchive) -> Option<OverwriteAnswer> {
    let (reply, answer) = reply_channel();
    asks(rig)
        .send(DeskAsk::Overwrite {
            archive,
            target_ghost: None,
            reply,
        })
        .expect("窓口の頼みの受信端は生きている");
    let mut got = None;
    rig.pump_talking_until(|_| {
        if got.is_none() {
            got = answer.try_recv().ok().flatten();
        }
        got.is_some()
    });
    got
}

/// 「降ろした」の答えのとおりに宛先へ展開する（展開できたら真）。
fn extract(answer: Option<OverwriteAnswer>) -> bool {
    match answer {
        Some(OverwriteAnswer::Down {
            archive,
            roots,
            running,
        }) => install_into(&roots, &archive, None, running.as_deref()).is_ok(),
        _ => false,
    }
}

/// 置き場にゴーストが居ない（降ろしたまま起こしていない）。
fn ghost_down(rig: &SwitchRig) -> bool {
    rig.world
        .get_non_send::<GhostSlot>()
        .is_none_or(|slot| slot.0.is_none())
}

/// 窓口は「降ろした」を返して起こすのを待たせ、窓もゴーストも無い間も取り出しの系を回す。展開し終えた
/// 知らせで同じゴーストを起こし直し、定常到達で返事をする。窓口は自分では展開しない。
#[test]
fn the_desk_hands_down_resumes_on_extracted_and_answers_on_steady() {
    let mut rig = running("A", &[]);
    let dir = TempPath::new("desk-overwrite-between-flow");
    let archive = NarArchive::open(&write_archive(&dir, "A", &[])).expect("書庫を開ける");
    let desk = asks(&rig);

    let (flow, events) = capture(|| {
        let down = ask_until_answered(&mut rig, archive);
        let held = (
            ghost_down(&rig),
            rig.world.get_non_send::<SwitchInFlight>().is_some(),
            rig.calls("A").len(),
        );
        let (reply, facts) = reply_channel();
        desk.send(DeskAsk::Facts { reply }).unwrap();
        let mut answered = None;
        rig.pump_input_until(|_| {
            if answered.is_none() {
                answered = facts.try_recv().ok();
            }
            answered.is_some()
        });
        let drained_while_down = (answered.flatten(), ghost_down(&rig));
        let installed = extract(down);
        let (reply, extracted) = reply_channel();
        desk.send(DeskAsk::Extracted { reply }).unwrap();
        rig.world.run_schedule(Input);
        let resumed = (ghost_down(&rig), matches!(extracted.try_recv(), Ok(None)));
        let mut steady = false;
        let finished = rig.pump_talking_until(|_| {
            steady = steady || matches!(extracted.try_recv(), Ok(Some(())));
            steady
        });
        (held, drained_while_down, installed, resumed, finished)
    });
    let boots = boots_of(&rig, "A").len();
    let current = current_of(&rig);
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let pending = rig.world.non_send::<InstallDesk>().overwrite.is_some();
    let body = std::fs::read(rig.root.ghost_dir("A").join(MARKER)).ok();
    assert!(rig.shutdown());

    assert_eq!(
        flow,
        (
            (true, true, 1),
            (Some(None), true),
            true,
            (false, true),
            true
        ),
        "(降ろして待つ・窓 0 の間に素性の頼みへ答える・展開した・起こしたが定常まで答えない・定常で答えた): {events:?}"
    );
    assert_eq!(
        (boots, current.as_deref(), reserved, pending),
        (2, Some("A"), false, false)
    );
    assert_eq!(body.as_deref(), Some(MARKER_BODY));
    assert!(
        events_named(&events, "install_overwrite_done").is_empty(),
        "窓口は展開しない: {events:?}"
    );
    let order_of = |name| first(&events, name).unwrap_or_else(|| panic!("{name}: {events:?}"));
    let closed = order_of("windows_closed_for_restart");
    let booted = order_of("ghost_switch_booted");
    let done = order_of("ghost_switch_done");
    assert!(closed < booted && booted < done, "{events:?}");
    assert!(
        events_named(&events, "ghost_switch_no_hold").is_empty(),
        "{events:?}"
    );
}

/// 展開の最中（背景のスレッドが書く段に居る）に終了が始まると、窓口は預かった書庫を段の語
/// `extracting` で捨て、書く前にやめた書庫とは記録しない。後から届いた展開し終えた知らせには
/// `warn!(install_overwrite_unexpected)` を残して返事だけし、ゴーストは起こさない。
#[test]
fn exit_during_extraction_discards_without_booting_and_answers_the_late_extracted() {
    let mut rig = running("A", &[]);
    let dir = TempPath::new("desk-overwrite-between-exit");
    let archive = NarArchive::open(&write_archive(&dir, "A", &[])).expect("書庫を開ける");
    let desk = asks(&rig);
    let gate = rig.world.non_send::<InstallDesk>().gate.clone();
    assert!(gate.begin("C:/A.nar".to_owned()));

    let (flow, events) = capture(|| {
        let down = ask_until_answered(&mut rig, archive);
        let writing = gate.enter_write();
        let _waits = begin_close(&mut rig.world);
        let installed = extract(down);
        gate.leave_write();
        gate.end();
        let (reply, extracted) = reply_channel();
        desk.send(DeskAsk::Extracted { reply }).unwrap();
        rig.world.run_schedule(Input);
        (
            writing,
            installed,
            matches!(extracted.try_recv(), Ok(Some(()))),
            ghost_down(&rig),
        )
    });
    let boots = boots_of(&rig, "A").len();
    assert!(rig.shutdown());

    assert_eq!(
        flow,
        (true, true, true, true),
        "(書く段に入れた・展開した・返事をした・起こさない): {events:?}"
    );
    assert_eq!(boots, 1);
    let discarded = events_named(&events, "install_pending_discarded");
    assert_eq!(discarded.len(), 1, "{events:?}");
    assert!(
        discarded[0]
            .field("overwrite")
            .is_some_and(|o| o.contains("\"A\"") && o.contains("extracting")),
        "{events:?}"
    );
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "{events:?}"
    );
    let unexpected = events_named(&events, "install_overwrite_unexpected");
    assert_eq!(unexpected.len(), 1, "{events:?}");
    assert_eq!(unexpected[0].level, tracing::Level::WARN);
    assert!(
        events_named(&events, "ghost_switch_booted").is_empty(),
        "{events:?}"
    );
}

/// 展開の最中に背景のスレッドが倒れても、窓口は倒れの知らせで待たせた切替を続け、同じゴーストを
/// 起こし直す（書庫は捨てる・窓口は空く）。
#[test]
fn a_worker_that_panics_while_extracting_lets_the_switch_boot_the_ghost() {
    let mut rig = running("A", &[]);
    let dir = TempPath::new("desk-overwrite-between-panic");
    let archive = NarArchive::open(&write_archive(&dir, "A", &[])).expect("書庫を開ける");
    let desk = asks(&rig);
    let gate = rig.world.non_send::<InstallDesk>().gate.clone();

    let (flow, events) = capture(|| {
        let worker = thread::spawn(move || {
            let _guard = OrderGuard {
                gate,
                desk: desk.clone(),
            };
            let (reply, answer) = reply_channel();
            desk.send(DeskAsk::Overwrite {
                archive,
                target_ghost: None,
                reply,
            })
            .expect("窓口の頼みの受信端は生きている");
            let down = matches!(
                answer.recv_timeout(SPIN_WAIT),
                Ok(OverwriteAnswer::Down { .. })
            );
            panic!("展開の最中に倒れる（降ろした: {down}）");
        });
        let finished = rig.pump_talking_until(|rig| {
            rig.calls("A").len() == 2 && rig.world.get_non_send::<SwitchInFlight>().is_none()
        });
        let panicked = join_thread_bounded("倒れる背景のスレッド", None, worker).is_err();
        (finished, panicked)
    });
    let current = current_of(&rig);
    let busy = crate::install::desk::is_busy(&rig.world);
    let touched = rig.root.ghost_dir("A").join(MARKER).exists();
    assert!(rig.shutdown());

    assert_eq!(flow, (true, true), "(起こし直した・倒れた): {events:?}");
    assert_eq!(
        (current.as_deref(), busy, touched),
        (Some("A"), false, false)
    );
    assert_eq!(
        events_named(&events, "ghost_switch_booted").len(),
        1,
        "{events:?}"
    );
    assert!(
        events_named(&events, "ghost_switch_no_hold").is_empty(),
        "{events:?}"
    );
}
