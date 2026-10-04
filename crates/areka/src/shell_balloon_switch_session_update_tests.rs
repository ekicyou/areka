//! 切替の後の更新の対象と読み直しの統合テスト（spec: areka-P0-shell-balloon-switch task 11.3・
//! 要件 6.5・11.8・design「Integration Tests」・Flow 5）。
//!
//! 完了 `network-update` の対象の解決（`desk_resolve_tests.rs` の
//! `resolve_targets_reads_the_slot_and_the_boot_context`）と読み直し（`desk_reload_tests.rs` の
//! `a_reload_of_the_running_ghost_boots_with_the_tail_head_and_sends_the_rest_after_the_switch`）の兄弟。
//! 土台は 11.1 の [`lap_rig`]（偽の SHIORI のゴースト A・2 つ目のシェル `second`・窓の一式・GPU 資源・
//! 本番の `Input`・`Update` の段）に、11.2 と同じ 2 つ目のバルーン `StayseeBalloon` を足したもの。
//! 差し替えは本番の台本の命令（`\![change,shell,…]` → `OnShellChanged` の台詞の
//! `\![change,balloon,…]`）で起こし、差し替えの後の状態は本番の後始末が作る。

use std::path::PathBuf;

use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::GhostSlot;
use super::shell_balloon_switch_session_lap_tests::{
    LapRig, SECOND, UNBOUNDED, copy_tree, got, idle, kinds, lap_rig,
};
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{BALLOON, SwitchRig, standard_script};
use crate::emo2_boot::spine::ScriptedShioriBackendBuilder;
use crate::update::desk::{ask_reload_for_test, resolve_targets};
use crate::update::{RawUpdateRequest, TargetKind};

/// 2 つ目のバルーン（検体 `StayseeBalloon` の複製・フォルダ名）。
const STAYSEE: &str = "StayseeBalloon";

/// A の `OnBoot`: 両スコープの面を出し、台詞の終わりで `second` への切替を命じる。
const BOOT_TO_SECOND: &str = r"\0\s[0]\1\s[10]\0A\![change,shell,second]\e";
/// `OnShellChanged` の台詞: 続けて 2 つ目のバルーンへの切替を命じる。
const CHANGED_TO_STAYSEE: &str = r"\0\![change,balloon,StayseeBalloon]\e";

/// 偽の SHIORI（起こすたびに新品の台本）: シェル → バルーンの順に差し替え、読み直しの列の 2 語には
/// 返事なしで応える。
fn script() -> ScriptedShioriBackendBuilder {
    standard_script(BOOT_TO_SECOND)
        .get("OnShellChanged", Ok(Some(CHANGED_TO_STAYSEE.to_owned())))
        .get("OnBalloonChange", Ok(None))
        .get("OnUpdateComplete", Ok(None))
        .get("OnUpdateResult", Ok(None))
}

/// 検体 `StayseeBalloon` を根の `balloon/StayseeBalloon` へ複製する。
fn add_staysee(rig: &SwitchRig) {
    let staysee = sample_ghost_kit::SampleRoot::acquire(STAYSEE).expect("登記済みの検体");
    copy_tree(staysee.folder(), &rig.root.balloon_dir(STAYSEE));
}

/// 土台を組み、A が定常に着くのを待つ（台詞の時計はまだ回さない＝差し替えは始まっていない）。
fn steady_rig() -> (LapRig, bool) {
    let lap = lap_rig(script);
    add_staysee(&lap.rig);
    let steady = lap.rig.wait_steady();
    (lap, steady)
}

/// シェル（`second`）とバルーン（`StayseeBalloon`）の差し替えを、台詞を進めて終わりまで回す。
fn swap_both(lap: &mut LapRig) -> bool {
    lap.frames_until(UNBOUNDED, |rig| got(rig, "OnBalloonChange", 1) && idle(rig))
}

/// 今の 3 つの対象の解決（メニューの「ネットワーク更新」と同じ要求）の（種別・フォルダ）の並び。
fn targets(rig: &SwitchRig) -> Vec<(TargetKind, PathBuf)> {
    resolve_targets(
        &rig.world,
        &RawUpdateRequest::Current(vec![
            TargetKind::Ghost,
            TargetKind::Shell,
            TargetKind::Balloon,
        ]),
    )
    .into_iter()
    .map(|t| (t.kind, t.dir))
    .collect()
}

/// 期待する対象の並び（ゴースト A・A の `shell/<shell>`・根の `balloon/<balloon>`）。
fn expected(rig: &SwitchRig, shell: &str, balloon: &str) -> Vec<(TargetKind, PathBuf)> {
    let a = rig.root.ghost_dir("A");
    vec![
        (TargetKind::Ghost, a.clone()),
        (TargetKind::Shell, a.join("shell").join(shell)),
        (TargetKind::Balloon, rig.root.balloon_dir(balloon)),
    ]
}

/// 置き場のゴーストの今のシェルのフォルダ名と、起動の文脈の今のバルーン（選び方・フォルダ名）。
fn current(rig: &SwitchRig) -> (Option<String>, Option<(String, Option<String>)>) {
    let shell = rig
        .world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.current_shell_folder());
    let balloon = rig
        .world
        .get_resource::<BootContext>()
        .map(|ctx| &ctx.current.balloon)
        .map(|b| (format!("{:?}", b.route), b.folder.clone()));
    (shell, balloon)
}

