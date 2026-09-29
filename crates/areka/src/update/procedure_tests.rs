//! 手続きの兄弟テスト（design「Testing Strategy / 手続き」・要件 9.1・9.5・9.6）。
//!
//! 偽の口と `Progress` の台本で回す（エンジンは呼ばない・ネットへ出ない）。イベント名は写しの
//! 定数を借りずに綴りで書く（写しの取り違えもここで赤になる）。

use std::collections::HashMap;
use std::path::Path;

use areka_update::FetchError;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::procedure_test_support::{
    Call, EXPECTED, EngineScript, FakePorts, GHOST_URL, SHELL_URL, WRONG, order, script_changed,
    script_fetch_failed, script_md5_mismatch, script_none, script_rollback_failed,
    script_updated_with_leftovers, spec,
};
use super::{EngineRun, GhostResources, OrderEnd, Raised, run_order};
use crate::update::refs::TargetEnd;
use crate::update::{SummaryKind, TargetKind, UpdateOrder, UpdateReason};

const SEP: &str = "\u{1}";

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_owned()).collect()
}

/// 対象の種別ごとの名の組（begin・ready・download・md5 の 3・complete・failure）。
struct Names {
    begin: &'static str,
    ready: &'static str,
    download: &'static str,
    md5_begin: &'static str,
    md5_complete: &'static str,
    md5_failure: &'static str,
    complete: &'static str,
    failure: &'static str,
}

fn names(kind: TargetKind) -> Names {
    match kind {
        TargetKind::Ghost => Names {
            begin: "OnUpdateBegin",
            ready: "OnUpdateReady",
            download: "OnUpdate.OnDownloadBegin",
            md5_begin: "OnUpdate.OnMD5CompareBegin",
            md5_complete: "OnUpdate.OnMD5CompareComplete",
            md5_failure: "OnUpdate.OnMD5CompareFailure",
            complete: "OnUpdateComplete",
            failure: "OnUpdateFailure",
        },
        TargetKind::Shell | TargetKind::Balloon => Names {
            begin: "OnUpdateOtherBegin",
            ready: "OnUpdateOtherReady",
            download: "OnUpdateOther.OnDownloadBegin",
            md5_begin: "OnUpdateOther.OnMD5CompareBegin",
            md5_complete: "OnUpdateOther.OnMD5CompareComplete",
            md5_failure: "OnUpdateOther.OnMD5CompareFailure",
            complete: "OnUpdateOtherComplete",
            failure: "OnUpdateOtherFailure",
        },
    }
}

/// Reference0〜2 に種別と理由を足した 5 欄。
fn five(r: [&str; 3], kind: &str, reason: &str) -> Vec<String> {
    s(&[r[0], r[1], r[2], kind, reason])
}

fn absolute(dir: &Path) -> String {
    std::path::absolute(dir)
        .expect("相対のフォルダを絶対にできる")
        .display()
        .to_string()
}

fn with_event<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn run(order: &UpdateOrder, ports: &FakePorts) -> (OrderEnd, Vec<CapturedEvent>) {
    capture(|| run_order(order, ports))
}

