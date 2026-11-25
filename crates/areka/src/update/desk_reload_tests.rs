//! 読み直しの頼みの兄弟テスト。
//!
//! 切替の土台（[`SwitchRig`]）の上で、本物の窓口の取り出しの系（`Input` の段）・切替の入口・kanade・
//! 偽の SHIORI を通す。背景スレッドは起こさず、読み直しの頼み（後送りの列つき）は窓口の頼みの送出端へ
//! 直接入れる。確かめること: 条件を満たせば同じゴーストへの知らせなしの切替が 1 回受け付けられ
//! （`OnGhostChanging`・`OnClose` 0 件・起動の知らせは列の先頭で `OnGhostChanged`・`OnBoot` 0 件）、
//! 切替の終わりで列の残りが新しいゴーストへ届き、読み直しの途中の切替の頼みは `Busy` で断られる／
//! 読み直せないとき、切替の予約が在る・引数の起動・入口が受け付けないなら列を今のゴーストへ送り、
//! 終了の後・別のゴーストなら捨てる／切替が既定へ戻ると列は捨てる。読み直しの切替が終わると成功した
//! 走行の残り（印つき）が消え、読み直しでない切替・失敗した切替では消さない。

use std::path::PathBuf;

use areka_kanade::ChangeOrigin;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;
use wintf::ecs::Input;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{UpdateDesk, ask_reload_for_test};
use crate::sample_test_support::switch_rig;
use crate::session_tests::events_named;
use crate::update::UpdateTail;
use crate::update::update_test_support::{plant_committed, refs_of};
use areka_session::boot_config::BootContext;
use areka_session::boot_resolve::{DEFAULT_GHOST_FOLDER, GhostRoute};
use areka_session::exit_wait::begin_close;
use areka_session::ghost_session::shell_balloon_switch_session_lap_test_support::kinds;
use areka_session::ghost_switch::{
    GhostSpec, PrevGhost, SwitchInFlight, SwitchRequest, SwitchStage, SwitchTarget, SwitchVerdict,
    request_ghost_switch,
};
use areka_session::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

/// 列を受ける回数の上限（偽の SHIORI の台本に積む数）。
const TAIL_ROUNDS: usize = 4;

/// 後送りの列（ゴーストの `OnUpdateComplete` → 総括）。
fn tail() -> UpdateTail {
    vec![
        (
            "OnUpdateComplete",
            vec!["changed".to_owned(), "a.txt".to_owned()],
        ),
        ("OnUpdateResult", vec!["ghost\x01OK\x011".to_owned()]),
    ]
}

/// 偽の SHIORI（起こすたびに新品の台本）。`OnGhostChanged` には台本で応え（204 だと起動の木が
/// `OnBoot` へ続く）、列の 2 語には返事なしで何回か応える。
fn reloading(folder: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        let script = format!(r"\0{folder}\e");
        let mut builder = standard_script(&script).get("OnGhostChanged", Ok(Some(script.clone())));
        for _ in 0..TAIL_ROUNDS {
            builder = builder
                .get("OnUpdateComplete", Ok(None))
                .get("OnUpdateResult", Ok(None));
        }
        builder
    }))
}

/// A を起こして定常に着いた土台（B は切替で起こせるが、起こされないことを数える）。
fn running_a() -> SwitchRig {
    let mut rig = switch_rig(vec![("A", reloading("A")), ("B", reloading("B"))]);
    // 起こし直すときの窓の準備が閉包を投函する先（`Input` の段では走らない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

fn boots_of(rig: &SwitchRig, folder: &str) -> Vec<Vec<String>> {
    rig.calls(folder).iter().map(|c| kinds(c)).collect()
}

/// `folder` の全起動で受けた `GET id` の数。
fn gets(rig: &SwitchRig, folder: &str, id: &str) -> usize {
    let want = format!("GET {id}");
    boots_of(rig, folder)
        .iter()
        .flatten()
        .filter(|c| **c == want)
        .count()
}

fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events.iter().filter(|e| e.level == Level::WARN).collect()
}

