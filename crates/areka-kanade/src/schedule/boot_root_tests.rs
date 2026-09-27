//! 起動の根の表（[`boot_root`]）と定常到達の通知のテスト
//! （areka-P0-ghost-shell-balloon-switch 要件 3.5・4.1〜4.5・6.2・10.13・11.3）。
//!
//! 起動記録の有無（`first_boot`）と起動の由来（`boot_origin`）の組ごとに、username 照会の応答の
//! 直後にどの根が出るか・根の応答の既存の腕（204 → `OnBoot`・台本 → `OnBoot` を飛ばす）が
//! 変わらず効くか・定常に入った時点で通知がちょうど 1 件出るかを `step` 経由で固定する。

use super::test_support::{assert_get, assert_notify, initial};
use super::*;
use crate::change::{BootOrigin, ChangedFrom, KanadeNotice};
use crate::msg::ShioriCall;
use crate::schedule::step;

fn cfg(first_boot: bool, boot_origin: BootOrigin) -> KanadeConfig {
    let mut c = KanadeConfig::new("master", "1.0.0");
    c.first_boot = first_boot;
    c.boot_origin = boot_origin;
    c
}

fn from() -> ChangedFrom {
    ChangedFrom {
        sakura_name: "かなで".to_string(),
        script: r"\0じゃあね\e".to_string(),
        name: "Kanade Ghost".to_string(),
        dir: r"C:\areka\ghost\kanade".to_string(),
    }
}

fn reply(s: State, outcome: ShioriOutcome, c: &KanadeConfig) -> (State, Vec<Action>) {
    step(
        s,
        Input::ShioriReply {
            outcome,
            origin: "test",
        },
        c,
    )
}

/// Boot → OnInitialize 完了 → username 照会 204 まで進め、照会の応答が返した Action 列を返す。
fn drive_past_prefetch(c: &KanadeConfig) -> (State, Vec<Action>) {
    let (s, _) = step(initial(), Input::Boot, c);
    let (s, _) = reply(s, ShioriOutcome::Notified, c);
    reply(s, ShioriOutcome::NoContent, c)
}

/// Action 列に含まれる GET の ID を順に並べる。
fn get_ids(actions: &[Action]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::ShioriRequest(ShioriCall::Get { id, .. }) => Some(id.as_str().to_string()),
            _ => None,
        })
        .collect()
}

/// 根（GET）を ID と Reference の組へ写す（`ShioriCall` は `PartialEq` を持たない）。
fn parts(call: Option<ShioriCall>) -> Option<(String, Vec<String>)> {
    call.map(|c| match c {
        ShioriCall::Get { id, references, .. } => (id.as_str().to_string(), references),
        ShioriCall::Notify { .. } => panic!("根は GET のはず"),
    })
}

fn notice_count(actions: &[Action]) -> usize {
    actions
        .iter()
        .filter(|a| matches!(a, Action::Notice(_)))
        .count()
}

#[test]
fn boot_root_table_picks_one_root_per_origin() {
    let snap = &ExecutionSnapshot::INACTIVE;
    // 起動記録なしは由来を問わず OnFirstBoot（初回起動が最優先・要件 4.4・11.3）。
    for origin in [
        BootOrigin::Plain,
        BootOrigin::ChangedFrom(from()),
        BootOrigin::Halted {
            ghost_name: "B".to_string(),
        },
    ] {
        let c = cfg(true, origin);
        assert_eq!(
            parts(boot_root(&c)),
            parts(Some(events::on_first_boot(snap, 0)))
        );
    }
    // 起動記録あり＋切替で来た → OnGhostChanged（要件 4.1）。
    let c = cfg(false, BootOrigin::ChangedFrom(from()));
    assert_eq!(
        parts(boot_root(&c)),
        parts(Some(events::on_ghost_changed(
            &from(),
            &c.shell_folder,
            snap
        )))
    );
    // 起動記録あり＋それ以外 → 根なし（OnBoot だけ）。
    assert!(boot_root(&cfg(false, BootOrigin::Plain)).is_none());
    assert!(
        boot_root(&cfg(
            false,
            BootOrigin::Halted {
                ghost_name: "B".to_string()
            }
        ))
        .is_none()
    );
}

#[test]
fn first_boot_wins_over_changed_from_and_sends_no_on_ghost_changed() {
    let c = cfg(true, BootOrigin::ChangedFrom(from()));
    let (s, actions) = drive_past_prefetch(&c);
    assert!(matches!(s.phase, Phase::BootType));
    assert_eq!(actions.len(), 2, "[sink, OnFirstBoot] の 2 件");
    assert_get(
        &actions[1],
        &events::on_first_boot(&ExecutionSnapshot::INACTIVE, 0),
    );
    // OnFirstBoot 204 → OnBoot（既存の腕）。どこにも OnGhostChanged は出ない。
    let (s, next) = reply(s, ShioriOutcome::NoContent, &c);
    assert!(matches!(s.phase, Phase::BootMain));
    let ids: Vec<String> = get_ids(&actions)
        .into_iter()
        .chain(get_ids(&next))
        .collect();
    assert!(
        !ids.iter().any(|id| id.contains("OnGhostChanged")),
        "OnGhostChanged は 0 件のはず（実際 {ids:?}）"
    );
    assert_get(&next[0], &events::on_boot(&c, &ExecutionSnapshot::INACTIVE));
}