fn count(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// 差し替えの後の対象の解決（要件 6.5・11.8・`resolve_targets_reads_the_slot_and_the_boot_context` の
/// 兄弟）: 差し替えの前はシェル `master`・バルーン `emo2-kakukaku` を指し、台本の命令でシェルを
/// `second`、続けてバルーンを `StayseeBalloon` へ差し替えた後は、同じ解決が新しいシェル（実行系の
/// マウントの書き換え）と新しいバルーン（起動の文脈の書き換え）を指す（警告 0 件）。
///
/// # 非空虚性
/// 差し替えの後始末が実行系の今のシェルを書き換えない（`set_shell_dir` を素通りにする）と、
/// シェルの対象が `master` のままで赤。起動の文脈の今のバルーンを書き換えないと、バルーンの対象が
/// `emo2-kakukaku` のままで赤。
#[test]
fn update_targets_follow_the_swapped_shell_and_balloon() {
    let (mut lap, steady) = steady_rig();
    let before = targets(&lap.rig);
    let (swapped, events) = capture(|| swap_both(&mut lap));
    let ((after, now), skipped) = capture(|| (targets(&lap.rig), current(&lap.rig)));
    let calls = kinds(&lap.rig.calls("A").into_iter().next().unwrap_or_default());
    let expect_before = expected(&lap.rig, "master", BALLOON);
    let expect_after = expected(&lap.rig, SECOND, STAYSEE);
    let shutdown_ok = lap.rig.shutdown();

    assert_eq!(
        (
            (steady, swapped),
            calls,
            count(&events, "skin_switch_done"),
            (before, after),
            now,
            count(&skipped, "update_target_skipped"),
            shutdown_ok,
        ),
        (
            (true, true),
            vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "GET OnTranslate".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "GET OnShellChanged".to_owned(),
                "GET OnTranslate".to_owned(),
                "GET OnBalloonChange".to_owned(),
            ],
            2,
            (expect_before, expect_after),
            (
                Some(SECOND.to_owned()),
                Some(("Memory".to_owned(), Some(STAYSEE.to_owned())))
            ),
            0,
            true,
        ),
        "((定常, 差し替わった), 呼出列, 完了の記録の数, (差し替えの前の対象, 後の対象), \
         (今のシェル, 今のバルーンの (選び方, フォルダ名)), 飛ばした対象の数, 降ろせた)"
    );
}

/// 読み直しで起き直した A の呼出列（2 度目の起動）に `OnUpdateResult` が届いたか。
fn reloaded(rig: &SwitchRig) -> bool {
    rig.calls("A")
        .get(1)
        .is_some_and(|calls| kinds(calls).iter().any(|c| c == "GET OnUpdateResult"))
}

/// 差し替えの後の読み直し（要件 6.5・11.8・`a_reload_of_the_running_ghost_boots_with_the_tail_head_and_
/// sends_the_rest_after_the_switch` の兄弟）: シェルを `second`・バルーンを `StayseeBalloon` へ差し替えた
/// 後、更新の窓口の読み直し（同じフォルダ・知らせなしのゴースト切替）で A を起こし直すと、記憶
/// （`LastShell`・`LastBalloon`）から同じシェル・バルーンで起きる: 実行系のマウントが `second`・
/// 起動の文脈のバルーンが記憶の選び方で `StayseeBalloon`・更新の対象の解決も同じ 2 つを指す。記憶の先が
/// 無いときの警告（`boot_shell_missing`）は 0 件で、起き直した A は列の先頭で起き、残りを受ける。
///
/// # 非空虚性
/// 差し替えの後始末が `LastShell` を書かない（`record_last_shell` を素通りにする）と、起き直した A の
/// シェルが既定の `master` に戻って赤。`LastBalloon` を書かない（`record_last_balloon` を素通りに
/// する）と、起き直した A のバルーンが起動のときの `emo2-kakukaku` に戻って赤。
#[test]
fn a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon() {
    let (mut lap, steady) = steady_rig();
    // 起こし直すときの窓の準備が閉包を投函する先。
    lap.rig.world.insert_resource(WintfTaskPool::new());
    let swapped = swap_both(&mut lap);
    let a_dir = lap.rig.root.ghost_dir("A");

    let (finished, events) = capture(|| {
        ask_reload_for_test(
            &lap.rig.world,
            a_dir,
            vec![
                (
                    "OnUpdateComplete",
                    vec!["changed".to_owned(), "a.txt".to_owned()],
                ),
                ("OnUpdateResult", vec!["ghost\x01OK\x011".to_owned()]),
            ],
        );
        lap.frames_until(UNBOUNDED, |rig| {
            rig.exit_requested()
                || (reloaded(rig) && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let boots: Vec<Vec<String>> = lap.rig.calls("A").iter().map(|c| kinds(c)).collect();
    let now = current(&lap.rig);
    let after = targets(&lap.rig);
    let expect_after = expected(&lap.rig, SECOND, STAYSEE);
    let exit_requested = lap.rig.exit_requested();
    let shutdown_ok = lap.rig.shutdown();

    assert_eq!(
        (
            (steady, swapped, finished),
            (count(&events, "update_reload_requested"), boots.len()),
            boots.get(1).cloned(),
            now,
            after,
            count(&events, "boot_shell_missing"),
            (exit_requested, shutdown_ok),
        ),
        (
            (true, true, true),
            (1, 2),
            Some(vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnUpdateComplete".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "GET OnUpdateResult".to_owned(),
            ]),
            (
                Some(SECOND.to_owned()),
                Some(("Memory".to_owned(), Some(STAYSEE.to_owned())))
            ),
            expect_after,
            0,
            (false, true),
        ),
        "((定常, 差し替わった, 読み直しが終わった), (読み直しの受付の記録の数, A の起動の回数), \
         起き直した A の呼出列, (今のシェル, 今のバルーンの (選び方, フォルダ名)), 更新の対象, \
         記憶の先が無い警告の数, (終了の指示, 降ろせた))"
    );
}