/// 条件を満たした読み直しは、同じフォルダ・知らせなし・出どころ「自動」の切替として 1 回受け付けられ
/// （`info!`）、A が `OnGhostChanging`・`OnClose` 無しに起き直る。起動の知らせは列の先頭（渡した
/// Reference の `OnUpdateComplete`）で `OnGhostChanged`・`OnBoot` 0 件、切替の終わりで列の残り
/// （`OnUpdateResult`）が続けて届く（`info!(update_tail_sent when=after_switch count=1)`）。
/// 読み直しの途中の切替の頼みは `Busy` で無視され、B は起こされない。
///
/// # 非空虚性
/// 列の先頭を切替の欄へ入れないと起動の知らせが `OnGhostChanged` になって赤。切替の終わりで残りを
/// 送らないと `OnUpdateResult` が届かず期限切れで赤。
#[test]
fn a_reload_of_the_running_ghost_boots_with_the_tail_head_and_sends_the_rest_after_the_switch() {
    let mut rig = running_a();
    let a_dir = rig.root.ghost_dir("A");

    let (flow, events) = capture(|| {
        ask_reload_for_test(&rig.world, a_dir.clone(), tail());
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
                boot_event: None,
            },
        );
        let finished = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("A").len() == 2
                    && rig.world.get_non_send::<SwitchInFlight>().is_none()
                    && gets(rig, "A", "OnUpdateResult") == 1)
        });
        (reserved, midway, finished)
    });
    let (calls, b) = (rig.calls("A"), boots_of(&rig, "B"));
    let exited = rig.exit_requested();
    let current = rig
        .world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone());
    let remembered = rig.world.non_send::<UpdateDesk>().after_switch.clone();
    assert!(rig.shutdown());

    assert_eq!(
        flow,
        (Some("A".to_owned()), SwitchVerdict::Busy, true),
        "(予約の切替先・途中の頼み・起き直して残りが届いた): {events:?}"
    );
    assert!(!exited, "終了しない: {events:?}");
    assert_eq!((calls.len(), b.len()), (2, 0), "A を 1 回起こし直すだけ");
    assert_eq!(current.as_deref(), Some("A"));
    assert_eq!(remembered, None, "覚えた物は使い切る");
    let a: Vec<Vec<String>> = calls.iter().map(|c| kinds(c)).collect();
    for name in ["GET OnGhostChanging", "NOTIFY OnClose", "GET OnClose"] {
        assert!(
            a.iter().flatten().all(|id| id != name),
            "{name} は 0 件: {a:?}"
        );
    }
    assert_eq!(
        a[1],
        [
            "NOTIFY OnInitialize",
            "GET username",
            "GET OnUpdateComplete",
            "NOTIFY basewareversion",
            "GET OnUpdateResult",
        ],
        "起き直した A の呼び出し: {events:?}"
    );
    assert_eq!(
        refs_of(&calls[1], "OnUpdateComplete"),
        Some(tail()[0].1.clone())
    );
    assert_eq!(
        refs_of(&calls[1], "OnUpdateResult"),
        Some(tail()[1].1.clone())
    );
    assert!(
        a[0].iter().all(|c| !c.contains("OnUpdate")),
        "古い A へは送らない"
    );

    let requested = events_named(&events, "update_reload_requested");
    assert_eq!(requested.len(), 1, "{events:?}");
    assert_eq!(requested[0].level, Level::INFO);
    assert_eq!(requested[0].field("verdict"), Some("Accepted"));
    assert!(events_named(&events, "update_reload_skipped").is_empty());
    let switched = events_named(&events, "ghost_switch_requested");
    assert_eq!(
        switched.len(),
        1,
        "切替を頼むのは読み直しの 1 回: {events:?}"
    );
    assert_eq!(switched[0].field_str("origin"), Some("automatic"));
    assert_eq!(switched[0].field("raise_event"), Some("false"));
    assert_eq!(switched[0].field("to"), Some("A"));
    assert_eq!(
        switched[0].field("boot_event"),
        Some("Some(\"OnUpdateComplete\")")
    );
    assert_eq!(
        events_named(&events, "ghost_switch_busy").len(),
        1,
        "{events:?}"
    );
    let sent = events_named(&events, "update_tail_sent");
    assert_eq!(sent.len(), 1, "{events:?}");
    assert_eq!(sent[0].level, Level::INFO);
    assert_eq!(sent[0].field_str("when"), Some("after_switch"));
    assert_eq!(sent[0].field("count"), Some("1"));
    assert!(events_named(&events, "update_tail_dropped").is_empty());
}

