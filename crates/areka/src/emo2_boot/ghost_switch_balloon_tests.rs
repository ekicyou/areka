//! 切替先のバルーンを作者の指定で決め、決まった段を 1 件記録する兄弟テスト
//! （areka-P0-ghost-standard-balloon タスク 4・design「Testing Strategy」切替・要件 4.1・5.8・7.6・7.7）。
//!
//! 切替の土台（[`SwitchRig`]・根は検体の複製＝ワークツリーの `target\` の下）に 2 つ目のバルーンを足し、
//! A から B へ本物の経路で切り替える。準備のたびに B に記憶（`[last] balloon`）が無いことを確かめる
//! （残っていると記憶の段が勝ち、場面が成り立たない）。

use areka_kanade::ChangeOrigin;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch};
use crate::boot_config::BootContext;
use crate::boot_resolve::{BalloonDecision, BalloonRoute, read_last_balloon};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

/// 2 つ目のバルーンのフォルダ名と `name`（フォルダ名と違う綴り＝`name` で当たったと分かる）。
const SECOND: &str = "second-balloon";
const SECOND_NAME: &str = "二つ目のバルーン";

/// A を起こして定常に着かせ、根に 2 つ目のバルーンを足した土台（B は記憶なし）。
fn running_a() -> SwitchRig {
    let a = FakeShiori::Scripted(Box::new(|| standard_script(r"\0A\e")));
    let b = FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0B\e").get("OnGhostChanged", Ok(Some(r"\0B\e".to_owned())))
    }));
    let mut rig = SwitchRig::new(vec![("A", a), ("B", b)]);
    // 切替先を起こすときの窓の準備が閉包を投函する先。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");
    rig.add_balloon_copy(SECOND, SECOND_NAME);
    assert_eq!(
        read_last_balloon(&rig.root.ghost_dir("B")),
        None,
        "切替先 B に記憶のバルーンが無い"
    );
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// B へ切り替え（知らせなし・出どころ「自動」）、B が起きて予約が下りるまで回す。
/// 起動の文脈の今のバルーンと記録を返す。
fn switch_to_b(mut rig: SwitchRig) -> (BalloonDecision, Vec<CapturedEvent>, SwitchRig) {
    let (flow, events) = capture(|| {
        let verdict = request_ghost_switch(
            &mut rig.world,
            SwitchRequest {
                ghost: GhostSpec::Folder("B".to_owned()),
                raise_event: false,
                origin: ChangeOrigin::Automatic,
                boot_event: None,
            },
        );
        let done = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("B").len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        });
        (verdict, done)
    });
    let exited = rig.exit_requested();
    assert!(rig.shutdown(), "B を降ろせる");
    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "終了しない: {events:?}");
    let balloon = rig.world.resource::<BootContext>().current.balloon.clone();
    (balloon, events, rig)
}

/// `switch_balloon_resolved` が 1 件だけで、切替先・段・フォルダが合う。
fn assert_one_record(events: &[CapturedEvent], rig: &SwitchRig, route: &str) {
    let records: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("switch_balloon_resolved"))
        .collect();
    assert_eq!(records.len(), 1, "記録はちょうど 1 件: {events:?}");
    let record = records[0];
    let dir = rig.root.balloon_dir(SECOND).display().to_string();
    assert_eq!(
        (
            record.level,
            record.field("ghost"),
            record.field("route"),
            record.field("dir")
        ),
        (
            tracing::Level::INFO,
            Some("B"),
            Some(route),
            Some(dir.as_str())
        ),
        "{record:?}"
    );
    assert!(
        !record.message().contains("バルーンを決めました"),
        "起動の記録の文面を含まない: {record:?}"
    );
}

/// 切替先の descript.txt の `balloon` が 2 つ目のバルーンの `name` を指すと、同梱（`emo2-kakukaku`）より
/// 先に descript の段で 2 つ目に決まり、記録が 1 件残る（要件 4.1・5.8）。
///
/// # 非空虚性
/// 切替で descript の段を見ないと同梱の段で `emo2-kakukaku` に決まり、記録が無いと件数が 0 で赤。
#[test]
fn switching_to_a_ghost_whose_descript_names_a_balloon_uses_it_and_records_the_descript_route() {
    let rig = running_a();
    let descript = rig
        .root
        .ghost_dir("B")
        .join("ghost")
        .join("master")
        .join("descript.txt");
    let text = std::fs::read_to_string(&descript).expect("B の descript.txt");
    std::fs::write(&descript, format!("{text}\r\nballoon,{SECOND_NAME}\r\n"))
        .expect("B の descript.txt に balloon を足す");

    let (balloon, events, rig) = switch_to_b(rig);

    assert_eq!(
        (balloon.route, balloon.folder.as_deref()),
        (BalloonRoute::Descript, Some(SECOND)),
        "{events:?}"
    );
    assert_eq!(balloon.dir, rig.root.balloon_dir(SECOND));
    assert_one_record(&events, &rig, "Descript");
}

/// 切替先の install.txt が番号付きだけ（`balloon0.directory` が 2 つ目）だと、同梱の段で 2 つ目に決まり、
/// 記録が 1 件残る（要件 4.1・5.8）。
///
/// # 非空虚性
/// 番号付きを読まないと同梱の段を素通りして無作為の段へ落ち、段の判定で赤。
#[test]
fn switching_to_a_ghost_with_numbered_companions_uses_the_first_and_records_the_companion_route() {
    let rig = running_a();
    std::fs::write(
        rig.root.ghost_dir("B").join("install.txt"),
        format!(
            "charset,UTF-8\r\ntype,ghost\r\nname,B\r\ndirectory,B\r\n\
             balloon0.directory,{SECOND}\r\nballoon1.directory,emo2-kakukaku\r\n"
        ),
    )
    .expect("B の install.txt を番号付きだけにする");

    let (balloon, events, rig) = switch_to_b(rig);

    assert_eq!(
        (balloon.route, balloon.folder.as_deref()),
        (BalloonRoute::Companion, Some(SECOND)),
        "{events:?}"
    );
    assert_one_record(&events, &rig, "Companion");
}
