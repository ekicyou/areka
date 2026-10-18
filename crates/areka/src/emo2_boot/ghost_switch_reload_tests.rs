//! ゴースト切替の起こし方に読み直しの由来を通す兄弟テスト
//! （areka-P0-mcp-reload タスク 7.1・design「GhostSwitch の口」・Flow 2・要件 3.2〜3.6・12.2・12.4）。
//!
//! 切替の土台（[`SwitchRig`]）の上で、読み直しの窓口・切替・kanade・偽の SHIORI を本物の経路で通す。
//! 確かめること: 印（窓口自身の予約）があれば `switch_to` は `Reloaded{Ghost}` で起こし、降ろす側に
//! `OnGhostChanging`・`OnClose`、起こす側に `OnBoot`・`OnGhostChanged`・`OnFirstBoot` が無く、情報の通知
//! （`OnInitialize`・`username`・`basewareversion`）はある／印が無い・他人の予約なら今までどおり `OnGhostChanged`。

use areka_kanade::ChangeOrigin;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::reload::{
    ReloadInFlight, ReloadOrigin, ReloadRequest, ReloadStage, ReloadVerdict, request_reload,
};
use crate::emo2_boot::reload_args::{DescriptNotes, ReloadTarget};
use crate::emo2_boot::spine::RecordedCall;

/// 呼び出しを「GET 名」「NOTIFY 名」の列にする。
fn ids(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|call| match call {
            RecordedCall::Get { id, .. } => format!("GET {id}"),
            RecordedCall::Notify { id, .. } => format!("NOTIFY {id}"),
            other => format!("{other:?}"),
        })
        .collect()
}

fn named(events: &[CapturedEvent], name: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .count()
}

/// A の台本（`OnGhostChanged` には台本で応える＝`OnBoot` へ続かない）。
fn a_script() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0A\e")
            .get("OnGhostChanged", Ok(Some(r"\0A\e".to_owned())))
            .get("OnGhostChanging", Ok(None))
    }))
}

/// A を起こして定常に着かせる（起動記録を置く＝`OnFirstBoot` を飛ばす）。
fn running_a() -> SwitchRig {
    let mut rig = SwitchRig::new(vec![("A", a_script())]);
    // 起こし直すときの窓の準備が閉包を投函する先。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// A の 2 回目の起動が済み、切替の予約と読み直しの印が下りた（定常到達）。
fn a_rebooted(rig: &SwitchRig) -> bool {
    rig.exit_requested()
        || (rig.calls("A").len() == 2
            && rig.world.get_non_send::<SwitchInFlight>().is_none()
            && rig.world.get_non_send::<ReloadInFlight>().is_none())
}

/// A 自身への知らせなしの切替（出どころ「自動」）を窓口を通さずに頼む。
fn plain_switch_to_a(rig: &mut SwitchRig) -> SwitchVerdict {
    request_ghost_switch(
        &mut rig.world,
        SwitchRequest {
            ghost: GhostSpec::Folder("A".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event: None,
        },
    )
}

/// ghost の読み直しは自分自身への切替の道を通り、降ろす側に `OnGhostChanging`・`OnClose` を、起こす側に
/// 台詞を求める GET を 1 つも送らず、情報の通知（`OnInitialize`・`username`・`basewareversion`）は送る。
/// 終わりの記録は `reload_done` の 1 件で、印は下りる。
///
/// # 非空虚性
/// `switch_to` が由来を上書きしないと、根が `OnGhostChanged` になって赤。`end_switch` が
/// `on_switch_end` を呼ばないと、印が残って待ちが打ち切られ赤。
#[test]
fn a_ghost_reload_reboots_with_no_boot_notice_but_with_the_information_notices() {
    let mut rig = running_a();

    let (flow, events) = capture(|| {
        let verdict = request_reload(
            &mut rig.world,
            ReloadRequest {
                target: ReloadTarget::Ghost,
                origin: ReloadOrigin::Script,
                notes: DescriptNotes::default(),
            },
        );
        (verdict, rig.pump_talking_until(a_rebooted))
    });
    let calls = rig.calls("A");
    let exited = rig.exit_requested();
    assert!(rig.shutdown());

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "終了しない: {events:?}");
    assert_eq!(calls.len(), 2, "A を 1 回起こし直す: {calls:?}");
    let old = ids(&calls[0]);
    for absent in ["GET OnGhostChanging", "NOTIFY OnClose", "GET OnClose"] {
        assert!(
            old.iter().all(|c| c != absent),
            "降ろす側に {absent}: {old:?}"
        );
    }
    let new = ids(&calls[1]);
    for absent in ["GET OnBoot", "GET OnGhostChanged", "GET OnFirstBoot"] {
        assert!(
            new.iter().all(|c| c != absent),
            "起こす側に {absent}: {new:?}"
        );
    }
    assert!(
        new.iter()
            .all(|c| !c.starts_with("GET ") || c == "GET username"),
        "台詞を求める GET が 0 件: {new:?}"
    );
    for present in [
        "NOTIFY OnInitialize",
        "GET username",
        "NOTIFY basewareversion",
    ] {
        assert!(
            new.iter().any(|c| c == present),
            "起こす側に {present}: {new:?}"
        );
    }
    assert_eq!(named(&events, "reload_done"), 1, "{events:?}");
    assert_eq!(named(&events, "reload_failed"), 0, "{events:?}");
}

/// 印が無い自分自身への切替は今までどおり `OnGhostChanged` で起きる（要件 3.6・12.4）。
#[test]
fn a_switch_without_the_mark_boots_with_on_ghost_changed_as_before() {
    let mut rig = running_a();

    let (flow, events) = capture(|| {
        (
            plain_switch_to_a(&mut rig),
            rig.pump_talking_until(a_rebooted),
        )
    });
    let calls = rig.calls("A");
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert_eq!(calls.len(), 2, "{calls:?}");
    let new = ids(&calls[1]);
    assert!(new.iter().any(|c| c == "GET OnGhostChanged"), "{new:?}");
    assert_eq!(named(&events, "reload_done"), 0, "{events:?}");
}

/// 印があっても進行中の予約が他人のもの（鍵が違う）なら由来は上書きされず `OnGhostChanged` で起き、
/// 印は `reload_failed(overtaken)` を残して下りる（design「述語の契約」）。
#[test]
fn a_foreign_switch_under_a_reload_mark_boots_as_before_and_drops_the_mark_as_overtaken() {
    let mut rig = running_a();
    rig.world.insert_non_send(ReloadInFlight {
        target: ReloadTarget::Ghost,
        origin: ReloadOrigin::Script,
        ghost: "A".to_owned(),
        stage: ReloadStage::Switching {
            reserved: rig.root.dir().join("not-a"),
        },
    });

    let (flow, events) = capture(|| {
        (
            plain_switch_to_a(&mut rig),
            rig.pump_talking_until(a_rebooted),
        )
    });
    let calls = rig.calls("A");
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert_eq!(calls.len(), 2, "{calls:?}");
    let new = ids(&calls[1]);
    assert!(new.iter().any(|c| c == "GET OnGhostChanged"), "{new:?}");
    let reasons: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_failed"))
        .map(|e| e.field_str("reason").map(str::to_owned))
        .collect();
    assert_eq!(reasons, [Some("overtaken".to_owned())], "{events:?}");
}
