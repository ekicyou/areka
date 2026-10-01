//! シェル・バルーンの切替とゴーストのセッション（spec: areka-P0-shell-balloon-switch）。
//!
//! 偽の SHIORI を持つゴースト A（`ghost_switch_test_support` の土台）を本番と同じ入口
//! （[`super::boot_ghost`]）で起こし、同じ World の上でセッションの持ち物と降ろし方を判定する。

use std::path::PathBuf;

use super::{GhostSession, GhostSlot};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinOrigin, SkinRequest, SkinSpec, SkinSwitchInFlight, SkinVerdict,
    request_skin_switch,
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

/// 走っているゴーストのメニューの `frame` 枠の子の（ラベル・印）の列と、ラベル `pick` の子の動作。
fn menu_children(
    world: &bevy_ecs::world::World,
    frame: crate::menu::Frame,
    pick: &str,
) -> (Vec<(String, Option<bool>)>, Option<crate::menu::MenuAction>) {
    use crate::menu::{ItemBody, MenuContext, MenuWiring};
    let items = world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 });
    let Some(ItemBody::Submenu(children)) = items
        .into_iter()
        .find_map(|(f, item)| (f == frame).then_some(item.body))
    else {
        return (Vec::new(), None);
    };
    let action = children
        .iter()
        .find(|c| c.label == pick)
        .and_then(|c| match &c.body {
            ItemBody::Action(action) => Some(action.clone()),
            ItemBody::Submenu(_) => None,
        });
    let shape = children.into_iter().map(|c| (c.label, c.checked)).collect();
    (shape, action)
}

/// 起こしたゴーストのメニューの「シェル」枠は、今のシェル（実行系のマウントの末尾＝`master`）に
/// 印を付ける。2 つ目を選ぶと、フォルダ名で指した出どころ＝メニューの要求 1 件を入口が受理し
/// （`OnShellChanging` が届く）、切替の印が 2 つ目を指す（要件 1.5・7.1・7.3・7.4）。
#[test]
fn menu_shell_frame_marks_the_mounted_shell_and_selecting_requests_a_switch() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| {
            standard_script("\\0A\\e").get("OnShellChanging", Ok(None))
        })),
    )]);
    let second = rig.root.ghost_dir("A").join("shell").join("second");
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(
        second.join("descript.txt"),
        "charset,UTF-8\r\nname,二つ目\r\n",
    )
    .unwrap();
    let master_name = areka_ghost::catalog::list_shells(&rig.root.ghost_dir("A"))
        .into_iter()
        .find(|e| e.identity.folder == "master")
        .and_then(|e| e.identity.name)
        .expect("検体のシェル master に name が在る");
    rig.boot("A");
    let steady = rig.wait_steady();

    let (children, action) = menu_children(&rig.world, crate::menu::Frame::Shell, "二つ目");
    let action = action.expect("「二つ目」の子が在る");
    let ((), events) =
        log_capture_kit::capture(|| action(&mut rig.world, &crate::menu::MenuContext { scope: 0 }));
    let requested: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("skin_switch_requested"))
        .map(|e| (e.field("kind"), e.field("to"), e.field("origin")))
        .map(|(k, t, o)| {
            (
                k.map(str::to_owned),
                t.map(str::to_owned),
                o.map(str::to_owned),
            )
        })
        .collect();
    let in_flight = rig
        .world
        .get_non_send::<SkinSwitchInFlight>()
        .map(|f| (f.kind, f.target.folder.clone()));
    let handle = rig.handle("A");
    let arrived = spin_wait_until(|| {
        handle
            .non_status_calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnShellChanging"))
    });
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (steady, children, requested, in_flight, arrived, shutdown_ok),
        (
            true,
            vec![
                (master_name, Some(true)),
                ("二つ目".to_owned(), Some(false))
            ],
            vec![(
                Some("Shell".to_owned()),
                Some("second".to_owned()),
                Some("Menu".to_owned())
            )],
            Some((SkinKind::Shell, "second".to_owned())),
            true,
            true
        ),
        "(定常, 子の（ラベル・印）, 入口の受理の記録, 切替の印, OnShellChanging が届いたか, 降ろせた) \
         {events:?}"
    );
}

/// 起こしたゴーストのメニューの「バルーン」枠は、今のバルーン（起動の文脈）に印を付ける。
/// 2 つ目を選ぶと、フォルダ名で指した出どころ＝メニューの要求 1 件を入口が受理し、切替の印が
/// 2 つ目を指す（要件 1.5・7.2・7.3・7.4）。
#[test]
fn menu_balloon_frame_marks_the_current_balloon_and_selecting_requests_a_switch() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    let second = rig.root.balloon_dir("second");
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(
        second.join("descript.txt"),
        "charset,UTF-8\r\nname,二つ目\r\n",
    )
    .unwrap();
    let current_label = areka_ghost::catalog::list_balloons(&rig.root)
        .into_iter()
        .find(|e| e.identity.folder == crate::emo2_boot::ghost_switch_test_support::BALLOON)
        .map(|e| e.identity.name.unwrap_or(e.identity.folder))
        .expect("検体のバルーンが根に在る");
    rig.boot("A");
    let steady = rig.wait_steady();

    let (children, action) = menu_children(&rig.world, crate::menu::Frame::Balloon, "二つ目");
    let action = action.expect("「二つ目」の子が在る");
    let ((), events) =
        log_capture_kit::capture(|| action(&mut rig.world, &crate::menu::MenuContext { scope: 0 }));
    let requested: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("skin_switch_requested"))
        .map(|e| (e.field("kind"), e.field("to"), e.field("origin")))
        .map(|(k, t, o)| {
            (
                k.map(str::to_owned),
                t.map(str::to_owned),
                o.map(str::to_owned),
            )
        })
        .collect();
    let in_flight = rig
        .world
        .get_non_send::<SkinSwitchInFlight>()
        .map(|f| (f.kind, f.target.folder.clone()));
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (steady, children, requested, in_flight, shutdown_ok),
        (
            true,
            vec![
                (current_label, Some(true)),
                ("二つ目".to_owned(), Some(false))
            ],
            vec![(
                Some("Balloon".to_owned()),
                Some("second".to_owned()),
                Some("Menu".to_owned())
            )],
            Some((SkinKind::Balloon, "second".to_owned())),
            true
        ),
        "(定常, 子の（ラベル・印）, 入口の受理の記録, 切替の印, 降ろせた) {events:?}"
    );
}