#[test]
fn changed_from_sends_on_ghost_changed_then_204_falls_through_to_on_boot() {
    let c = cfg(false, BootOrigin::ChangedFrom(from()));
    let (s, actions) = drive_past_prefetch(&c);
    assert!(matches!(s.phase, Phase::BootType));
    assert_eq!(actions.len(), 2, "[sink, OnGhostChanged] の 2 件");
    assert_get(
        &actions[1],
        &events::on_ghost_changed(&from(), &c.shell_folder, &ExecutionSnapshot::INACTIVE),
    );
    // 204 → OnBoot（Ref0 だけ・要件 4.2）。
    let (s, next) = reply(s, ShioriOutcome::NoContent, &c);
    assert!(matches!(s.phase, Phase::BootMain));
    assert_eq!(next.len(), 1);
    assert_get(&next[0], &events::on_boot(&c, &ExecutionSnapshot::INACTIVE));
    match &next[0] {
        Action::ShioriRequest(ShioriCall::Get { references, .. }) => {
            assert_eq!(references, &vec!["master".to_string()])
        }
        _ => unreachable!(),
    }
}

#[test]
fn changed_from_with_script_skips_on_boot() {
    let c = cfg(false, BootOrigin::ChangedFrom(from()));
    let (s, _) = drive_past_prefetch(&c);
    assert!(
        matches!(s.phase, Phase::BootType),
        "根 OnGhostChanged の応答待ち"
    );
    // 台本 → OnBoot を飛ばして basewareversion（要件 4.3）。
    let (s, next) = reply(s, ShioriOutcome::Value("交代".to_string()), &c);
    assert!(matches!(s.phase, Phase::BootVersion { talk: Some(_) }));
    assert!(
        get_ids(&next).is_empty(),
        "OnBoot は 0 件のはず（GET が出ていない）"
    );
    assert!(matches!(next[0], Action::StartTalk(_)));
    assert_notify(
        &next[1],
        &events::baseware_version(&c, &snapshot_of(&s.phase)),
    );
}

#[test]
fn halted_sends_on_boot_with_ref6_halt_and_ref7_name_directly() {
    let c = cfg(
        false,
        BootOrigin::Halted {
            ghost_name: "Broken Ghost".to_string(),
        },
    );
    let (s, actions) = drive_past_prefetch(&c);
    assert!(matches!(s.phase, Phase::BootMain));
    assert_eq!(actions.len(), 2, "[sink, OnBoot] の 2 件");
    match &actions[1] {
        Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => {
            assert_eq!(id.as_str(), "OnBoot");
            assert_eq!(
                references,
                &["master", "", "", "", "", "", "halt", "Broken Ghost"]
                    .map(String::from)
                    .to_vec()
            );
        }
        _ => panic!("OnBoot の GET のはず"),
    }
}

#[test]
fn plain_sends_on_boot_with_ref0_only() {
    let c = cfg(false, BootOrigin::Plain);
    let (s, actions) = drive_past_prefetch(&c);
    assert!(matches!(s.phase, Phase::BootMain));
    assert_eq!(actions.len(), 2, "[sink, OnBoot] の 2 件");
    match &actions[1] {
        Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => {
            assert_eq!(id.as_str(), "OnBoot");
            assert_eq!(references, &vec!["master".to_string()]);
        }
        _ => panic!("OnBoot の GET のはず"),
    }
}

#[test]
fn steady_notice_is_emitted_exactly_once_when_entering_steady() {
    for c in [
        cfg(true, BootOrigin::Plain),
        cfg(false, BootOrigin::ChangedFrom(from())),
        cfg(false, BootOrigin::Plain),
    ] {
        let (s, a0) = step(initial(), Input::Boot, &c);
        let (s, a1) = reply(s, ShioriOutcome::Notified, &c);
        let (mut s, a2) = reply(s, ShioriOutcome::NoContent, &c);
        let mut total = notice_count(&a0) + notice_count(&a1) + notice_count(&a2);
        if matches!(s.phase, Phase::BootType) {
            let (n, a) = reply(s, ShioriOutcome::NoContent, &c);
            total += notice_count(&a);
            s = n;
        }
        let (s, a3) = reply(s, ShioriOutcome::NoContent, &c); // BootMain 204 → BootVersion
        total += notice_count(&a3);
        assert_eq!(total, 0, "定常に入る前に通知は出ない");
        let (s, done) = reply(s, ShioriOutcome::Notified, &c); // BootVersion → Steady
        assert!(matches!(s.phase, Phase::Steady { talk: None }));
        assert!(
            matches!(done.as_slice(), [Action::Notice(KanadeNotice::Steady)]),
            "定常到達の通知がちょうど 1 件のはず（実際 {} 件）",
            done.len()
        );
    }
}
