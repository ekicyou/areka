//! 切替の記憶の時系列の統合テスト（areka-P0-ghost-shell-balloon-switch task 11.5・要件 12.5〜12.8・12.12・
//! design Flow 6）。
//!
//! 偽の SHIORI の土台で実行系に App スコープの置き場を渡し（[`SwitchRig::wire_app_memory`]）、最後に使った
//! ゴースト（`areka.last.ghost`）と起動中の印（`areka.last.running`）が、切替の各時点でどう書かれているかを
//! 実 fs から読む。動いている実行系の記憶は書き手への投函なので、読む前に置き場のゴーストの実行系の記憶の
//! 書き手へ反映の柵（`barrier`）を掛けて確定させる（時間では待たない）。降ろした後の読みは、降ろす処理が
//! 書き手を処理し切っているので柵を掛けない。判定は集めてから 1 回・降ろすのは必ず有界に行う。

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, load_scope};

use super::fallback_tests::{B_NAME, default_ghost, fallback_rig};
use super::*;
use crate::app_exit::quit_app;
use crate::boot_config::BootContext;
use crate::boot_resolve::{
    DEFAULT_GHOST_FOLDER, read_last_ghost, read_session_mark, write_session_mark,
};
use crate::emo2_boot::ghost_switch::{SwitchStage, WelcomeAttempt};
use crate::emo2_boot::ghost_switch_test_support::BALLOON;
use crate::ghost_session::GhostSlot;
use crate::{MarkVerdict, Teardown, after_run, settle_session_mark};

/// App スコープの記憶: （最後に使ったゴースト, 起動中の印）。
type AppMemory = (Option<String>, Option<String>);

/// 切替先の Ghost スコープの記憶: （最後のバルーン, 最後のシェル）。
type GhostMemory = (Option<String>, Option<String>);

/// argv で始まったプロセスの前から在る印。
const PRIOR_MARK: &str = "前の印";

/// App スコープの記憶を実 fs から読む。
fn app_memory(rig: &SwitchRig) -> AppMemory {
    let app = rig.app_dir();
    (read_last_ghost(&app), read_session_mark(&app))
}

/// `folder` の Ghost スコープの記憶を実 fs から読む（起動の結線が据える根と同じ場所）。
fn ghost_memory(rig: &SwitchRig, folder: &str) -> GhostMemory {
    let roots = ScopeRoots {
        ghost: Some(profile_areka_root(
            &rig.root.ghost_dir(folder).join("ghost").join("master"),
        )),
        ..ScopeRoots::default()
    };
    let entries = load_scope(PersistScope::Ghost, &roots, &FsPersistIo);
    let find = |key: PersistKey| {
        entries
            .iter()
            .find_map(|(k, v)| (*k == key).then(|| v.clone()))
    };
    (find(PersistKey::LastBalloon), find(PersistKey::LastShell))
}

/// 置き場のゴーストの実行系の記憶の書き手へ反映の柵を掛ける（それまでの投函が実 fs へ届いた）。
/// 置き場か実行系が無いか、書き手が止まっていれば `false`。
fn fence(rig: &SwitchRig) -> bool {
    rig.world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime())
        .is_some_and(|runtime| runtime.sylphya_publisher().barrier().is_ok())
}

/// 切替の段が「迎え入れ（`attempt`）」になるまで台詞を進めながら回す（終了が指示されたら打ち切る）。
/// 段が変わった呼び出しの中では、起こしたゴーストの定常到達はまだ処理されていない（通知の相は
/// 呼び出しの頭で受け口を読み切ってから振り分けるので、その後に起きたゴーストの通知は次の呼び出し）。
fn pump_to_welcoming(rig: &mut SwitchRig, attempt: WelcomeAttempt) -> bool {
    rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || rig.world.get_non_send::<SwitchInFlight>().map(|f| f.stage)
                == Some(SwitchStage::Welcoming { attempt })
    })
}

/// 目録の `folder` の descript の `name`（無ければフォルダ名）＝印に書かれるはずの名前。
fn name_of(rig: &SwitchRig, folder: &str) -> String {
    areka_ghost::catalog::list_ghosts(&rig.root)
        .into_iter()
        .find(|e| e.identity.folder == folder)
        .and_then(|e| e.identity.name)
        .unwrap_or_else(|| folder.to_owned())
}

/// `run()` の後の後始末を `fn main` と同じ順で踏む: 置き場から取り出し → 有界に降ろす → 印の始末。
/// 戻りは（降ろせたか・印の判定）。
fn after_run_and_settle(rig: &mut SwitchRig) -> (bool, Option<MarkVerdict>) {
    let mut after = after_run(&mut rig.world);
    rig.world.insert_non_send(GhostSlot(after.session.take()));
    let down_ok = rig.shutdown();
    let verdict = after.mark.as_ref().map(|mark| {
        settle_session_mark(
            mark,
            Teardown {
                run_ok: true,
                down_ok,
                shiori_cut: false,
            },
        )
    });
    (down_ok, verdict)
}

