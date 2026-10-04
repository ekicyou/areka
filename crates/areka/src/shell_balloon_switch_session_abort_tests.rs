//! シェルの切替の中止・失敗・終了・ゴースト切替の統合テスト（spec: areka-P0-shell-balloon-switch
//! task 11.2・要件 5.1〜5.3・5.5〜5.7・8.5・11.4〜11.6・design「Integration Tests」・Flow 4）。
//!
//! 土台は 11.1 の [`lap_rig_of`]（偽の SHIORI のゴースト A〔B も置ける〕・A の 2 つ目のシェル・窓の
//! 一式・GPU 資源・本番の `Input`・`Update` の段）。判定は集めてから 1 回。
//!
//! 「記憶 0」は、切替の経路の `LastShell` の投函（`last_shell_recorded`）が 0 件で、実 fs の
//! `LastShell` が起動の成功の書いた `master` のまま、の 2 つで見る。

use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;
use wintf::ecs::pointer::DoubleClick;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::GhostSlot;
use super::shell_balloon_switch_session_balloon_tests::scope0_text;
use super::shell_balloon_switch_session_lap_tests::{
    ACCEPT, BOOT_TO_SECOND, BOOT_TO_SECOND_PLAIN, CREEP, LapRig, LookLog, NO_TICKS, SECOND,
    UNBOUNDED, calls_a, done_marks, got, idle, kinds, lap_rig_of, last_shell, look_of,
    recorded_shells,
};
use crate::app_exit::{ExitOrigin, FirstExit, on_ghost_os_close};
use crate::boot_resolve::read_last_shell;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, SwitchInFlight, SwitchRequest, request_ghost_switch,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinOrigin, SkinRequest, SkinSpec, SkinVerdict, request_skin_switch,
};
use crate::emo2_boot::spine::ScriptedShioriBackendBuilder;
use crate::emo2_boot::target_map::balloon_target;
use crate::input_events::user_break::on_left_press;

/// A だけの土台（2 つ目のシェルつき）。
fn rig_a(script: impl Fn() -> ScriptedShioriBackendBuilder + 'static) -> LapRig {
    lap_rig_of(
        vec![("A", FakeShiori::Scripted(Box::new(script)))],
        &[SECOND],
    )
}

/// `event` の記録の数。
fn count(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// 取りやめ（`skin_switch_dropped`）の理由の並び。
fn dropped_reasons(events: &[CapturedEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("skin_switch_dropped"))
        .map(|e| e.field_str("reason").map(str::to_owned))
        .collect()
}

/// UI スレッドの `error!` の（目印・段）の並び。
fn errors(events: &[CapturedEvent]) -> Vec<(Option<String>, Option<String>)> {
    events
        .iter()
        .filter(|e| e.level == Level::ERROR)
        .map(|e| {
            (
                e.field_str("event").map(str::to_owned),
                e.field_str("stage").map(str::to_owned),
            )
        })
        .collect()
}

/// 切替の終わり方の記録（完了・中止・失敗・取りやめ）の数。
fn outcomes(events: &[CapturedEvent]) -> [usize; 4] {
    [
        count(events, "skin_switch_done"),
        count(events, "skin_switch_cancelled"),
        count(events, "skin_switch_failed"),
        count(events, "skin_switch_dropped"),
    ]
}

/// scope 0 のバルーンの表示。
fn balloon0_visible(rig: &SwitchRig) -> Option<bool> {
    rig.world
        .get_non_send::<Emo2Wiring>()
        .and_then(|w| w.presenter().target_visible(balloon_target(0)))
}

/// 置き場のゴーストの今のシェルのフォルダ名。
fn current_shell(rig: &SwitchRig) -> Option<String> {
    rig.world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.current_shell_folder())
}

// ---------------------------------------------------------------- 中止

/// `\-` 入りの `OnShellChanging` の台詞（待ちの間に中断する）。
const CHANGING_WITH_QUIT: &str = r"\0着替えます\_w[5000]\-\e";