/// 読み直せないときの列の行き先。
enum Fate {
    /// 今のゴーストへ順に送る（`info!(update_tail_sent when=no_reload)`）。
    Sent,
    /// 捨てる（`warn!(update_tail_dropped reason)`）。
    Dropped(&'static str),
}

/// 1 件の読み直しの頼みを取り出しの系で捌き、切替の要求 0 件・判定の記録 `warn` 1 件（`event`・
/// `field` の組）と列の行き先を判定する。送るなら A の最初の起動が列の 2 語を受けるまで回す。
fn assert_not_reloaded(
    rig: &mut SwitchRig,
    ghost_dir: PathBuf,
    verdict: (&str, &str, &str),
    fate: Fate,
) {
    let (label, field, value) = verdict;
    let before = gets(rig, "A", "OnUpdateResult");
    let ((), events) = capture(|| {
        ask_reload_for_test(&rig.world, ghost_dir, tail());
        rig.world.run_schedule(Input);
    });
    assert!(
        events_named(&events, "ghost_switch_requested").is_empty()
            && events_named(&events, "ghost_switch_busy").is_empty(),
        "{value}: 切替の要求 0 件: {events:?}"
    );
    let decided = events_named(&events, label);
    assert_eq!(decided.len(), 1, "{value}: {events:?}");
    assert_eq!(decided[0].level, Level::WARN, "{value}");
    // `reason` は文字列・`verdict` は Debug の綴り。
    let got = decided[0].field_str(field).or(decided[0].field(field));
    assert_eq!(got, Some(value), "{value}: {events:?}");
    let (sent, dropped) = (
        events_named(&events, "update_tail_sent"),
        events_named(&events, "update_tail_dropped"),
    );
    match fate {
        Fate::Sent => {
            assert!(dropped.is_empty(), "{value}: {events:?}");
            assert_eq!(sent.len(), 1, "{value}: {events:?}");
            assert_eq!(sent[0].level, Level::INFO);
            assert_eq!(sent[0].field_str("when"), Some("no_reload"), "{value}");
            assert_eq!(sent[0].field("count"), Some("2"), "{value}");
            assert!(
                rig.pump_input_until(|rig| gets(rig, "A", "OnUpdateResult") == before + 1),
                "{value}: 列が今のゴーストへ届く"
            );
            let a = &boots_of(rig, "A")[0];
            assert_eq!(
                a[a.len() - 2..],
                ["GET OnUpdateComplete", "GET OnUpdateResult"],
                "{value}: 列の順"
            );
        }
        Fate::Dropped(reason) => {
            assert!(sent.is_empty(), "{value}: {events:?}");
            assert_eq!(dropped.len(), 1, "{value}: {events:?}");
            assert_eq!(dropped[0].level, Level::WARN);
            assert_eq!(dropped[0].field_str("reason"), Some(reason), "{events:?}");
            assert_eq!(gets(rig, "A", "OnUpdateResult"), before, "{value}");
        }
    }
    // 判定の記録と列を捨てた記録のほかに `warn!` は無い（入口が断った記録を除く）。
    let others: Vec<_> = warns(&events)
        .into_iter()
        .filter(|e| {
            !matches!(
                e.field_str("event"),
                Some("update_reload_skipped" | "update_reload_requested")
                    | Some("update_tail_dropped" | "ghost_switch_unknown")
            )
        })
        .collect();
    assert!(others.is_empty(), "{value}: {others:?}");
}

/// 読み直しを頼めないときの列の行き先: 別のゴースト・終了の後は捨て（`warn!` 1 件）、切替の予約が在る・
/// 引数の起動・入口が受け付けない（`NotFound`）は今のゴーストへ順に送る（`info!` 1 件）。どの場合も
/// 切替の要求 0 件で A は起こし直されず、切替の予約は残らない。
#[test]
fn a_reload_that_cannot_run_sends_or_drops_the_tail_by_reason() {
    let mut rig = running_a();
    let a_dir = rig.root.ghost_dir("A");
    let skipped = |reason| ("update_reload_skipped", "reason", reason);

    // 頼みのゴーストは置き場のゴーストでない（途中で切り替わった）→ 捨てる。
    let b_dir = rig.root.ghost_dir("B");
    assert_not_reloaded(
        &mut rig,
        b_dir,
        skipped("other_ghost"),
        Fate::Dropped("other_ghost"),
    );

    // 切替の予約が在る（UI の側だけに置く＝kanade へは届かない）→ 今のゴーストへ送る。
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
        boot_event: None,
        copy: areka_session::switch_copy::SwitchCopy::None,
    });
    assert_not_reloaded(&mut rig, a_dir.clone(), skipped("switching"), Fate::Sent);
    rig.world.remove_non_send::<SwitchInFlight>();

    // 引数の起動（根の目録の外＝フォルダ名が無い）→ 今のゴーストへ送る。
    let ghost = rig.world.resource::<BootContext>().current.ghost.clone();
    {
        let mut ctx = rig.world.resource_mut::<BootContext>();
        ctx.argv_session = true;
        ctx.current.ghost.route = GhostRoute::Argv;
        ctx.current.ghost.folder = None;
    }
    assert_not_reloaded(&mut rig, a_dir.clone(), skipped("argv"), Fate::Sent);

    // 切替の入口が受け付けない（目録に無いフォルダ名＝`NotFound`）→ 今のゴーストへ送る。
    {
        let mut ctx = rig.world.resource_mut::<BootContext>();
        ctx.argv_session = false;
        ctx.current.ghost = ghost;
        ctx.current.ghost.folder = Some("Z".to_owned());
    }
    assert_not_reloaded(
        &mut rig,
        a_dir.clone(),
        ("update_reload_requested", "verdict", "NotFound"),
        Fate::Sent,
    );
    rig.world.resource_mut::<BootContext>().current.ghost.folder = Some("A".to_owned());

    // 終了が始まった後に届いた頼みは落とし、列も捨てる。
    let _ = begin_close(&mut rig.world);
    assert_not_reloaded(
        &mut rig,
        a_dir,
        skipped("closing"),
        Fate::Dropped("closing"),
    );

    let (a, exited) = (boots_of(&rig, "A").len(), rig.exit_requested());
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    assert!(rig.shutdown());
    assert_eq!((a, exited, reserved), (1, false, false));
}