/// A → B（`raise_event`）の 1 周の各時点の観測。
#[derive(Debug, PartialEq, Eq)]
struct LapMemory {
    /// A の定常到達のあと（柵を掛けた・App）。
    booted_a: (bool, bool, AppMemory),
    /// 降ろした直後＝迎え入れ（切替先）の間（届いた・柵を掛けた・App・B の Ghost）。
    welcoming: (bool, bool, AppMemory, GhostMemory),
    /// B の定常到達の処理のあと（届いた・柵を掛けた・App・B の Ghost）。
    steady: (bool, bool, AppMemory, GhostMemory),
    /// メニューの「終了」相当できれいに終わった後始末のあと（降ろせた・判定・App）。
    exited: (bool, Option<MarkVerdict>, AppMemory),
}

/// A を起こして B へ切り替え、定常ののちきれいに終える 1 周。`argv_session` なら起動の文脈の argv の
/// 旗を立てる（前から在る印は [`PRIOR_MARK`]）。そうでなければ `fn main` と同じく起こす前に印＝A を書く。
/// 戻りは各時点の観測と、A の起動より後の記録。
fn switch_lap(argv_session: bool) -> (LapMemory, Vec<CapturedEvent>) {
    let mut rig = lap_rig(ghost_a(A_TO_B, None), ghost_b());
    rig.plant_boot_record("B");
    rig.wire_app_memory();
    let prior = if argv_session { PRIOR_MARK } else { "A" };
    write_session_mark(&rig.app_dir(), prior);
    rig.boot("A");
    rig.world.resource_mut::<BootContext>().argv_session = argv_session;

    let (lap, events) = capture(|| {
        let steady_a = rig.wait_steady();
        let booted_a = (steady_a, fence(&rig), app_memory(&rig));
        let reached = pump_to_welcoming(&mut rig, WelcomeAttempt::Target);
        let welcoming = (
            reached,
            fence(&rig),
            app_memory(&rig),
            ghost_memory(&rig, "B"),
        );
        let welcomed = pump_to_welcomed(&mut rig, "B", 1);
        let steady = (
            welcomed,
            fence(&rig),
            app_memory(&rig),
            ghost_memory(&rig, "B"),
        );
        quit_app(
            &mut rig.world,
            ExitOrigin::KanadeStopped(KanadeStopCause::Quit),
        );
        let (down_ok, verdict) = after_run_and_settle(&mut rig);
        LapMemory {
            booted_a,
            welcoming,
            steady,
            exited: (down_ok, verdict, app_memory(&rig)),
        }
    });
    (lap, events)
}

/// `event` の記録の `field` の値の列。
fn field_of(events: &[CapturedEvent], event: &str, field: &str) -> Vec<Option<String>> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .map(|e| e.field_str(field).map(str::to_owned))
        .collect()
}

fn some(s: &str) -> Option<String> {
    Some(s.to_owned())
}

/// A → B の切替の記憶の時系列（要件 12.6・12.2）: 降ろした直後（迎え入れの間）は最後のゴースト＝既定・
/// 印＝B・B のバルーンの記憶なし → B の定常到達のあと最後のゴースト＝B・印＝B・B のバルーンとシェルの
/// 記憶あり → メニューの「終了」相当できれいに終わると印が消え、最後のゴーストは B のまま。
///
/// # 非空虚性
/// 起動の直後の記憶を経路で止めずに書くと（`on_boot_ok` が `record_last_used_at_boot` を通らない）、
/// 迎え入れの間の最後のゴーストが B になって赤。定常到達の記憶を書かないと、定常のあとの最後のゴーストが
/// 既定のまま・B のバルーンとシェルが無くて赤。降ろした直後の書き込みを外すと迎え入れの間が A・A で赤。
#[test]
fn switch_memory_is_default_while_welcoming_then_target_at_steady() {
    let (lap, events) = switch_lap(false);

    assert_eq!(
        (
            lap,
            field_of(&events, "switch_drop_recorded", "mark"),
            levels_of(&events, "session_mark_steady"),
        ),
        (
            LapMemory {
                booted_a: (true, true, (some("A"), some("A"))),
                welcoming: (
                    true,
                    true,
                    (some(DEFAULT_GHOST_FOLDER), some("B")),
                    (None, None)
                ),
                steady: (
                    true,
                    true,
                    (some("B"), some("B")),
                    (some(BALLOON), some("master"))
                ),
                exited: (true, Some(MarkVerdict::Clear), (some("B"), None)),
            },
            vec![some("B")],
            vec![tracing::Level::INFO],
        ),
        "切替の記憶の時系列が崩れた（各時点の観測・降ろした直後の印の記録・定常到達の印の記録）: {events:?}"
    );
}