/// 4 経路 × ゴースト／シェルの名で、送るイベントの名前・順・Reference と締めが 1 件だけのこと（要件 9.1・2.12）。
#[test]
fn four_paths_send_canonical_events_for_ghost_and_shell() {
    for kind in [TargetKind::Ghost, TargetKind::Shell] {
        let n = names(kind);
        let k = kind.as_ref_str();
        let target = spec(kind, "emo2", Some(GHOST_URL));
        let dir = absolute(&target.dir);
        let begin = (n.begin, five(["emo2", &dir, ""], k, "script"));
        let paths: [(
            &str,
            EngineScript,
            Vec<(&str, Vec<String>)>,
            String,
            TargetEnd,
        ); 4] = [
            (
                "none",
                script_none(),
                vec![
                    begin.clone(),
                    (n.complete, five(["none", "", ""], k, "script")),
                ],
                format!("{k}{SEP}OK{SEP}0"),
                TargetEnd::Unchanged,
            ),
            (
                "changed",
                script_changed(),
                vec![
                    begin.clone(),
                    (n.ready, five(["1", "a.txt,b.txt", ""], k, "script")),
                    (n.download, five(["a.txt", "0", "1"], k, "script")),
                    (
                        n.md5_begin,
                        five(["a.txt", EXPECTED, EXPECTED], k, "script"),
                    ),
                    (
                        n.md5_complete,
                        five(["a.txt", EXPECTED, EXPECTED], k, "script"),
                    ),
                    (n.download, five(["b.txt", "1", "1"], k, "script")),
                    (
                        n.md5_begin,
                        five(["b.txt", EXPECTED, EXPECTED], k, "script"),
                    ),
                    (
                        n.md5_complete,
                        five(["b.txt", EXPECTED, EXPECTED], k, "script"),
                    ),
                    (
                        n.complete,
                        five(["changed", "a.txt,b.txt", ""], k, "script"),
                    ),
                ],
                format!("{k}{SEP}OK{SEP}2"),
                TargetEnd::Changed(2),
            ),
            (
                "fetch",
                script_fetch_failed(&target.dir),
                vec![
                    begin.clone(),
                    (n.ready, five(["0", "a.txt", ""], k, "script")),
                    (n.download, five(["a.txt", "0", "0"], k, "script")),
                    (n.failure, five(["timeout", "a.txt", ""], k, "script")),
                ],
                format!("{k}{SEP}NG{SEP}timeout{SEP}a.txt"),
                TargetEnd::Failed {
                    word: "timeout".to_owned(),
                    file: Some("a.txt".to_owned()),
                },
            ),
            (
                "md5",
                script_md5_mismatch(&target.dir),
                vec![
                    begin.clone(),
                    (n.ready, five(["0", "a.txt", ""], k, "script")),
                    (n.download, five(["a.txt", "0", "0"], k, "script")),
                    (n.md5_begin, five(["a.txt", EXPECTED, WRONG], k, "script")),
                    (n.md5_failure, five(["a.txt", EXPECTED, WRONG], k, "script")),
                    (n.failure, five(["md5 miss", "a.txt", ""], k, "script")),
                ],
                format!("{k}{SEP}NG{SEP}md5 miss{SEP}a.txt"),
                TargetEnd::Failed {
                    word: "md5 miss".to_owned(),
                    file: Some("a.txt".to_owned()),
                },
            ),
        ];
        for (label, script, mut expected, summary, end) in paths {
            let ports = FakePorts::new([script]);
            let order = order(
                vec![target.clone()],
                UpdateReason::Script,
                SummaryKind::Result,
            );
            let (order_end, events) = run(&order, &ports);

            expected.push(("OnUpdateResult", vec![summary]));
            assert_eq!(ports.raised(), expected, "{k} {label}");
            let closings = ports
                .raised_ids()
                .into_iter()
                .filter(|id| *id == n.complete || *id == n.failure)
                .count();
            assert_eq!(closings, 1, "{k} {label}: 締めは 1 件だけ");
            assert_eq!(order_end.ends, vec![end.clone()], "{k} {label}");
            assert!(order_end.summarised, "{k} {label}");

            let failed = with_event(&events, "update_failed");
            if matches!(end, TargetEnd::Failed { .. }) {
                assert_eq!(failed.len(), 1, "{k} {label}: {events:?}");
                assert_eq!(failed[0].level, Level::ERROR);
                assert_eq!(failed[0].field_str("homeurl"), Some(GHOST_URL));
                let target_dir = target.dir.display().to_string();
                assert_eq!(failed[0].field("target"), Some(target_dir.as_str()));
                assert_eq!(failed[0].field_str("file"), Some("a.txt"));
                let kind_word = if label == "fetch" {
                    "FileFetch"
                } else {
                    "Md5Mismatch"
                };
                assert_eq!(failed[0].field_str("kind"), Some(kind_word));
                assert!(failed[0].field("stage").is_some(), "{:?}", failed[0]);
                assert_eq!(failed[0].field("rolled_back"), Some("true"));
            } else {
                assert!(failed.is_empty(), "{k} {label}: {events:?}");
            }
            let errors = events.iter().filter(|e| e.level == Level::ERROR).count();
            assert_eq!(errors, failed.len(), "{k} {label}: {events:?}");
        }
    }
}

