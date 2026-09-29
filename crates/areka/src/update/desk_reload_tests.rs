//! 読み直しの頼みの兄弟テスト（design「窓口と入口」の読み直しの条件と終了・要件 5.2・5.5・5.8・7.4・
//! 9.9・9.12）。
//!
//! 切替の土台（[`SwitchRig`]）の上で、本物の窓口の取り出しの系（`Input` の段）・切替の入口・kanade・
//! 偽の SHIORI を通す。背景スレッドは起こさず、読み直しの頼みは窓口の頼みの送出端へ直接入れる。
//! 確かめること: 条件を満たせば同じゴーストへの知らせなしの切替が 1 回受け付けられ（`OnGhostChanging`・
//! `OnClose` 0 件・起動の根は `OnGhostChanged`）、読み直しの途中の切替の頼みは今日どおり無視される／
//! 終了の後・切替の予約が在る・別のゴースト・引数の起動では切替の要求 0 件で `warn!` が 1 件。
//! 読み直しの切替が終わると成功した走行の残り（印つき）が消え、読み直しでない切替・失敗した切替では
//! 消さない（要件 5.9・9.9・9.10）。

use std::path::{Path, PathBuf};

use areka_kanade::ChangeOrigin;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;
use wintf::ecs::Input;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::UpdateDesk;
use crate::boot_config::BootContext;
use crate::boot_resolve::{DEFAULT_GHOST_FOLDER, GhostRoute};
use crate::emo2_boot::ghost_switch::{
    GhostSpec, PrevGhost, SwitchInFlight, SwitchRequest, SwitchStage, SwitchTarget, SwitchVerdict,
    request_ghost_switch,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;
use crate::exit_wait::begin_close;
use crate::update::worker::DeskAsk;

/// 起こし直しに台本で応える偽の SHIORI（起こすたびに新品の台本）。`OnGhostChanged` が 204 だと
/// 起動の木が `OnBoot` へ続くので、台本で応えて起動の根が `OnGhostChanged` だけであることを見る。
fn reloading(folder: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        let script = format!(r"\0{folder}\e");
        standard_script(&script).get("OnGhostChanged", Ok(Some(script.clone())))
    }))
}

/// ゴーストの `.update-work/` の下に、成功した走行の残り（印 `committed` と古い DLL の写し）を置く。
fn plant_committed(ghost_dir: &Path, run: &str) -> PathBuf {
    let dir = ghost_dir.join(".update-work").join(run);
    std::fs::create_dir_all(dir.join("old")).expect("走行フォルダを組む");
    std::fs::write(dir.join("old").join("yaya.dll"), b"old").expect("古い写しを置く");
    std::fs::write(dir.join("committed"), b"").expect("印を置く");
    dir
}

/// A を起こして定常に着いた土台（B は切替で起こせるが、起こされないことを数える）。
fn running_a() -> SwitchRig {
    let mut rig = SwitchRig::new(vec![("A", reloading("A")), ("B", reloading("B"))]);
    // 起こし直すときの窓の準備が閉包を投函する先（`Input` の段では走らない）。
    rig.world.insert_resource(WintfTaskPool::new());
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// 背景スレッドと同じ道（窓口の頼みの送出端）で、`ghost_dir` の読み直しを頼む。
fn ask_reload(rig: &SwitchRig, ghost_dir: PathBuf) {
    rig.world
        .non_send::<UpdateDesk>()
        .asks_tx
        .send(DeskAsk::Reload { ghost_dir })
        .expect("窓口の頼みの受信端は生きている");
}

/// 呼び出しの名前の列（Reference は見ない）。
fn ids(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|call| match call {
            RecordedCall::Get { id, .. } | RecordedCall::Notify { id, .. } => id.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn boots_of(rig: &SwitchRig, folder: &str) -> Vec<Vec<String>> {
    rig.calls(folder).iter().map(|c| ids(c)).collect()
}

fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events.iter().filter(|e| e.level == Level::WARN).collect()
}

/// 条件を満たした読み直しは、同じフォルダ・知らせなし・出どころ「自動」の切替として 1 回受け付けられ
/// （`info!`）、A が `OnGhostChanging`・`OnClose` 無しに起き直って起動の根は `OnGhostChanged`
/// （`OnBoot` 0 件・要件 5.5）。読み直しの途中の切替の頼みは `Busy` で無視され、B は起こされない（要件 5.8）。
#[test]
fn a_reload_of_the_running_ghost_switches_to_itself_silently_and_ignores_a_midway_switch() {
    let mut rig = running_a();
    let a_dir = rig.root.ghost_dir("A");

    let (flow, events) = capture(|| {
        ask_reload(&rig, a_dir.clone());
        rig.world.run_schedule(Input);
        let reserved = rig
            .world
            .get_non_send::<SwitchInFlight>()
            .map(|f| f.target.folder.clone());
        let midway = request_ghost_switch(
            &mut rig.world,
            SwitchRequest {
                ghost: GhostSpec::Folder("B".to_owned()),
                raise_event: true,
                origin: ChangeOrigin::Manual,
            },
        );
        let finished = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("A").len() == 2
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        });
        (reserved, midway, finished)
    });
    let (a, b) = (boots_of(&rig, "A"), boots_of(&rig, "B"));
    let exited = rig.exit_requested();
    let current = rig
        .world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone());
    assert!(rig.shutdown());

    assert_eq!(
        flow,
        (Some("A".to_owned()), SwitchVerdict::Busy, true),
        "(予約の切替先・途中の頼み・起き直した): {events:?}"
    );
    assert!(!exited, "終了しない: {events:?}");
    assert_eq!((a.len(), b.len()), (2, 0), "A を 1 回起こし直すだけ: {a:?}");
    assert_eq!(current.as_deref(), Some("A"));
    for name in ["OnGhostChanging", "OnClose"] {
        assert!(
            a.iter().flatten().all(|id| id != name),
            "{name} は 0 件: {a:?}"
        );
    }
    assert!(a[1].iter().any(|id| id == "OnGhostChanged"), "{a:?}");
    assert!(a[1].iter().all(|id| id != "OnBoot"), "{a:?}");

    let requested = named(&events, "update_reload_requested");
    assert_eq!(requested.len(), 1, "{events:?}");
    assert_eq!(requested[0].level, Level::INFO);
    assert_eq!(requested[0].field("verdict"), Some("Accepted"));
    assert!(named(&events, "update_reload_skipped").is_empty());
    let switched = named(&events, "ghost_switch_requested");
    assert_eq!(
        switched.len(),
        1,
        "切替を頼むのは読み直しの 1 回: {events:?}"
    );
    assert_eq!(switched[0].field_str("origin"), Some("automatic"));
    assert_eq!(switched[0].field("raise_event"), Some("false"));
    assert_eq!(switched[0].field("to"), Some("A"));
    assert_eq!(named(&events, "ghost_switch_busy").len(), 1, "{events:?}");
}

