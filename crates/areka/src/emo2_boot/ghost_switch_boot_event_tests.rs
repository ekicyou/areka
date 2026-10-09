//! 切替の要求の起動の知らせ（`SwitchRequest.boot_event`）の兄弟テスト
//! （areka-P0-network-update タスク 12.2・design「`SwitchRequest.boot_event`」・決めたこと 23・要件 5.5・5.6）。
//!
//! 切替の土台（[`SwitchRig`]）の上で、切替の入口・kanade・偽の SHIORI を本物の経路で通す。
//! 確かめること: 欄付きの自分自身への切替で起き直したゴーストの最初の知らせが渡したイベントと
//! Reference で、`OnGhostChanged`・`OnBoot` は 0 件（204 でも）／欄が空なら今日どおり `OnGhostChanged`／
//! 切替先が起動に失敗して既定へ戻ると、既定は `OnBoot`（Reference6＝`halt`）で起き知らせは届かない。

use areka_kanade::ChangeOrigin;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch};
use crate::boot_resolve::DEFAULT_GHOST_FOLDER;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;

/// 渡す起動の知らせの Reference（読み直しの `OnUpdateComplete` の形）。
fn update_refs() -> Vec<String> {
    ["changed", "descript.txt,ghost/master/yaya.dll"]
        .map(str::to_owned)
        .to_vec()
}

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

/// `id` の GET の Reference（最初の 1 件）。
fn get_refs(calls: &[RecordedCall], want: &str) -> Option<Vec<String>> {
    calls.iter().find_map(|call| match call {
        RecordedCall::Get { id, references } if id == want => Some(references.clone()),
        _ => None,
    })
}

fn named(events: &[CapturedEvent], name: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .count()
}

/// `scripts` の土台で A を起こして定常に着かせる（起動記録は全員に置く＝`OnFirstBoot` を飛ばす）。
fn running_a(scripts: Vec<(&str, FakeShiori)>) -> SwitchRig {
    let folders: Vec<String> = scripts.iter().map(|(f, _)| (*f).to_owned()).collect();
    let mut rig = SwitchRig::new(scripts);
    // 起こし直すときの窓の準備が閉包を投函する先。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    for folder in &folders {
        rig.plant_boot_record(folder);
    }
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// A 自身への知らせなしの切替（出どころ「自動」）を `boot_event` 付きで頼み、`done` まで回す。
fn switch_to_a(
    rig: &mut SwitchRig,
    boot_event: Option<(&'static str, Vec<String>)>,
    done: impl FnMut(&SwitchRig) -> bool,
) -> (SwitchVerdict, bool) {
    let verdict = request_ghost_switch(
        &mut rig.world,
        SwitchRequest {
            ghost: GhostSpec::Folder("A".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event,
        },
    );
    (verdict, rig.pump_talking_until(done))
}

/// A の 2 回目の起動が済み、切替の予約が下りた（定常到達）。
fn a_rebooted(rig: &SwitchRig) -> bool {
    rig.exit_requested()
        || (rig.calls("A").len() == 2 && rig.world.get_non_send::<SwitchInFlight>().is_none())
}

/// 欄付きの切替で起き直した A の最初の知らせは渡したイベントと Reference の GET で、204 でも
/// `OnGhostChanged`・`OnBoot` は 0 件のまま定常に入る。切替の要求の記録に知らせの名前が載る。
///
/// # 非空虚性
/// `switch_to` が欄を見ずに「切替で来た」で起こすと、根が `OnGhostChanged`（204 → `OnBoot`）になって赤。
#[test]
fn a_switch_with_a_boot_event_boots_the_target_with_that_event_instead_of_changed_or_boot() {
    let a = FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0A\e").get("OnUpdateComplete", Ok(None))
    }));
    let mut rig = running_a(vec![("A", a)]);

    let (flow, events) = capture(|| {
        switch_to_a(
            &mut rig,
            Some(("OnUpdateComplete", update_refs())),
            a_rebooted,
        )
    });
    let calls = rig.calls("A");
    let exited = rig.exit_requested();
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "終了しない: {events:?}");
    assert_eq!(calls.len(), 2, "A を 1 回起こし直す: {calls:?}");
    assert_eq!(
        ids(&calls[1]),
        [
            "NOTIFY OnInitialize",
            "GET username",
            "GET OnUpdateComplete",
            "NOTIFY basewareversion",
        ],
        "起き直した A の呼び出し: {events:?}"
    );
    assert_eq!(
        get_refs(&calls[1], "OnUpdateComplete"),
        Some(update_refs()),
        "渡した Reference のまま"
    );
    assert_eq!(named(&events, "ghost_switch_requested"), 1, "{events:?}");
    let requested = events
        .iter()
        .find(|e| e.field_str("event") == Some("ghost_switch_requested"))
        .and_then(|e| e.field("boot_event"));
    assert_eq!(requested, Some("Some(\"OnUpdateComplete\")"), "{events:?}");
}