/// `OnUpdateBegin` の Reference1 は対象のフォルダの絶対パス（要件 2.4）。
#[test]
fn begin_reference1_is_absolute_folder() {
    let target = spec(TargetKind::Balloon, "kaku", Some(SHELL_URL));
    assert!(target.dir.is_relative());
    let ports = FakePorts::new([script_none()]);
    let order = order(
        vec![target.clone()],
        UpdateReason::Manual,
        SummaryKind::Result,
    );
    run_order(&order, &ports);
    let (id, refs) = &ports.raised()[1];
    assert_eq!(*id, "OnUpdateOtherBegin");
    assert!(Path::new(&refs[1]).is_absolute(), "{refs:?}");
    assert_eq!(refs[1], absolute(&target.dir));
    assert_eq!(refs[0], "kaku");
    assert_eq!(&refs[3..], &s(&["balloon", "manual"])[..]);
}

/// メニューで `OnUpdateProcessExec` に台本が返れば、標準の手続き 0 件・総括 0・`info!` 1 件（要件 1.12）。
#[test]
fn process_exec_answered_by_script_stops_the_order() {
    let mut ports = FakePorts::new([]);
    ports.replies = HashMap::from([("OnUpdateProcessExec", Raised::Script)]);
    let order = order(
        vec![spec(TargetKind::Ghost, "emo2", Some(GHOST_URL))],
        UpdateReason::Manual,
        SummaryKind::Result,
    );
    let (end, events) = run(&order, &ports);

    assert_eq!(
        ports.calls(),
        vec![Call::Raise("OnUpdateProcessExec", s(&["manual"]))]
    );
    assert_eq!(end, OrderEnd::default());
    let answered = with_event(&events, "update_process_exec");
    assert_eq!(answered.len(), 1, "{events:?}");
    assert_eq!(answered[0].level, Level::INFO);
    assert_eq!(answered[0].field("answered"), Some("true"));
}

/// メニューで返事が無ければ標準へ進み、標準が始まった知らせは照会より前に 1 回（要件 1.12・9.6）。
#[test]
fn process_exec_without_reply_goes_standard_and_notifies_before_query() {
    let ports = FakePorts::new([script_none()]);
    let order = order(
        vec![spec(TargetKind::Ghost, "emo2", Some(GHOST_URL))],
        UpdateReason::Manual,
        SummaryKind::Result,
    );
    run_order(&order, &ports);

    let calls = ports.calls();
    assert_eq!(calls[0], Call::Raise("OnUpdateProcessExec", s(&["manual"])));
    assert_eq!(calls[1], Call::Started);
    assert_eq!(calls[2], Call::Resources);
    assert_eq!(calls.iter().filter(|c| **c == Call::Started).count(), 1);
    assert_eq!(
        ports.raised_ids(),
        [
            "OnUpdateProcessExec",
            "OnUpdateBegin",
            "OnUpdateComplete",
            "OnUpdateResult"
        ]
    );
}