/// argv で始まったプロセス（要件 12.5）: 最後のゴーストの書き方は同じ時系列（迎え入れの間は既定・定常の
/// あとは B）で、印は一度も書かれず消されもしない（前から在った印がそのまま残る）。
///
/// # 非空虚性
/// 降ろした直後か定常到達で argv の旗を見ずに印を書くと印が B になって赤。後始末で旗を見ずに消すと
/// 印が無くなって赤。
#[test]
fn argv_session_switch_follows_last_ghost_but_never_touches_mark() {
    let (lap, events) = switch_lap(true);
    let prior = some(PRIOR_MARK);

    assert_eq!(
        (
            lap,
            field_of(&events, "switch_drop_recorded", "mark"),
            (
                levels_of(&events, "session_mark_written"),
                levels_of(&events, "session_mark_steady"),
                levels_of(&events, "session_mark_cleared"),
                levels_of(&events, "session_mark_untouched_argv"),
            ),
        ),
        (
            LapMemory {
                booted_a: (true, true, (some("A"), prior.clone())),
                welcoming: (
                    true,
                    true,
                    (some(DEFAULT_GHOST_FOLDER), prior.clone()),
                    (None, None)
                ),
                steady: (
                    true,
                    true,
                    (some("B"), prior.clone()),
                    (some(BALLOON), some("master"))
                ),
                exited: (true, Some(MarkVerdict::Untouched), (some("B"), prior)),
            },
            vec![some("-")],
            (
                vec![],
                vec![],
                vec![],
                vec![tracing::Level::DEBUG, tracing::Level::DEBUG]
            ),
        ),
        "argv のプロセスの切替が印に触れたか最後のゴーストの時系列が崩れた（各時点の観測・降ろした直後の印の\
         記録・印を書いた／定常で書いた／消した／触れなかった記録）: {events:?}"
    );
}

/// B が接続に失敗 → 既定へ戻す（要件 12.7）: 戻しの間（迎え入れ（既定））は印＝B の名前・最後のゴースト＝
/// 既定 → 既定の定常到達のあと印＝既定の名前・最後のゴースト＝既定。
///
/// # 非空虚性
/// 既定へ戻す処理が印を書き換えると戻しの間の印が既定の名前になって赤。定常到達で段を見て既定の段を
/// 飛ばすと、定常のあとの印が B の名前のままで赤。
#[test]
fn default_fallback_keeps_target_mark_until_default_steady() {
    let mut rig = fallback_rig(
        ghost_a(A_TO_B, None),
        FakeShiori::ConnectFail,
        Some(default_ghost()),
    );
    rig.wire_app_memory();
    write_session_mark(&rig.app_dir(), "A");
    rig.boot("A");
    let steady_a = rig.wait_steady();
    let reached = pump_to_welcoming(&mut rig, WelcomeAttempt::Default);
    let falling_back = (reached, fence(&rig), app_memory(&rig));
    let welcomed = pump_to_welcomed(&mut rig, DEFAULT_GHOST_FOLDER, 1);
    let steady = (welcomed, fence(&rig), app_memory(&rig));
    let exit_requested = rig.exit_requested();
    let default_name = name_of(&rig, DEFAULT_GHOST_FOLDER);
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (steady_a, falling_back, steady, exit_requested, shutdown_ok),
        (
            true,
            (true, true, (some(DEFAULT_GHOST_FOLDER), some(B_NAME))),
            (true, true, (some(DEFAULT_GHOST_FOLDER), Some(default_name))),
            false,
            true,
        ),
        "既定へ戻す間と既定の定常のあとの記憶が崩れた（A の定常・戻しの間・既定の定常のあと・終了の指示・\
         降ろせた）"
    );
}

/// 根に既定ゴーストが無い＋B の失敗（致命・要件 12.8）: 最後のゴースト＝既定・印＝B の名前のまま。
/// 後始末を通しても印は残る（次の起動は既定ゴーストで Ref7＝B の名前）。
///
/// # 非空虚性
/// 致命の経路が印を書き換えるか後始末が消すと、印が B の名前でなくなって赤。
#[test]
fn switch_fatal_leaves_default_last_ghost_and_target_mark() {
    let mut rig = fallback_rig(ghost_a(A_TO_B, None), FakeShiori::ConnectFail, None);
    rig.wire_app_memory();
    write_session_mark(&rig.app_dir(), "A");
    rig.boot("A");
    let steady_a = rig.wait_steady();
    let exited = rig.pump_talking_until(SwitchRig::exit_requested);
    let at_fatal = (fence(&rig), app_memory(&rig));
    let settled = after_run_and_settle(&mut rig);
    let after_cleanup = app_memory(&rig);

    assert_eq!(
        (steady_a, exited, at_fatal, settled, after_cleanup),
        (
            true,
            true,
            (true, (some(DEFAULT_GHOST_FOLDER), some(B_NAME))),
            (true, Some(MarkVerdict::Keep("switch_fatal"))),
            (some(DEFAULT_GHOST_FOLDER), some(B_NAME)),
        ),
        "致命のあとの記憶が崩れた（A の定常・終了の指示・致命の時点（柵・App）・後始末（降ろせた・判定）・\
         後始末のあと）"
    );
}
