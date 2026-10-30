//! 常に手前の状態と台本の重なりの組の寿命（ゴーストの側）の決定論テスト（stay-on-top-directives
//! task 5.3・要件 4.1・4.2・4.3・5.3・5.4・6.4・7.2・7.4・design「常に手前の状態の寿命」）。
//!
//! 切替の土台（[`SwitchRig`]）で A を起こし、結線状態の常に手前をオンにして台本の組を 1 本載せてから、
//! ゴーストの読み直し・ゴーストの切り替え・SHIORI だけの読み直しを本物の経路で通す。
//! 確かめること: 結線状態を作り直す経路（読み直し・切り替え）ではオフから始まり、台本の組は無く、
//! オンのまま降ろした記録が理由つきで 1 行残る。作り直さない経路（SHIORI だけの読み直し）では
//! 状態と台本の組がそのまま残り、降ろした記録は残らない。
//! シェルとバルーンの切り替えは `frame/switch_stay_on_top_tests.rs` が固定する。

use areka_kanade::ChangeOrigin;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::Emo2Wiring;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::reload::{ReloadInFlight, ReloadOrigin, ReloadRequest, request_reload};
use crate::emo2_boot::reload_args::{DescriptNotes, ReloadTarget};
use crate::placement::zorder_group_ledger::{GroupSource, parse_zorder_tokens};

/// 切替の握手と SHIORI だけの読み直しに応える台本。
fn script(name: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        let talk = format!(r"\0{name}\e");
        standard_script(&talk)
            .get("OnGhostChanged", Ok(Some(talk.clone())))
            .get("OnGhostChanging", Ok(None))
            .notify("OnDestroy", Ok(()))
    }))
}

/// A（と切替先の B）を組み、A を起こして定常に着かせ、常に手前をオンにして台本の組を 1 本載せる。
fn running_a_on() -> SwitchRig {
    let mut rig = SwitchRig::new(vec![("A", script("A")), ("B", script("B"))]);
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    {
        let mut wiring = rig
            .world
            .get_non_send_mut::<Emo2Wiring>()
            .expect("A の結線状態が据わっている");
        wiring.window_state.on = true;
        let (members, _) = parse_zorder_tokens(&["0", "1"]).expect("0,1 は読める");
        wiring
            .zorder_ledger
            .try_add_tag_group(members)
            .expect("台本の組を載せる");
    }
    assert_eq!(state(&rig), Some((true, 1)), "前提: オンで台本の組が 1 本");
    rig
}

/// 今の結線状態の（常に手前か, 台本の組の数）。結線状態が無ければ `None`。
fn state(rig: &SwitchRig) -> Option<(bool, usize)> {
    rig.world.get_non_send::<Emo2Wiring>().map(|wiring| {
        let tags = wiring
            .zorder_ledger
            .groups()
            .iter()
            .filter(|group| group.source == GroupSource::Tag)
            .count();
        (wiring.window_state.on, tags)
    })
}

/// オンのまま降ろした記録の（ゴースト, 理由）の列。
fn discards(events: &[CapturedEvent]) -> Vec<(Option<String>, Option<String>)> {
    events
        .iter()
        .filter(|e| e.message().contains("常に手前のまま降ろした"))
        .map(|e| {
            (
                e.field_str("ghost").map(str::to_owned),
                e.field_str("reason").map(str::to_owned),
            )
        })
        .collect()
}

/// 読み直し（`target`）を台本の出どころで頼み、A を組み直し終えるまで回す。
fn reload(rig: &mut SwitchRig, target: ReloadTarget) -> Vec<CapturedEvent> {
    let (flow, events) = capture(|| {
        let verdict = request_reload(
            &mut rig.world,
            ReloadRequest {
                target,
                origin: ReloadOrigin::Script,
                notes: DescriptNotes::default(),
            },
        );
        let done = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls("A").len() == 2
                    && rig.world.get_non_send::<SwitchInFlight>().is_none()
                    && rig.world.get_non_send::<ReloadInFlight>().is_none())
        });
        (verdict, done)
    });
    assert_eq!(
        flow,
        (crate::emo2_boot::reload::ReloadVerdict::Accepted, true),
        "{events:?}"
    );
    assert!(!rig.exit_requested(), "終了しない: {events:?}");
    events
}

/// ゴーストの読み直し（要件 4.2・5.3・6.4）: 新しい結線状態はオフで台本の組が無く（descript の基底だけ）、
/// オンのまま降ろした記録が理由 `reload` で 1 行残る。
///
/// # 非空虚性
/// 読み直しが結線状態を作り直さないとオンと台本の組が残って赤。降ろす前の記録の理由を切り替えと
/// 取り違えると `switch` になって赤。
#[test]
fn a_ghost_reload_starts_off_with_no_script_group_and_logs_the_reload_discard() {
    let mut rig = running_a_on();
    let events = reload(&mut rig, ReloadTarget::Ghost);
    let after = state(&rig);
    assert!(rig.shutdown());

    assert_eq!(
        after,
        Some((false, 0)),
        "(常に手前, 台本の組の数): {events:?}"
    );
    assert_eq!(
        discards(&events),
        [(Some("A".to_owned()), Some("reload".to_owned()))],
        "{events:?}"
    );
}

/// ゴーストの切り替え（要件 4.1・6.4）: 切替先 B はオフで台本の組が無く、オンのまま降ろした記録が
/// 理由 `switch` で 1 行残る。
///
/// # 非空虚性
/// 切り替えが結線状態を作り直さないとオンが B へ持ち越して赤。降ろす前に記録しないと 0 行で赤。
#[test]
fn a_ghost_switch_starts_the_target_off_and_logs_the_switch_discard() {
    let mut rig = running_a_on();
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
    let after = state(&rig);
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "終了しない: {events:?}");
    assert_eq!(
        after,
        Some((false, 0)),
        "(常に手前, 台本の組の数): {events:?}"
    );
    assert_eq!(
        discards(&events),
        [(Some("A".to_owned()), Some("switch".to_owned()))],
        "{events:?}"
    );
}

/// SHIORI だけの読み直し（要件 4.3・5.4）: 結線状態は残り、常に手前のオンと台本の組がそのまま残る。
/// 降ろした記録は残らない（オフへ戻していない）。
///
/// # 非空虚性
/// SHIORI だけの読み直しが窓と結線状態を作り直すと、オフで台本の組が無くなって赤。
#[test]
fn a_shiori_reload_keeps_the_stay_on_top_state_and_the_script_group() {
    let mut rig = running_a_on();
    let events = reload(&mut rig, ReloadTarget::Shiori);
    let after = state(&rig);
    assert!(rig.shutdown());

    assert_eq!(
        after,
        Some((true, 1)),
        "(常に手前, 台本の組の数): {events:?}"
    );
    assert_eq!(discards(&events), [], "{events:?}");
}