/// `OnShellChanging` の台詞の中断（要件 5.1〜5.3・11.4）: `\-` 入りの印の台詞をバルーンの左
/// ダブルクリックで止めると、切替は中止（`skin_switch_cancelled` 1 件）で、差し替え・
/// `OnShellChanged`・記憶は 0、終了へ結ばれず（`OnClose` も終了の指示も無い）、バルーンは今日の
/// 規則で隠れる。直後の新しい切替要求は受け付けられ（kanade は定常のまま）、最後まで通る。
///
/// # 非空虚性
/// 印の台詞の中断を見分けず今日どおり `\-` の予約で終了系列へ結ぶと（kanade の印の台詞の中断の
/// 判定を外す）、中止の記録が出ず SHIORI が降ろされ（`UNLOAD`）終了が指示されて赤。中止を差し替えへ
/// 進めると完了の記録と `OnShellChanged` が出て赤。
#[test]
fn marked_talk_break_cancels_the_shell_switch_without_quitting() {
    let mut lap = rig_a(|| {
        standard_script(BOOT_TO_SECOND)
            .get("OnShellChanging", Ok(Some(CHANGING_WITH_QUIT.to_owned())))
            .get("OnShellChanging", Ok(None))
            .get("OnShellChanged", Ok(None))
    });
    let steady = lap.rig.wait_steady();
    let mut log = LookLog::default();
    let windows = lap.windows.clone();

    // 印の台詞の文字が全部出て `\_w[5000]` に入るまで 1 ms ずつ進める（中断がその台詞に当たる）。
    let playing = lap.frames_until(CREEP, |rig| {
        log.record(look_of(rig, &windows));
        got(rig, "OnShellChanging", 1) && scope0_text(rig).contains("着替えます")
    });
    let (cancelled, events) = capture(|| {
        let accepted = on_left_press(&mut lap.rig.world, 0, DoubleClick::Left, false);
        let settled = lap.frames_until(CREEP, |rig| {
            log.record(look_of(rig, &windows));
            idle(rig)
        });
        (accepted, settled)
    });
    let after_cancel = (
        kinds(&calls_a(&lap.rig)),
        balloon0_visible(&lap.rig),
        lap.rig.exit_requested(),
        current_shell(&lap.rig),
    );

    // 直後の新しい切替要求（メニュー相当）は受理され、最後まで通る。
    let verdict = request_skin_switch(
        &mut lap.rig.world,
        SkinRequest {
            kind: SkinKind::Shell,
            target: SkinSpec::Folder(SECOND.to_owned()),
            origin: SkinOrigin::Menu,
        },
    );
    let next_done = lap.frames_until(UNBOUNDED, |rig| got(rig, "OnShellChanged", 1) && idle(rig));
    let last = last_shell(&lap.rig);
    let shutdown_ok = lap.rig.shutdown();

    assert_eq!(
        (
            (steady, playing, cancelled),
            outcomes(&events),
            (recorded_shells(&events), done_marks(&events)),
            after_cancel,
            (log.distinct.len(), log.blank_after_full),
            (verdict, next_done, last, shutdown_ok),
        ),
        (
            (true, true, (true, true)),
            [0, 1, 0, 0],
            (Vec::new(), Vec::new()),
            (
                vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnBoot".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                    "GET OnShellChanging".to_owned(),
                    "GET OnTranslate".to_owned(),
                ],
                Some(false),
                false,
                Some("master".to_owned())
            ),
            (1, false),
            (SkinVerdict::Accepted, true, Some(SECOND.to_owned()), true),
        ),
        "((定常, 印の台詞が出た, (中断を受けた, 印が消えた)), [完了, 中止, 失敗, 取りやめ], \
         (LastShell の投函, 完了の記録), 中止の直後の (呼出列, バルーンの表示, 終了の指示, 今のシェル), \
         (キャラ窓の見え方の数, 子の欠けたフレーム), (次の要求の判定, 通った, LastShell, 降ろせた))"
    );
}

// ---------------------------------------------------------------- 失敗

/// 復号できない面を持つシェル（2 つ目のシェルの写しの `surface0000.png` を壊したもの）。
const BROKEN: &str = "broken";