/// 置き場にゴーストが居ないと、読み直しの頼みは別のゴーストとして断られ、列は捨てる（送り先が無い）。
#[test]
fn a_reload_without_a_running_ghost_drops_the_tail() {
    let mut rig = running_a();
    let a_dir = rig.root.ghost_dir("A");
    assert!(rig.shutdown(), "A を降ろす");

    let ((), events) = capture(|| {
        ask_reload_for_test(&rig.world, a_dir, tail());
        rig.world.run_schedule(Input);
    });
    let skipped = events_named(&events, "update_reload_skipped");
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].field_str("reason"), Some("other_ghost"));
    let dropped = events_named(&events, "update_tail_dropped");
    assert_eq!(dropped.len(), 1, "{events:?}");
    assert_eq!(dropped[0].level, Level::WARN);
    assert_eq!(dropped[0].field_str("reason"), Some("other_ghost"));
    assert!(
        events_named(&events, "update_tail_sent").is_empty(),
        "{events:?}"
    );
    assert!(events_named(&events, "ghost_switch_requested").is_empty());
}

/// 読み直しの切替が終わる（古い SHIORI を降ろして起き直ったゴーストが定常に入る）と、そのゴーストの
/// 印つきの残りが消え `info!(update_purge_done)`（`removed`＝1）が 1 件・`update_purge_held` 0 件。
/// 印の無いフォルダ（戻せなかった走行）は触らない。覚えた物は使い切る。列の残りが空なら送らない。
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
        // 列は先頭だけ（ゴーストが対象でない回の総括）。
        ask_reload_for_test(&rig.world, a_dir.clone(), tail().split_off(1));
        rig.world.run_schedule(Input);
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("A").len() == 2
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let remembered = rig.world.non_send::<UpdateDesk>().after_switch.clone();
    let second = boots_of(&rig, "A")[1].clone();
    assert!(rig.shutdown());

    assert!(finished, "読み直しが終わる: {events:?}");
    assert!(!marked.exists(), "印つきの残りは消える: {events:?}");
    assert!(unmarked.exists(), "印の無い走行は触らない");
    assert_eq!(remembered, None, "覚えた物は使い切る");
    assert!(
        second.contains(&"GET OnUpdateResult".to_owned()),
        "起動の知らせは総括: {second:?}"
    );
    let done = events_named(&events, "update_purge_done");
    assert_eq!(done.len(), 1, "{events:?}");
    assert_eq!(done[0].level, Level::INFO);
    assert_eq!(done[0].field("removed"), Some("1"));
    assert!(
        events_named(&events, "update_purge_held").is_empty(),
        "{events:?}"
    );
    assert!(
        events_named(&events, "update_tail_sent").is_empty(),
        "{events:?}"
    );
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
                boot_event: None,
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
    for name in ["update_purge_done", "update_purge_held", "update_tail_sent"] {
        assert!(events_named(&events, name).is_empty(), "{name}: {events:?}");
    }
}