/// 欄が空の切替は今日どおり: 起き直した A の根は `OnGhostChanged`（台本で応える＝`OnBoot` へ続かない）で、
/// 知らせのイベントは届かない。
#[test]
fn a_switch_without_a_boot_event_boots_the_target_with_on_ghost_changed_as_today() {
    let a = FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0A\e")
            .get("OnGhostChanged", Ok(Some(r"\0A\e".to_owned())))
            .get("OnUpdateComplete", Ok(None))
    }));
    let mut rig = running_a(vec![("A", a)]);

    let (flow, events) = capture(|| switch_to_a(&mut rig, None, a_rebooted));
    let calls = rig.calls("A");
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert_eq!(calls.len(), 2, "{calls:?}");
    let second = ids(&calls[1]);
    assert!(
        second.iter().any(|c| c == "GET OnGhostChanged"),
        "{second:?}"
    );
    assert!(
        second.iter().all(|c| c != "GET OnUpdateComplete"),
        "{second:?}"
    );
}

/// 欄付きの切替で切替先（A）が起動に失敗すると既定ゴーストへ戻り、既定は `OnBoot`（Reference6＝`halt`・
/// Reference7＝A の名前）で起きる。起動の知らせは既定へ届かない（要件 5.6）。
///
/// # 非空虚性
/// 既定へ戻す由来まで欄から作ると、既定の根が `OnUpdateComplete` になって赤。
#[test]
fn a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails() {
    let a = FakeShiori::ScriptedThenConnectFail(Box::new(|| standard_script(r"\0A\e")));
    let default = FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0emo2\e").get("OnUpdateComplete", Ok(None))
    }));
    let mut rig = running_a(vec![("A", a), (DEFAULT_GHOST_FOLDER, default)]);

    let (flow, events) = capture(|| {
        switch_to_a(&mut rig, Some(("OnUpdateComplete", update_refs())), |rig| {
            rig.exit_requested()
                || (rig.calls(DEFAULT_GHOST_FOLDER).len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let defaults = rig.calls(DEFAULT_GHOST_FOLDER);
    let exited = rig.exit_requested();
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "既定へ戻って続く: {events:?}");
    assert_eq!(named(&events, "ghost_switch_target_fault"), 1, "{events:?}");
    assert_eq!(defaults.len(), 1, "{defaults:?}");
    let on_boot = get_refs(&defaults[0], "OnBoot").expect("既定は OnBoot で起きる");
    assert_eq!(
        (
            on_boot.get(6).map(String::as_str),
            on_boot.get(7).map(String::as_str)
        ),
        (Some("halt"), Some("A")),
        "{on_boot:?}"
    );
    let ids = ids(&defaults[0]);
    assert!(ids.iter().all(|c| c != "GET OnUpdateComplete"), "{ids:?}");
    assert!(ids.iter().all(|c| c != "GET OnGhostChanged"), "{ids:?}");
}
