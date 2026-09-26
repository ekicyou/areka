//! 偽の SHIORI を持つゴーストを同じ World で起こす統合テスト（areka-P0-ghost-shell-balloon-switch）。
//!
//! 土台は `emo2_boot::ghost_switch_test_support`。ここでは土台そのものが使えることを確かめる:
//! 作り口がフォルダ名で偽の SHIORI を選ぶこと・先に書いた起動記録が効くこと・接続に失敗する
//! 版が kanade を `Fault` で止めること。判定は集めてから 1 回・降ろすのは必ず有界に行う。

use areka_kanade::{KanadeStopCause, ShioriFaultKind};

use crate::app_exit::{ExitOrigin, FirstExit};
use crate::emo2_boot::ghost_switch_test_support::{
    CONNECT_ERR, FakeShiori, SwitchRig, standard_script,
};
use crate::emo2_boot::spine::RecordedCall;

/// 呼出列を「種類 名前」の列へ写す（Reference は見ない）。
fn kinds(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|c| match c {
            RecordedCall::Get { id, .. } => format!("GET {id}"),
            RecordedCall::Notify { id, .. } => format!("NOTIFY {id}"),
            RecordedCall::Unload => "UNLOAD".to_owned(),
            RecordedCall::Status => "STATUS".to_owned(),
        })
        .collect()
}

/// `folder` を 1 回起こした記録に `basewareversion` まで届いたか（起動系列の終わり）。
fn booted(rig: &SwitchRig, folder: &str) -> bool {
    rig.calls(folder).last().is_some_and(|calls| {
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "basewareversion"))
    })
}

/// 土台だけで A（起動記録なし）を起こすと、A の偽の SHIORI に起動系列が
/// （`OnFirstBoot` 204 → `OnBoot` を含めて）記録され、B の偽の SHIORI は作られない。
/// 定常到達の通知を通知の相が受けても終了は指示されない。
#[test]
fn rig_boots_a_and_records_its_boot_sequence() {
    let mut rig = SwitchRig::new(vec![
        (
            "A",
            FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
        ),
        (
            "B",
            FakeShiori::Scripted(Box::new(|| standard_script("\\0B\\e"))),
        ),
    ]);
    rig.boot("A");
    let reached = rig.pump_until(|rig| booted(rig, "A"));
    let a_calls: Vec<Vec<String>> = rig.calls("A").iter().map(|c| kinds(c)).collect();
    let on_boot_ref_count = rig.calls("A").last().and_then(|calls| {
        calls.iter().find_map(|c| match c {
            RecordedCall::Get { id, references } if id == "OnBoot" => Some(references.len()),
            _ => None,
        })
    });
    let b_boots = rig.calls("B").len();
    let exit_requested = rig.exit_requested();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            reached,
            a_calls,
            on_boot_ref_count,
            b_boots,
            exit_requested,
            shutdown_ok
        ),
        (
            true,
            vec![vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnFirstBoot".to_owned(),
                "GET OnBoot".to_owned(),
                "NOTIFY basewareversion".to_owned(),
            ]],
            Some(1),
            0,
            false,
            true,
        ),
        "A の起動系列が記録されない（届いた・A の呼出列・OnBoot の Reference の数＝由来がふつうなら \
         Ref0 だけ・B を起こした回数・終了の指示・降ろせた）"
    );
}

/// 先に起動記録を書いた B を起こすと `OnFirstBoot` を飛ばして `OnBoot` から起きる。
#[test]
fn rig_boot_record_skips_first_boot() {
    let mut rig = SwitchRig::new(vec![(
        "B",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0B\\e"))),
    )]);
    rig.plant_boot_record("B");
    rig.boot("B");
    let reached = rig.pump_until(|rig| booted(rig, "B"));
    let b_calls: Vec<Vec<String>> = rig.calls("B").iter().map(|c| kinds(c)).collect();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (reached, b_calls, shutdown_ok),
        (
            true,
            vec![vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "NOTIFY basewareversion".to_owned(),
            ]],
            true,
        ),
        "起動記録が効かない（届いた・B の呼出列・降ろせた）"
    );
}

/// 接続に失敗する版を選ぶと kanade が `Fault`（接続できなかった）で止まり、切替の予約が無い
/// 今は通知の相が終了を指示する。台本は作られない。
#[test]
fn rig_connect_fail_stops_kanade_with_fault() {
    let mut rig = SwitchRig::new(vec![("B", FakeShiori::ConnectFail)]);
    rig.boot("B");
    let reached = rig.pump_until(SwitchRig::exit_requested);
    let fault = rig
        .world
        .get_resource::<FirstExit>()
        .and_then(|first| match &first.0 {
            ExitOrigin::KanadeStopped(KanadeStopCause::Fault(f)) => {
                Some((f.kind, f.reason.contains(CONNECT_ERR)))
            }
            _ => None,
        });
    let b_boots = rig.calls("B").len();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (reached, fault, b_boots, shutdown_ok),
        (true, Some((ShioriFaultKind::ConnectFailed, true)), 0, true),
        "接続に失敗する版が効かない（終了の指示・最初の出所の Fault・台本の数・降ろせた）"
    );
}
