//! シェル・バルーンの切替とゴーストのセッション（spec: areka-P0-shell-balloon-switch）。
//!
//! 偽の SHIORI を持つゴースト A（`ghost_switch_test_support` の土台）を本番と同じ入口
//! （[`super::boot_ghost`]）で起こし、同じ World の上でセッションの持ち物と降ろし方を判定する。

use std::path::PathBuf;

use super::{GhostSession, GhostSlot};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinOrigin, SkinRequest, SkinSpec, SkinVerdict, request_skin_switch,
};
use crate::emo2_boot::spine::{RecordedCall, spin_wait_until};

/// seriko の送り手の複製は結線ありのセッションだけが持ち（テスト用の組み立ては持たない）、
/// 持ったセッションを降ろしても seriko の join が止まらずに戻る（要件 8.5）。複製を降ろす最初の
/// 段で落とさないと、③ の join が残った送り手を待ち続けて期限（20 秒）で赤になる。
#[test]
fn session_holding_seriko_sink_shuts_down_within_bound() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.boot("A");
    let held = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .map(|session| session.seriko_sink().is_some());
    let shutdown_ok = rig.shutdown();
    let for_test_held = GhostSession::for_test(None, PathBuf::from("for_test"))
        .seriko_sink()
        .is_some();

    assert_eq!(
        (held, shutdown_ok, for_test_held),
        (Some(true), true, false),
        "seriko の送り手の持ち主が違うか、降ろすのが戻らない（結線ありのセッションが持つ・\
         降ろせた・テスト用の組み立ては持たない）"
    );
}

/// メニューのシェルの受理で、走っているゴーストの SHIORI に `OnShellChanging` が GET で 1 件届き、
/// Ref0＝切替先の `name`・Ref1＝今のシェル（実行系のマウントのシェル）の `name`・Ref2＝切替先の
/// 絶対パスが載る（要件 1.5・2.1）。切替先は `descript.txt` だけの 2 つ目のシェルで足りる
/// （印のイベントは資産づくりを待たずに送られる）。
#[test]
fn menu_shell_request_raises_on_shell_changing_with_the_current_shell_as_ref1() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| {
            standard_script("\\0A\\e").get("OnShellChanging", Ok(None))
        })),
    )]);
    let shell = rig.root.ghost_dir("A").join("shell");
    let second = shell.join("second");
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(
        second.join("descript.txt"),
        "charset,UTF-8\r\nname,二つ目\r\n",
    )
    .unwrap();
    let current_name = areka_ghost::catalog::list_all_shells(&rig.root.ghost_dir("A"))
        .into_iter()
        .find(|e| e.identity.folder == "master")
        .and_then(|e| e.identity.name)
        .expect("検体のシェル master に name が在る");
    rig.boot("A");
    let steady = rig.wait_steady();

    let verdict = request_skin_switch(
        &mut rig.world,
        SkinRequest {
            kind: SkinKind::Shell,
            target: SkinSpec::Folder("second".to_owned()),
            origin: SkinOrigin::Menu,
        },
    );
    let handle = rig.handle("A");
    let changing = || -> Vec<Vec<String>> {
        handle
            .non_status_calls()
            .into_iter()
            .filter_map(|c| match c {
                RecordedCall::Get { id, references } if id == "OnShellChanging" => Some(references),
                _ => None,
            })
            .collect()
    };
    let arrived = spin_wait_until(|| !changing().is_empty());
    let got = changing();
    let shutdown_ok = rig.shutdown();

    let want_path = std::path::absolute(&second).unwrap().display().to_string();
    assert_eq!(
        (steady, verdict, arrived, got, shutdown_ok),
        (
            true,
            SkinVerdict::Accepted,
            true,
            vec![vec!["二つ目".to_owned(), current_name, want_path]],
            true
        ),
        "(定常, 判定, 届いたか, OnShellChanging の Reference, 降ろせた)"
    );
}
