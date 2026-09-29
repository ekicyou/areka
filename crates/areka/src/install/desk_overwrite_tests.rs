//! 起動中のゴーストへ入れる一周の決定論テスト（design「System Flows／起動中のゴーストへ入れる一周」・
//! 「Testing Strategy / 窓口と入口」・要件 6.5・7.1・7.2・7.3・11.7）。
//!
//! 切替の土台（[`SwitchRig`]）の上で、本物の窓口・kanade・背景のスレッド・口（`DeskPorts`）・切替の
//! 道筋を通し、偽の SHIORI に届いた呼び出しの列と UI スレッドの記録の順を突き合わせる。確かめること:
//! `OnGhostChanging` も `OnClose` も 0 件・降ろした後（全窓を閉じた後・起こす前）に宛先の中身が
//! 替わる・同じゴーストが起き直す・定常到達の後に締めの知らせが出る。
//!
//! 実時間の待ちに依らない: 台詞の時計は合成の Tick で進め（再生中の台詞の終わりを kanade の保留が
//! 待つ）、依頼の終わりは窓口が背景のスレッドから終わりの知らせを受けたことで揃える。

use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{NarBuilder, install_txt};
use temp_path_kit::TempPath;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::InstallDesk;
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::{LastInstalledGhost, SwitchInFlight};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;
use crate::install::{InstallOrder, InstallOrigin, SubmitVerdict, submit};

/// 書庫が置くファイル（宛先のゴーストのフォルダからの相対）と中身。
const MARKER: &str = "ghost/master/overwritten.txt";
const MARKER_BODY: &[u8] = b"new";

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

/// `name` の記録が最初に現れた位置。
fn first(events: &[CapturedEvent], name: &str) -> Option<usize> {
    events
        .iter()
        .position(|e| e.field_str("event") == Some(name))
}

/// 起動中のゴースト A（`directory,A` の `ghost`）へ、ファイルを 1 つ足す書庫を本番の道筋で入れる。
#[test]
fn overwriting_the_running_ghost_takes_it_down_installs_and_boots_it_again() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| {
            standard_script(r"\0A\e")
                .get("OnGhostChanged", Ok(None))
                .get("OnInstallBegin", Ok(None))
                .get("OnInstallCompleteEx", Ok(None))
                .get("OnInstallComplete", Ok(None))
        })),
    )]);
    // 起こし直すときの窓の準備が閉包を投函する先（`Input` の段では走らない）。
    rig.world.insert_resource(WintfTaskPool::new());
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    let marker = rig.root.ghost_dir("A").join(MARKER);
    assert!(!marker.exists(), "入れる前は無い");

    let dir = TempPath::new("desk-overwrite-running");
    let path = dir.child("a.nar");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&["charset,UTF-8", "type,ghost", "name,A", "directory,A"]),
        )
        .done()
        .file(MARKER, MARKER_BODY)
        .done()
        .write_to(&path)
        .expect("書庫を置ける");
    let order = InstallOrder {
        archives: vec![path],
        origin: InstallOrigin::Menu,
    };
    assert_eq!(submit(&mut rig.world, order), SubmitVerdict::Queued);

    let (finished, events) = capture(|| {
        rig.pump_talking_until(|rig| {
            let desk = rig.world.non_send::<InstallDesk>();
            rig.exit_requested() || (desk.queue.is_empty() && !desk.busy)
        })
    });
    assert!(finished, "依頼が終わる（期限切れ）: {events:?}");
    assert!(!rig.exit_requested(), "終了しない: {events:?}");

    let boots: Vec<Vec<String>> = rig.calls("A").iter().map(|c| ids(c)).collect();
    let current = rig
        .world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone());
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let last = rig
        .world
        .get_resource::<LastInstalledGhost>()
        .map(|g| g.0.clone());
    let body = std::fs::read(&marker).ok();
    assert!(rig.shutdown());

    // 同じゴーストが起き直す（ほかのゴーストは起こさない）。予約は定常到達で下りている。
    assert_eq!(boots.len(), 2, "A を 1 回起こし直す: {boots:?}");
    assert_eq!(current.as_deref(), Some("A"));
    assert!(!reserved, "予約は残らない");
    // 知らせを送らない切替（要件 7.2）。
    for name in ["OnGhostChanging", "OnClose"] {
        assert!(
            boots.iter().flatten().all(|id| id != name),
            "{name} は 0 件: {boots:?}"
        );
    }
    // 始まりの知らせは降ろす前のゴーストへ、締めの知らせは起こし直したゴーストの起動の後へ。
    assert!(
        boots[0].iter().any(|id| id == "OnInstallBegin"),
        "{boots:?}"
    );
    assert!(
        boots[0]
            .iter()
            .all(|id| !id.starts_with("OnInstallComplete")),
        "{boots:?}"
    );
    let second = &boots[1];
    let booted = second
        .iter()
        .position(|id| id == "basewareversion")
        .expect("起こし直した起動の系列");
    assert_eq!(
        second[booted + 1..],
        ["OnInstallCompleteEx", "OnInstallComplete"],
        "{boots:?}"
    );
    // 降ろした後に宛先の中身が替わる（全窓を閉じた後・起こす前に展開する＝要件 7.3）。
    assert_eq!(body.as_deref(), Some(MARKER_BODY));
    let order_of = |name| first(&events, name).unwrap_or_else(|| panic!("{name}: {events:?}"));
    let down = order_of("ghost_switch_down_ms");
    let done = order_of("install_overwrite_done");
    let booted = order_of("ghost_switch_booted");
    let steady = order_of("ghost_switch_done");
    assert!(down < done && done < booted, "{events:?}");
    let done_event = &events[done];
    assert_eq!(done_event.field("ok"), Some("true"), "{done_event:?}");
    assert!(done_event.field("ms").is_some(), "{done_event:?}");
    // 締めの知らせは定常到達の後に送る。
    let sent_ex = events
        .iter()
        .position(|e| {
            e.field_str("event") == Some("install_event_sent")
                && e.field_str("id") == Some("OnInstallCompleteEx")
        })
        .expect("締めの知らせを送った");
    assert!(steady < sent_ex, "{events:?}");
    // 起動中のゴースト自身を入れても、追加の切替はしない（要件 6.5）。受け皿へは書く。
    assert_eq!(
        events
            .iter()
            .filter(|e| e.field_str("event") == Some("ghost_switch_requested"))
            .count(),
        1,
        "{events:?}"
    );
    assert_eq!(last.as_deref(), Some("A"));
}