/// 1 件の読み直しの頼みを取り出しの系で捌き、切替の要求 0 件・`warn!(update_reload_skipped)` が
/// `reason` つきで 1 件だけ（ほかの `warn!` 0 件）であることを判定する。
fn assert_skipped(rig: &mut SwitchRig, ghost_dir: PathBuf, reason: &str) {
    let ((), events) = capture(|| {
        ask_reload(rig, ghost_dir);
        rig.world.run_schedule(Input);
    });
    assert!(
        named(&events, "ghost_switch_requested").is_empty()
            && named(&events, "ghost_switch_busy").is_empty()
            && named(&events, "update_reload_requested").is_empty(),
        "{reason}: 切替の入口を呼ばない: {events:?}"
    );
    let warned = warns(&events);
    assert_eq!(warned.len(), 1, "{reason}: {events:?}");
    assert_eq!(
        warned[0].field_str("event"),
        Some("update_reload_skipped"),
        "{events:?}"
    );
    assert_eq!(warned[0].field_str("reason"), Some(reason), "{events:?}");
}

/// 条件を欠く読み直しの頼みは、切替の要求 0 件・理由つきの `warn!` 1 件で落とす（頼み直さない）:
/// 切替の予約が在る／置き場のゴーストが頼みのゴーストと違う／引数の起動（フォルダ名が無い）／終了の後。
/// どの場合も A は起こし直されず、切替の予約は残らない。
#[test]
fn a_reload_that_misses_a_condition_requests_no_switch_and_warns_once() {
    let mut rig = running_a();
    let a_dir = rig.root.ghost_dir("A");

    // 切替の予約が在る（UI の側だけに置く＝kanade へは届かない）。
    rig.world.insert_non_send(SwitchInFlight {
        target: SwitchTarget {
            dir: rig.root.ghost_dir("B"),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: a_dir.clone(),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage: SwitchStage::SendOff,
    });
    assert_skipped(&mut rig, a_dir.clone(), "switching");
    rig.world.remove_non_send::<SwitchInFlight>();

    // 頼みのゴーストは置き場のゴーストでない（途中で切り替わった）。
    let b_dir = rig.root.ghost_dir("B");
    assert_skipped(&mut rig, b_dir, "other_ghost");

    // 引数の起動（根の目録の外＝フォルダ名が無い・裁定 17）。
    let ghost = rig.world.resource::<BootContext>().current.ghost.clone();
    {
        let mut ctx = rig.world.resource_mut::<BootContext>();
        ctx.argv_session = true;
        ctx.current.ghost.route = GhostRoute::Argv;
        ctx.current.ghost.folder = None;
    }
    assert_skipped(&mut rig, a_dir.clone(), "argv");
    {
        let mut ctx = rig.world.resource_mut::<BootContext>();
        ctx.argv_session = false;
        ctx.current.ghost = ghost;
    }

    // 終了が始まった後に届いた頼みは落とす（要件 7.4）。
    let _ = begin_close(&mut rig.world);
    assert_skipped(&mut rig, a_dir, "closing");

    let (a, exited) = (boots_of(&rig, "A").len(), rig.exit_requested());
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    assert!(rig.shutdown());
    assert_eq!((a, exited, reserved), (1, false, false));
}

/// 読み直しの切替が終わる（古い SHIORI を降ろして起き直ったゴーストが定常に入る）と、そのゴーストの
/// 印つきの残りが消え `info!(update_purge_done)`（`removed`＝1）が 1 件・`update_purge_held` 0 件。
/// 印の無いフォルダ（戻せなかった走行）は触らない。覚えたフォルダは使い切る。
///
/// # 非空虚性
/// 切替を終える腕から窓口を呼ばないと残りが消えず赤。受け付けで覚えないと同じく赤。
#[test]
fn a_finished_reload_purges_the_marked_leftover_once() {
    let mut rig = running_a();
    let a_dir = rig.root.ghost_dir("A");
    let marked = plant_committed(&a_dir, "run1");
    let unmarked = a_dir.join(".update-work").join("run2").join("old");
    std::fs::create_dir_all(&unmarked).expect("印の無い走行を組む");

    let (finished, events) = capture(|| {
        ask_reload(&rig, a_dir.clone());
        rig.world.run_schedule(Input);
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("A").len() == 2
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let remembered = rig
        .world
        .non_send::<UpdateDesk>()
        .purge_after_switch
        .clone();
    assert!(rig.shutdown());

    assert!(finished, "読み直しが終わる: {events:?}");
    assert!(!marked.exists(), "印つきの残りは消える: {events:?}");
    assert!(unmarked.exists(), "印の無い走行は触らない");
    assert_eq!(remembered, None, "覚えたフォルダは使い切る");
    let done = named(&events, "update_purge_done");
    assert_eq!(done.len(), 1, "{events:?}");
    assert_eq!(done[0].level, Level::INFO);
    assert_eq!(done[0].field("removed"), Some("1"));
    assert!(named(&events, "update_purge_held").is_empty(), "{events:?}");
}

/// 読み直しでない切替（メニューから同じゴーストへ）が終わっても、印つきの残りは消さない
/// （`update_purge_done`・`update_purge_held` 0 件）。
#[test]
fn a_switch_without_a_reload_leaves_the_marked_leftover() {
    let mut rig = running_a();
    let a_dir = rig.root.ghost_dir("A");
    let marked = plant_committed(&a_dir, "run1");

    let ((verdict, finished), events) = capture(|| {
        let verdict = request_ghost_switch(
            &mut rig.world,
            SwitchRequest {
                ghost: GhostSpec::Folder("A".to_owned()),
                raise_event: false,
                origin: ChangeOrigin::Manual,
            },
        );
        let finished = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("A").len() == 2
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        });
        (verdict, finished)
    });
    assert!(rig.shutdown());

    assert_eq!(
        (verdict, finished),
        (SwitchVerdict::Accepted, true),
        "{events:?}"
    );
    assert!(marked.exists(), "読み直しでない切替では消さない");
    for name in ["update_purge_done", "update_purge_held"] {
        assert!(named(&events, name).is_empty(), "{name}: {events:?}");
    }
}

/// 読み直しの切替が失敗する（起き直った A の SHIORI が接続に失敗し既定ゴーストへ戻る）と、覚えた
/// フォルダを捨てて消さない（`update_purge_done`・`update_purge_held` 0 件＝次の走行の始めに消える）。
///
/// # 非空虚性
/// 既定へ戻った定常到達でも消すと、A の残りが消えて赤。
#[test]
fn a_failed_reload_switch_drops_the_remembered_folder() {
    let a_script = FakeShiori::ScriptedThenConnectFail(Box::new(|| {
        standard_script(r"\0A\e").get("OnGhostChanged", Ok(Some(r"\0A\e".to_owned())))
    }));
    let default = FakeShiori::Scripted(Box::new(|| standard_script(r"\0emo2\e")));
    let mut rig = SwitchRig::new(vec![("A", a_script), (DEFAULT_GHOST_FOLDER, default)]);
    rig.world.insert_resource(WintfTaskPool::new());
    rig.plant_boot_record("A");
    rig.plant_boot_record(DEFAULT_GHOST_FOLDER);
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    let a_dir = rig.root.ghost_dir("A");
    let marked = plant_committed(&a_dir, "run1");

    let (welcomed, events) = capture(|| {
        ask_reload(&rig, a_dir.clone());
        rig.world.run_schedule(Input);
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls(DEFAULT_GHOST_FOLDER).len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let exited = rig.exit_requested();
    let remembered = rig
        .world
        .non_send::<UpdateDesk>()
        .purge_after_switch
        .clone();
    assert!(rig.shutdown());

    assert!(welcomed && !exited, "既定ゴーストへ戻る: {events:?}");
    assert_eq!(
        named(&events, "update_reload_requested").len(),
        1,
        "{events:?}"
    );
    assert_eq!(
        named(&events, "ghost_switch_target_fault").len(),
        1,
        "{events:?}"
    );
    assert!(marked.exists(), "失敗した切替では消さない");
    assert_eq!(remembered, None, "覚えたフォルダは捨てる");
    for name in ["update_purge_done", "update_purge_held"] {
        assert!(named(&events, name).is_empty(), "{name}: {events:?}");
    }
}