/// 台本の入口では `OnUpdateProcessExec` を送らず、標準が始まった知らせは照会より前に 1 回（要件 1.13）。
#[test]
fn script_entry_never_sends_process_exec() {
    let ports = FakePorts::new([script_none()]);
    let order = order(
        vec![spec(TargetKind::Ghost, "emo2", Some(GHOST_URL))],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    run_order(&order, &ports);

    let calls = ports.calls();
    assert_eq!(calls[0], Call::Started);
    assert_eq!(calls[1], Call::Resources);
    assert_eq!(calls.iter().filter(|c| **c == Call::Started).count(), 1);
    assert!(!ports.raised_ids().contains(&"OnUpdateProcessExec"));
}

/// 更新先の 3 通り: ゴーストは照会の値が勝ち、無ければ `descript.txt`。シェルは照会を見ない（要件 1.9）。
#[test]
fn homeurl_resource_wins_for_ghost_then_descript_and_shell_uses_descript() {
    let cases: [(TargetKind, Option<&str>, &str); 3] = [
        (
            TargetKind::Ghost,
            Some("https://shiori.invalid/"),
            "https://shiori.invalid/",
        ),
        (TargetKind::Ghost, None, GHOST_URL),
        (
            TargetKind::Shell,
            Some("https://shiori.invalid/"),
            GHOST_URL,
        ),
    ];
    for (kind, resource, expected) in cases {
        let mut ports = FakePorts::new([script_none()]);
        ports.resources = Some(GhostResources {
            homeurl: resource.map(str::to_owned),
            useorigin1: None,
        });
        let target = spec(kind, "emo2", Some(GHOST_URL));
        let order = order(
            vec![target.clone()],
            UpdateReason::Script,
            SummaryKind::Result,
        );
        run_order(&order, &ports);
        let engine: Vec<Call> = ports
            .calls()
            .into_iter()
            .filter(|c| matches!(c, Call::RunEngine { .. }))
            .collect();
        assert_eq!(
            engine,
            vec![Call::RunEngine {
                homeurl: expected.to_owned(),
                target: target.dir.clone(),
            }],
            "{kind:?} {resource:?}"
        );
    }
}

/// 更新先の無い対象は飛ばし（イベント 0・総括に載せない・`warn!` 1 件）、全部飛ばせば総括 0（要件 1.10・3.5）。
#[test]
fn targets_without_homeurl_are_skipped_and_all_skipped_sends_no_summary() {
    // 一部だけ飛ばす: シェルに更新先が無い。
    let ports = FakePorts::new([script_none()]);
    let order_mixed = order(
        vec![
            spec(TargetKind::Ghost, "emo2", Some(GHOST_URL)),
            spec(TargetKind::Shell, "master", None),
        ],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    let (end, events) = run(&order_mixed, &ports);
    assert_eq!(
        ports.raised_ids(),
        ["OnUpdateBegin", "OnUpdateComplete", "OnUpdateResult"]
    );
    assert_eq!(ports.raised().last().unwrap().1, s(&["ghost\u{1}OK\u{1}0"]));
    assert_eq!(end.ends, vec![TargetEnd::Unchanged, TargetEnd::Skipped]);
    let skipped = with_event(&events, "update_target_skipped");
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, Level::WARN);

    // 全部飛ばす: 照会にも descript.txt にも無い。
    let ports = FakePorts::new([]);
    let order_none = order(
        vec![
            spec(TargetKind::Ghost, "emo2", None),
            spec(TargetKind::Balloon, "kaku", None),
        ],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    let (end, events) = run(&order_none, &ports);
    assert!(ports.raised().is_empty(), "{:?}", ports.raised());
    assert!(
        !ports
            .calls()
            .iter()
            .any(|c| matches!(c, Call::RunEngine { .. }))
    );
    assert_eq!(end.ends, vec![TargetEnd::Skipped, TargetEnd::Skipped]);
    assert!(!end.summarised);
    assert!(!end.reload_requested);
    assert_eq!(with_event(&events, "update_target_skipped").len(), 2);
}

/// 取得口を作れなければ `OnUpdateBegin` → `OnUpdateFailure(connect)` と `error!` 1 件（要件 4.4）。
#[test]
fn unavailable_fetch_reports_connect_with_one_error() {
    let ports = FakePorts::new([EngineScript {
        progress: Vec::new(),
        result: EngineRun::Unavailable(FetchError::Other { code: 12007 }),
    }]);
    let target = spec(TargetKind::Ghost, "emo2", Some(GHOST_URL));
    let order = order(
        vec![target.clone()],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    let (end, events) = run(&order, &ports);

    assert_eq!(
        ports.raised(),
        vec![
            (
                "OnUpdateBegin",
                five(["emo2", &absolute(&target.dir), ""], "ghost", "script")
            ),
            (
                "OnUpdateFailure",
                five(["connect", "", ""], "ghost", "script")
            ),
            ("OnUpdateResult", s(&["ghost\u{1}NG\u{1}connect"])),
        ]
    );
    assert_eq!(
        end.ends,
        vec![TargetEnd::Failed {
            word: "connect".to_owned(),
            file: None,
        }]
    );
    let errors: Vec<_> = events.iter().filter(|e| e.level == Level::ERROR).collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    assert_eq!(
        errors[0].field_str("event"),
        Some("update_fetch_unavailable")
    );
}

/// 対象が複数なら 1 つ終えてから次・照会は 1 回・`useorigin1` の読みは全対象で同じ・総括は最後（要件 2.1・2.14・3.4）。
#[test]
fn multiple_targets_run_one_by_one_with_one_query_and_summary_last() {
    let mut ports = FakePorts::new([script_changed(), script_none()]);
    ports.resources = Some(GhostResources {
        homeurl: None,
        useorigin1: Some("1".to_owned()),
    });
    let order = order(
        vec![
            spec(TargetKind::Ghost, "emo2", Some(GHOST_URL)),
            spec(TargetKind::Balloon, "kaku", Some(SHELL_URL)),
        ],
        UpdateReason::Script,
        SummaryKind::ResultEx,
    );
    let (end, _) = run(&order, &ports);

    assert_eq!(
        ports
            .calls()
            .iter()
            .filter(|c| **c == Call::Resources)
            .count(),
        1
    );
    assert_eq!(
        ports.raised_ids(),
        [
            "OnUpdateBegin",
            "OnUpdateReady",
            "OnUpdate.OnDownloadBegin",
            "OnUpdate.OnMD5CompareBegin",
            "OnUpdate.OnMD5CompareComplete",
            "OnUpdate.OnDownloadBegin",
            "OnUpdate.OnMD5CompareBegin",
            "OnUpdate.OnMD5CompareComplete",
            "OnUpdateComplete",
            "OnUpdateOtherBegin",
            "OnUpdateOtherComplete",
            "OnUpdateResultEx",
        ]
    );
    let raised = ports.raised();
    // 1 始まり: 件数そのもの・番号は 1 から。
    assert_eq!(
        raised[1].1,
        five(["2", "a.txt,b.txt", ""], "ghost", "script")
    );
    assert_eq!(raised[2].1, five(["a.txt", "1", "2"], "ghost", "script"));
    assert_eq!(
        raised.last().unwrap().1,
        s(&[
            "emo2\u{1}ghost\u{1}OK\u{1}2",
            "kaku\u{1}balloon\u{1}OK\u{1}0"
        ])
    );
    assert_eq!(end.ends, vec![TargetEnd::Changed(2), TargetEnd::Unchanged]);
    // 2 周目のエンジンは 1 周目の締めの後。
    let calls = ports.calls();
    let complete = calls
        .iter()
        .position(|c| matches!(c, Call::Raise("OnUpdateComplete", _)))
        .unwrap();
    let second_engine = calls
        .iter()
        .rposition(|c| matches!(c, Call::RunEngine { .. }))
        .unwrap();
    assert!(complete < second_engine, "{calls:?}");
}

/// 照会に答えが無ければ（kanade が居ない）イベント 0 件でやめる。
#[test]
fn missing_resources_abandons_before_any_target() {
    let mut ports = FakePorts::new([]);
    ports.resources = None;
    let order = order(
        vec![spec(TargetKind::Ghost, "emo2", Some(GHOST_URL))],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    let (end, events) = run(&order, &ports);
    assert!(ports.raised().is_empty());
    assert_eq!(end, OrderEnd::default());
    let abandoned = with_event(&events, "update_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(abandoned[0].field_str("at"), Some("resources"));
}

/// 成功したが消せなかった物・作業場所が残れば、成功として扱い `warn!` に場所を残す（要件 4.8）。
#[test]
fn leftovers_after_success_are_warned_and_still_changed() {
    let target = spec(TargetKind::Ghost, "emo2", Some(GHOST_URL));
    let undeletable = target.dir.join("old.dll");
    let leftover = target.dir.join(".update-work").join("x");
    let ports = FakePorts::new([script_updated_with_leftovers(&undeletable, &leftover)]);
    let order = order(
        vec![target.clone()],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    let (end, events) = run(&order, &ports);

    let raised = ports.raised();
    let closings: Vec<_> = raised
        .iter()
        .filter(|(id, _)| *id == "OnUpdateComplete" || *id == "OnUpdateFailure")
        .collect();
    assert_eq!(closings.len(), 1, "{raised:?}");
    assert_eq!(closings[0].0, "OnUpdateComplete");
    assert_eq!(closings[0].1[..2], s(&["changed", "a.txt"])[..]);
    assert_eq!(end.ends, vec![TargetEnd::Changed(1)]);
    assert_eq!(
        raised.last().unwrap(),
        &("OnUpdateResult", s(&["ghost\u{1}OK\u{1}1"]))
    );

    let left = with_event(&events, "update_leftover");
    assert_eq!(left.len(), 2, "{events:?}");
    assert!(left.iter().all(|e| e.level == Level::WARN));
    let paths: Vec<&str> = left.iter().map(|e| e.field("path").unwrap()).collect();
    assert_eq!(
        paths,
        [
            undeletable.display().to_string(),
            leftover.display().to_string()
        ]
    );
    assert_eq!(events.iter().filter(|e| e.level == Level::ERROR).count(), 0);
}

/// 戻せなかった失敗は `fileio`・`error!` 1 件に戻れなかったことと作業場所を載せる（要件 4.5・4.6）。
#[test]
fn rollback_failure_reports_fileio_and_logs_work_area() {
    let target = spec(TargetKind::Ghost, "emo2", Some(GHOST_URL));
    let work = target.dir.join(".update-work").join("run1");
    let ports = FakePorts::new([script_rollback_failed(&target.dir, &work)]);
    let order = order(
        vec![target.clone()],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    let (end, events) = run(&order, &ports);

    let failure: Vec<_> = ports
        .raised()
        .into_iter()
        .filter(|(id, _)| *id == "OnUpdateFailure")
        .collect();
    assert_eq!(failure.len(), 1);
    assert_eq!(failure[0].1[..2], s(&["fileio", "a.txt"])[..]);
    assert_eq!(
        end.ends,
        vec![TargetEnd::Failed {
            word: "fileio".to_owned(),
            file: Some("a.txt".to_owned()),
        }]
    );

    let errors: Vec<_> = events.iter().filter(|e| e.level == Level::ERROR).collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    let failed = errors[0];
    assert_eq!(failed.field_str("event"), Some("update_failed"));
    assert_eq!(failed.field_str("kind"), Some("RollbackFailed"));
    assert_eq!(failed.field("rolled_back"), Some("false"));
    let work_text = work.display().to_string();
    assert_eq!(failed.field_str("work"), Some(work_text.as_str()));
    let target_text = target.dir.display().to_string();
    assert_eq!(failed.field("target"), Some(target_text.as_str()));
}
