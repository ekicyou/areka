//! 読み直しの起動（[`BootOrigin::Reloaded`]）のテスト（areka-P0-mcp-reload 要件 3.3・3.4・4.3・12.2）。
//!
//! 起動記録の有無（`first_boot`）と読み直しの対象（[`ReloadScope`]）の組ごとに、起動の系列が
//! 台詞を求める GET を 1 つも送らず（username 照会だけ）・basewareversion を送って定常へ入ること、
//! `OnInitialize` の References が対象で分かれることを `step` 経由で固定する。

use super::test_support::assert_notify;
use super::*;
use crate::change::{BootOrigin, KanadeNotice, ReloadScope};
use crate::msg::ShioriCall;
use crate::schedule::step;
use crate::status::ExecutionSnapshot;
use crate::talk::EpilogueCommand;

fn cfg(first_boot: bool, scope: ReloadScope) -> KanadeConfig {
    let mut c = KanadeConfig::new("master", "1.0.0");
    c.first_boot = first_boot;
    c.boot_origin = BootOrigin::Reloaded { scope };
    c
}

fn reply(s: State, outcome: ShioriOutcome, c: &KanadeConfig) -> (State, Vec<Action>) {
    let input = Input::ShioriReply {
        outcome,
        origin: "test",
    };
    step(s, input, c)
}

/// Boot から定常まで（どの GET にも 204 で答える）進め、各段の Action 列を順に返す。
fn drive_to_steady(c: &KanadeConfig) -> Vec<Vec<Action>> {
    let (s, boot) = step(State::initial(), Input::Boot, c);
    let (s, init) = reply(s, ShioriOutcome::Notified, c);
    let (s, prefetch) = reply(s, ShioriOutcome::NoContent, c);
    let (s, version) = reply(s, ShioriOutcome::Notified, c);
    assert!(
        matches!(s.phase, Phase::Steady { talk: None }),
        "定常へ入るはず"
    );
    vec![boot, init, prefetch, version]
}

fn get_ids(steps: &[Vec<Action>]) -> Vec<String> {
    steps
        .iter()
        .flatten()
        .filter_map(|a| match a {
            Action::ShioriRequest(ShioriCall::Get { id, .. }) => Some(id.as_str().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn reload_sends_no_talk_get_even_on_first_boot() {
    let snap = &ExecutionSnapshot::INACTIVE;
    for first_boot in [false, true] {
        for scope in [ReloadScope::Ghost, ReloadScope::Shiori] {
            let c = cfg(first_boot, scope);
            let steps = drive_to_steady(&c);
            // GET は username 照会だけ（OnFirstBoot・OnGhostChanged・OnBoot を送らない・要件 3.3・4.3）。
            assert_eq!(
                get_ids(&steps),
                ["username"],
                "first_boot={first_boot} scope={scope:?}"
            );
            // 照会の応答の直後は [sink, basewareversion]（情報の通知は送る・要件 3.4）。
            let prefetch = &steps[2];
            assert_eq!(prefetch.len(), 2, "first_boot={first_boot} scope={scope:?}");
            assert!(matches!(
                prefetch[0],
                Action::ResourceOutcome { id: "username", .. }
            ));
            assert_notify(&prefetch[1], &events::baseware_version(&c, snap));
            // 定常に入った通知がちょうど 1 件。
            assert!(matches!(
                steps[3].as_slice(),
                [Action::Notice(KanadeNotice::Steady)]
            ));
        }
    }
}

#[test]
fn on_initialize_references_differ_by_scope() {
    let snap = &ExecutionSnapshot::INACTIVE;
    let (_, ghost) = step(
        State::initial(),
        Input::Boot,
        &cfg(false, ReloadScope::Ghost),
    );
    assert_notify(&ghost[0], &events::on_initialize(snap));
    let (_, shiori) = step(
        State::initial(),
        Input::Boot,
        &cfg(false, ReloadScope::Shiori),
    );
    assert_notify(
        &shiori[0],
        &events::raise(
            "OnInitialize",
            vec!["reload".to_string()],
            ShioriMethod::Notify,
            snap,
        ),
    );
    // References だけが違う（取り違えの檻）。
    assert!(matches!(
        &shiori[0],
        Action::ShioriRequest(ShioriCall::Notify { references, .. }) if references == &["reload"]
    ));
    assert!(matches!(
        &ghost[0],
        Action::ShioriRequest(ShioriCall::Notify { references, .. }) if references.is_empty()
    ));
}

#[test]
fn reload_on_first_boot_runs_only_the_epilogue_talk() {
    let snap = &ExecutionSnapshot::INACTIVE;
    let mut c = cfg(true, ReloadScope::Ghost);
    c.first_boot_epilogue = vec![EpilogueCommand {
        name: "areka.prop.set".to_string(),
        tokens: vec!["areka.boot.count".to_string(), "1".to_string()],
    }];
    let (s, boot) = step(State::initial(), Input::Boot, &c);
    let (s, init) = reply(s, ShioriOutcome::Notified, &c);
    let (_, prefetch) = reply(s, ShioriOutcome::NoContent, &c);
    let steps = [boot, init, prefetch];
    let prefetch = &steps[2];
    // GET は username 照会だけ（OnFirstBoot を送らない）。
    assert_eq!(get_ids(&steps), ["username"]);
    // [sink, 記録だけのトーク（台詞なし）, basewareversion]（初回の記録は既存の腕のまま）。
    assert_eq!(prefetch.len(), 3);
    assert!(matches!(
        prefetch[0],
        Action::ResourceOutcome { id: "username", .. }
    ));
    match &prefetch[1] {
        Action::StartTalk(st) => {
            assert_eq!(st.script, "");
            assert_eq!(st.epilogue, c.first_boot_epilogue);
        }
        _ => panic!("記録だけのトークが出るはず"),
    }
    assert_notify(&prefetch[2], &events::baseware_version(&c, snap));
}