/// 復号できない画像のシェル（要件 5.5・5.6・11.6）: `OnShellChanging` は送られる（切り替え前の
/// イベントだけが届く）が、資産づくりの失敗で UI スレッドの `error!` は `skin_switch_failed`
/// （stage＝build）の 1 件だけ（背景のスレッドの `switch_assets_failed` は捕捉の窓の外＝
/// 6.3 の単体が数える）。元の装着のまま（キャラ窓の見え方は 1 通り・今のシェルは `master`）、
/// `OnShellChanged` と記憶の投函は 0（実 fs の `LastShell` は起動の成功が書いた `master` のまま）。
///
/// # 非空虚性
/// 起動と同じく復号できない絵を読み飛ばして差し替えると、完了の記録・`OnShellChanged`・
/// `LastShell` が出て、窓寸の違う見え方が増えて赤。
#[test]
fn undecodable_shell_keeps_the_old_mount_and_records_one_error() {
    let mut lap = lap_rig_of(
        vec![(
            "A",
            FakeShiori::Scripted(Box::new(|| {
                standard_script(r"\0\s[0]\1\s[10]\0A\![change,shell,broken,--option=raise-event]\e")
                    .get("OnShellChanging", Ok(None))
            })),
        )],
        &[BROKEN],
    );
    let broken = lap.rig.root.ghost_dir("A").join("shell").join(BROKEN);
    std::fs::write(broken.join("surface0000.png"), b"not a png").expect("壊れた面を書く");
    let steady = lap.rig.wait_steady();
    let mut log = LookLog::default();
    let windows = lap.windows.clone();

    let (settled, events) = capture(|| {
        lap.frames_until(CREEP, |rig| {
            log.record(look_of(rig, &windows));
            got(rig, "OnShellChanging", 1) && idle(rig)
        })
    });
    let calls = kinds(&calls_a(&lap.rig));
    let shell = current_shell(&lap.rig);
    let exit_requested = lap.rig.exit_requested();
    let last = last_shell(&lap.rig);
    let shutdown_ok = lap.rig.shutdown();

    assert_eq!(
        (
            (steady, settled),
            outcomes(&events),
            errors(&events),
            recorded_shells(&events),
            calls,
            (shell, last),
            (log.distinct.len(), log.blank_after_full),
            (exit_requested, shutdown_ok),
        ),
        (
            (true, true),
            [0, 0, 1, 0],
            vec![(
                Some("skin_switch_failed".to_owned()),
                Some("build".to_owned())
            )],
            Vec::new(),
            vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "GET OnTranslate".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "GET OnShellChanging".to_owned(),
            ],
            (Some("master".to_owned()), Some("master".to_owned())),
            (1, false),
            (false, true),
        ),
        "((定常, 印が消えた), [完了, 中止, 失敗, 取りやめ], UI スレッドの error! の (目印, 段), \
         LastShell の投函, 呼出列, (今のシェル, 実 fs の LastShell), (キャラ窓の見え方の数, \
         子の欠けたフレーム), (終了の指示, 降ろせた))"
    );
}

// ---------------------------------------------------------------- 終了

/// 待ちの間の閉鎖要求（要件 5.7・11.6）: `raise-event` 無しの切替を受理し、命令を運んだ台本の
/// 終わりを待つ間に、キャラ窓への OS の閉鎖要求（メニューの「終了」と同じ終了要求）が届くと、
/// 切替は取りやめ（`closing`）で差し替え・イベント・記憶は 0。台本が終わると今日の終了経路
/// （`GET OnClose` → kanade の停止 → 全窓を閉じて終了の指示）で終わる。
///
/// # 非空虚性
/// 待ちの見張りが終了の保留を見ないと（kanade の見極めの終了の保留の段を外す）、台詞の時計を止めた
/// 間に取りやめが届かず、取りやめの記録も出ないまま終わって赤。
#[test]
fn close_request_while_waiting_drops_the_switch_and_exits_as_today() {
    let mut lap = rig_a(|| standard_script(BOOT_TO_SECOND_PLAIN).get("OnClose", Ok(None)));
    let steady = lap.rig.wait_steady();
    let mut log = LookLog::default();
    let windows = lap.windows.clone();

    let started = lap.frames_until(CREEP, |rig| {
        log.record(look_of(rig, &windows));
        !log.distinct.is_empty()
    });
    let requested = lap.frames_until(ACCEPT, |rig| {
        log.record(look_of(rig, &windows));
        !idle(rig)
    });
    let char0 = windows.char_window(0).expect("キャラ窓 0");
    let ((dropped, exited), events) = capture(|| {
        on_ghost_os_close(&mut lap.rig.world, char0);
        // 台詞の時計を止めたまま、取りやめを見る（台本はまだ終わっていない）。
        let dropped = lap.frames_until(NO_TICKS, |rig| {
            log.record(look_of(rig, &windows));
            idle(rig)
        });
        let exited = lap.frames_until(UNBOUNDED, SwitchRig::exit_requested);
        (dropped, exited)
    });
    let first_exit = lap
        .rig
        .world
        .get_resource::<FirstExit>()
        .map(|f| matches!(f.0, ExitOrigin::KanadeStopped(_)));
    let calls = kinds(&calls_a(&lap.rig));
    let shutdown_ok = lap.rig.shutdown();
    // 起動の成功の `LastShell` は投函だけで待たない。降ろす処理が記憶の書き手に柵を掛けて閉じた
    // 後に実 fs を読む（kanade が止まった後でも降ろす処理は書き手を畳む）。
    let last = read_last_shell(&lap.rig.root.ghost_dir("A"));

    assert_eq!(
        (
            (steady, started, requested, dropped, exited),
            outcomes(&events),
            dropped_reasons(&events),
            recorded_shells(&events),
            calls,
            last,
            (log.distinct.len(), log.blank_after_full),
            (first_exit, shutdown_ok),
        ),
        (
            (true, true, true, true, true),
            [0, 0, 0, 1],
            vec![Some("closing".to_owned())],
            Vec::new(),
            vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "GET OnTranslate".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "GET OnClose".to_owned(),
                "UNLOAD".to_owned(),
            ],
            Some("master".to_owned()),
            (1, false),
            (Some(true), true),
        ),
        "((定常, 台詞が始まった, 受理, 取りやめ, 終了の指示), [完了, 中止, 失敗, 取りやめ], 取りやめの理由, \
         LastShell の投函, 呼出列, 実 fs の LastShell, (キャラ窓の見え方の数, 子の欠けたフレーム), \
         (最初の終了の出所が kanade の停止, 降ろせた))"
    );
}