/// 読み直しの切替が失敗する（起き直った A の SHIORI が接続に失敗し既定ゴーストへ戻る）と、覚えた
/// フォルダと列の残りを捨てる: 残りは消さず（`update_purge_done`・`update_purge_held` 0 件＝次の走行の
/// 始めに消える）、既定ゴーストは `OnBoot` で起きて列の語を 1 件も受けず、`warn!(update_tail_dropped
/// reason=switch_failed)` が 1 件。
///
/// # 非空虚性
/// 既定へ戻った定常到達でも消す・送ると、A の残りが消える・既定へ `OnUpdateResult` が届いて赤。
#[test]
fn a_failed_reload_switch_drops_the_remembered_folder_and_the_tail() {
    let a_script = FakeShiori::ScriptedThenConnectFail(Box::new(|| {
        standard_script(r"\0A\e").get("OnGhostChanged", Ok(Some(r"\0A\e".to_owned())))
    }));
    let default = FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0emo2\e")
            .get("OnUpdateComplete", Ok(None))
            .get("OnUpdateResult", Ok(None))
    }));
    let mut rig = switch_rig(vec![("A", a_script), (DEFAULT_GHOST_FOLDER, default)]);
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record(DEFAULT_GHOST_FOLDER);
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    let a_dir = rig.root.ghost_dir("A");
    let marked = plant_committed(&a_dir, "run1");

    let (welcomed, events) = capture(|| {
        ask_reload_for_test(&rig.world, a_dir.clone(), tail());
        rig.world.run_schedule(Input);
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls(DEFAULT_GHOST_FOLDER).len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let exited = rig.exit_requested();
    let remembered = rig.world.non_send::<UpdateDesk>().after_switch.clone();
    let defaults = boots_of(&rig, DEFAULT_GHOST_FOLDER);
    assert!(rig.shutdown());

    assert!(welcomed && !exited, "既定ゴーストへ戻る: {events:?}");
    assert_eq!(
        events_named(&events, "update_reload_requested").len(),
        1,
        "{events:?}"
    );
    assert_eq!(
        events_named(&events, "ghost_switch_target_fault").len(),
        1,
        "{events:?}"
    );
    assert!(marked.exists(), "失敗した切替では消さない");
    assert_eq!(remembered, None, "覚えた物は捨てる");
    for name in ["update_purge_done", "update_purge_held", "update_tail_sent"] {
        assert!(events_named(&events, name).is_empty(), "{name}: {events:?}");
    }
    assert!(
        defaults[0].contains(&"GET OnBoot".to_owned()),
        "{defaults:?}"
    );
    assert!(
        defaults[0].iter().all(|c| !c.contains("OnUpdate")),
        "既定ゴーストへ列は送らない: {defaults:?}"
    );
    let dropped = events_named(&events, "update_tail_dropped");
    assert_eq!(dropped.len(), 1, "{events:?}");
    assert_eq!(dropped[0].level, Level::WARN);
    assert_eq!(dropped[0].field_str("reason"), Some("switch_failed"));
}
