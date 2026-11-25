//! 手続きの成功の列のテスト。
//!
//! 偽の口の上で、本物の `areka-nar` を一時の根へ走らせ、書庫 5 種（ゴースト・バルーン同梱の
//! ゴースト・バルーン・`accept` 付きのシェル・`accept` 付きの追加ファイル）のイベントの名前・
//! 順・Reference と、口への呼び出しの順を判定する。

use areka_nar::InstallKind;

use super::procedure_test_support::{
    Call, FakePorts, OverwriteScript, RUNNING, RUNNING_ALIAS, RUNNING_SAKURA, balloon_nar,
    ghost_nar, ghost_with_balloon_descript, ghost_with_balloon_nar, joined, run_many, run_one,
    shell_nar, supplement_nar, with_terms,
};
use super::*;
use crate::session_tests::{events_named, write_descript};

fn refs(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// 根の下のフォルダの絶対パスの綴り（Reference2 の場所）。
fn place(ports: &FakePorts, relative: &str) -> String {
    ports.under_root(relative).display().to_string()
}

fn item(kind: &'static str, name: &str, place: String) -> InstalledItem {
    InstalledItem {
        kind,
        name: name.to_owned(),
        place,
    }
}

#[test]
fn ghost_runs_begin_install_record_complete_ex_then_legacy_when_no_reply() {
    let mut ports = FakePorts::new();
    let (ends, _) = run_one(&mut ports, ghost_nar("newbie", &[]));

    let at = place(&ports, "ghost/newbie");
    assert_eq!(
        ends,
        vec![ArchiveEnd::Installed(vec![item(
            "ghost",
            "あたらしい",
            at.clone()
        )])]
    );
    assert_eq!(
        ports.calls,
        vec![
            Call::Raise("OnInstallBegin", Vec::new()),
            Call::Facts,
            Call::Elsewhere(None),
            Call::Record(InstalledRecord {
                kind: InstallKind::Ghost,
                object_name: "あたらしい".to_owned(),
                folder: "newbie".to_owned(),
                ghost_folder: Some("newbie".to_owned()),
            }),
            Call::Raise(
                "OnInstallCompleteEx",
                refs(&["ghost", "あたらしい", at.as_str()])
            ),
            Call::Raise("OnInstallComplete", refs(&["ghost", "あたらしい", ""])),
        ],
        "始まり → 照合 → 展開 → 記録 → 締め。応えが無いので旧仕様へ続く"
    );
    assert!(
        ports
            .under_root("ghost/newbie/ghost/master/descript.txt")
            .is_file(),
        "本物の areka-nar が根へ入れた"
    );
}

#[test]
fn complete_ex_with_a_script_sends_no_legacy_complete() {
    let mut ports = FakePorts::new();
    ports.replies.insert("OnInstallCompleteEx", Raised::Script);
    run_one(&mut ports, ghost_nar("newbie", &[]));
    assert_eq!(
        ports.raised_ids(),
        vec!["OnInstallBegin", "OnInstallCompleteEx"],
        "台本が返れば OnInstallComplete は 0 件"
    );
}

#[test]
fn ghost_with_balloon_lists_body_first_then_the_companion() {
    for (reply, legacy) in [(Raised::NoReply, 1), (Raised::Script, 0)] {
        let mut ports = FakePorts::new();
        ports.replies.insert("OnInstallCompleteEx", reply);
        run_one(&mut ports, ghost_with_balloon_nar());

        let ghost_at = place(&ports, "ghost/newbie");
        let balloon_at = place(&ports, "ghost/newbie/balloon/kaku");
        let mut expected = vec![
            ("OnInstallBegin", Vec::new()),
            (
                "OnInstallCompleteEx",
                vec![
                    joined(&["ghost", "balloon"]),
                    // 同梱バルーンの名前は入れた後の目録の `name`。
                    joined(&["あたらしい", "Kaku"]),
                    joined(&[&ghost_at, &balloon_at]),
                ],
            ),
        ];
        if legacy == 1 {
            // 旧仕様の Reference0 は同梱でも `ghost`・Reference2 は同梱バルーンの名前。
            expected.push(("OnInstallComplete", refs(&["ghost", "あたらしい", "Kaku"])));
        }
        assert_eq!(ports.raised(), expected, "応え {reply:?}");
        assert!(
            ports
                .raised()
                .iter()
                .flat_map(|(_, r)| r)
                .all(|r| !r.contains("with")),
            "`ghost with balloon` は送らない"
        );
        assert!(
            ports
                .under_root("ghost/newbie/balloon/kaku/descript.txt")
                .is_file()
        );
    }
}

#[test]
fn companion_balloon_without_a_name_falls_back_to_its_folder_name() {
    let mut ports = FakePorts::new();
    run_one(
        &mut ports,
        ghost_with_balloon_descript(b"charset,UTF-8\r\n"),
    );
    let raised = ports.raised();
    assert_eq!(raised[1].0, "OnInstallCompleteEx");
    assert_eq!(
        raised[1].1[1],
        joined(&["あたらしい", "kaku"]),
        "目録に名前が無ければフォルダ名"
    );
    assert_eq!(
        raised[2],
        ("OnInstallComplete", refs(&["ghost", "あたらしい", "kaku"]))
    );
}

/// 同梱の表示名は入れたゴーストの中のバルーンの `name` で、根の置き場の同じフォルダ名の
/// 別の `name` を取らない。
#[test]
fn companion_name_comes_from_inside_the_ghost_not_the_root_of_the_same_folder() {
    let mut ports = FakePorts::new();
    write_descript(&ports.under_root("balloon/kaku"), "name,RootKaku");
    run_one(&mut ports, ghost_with_balloon_nar());

    let raised = ports.raised();
    assert_eq!(raised[1].0, "OnInstallCompleteEx");
    assert_eq!(
        raised[1].1[1],
        joined(&["あたらしい", "Kaku"]),
        "ゴーストの中の name（根の RootKaku ではない）"
    );
    assert_eq!(
        raised[1].1[2],
        joined(&[
            &place(&ports, "ghost/newbie"),
            &place(&ports, "ghost/newbie/balloon/kaku")
        ]),
        "入れた場所はゴーストの中のまま"
    );
}

#[test]
fn balloon_is_installed_elsewhere_and_recorded_without_a_ghost() {
    let mut ports = FakePorts::new();
    run_one(&mut ports, balloon_nar());

    let at = place(&ports, "balloon/round");
    assert_eq!(
        ports.calls,
        vec![
            Call::Raise("OnInstallBegin", Vec::new()),
            Call::Facts,
            Call::Elsewhere(None),
            Call::Record(InstalledRecord {
                kind: InstallKind::Balloon,
                object_name: "まるい".to_owned(),
                folder: "round".to_owned(),
                ghost_folder: None,
            }),
            Call::Raise(
                "OnInstallCompleteEx",
                refs(&["balloon", "まるい", at.as_str()])
            ),
            Call::Raise("OnInstallComplete", refs(&["balloon", "まるい", ""])),
        ]
    );
}

#[test]
fn shell_named_by_sakura_name_goes_into_the_running_ghost_shell_folder() {
    let mut ports = FakePorts::new();
    run_one(&mut ports, shell_nar(RUNNING_SAKURA, &[]));

    let at = place(&ports, &format!("ghost/{RUNNING}/shell/dress"));
    assert_eq!(
        ports.calls,
        vec![
            Call::Raise("OnInstallBegin", Vec::new()),
            Call::Facts,
            // シェルは起動中のゴーストを降ろさずに入れる。
            Call::Elsewhere(Some(RUNNING.to_owned())),
            Call::Record(InstalledRecord {
                kind: InstallKind::Shell,
                object_name: "きがえ".to_owned(),
                folder: "dress".to_owned(),
                ghost_folder: Some(RUNNING.to_owned()),
            }),
            Call::Raise(
                "OnInstallCompleteEx",
                refs(&["shell", "きがえ", at.as_str()])
            ),
            Call::Raise("OnInstallComplete", refs(&["shell", "きがえ", ""])),
        ]
    );
    assert!(
        ports
            .under_root(&format!("ghost/{RUNNING}/shell/dress/descript.txt"))
            .is_file()
    );
}

#[test]
fn supplement_returned_as_not_running_is_installed_elsewhere_by_the_procedure() {
    let mut ports = FakePorts::new();
    ports.overwrite = OverwriteScript::NotRunning;
    let (ends, _) = run_one(&mut ports, supplement_nar(RUNNING_ALIAS));

    let at = place(&ports, &format!("ghost/{RUNNING}"));
    assert!(
        matches!(ends.as_slice(), [ArchiveEnd::Installed(_)]),
        "{ends:?}"
    );
    assert_eq!(
        ports.calls,
        vec![
            Call::Raise("OnInstallBegin", Vec::new()),
            Call::Facts,
            Call::Overwrite(Some(RUNNING.to_owned())),
            Call::Elsewhere(Some(RUNNING.to_owned())),
            Call::Record(InstalledRecord {
                kind: InstallKind::Supplement,
                object_name: "ついか".to_owned(),
                folder: RUNNING.to_owned(),
                ghost_folder: Some(RUNNING.to_owned()),
            }),
            Call::Raise(
                "OnInstallCompleteEx",
                refs(&["supplement", "ついか", at.as_str()])
            ),
            Call::Raise("OnInstallComplete", refs(&["supplement", "ついか", ""])),
        ],
        "返された書庫は手続きがよそへ入れる"
    );
    assert!(
        ports
            .under_root(&format!("ghost/{RUNNING}/ghost/master/extra.dic"))
            .is_file()
    );
}

#[test]
fn abandoning_an_order_logs_the_archives_not_yet_started() {
    let mut ports = FakePorts::new();
    ports.replies.insert("OnInstallBegin", Raised::Closed);

    let (ends, events) = run_many(&mut ports, vec![ghost_nar("newbie", &[]); 3]);

    assert_eq!(
        ends,
        vec![ArchiveEnd::Abandoned],
        "最初に閉じたら以後の書庫は扱わない"
    );
    assert_eq!(ports.raised_ids(), vec!["OnInstallBegin"]);
    let abandoned: Vec<_> = events_named(&events, "install_abandoned");
    assert_eq!(abandoned.len(), 1);
    assert_eq!(
        abandoned[0].field("skipped"),
        Some("2"),
        "始めていない書庫の件数"
    );
    let skipped = abandoned[0].field("skipped_archives").expect("残りのパス");
    assert!(
        skipped.contains("archive1.nar")
            && skipped.contains("archive2.nar")
            && !skipped.contains("archive0.nar"),
        "{skipped}"
    );
}

#[test]
fn supplement_named_by_install_accept_overwrites_the_running_ghost() {
    let mut ports = FakePorts::new();
    run_one(&mut ports, supplement_nar(RUNNING_ALIAS));

    let at = place(&ports, &format!("ghost/{RUNNING}"));
    assert_eq!(
        ports.calls,
        vec![
            Call::Raise("OnInstallBegin", Vec::new()),
            Call::Facts,
            // 宛先は起動中のゴーストのフォルダそのもの＝降ろして入れる口へ。
            Call::Overwrite(Some(RUNNING.to_owned())),
            Call::Record(InstalledRecord {
                kind: InstallKind::Supplement,
                object_name: "ついか".to_owned(),
                folder: RUNNING.to_owned(),
                ghost_folder: Some(RUNNING.to_owned()),
            }),
            Call::Raise(
                "OnInstallCompleteEx",
                refs(&["supplement", "ついか", at.as_str()])
            ),
            Call::Raise("OnInstallComplete", refs(&["supplement", "ついか", ""])),
        ]
    );
    assert!(
        ports
            .under_root(&format!("ghost/{RUNNING}/ghost/master/extra.dic"))
            .is_file()
    );
}

#[test]
fn accepted_terms_come_after_begin_and_before_the_install() {
    let mut ports = FakePorts::new();
    run_one(&mut ports, with_terms(ghost_nar("newbie", &[])));
    let order: Vec<&str> = ports
        .calls
        .iter()
        .map(|call| match call {
            Call::Raise(id, _) => *id,
            Call::Facts => "facts",
            Call::Ask(_) => "ask",
            Call::Elsewhere(_) => "elsewhere",
            Call::Overwrite(_) => "overwrite",
            Call::Record(_) => "record",
            Call::Warn { .. } => "warn",
        })
        .collect();
    assert_eq!(
        order,
        vec![
            "OnInstallBegin",
            "facts",
            "ask",
            "OnGhostTermsAccept",
            "elsewhere",
            "record",
            "OnInstallCompleteEx",
            "OnInstallComplete",
        ],
        "始まりが先・照合と利用条件はその後・展開の前"
    );
}

#[test]
fn every_kind_sends_only_install_events_and_exactly_one_closing_notice() {
    // 口に切替を頼む関数は無い（型の上で起きない）。送るのは正典のインストール系の語だけ。
    const INSTALL_EVENTS: [&str; 8] = [
        "OnInstallBegin",
        "OnInstallComplete",
        "OnInstallCompleteEx",
        "OnInstallCompleteAll",
        "OnInstallFailure",
        "OnInstallRefuse",
        "OnGhostTermsAccept",
        "OnGhostTermsDecline",
    ];
    let kinds = [
        ghost_nar("newbie", &[]),
        ghost_with_balloon_nar(),
        balloon_nar(),
        shell_nar(RUNNING_SAKURA, &[]),
        supplement_nar(RUNNING_ALIAS),
    ];
    for builder in kinds {
        for reply in [Raised::NoReply, Raised::Script] {
            let mut ports = FakePorts::new();
            ports.replies.insert("OnInstallCompleteEx", reply);
            let (ends, _) = run_one(&mut ports, builder.clone());
            let ids = ports.raised_ids();
            assert!(matches!(ends.as_slice(), [ArchiveEnd::Installed(_)]));
            assert!(ids.iter().all(|id| INSTALL_EVENTS.contains(id)), "{ids:?}");
            assert_eq!(
                ids.iter()
                    .filter(|id| **id == "OnInstallCompleteEx")
                    .count(),
                1,
                "締めの知らせは 1 つ"
            );
            let legacy = ids.iter().filter(|id| **id == "OnInstallComplete").count();
            assert_eq!(
                legacy,
                usize::from(reply == Raised::NoReply),
                "旧仕様は応えが無いときだけ"
            );
            assert!(
                !ids.contains(&"OnInstallCompleteAll"),
                "1 本だけの依頼では送らない"
            );
        }
    }
}

#[test]
fn each_stage_is_logged() {
    let (_, events) = run_one(&mut FakePorts::new(), ghost_nar("newbie", &[]));
    let names: Vec<&str> = events
        .iter()
        .filter_map(|event| event.field_str("event"))
        .filter(|name| name.starts_with("install_"))
        .collect();
    assert_eq!(
        names,
        vec![
            "install_begin",
            "install_event",
            "install_accept",
            "install_done",
            "install_event",
            "install_event",
        ],
        "各段の記録"
    );
    let begin = events_named(&events, "install_begin")
        .first()
        .copied()
        .expect("始まりの記録");
    assert_eq!(begin.field("origin"), Some("Menu"));
    assert!(
        events
            .iter()
            .all(|event| event.level != tracing::Level::ERROR),
        "成功の列に error! は 0 件"
    );
}