// ---------------------------------------------------------------- ゴースト切替

/// B の偽の SHIORI（起動記録あり・`OnGhostChanged` 204）。
fn ghost_b() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0B\e").get("OnGhostChanged", Ok(None))
    }))
}

/// A・B の土台（A に 2 つ目のシェル・B は起動記録あり・切替先の窓の準備の閉包の投函先つき）。
fn rig_ab(a: impl Fn() -> ScriptedShioriBackendBuilder + 'static) -> LapRig {
    let lap = lap_rig_of(
        vec![("A", FakeShiori::Scripted(Box::new(a))), ("B", ghost_b())],
        &[SECOND],
    );
    lap.rig.plant_boot_record("B");
    lap
}

/// B が切替で起きて迎え入れまで済んだ（ゴースト切替の予約が消えた）か。
fn welcomed_b(rig: &SwitchRig) -> bool {
    !rig.calls("B").is_empty() && rig.world.get_non_send::<SwitchInFlight>().is_none()
}

/// シェル切替の待ちの間に来たゴースト切替（要件 1.12・11.5・8.5）: `raise-event` 無しのシェル切替を
/// 受理し、台本の終わりを待つ間にメニュー相当のゴースト切替（B へ・`OnGhostChanging` なし）を
/// 入口へ渡すと、ゴースト切替が通って B が起き、シェル切替は差し替えの前に取りやめ（`info!` 1 件）・
/// 差し替え・イベント・記憶は 0。A のセッションを降ろす処理は戻り、シェル切替の印は残らない。
///
/// # 非空虚性
/// 待ちの見張りがゴースト切替（保留・降ろす相）を終了と取り違えると、取りやめの理由が `closing` に
/// なって赤。シェル切替の印が残ると最後の `idle` が偽のまま期限切れで赤。
#[test]
fn ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind() {
    let mut lap = rig_ab(|| standard_script(BOOT_TO_SECOND_PLAIN));
    lap.rig.world.insert_resource(WintfTaskPool::new());
    let steady = lap.rig.wait_steady();
    let mut log = LookLog::default();
    let windows = lap.windows.clone();

    let started = lap.frames_until(CREEP, |rig| {
        log.record(look_of(rig, &windows));
        !log.distinct.is_empty()
    });
    let requested = lap.frames_until(ACCEPT, |rig| {
        log.record(look_of(rig, &windows));
        !idle(rig)
    });
    let shapes_before = (log.distinct.len(), log.blank_after_full);
    let ((verdict, switched), events) = capture(|| {
        let verdict = request_ghost_switch(
            &mut lap.rig.world,
            SwitchRequest {
                ghost: GhostSpec::Folder("B".to_owned()),
                raise_event: false,
                origin: areka_kanade::ChangeOrigin::Manual,
                boot_event: None,
            },
        );
        let switched = lap.frames_until(UNBOUNDED, |rig| welcomed_b(rig) && idle(rig));
        (verdict, switched)
    });
    let a_calls = kinds(&calls_a(&lap.rig));
    let b_boots = lap.rig.calls("B").len();
    let exit_requested = lap.rig.exit_requested();
    let shutdown_ok = lap.rig.shutdown();
    // A はゴースト切替が同期で降ろした（記憶の書き手に柵を掛けて閉じた）ので、実 fs は確定している。
    let last_a = read_last_shell(&lap.rig.root.ghost_dir("A"));

    assert_eq!(
        (
            (steady, started, requested, shapes_before),
            (format!("{verdict:?}"), switched),
            outcomes(&events),
            dropped_reasons(&events),
            recorded_shells(&events),
            a_calls,
            (b_boots, last_a),
            (exit_requested, shutdown_ok),
        ),
        (
            (true, true, true, (1, false)),
            ("Accepted".to_owned(), true),
            [0, 0, 0, 1],
            vec![Some("ghost_change".to_owned())],
            Vec::new(),
            vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "GET OnTranslate".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "UNLOAD".to_owned(),
            ],
            (1, Some("master".to_owned())),
            (false, true),
        ),
        "((定常, 台詞が始まった, 受理, (切替前の見え方の数, 子の欠けたフレーム)), (ゴースト切替の判定, \
         B が迎え入れられシェル切替の印も無い), [完了, 中止, 失敗, 取りやめ], 取りやめの理由, \
         LastShell の投函, A の呼出列, (B の起動の回数, A の実 fs の LastShell), (終了の指示, 降ろせた)) \
        "
    );
}

/// シェルを 1 度差し替えた後のゴースト切替（要件 8.5）: A で `second` へ差し替え（`OnShellChanged`・
/// `LastShell`＝`second`）、その応答の台本の `\![change,ghost,B]` で B へ切り替える。差し替えで
/// seriko へ定義を送った後でも、A のセッションを降ろす処理は戻り（seriko の join が止まらない）、
/// B が迎え入れられる。B を降ろす処理も戻る。
///
/// # 非空虚性
/// セッションが seriko の送り手の複製を最初の段で落とさないと、A の降ろしが join で止まって
/// B が起きず期限切れで赤。
#[test]
fn ghost_switch_after_a_shell_swap_unloads_the_session() {
    let mut lap = rig_ab(|| {
        standard_script(r"\0\s[0]\1\s[10]\0A\![change,shell,second]\e").get(
            "OnShellChanged",
            Ok(Some(r"\0\_w[3000]\![change,ghost,B]\e".to_owned())),
        )
    });
    lap.rig.world.insert_resource(WintfTaskPool::new());
    let steady = lap.rig.wait_steady();

    // 差し替えまでは 1 ms ずつ（応答の台本の `\_w[3000]` の間に差し替えの完了を見る）。
    let (swapped, events) =
        capture(|| lap.frames_until(CREEP, |rig| got(rig, "OnShellChanged", 1) && idle(rig)));
    let shell_after_swap = current_shell(&lap.rig);
    let switched = lap.frames_until(UNBOUNDED, |rig| welcomed_b(rig) && idle(rig));
    let a_calls = kinds(&calls_a(&lap.rig));
    let b_boots = lap.rig.calls("B").len();
    let exit_requested = lap.rig.exit_requested();
    let shutdown_ok = lap.rig.shutdown();
    // A はゴースト切替が同期で降ろした（記憶の書き手に柵を掛けて閉じた）ので、実 fs は確定している。
    let last_a = read_last_shell(&lap.rig.root.ghost_dir("A"));

    assert_eq!(
        (
            (steady, swapped, switched),
            (
                outcomes(&events),
                recorded_shells(&events),
                shell_after_swap
            ),
            a_calls,
            (b_boots, last_a),
            (exit_requested, shutdown_ok),
        ),
        (
            (true, true, true),
            (
                [1, 0, 0, 0],
                vec![Some(SECOND.to_owned())],
                Some(SECOND.to_owned())
            ),
            vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "GET OnTranslate".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "GET OnShellChanged".to_owned(),
                "GET OnTranslate".to_owned(),
                "UNLOAD".to_owned(),
            ],
            (1, Some(SECOND.to_owned())),
            (false, true),
        ),
        "((定常, 差し替わった, B が迎え入れられた), ([完了, 中止, 失敗, 取りやめ], LastShell の投函, \
         差し替えの後の今のシェル), A の呼出列, (B の起動の回数, A の実 fs の LastShell), (終了の指示, 降ろせた))"
    );
}
