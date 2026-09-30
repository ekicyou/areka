//! 後送りの列と読み直しの頼みの時機、途中で閉じたときの捨て方の兄弟テスト（design「Testing Strategy /
//! 手続き」の後送りの列と読み直し・決めたこと 21・要件 2.16・3.1・3.4・5.2・5.4・5.7・7.4・9.9）。
//!
//! 偽の口の `raise` に「閉じた」を返させ、送ったイベント・読み直しの頼みに添えた列・記録を判定する。

use std::collections::HashMap;

use areka_update::FetchError;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::procedure_test_support::{
    Call, EngineScript, FakePorts, GHOST_URL, SHELL_URL, order, script_changed,
    script_fetch_failed, script_md5_mismatch, script_none, spec,
};
use super::{EngineRun, OrderEnd, Raised, run_order};
use crate::update::refs::TargetEnd;
use crate::update::{SummaryKind, TargetKind, TargetSpec, UpdateOrder, UpdateReason};

fn with_event<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn run(order: &UpdateOrder, ports: &FakePorts) -> (OrderEnd, Vec<CapturedEvent>) {
    capture(|| run_order(order, ports))
}

fn reloads(ports: &FakePorts) -> usize {
    ports
        .calls()
        .iter()
        .filter(|c| matches!(c, Call::Reload(..)))
        .count()
}

fn engine_runs(ports: &FakePorts) -> usize {
    ports
        .calls()
        .iter()
        .filter(|c| matches!(c, Call::RunEngine { .. }))
        .count()
}

fn ghost() -> TargetSpec {
    spec(TargetKind::Ghost, "emo2", Some(GHOST_URL))
}

fn balloon() -> TargetSpec {
    spec(TargetKind::Balloon, "kaku", Some(SHELL_URL))
}

fn ids(events: &[(&'static str, Vec<String>)]) -> Vec<&'static str> {
    events.iter().map(|(id, _)| *id).collect()
}

/// `changed` が 1 つでもあれば、全対象の後に読み直しを後送りの列つきで 1 回だけ頼み（最後の呼び出し）、
/// 列は `raise` しない。列はゴーストの成功の締め（`none` でも）→ 総括。`changed` はバルーン（2 つ目）
/// でも同じ＝対象の種別を問わない。`debug!(update_tail_deferred)` に列の件数（要件 2.16・3.1・5.2・9.9）。
#[test]
fn reload_is_requested_once_after_all_targets_with_the_tail() {
    for summary in [SummaryKind::Result, SummaryKind::ResultEx] {
        let ports = FakePorts::new([script_none(), script_changed()]);
        let order = order(vec![ghost(), balloon()], UpdateReason::Script, summary);
        let (end, events) = run(&order, &ports);

        let calls = ports.calls();
        let id = match summary {
            SummaryKind::Result => "OnUpdateResult",
            SummaryKind::ResultEx => "OnUpdateResultEx",
        };
        assert_eq!(reloads(&ports), 1, "{calls:?}");
        let Some(Call::Reload(dir, tail)) = calls.last() else {
            panic!("読み直しの頼みが最後: {calls:?}");
        };
        assert_eq!(dir, &order.ghost_dir);
        assert_eq!(ids(tail), ["OnUpdateComplete", id], "{calls:?}");
        assert_eq!(tail[0].1[0], "none", "ゴーストの none の締めも列へ");
        // 列の中身は raise しない（締めはバルーンの分だけ・総括 0）。
        let raised = ports.raised_ids();
        assert!(!raised.contains(&"OnUpdateComplete"), "{raised:?}");
        assert!(!raised.contains(&id), "{raised:?}");
        assert_eq!(raised.last(), Some(&"OnUpdateOtherComplete"));
        assert_eq!(end.ends, vec![TargetEnd::Unchanged, TargetEnd::Changed(2)]);
        assert!(end.summarised);
        assert!(end.reload_requested);
        let asked = with_event(&events, "update_reload_requested");
        assert_eq!(asked.len(), 1, "{events:?}");
        assert_eq!(asked[0].level, Level::INFO);
        let deferred = with_event(&events, "update_tail_deferred");
        assert_eq!(deferred.len(), 1, "{events:?}");
        assert_eq!(deferred[0].level, Level::DEBUG);
        assert_eq!(deferred[0].field("count"), Some("2"));
        assert!(with_event(&events, "update_tail_sent").is_empty());
    }
}

/// ゴーストが失敗した・対象でないときは、列の先頭は総括（要件 5.5 の「ゴーストが成功していないとき」）。
#[test]
fn tail_head_is_the_summary_when_the_ghost_did_not_succeed() {
    let g = ghost();
    let cases: [(&str, Vec<TargetSpec>, Vec<EngineScript>); 2] = [
        (
            "ghost failed",
            vec![g.clone(), balloon()],
            vec![script_md5_mismatch(&g.dir), script_changed()],
        ),
        ("no ghost", vec![balloon()], vec![script_changed()]),
    ];
    for (label, targets, scripts) in cases {
        let ports = FakePorts::new(scripts);
        let order = order(targets, UpdateReason::Script, SummaryKind::Result);
        let (end, _) = run(&order, &ports);

        let tails = ports.reload_tails();
        assert_eq!(tails.len(), 1, "{label}: {:?}", ports.calls());
        assert_eq!(ids(&tails[0]), ["OnUpdateResult"], "{label}");
        assert!(end.reload_requested, "{label}");
    }
}

/// 全部 `none`・失敗・その混ざりなら読み直しは 0 回で、列を最後に `raise` で順に送る
/// （`info!(update_tail_sent when=at_end)`・要件 3.1・5.2・5.4・9.9）。
#[test]
fn no_reload_sends_the_tail_at_the_end_when_nothing_changed() {
    let g = ghost();
    let unavailable = || EngineScript {
        progress: Vec::new(),
        result: EngineRun::Unavailable(FetchError::Other { code: 12007 }),
    };
    let cases: [(&str, Vec<TargetSpec>, Vec<EngineScript>, &[&str]); 5] = [
        (
            "none",
            vec![g.clone()],
            vec![script_none()],
            &["OnUpdateComplete", "OnUpdateResult"],
        ),
        (
            "fetch",
            vec![g.clone()],
            vec![script_fetch_failed(&g.dir)],
            &["OnUpdateFailure", "OnUpdateResult"],
        ),
        (
            "md5",
            vec![g.clone()],
            vec![script_md5_mismatch(&g.dir)],
            &["OnUpdateFailure", "OnUpdateResult"],
        ),
        (
            "connect",
            vec![g.clone()],
            vec![unavailable()],
            &["OnUpdateFailure", "OnUpdateResult"],
        ),
        (
            "none+md5",
            vec![balloon(), g.clone()],
            vec![script_none(), script_md5_mismatch(&g.dir)],
            &["OnUpdateFailure", "OnUpdateResult"],
        ),
    ];
    for (label, targets, scripts, last) in cases {
        let ports = FakePorts::new(scripts);
        let order = order(targets, UpdateReason::Manual, SummaryKind::Result);
        let (end, events) = run(&order, &ports);

        assert_eq!(reloads(&ports), 0, "{label}: {:?}", ports.calls());
        let raised = ports.raised_ids();
        assert_eq!(&raised[raised.len() - 2..], last, "{label}");
        assert!(end.summarised, "{label}");
        assert!(!end.reload_requested, "{label}");
        assert!(
            with_event(&events, "update_reload_requested").is_empty(),
            "{label}"
        );
        let sent = with_event(&events, "update_tail_sent");
        assert_eq!(sent.len(), 1, "{label}: {events:?}");
        assert_eq!(sent[0].level, Level::INFO);
        assert_eq!(sent[0].field_str("when"), Some("at_end"), "{label}");
        let count = if label == "none" { "2" } else { "1" };
        assert_eq!(sent[0].field("count"), Some(count), "{label}");
    }
}

/// 進捗の途中で「閉じた」が返れば、エンジンの一周は最後まで回し（観測は全部流れる）、以後の進捗は
/// 送らず 1 件ごとに `warn!`、その対象の締め・残りの対象・総括・読み直しは 0 件（要件 5.7・7.4・9.9・裁定 16）。
#[test]
fn closed_mid_progress_drops_the_rest_but_runs_the_engine_to_the_end() {
    let mut ports = FakePorts::new([script_changed(), script_none()]);
    ports.replies = HashMap::from([("OnUpdate.OnDownloadBegin", Raised::Closed)]);
    let order = order(
        vec![ghost(), balloon()],
        UpdateReason::Script,
        SummaryKind::Result,
    );
    let (end, events) = run(&order, &ports);

    // 閉じたと返した 1 件までは送り、以後 0 件（締め・2 つ目の対象・総括を含む）。
    assert_eq!(
        ports.raised_ids(),
        ["OnUpdateBegin", "OnUpdateReady", "OnUpdate.OnDownloadBegin"]
    );
    assert_eq!(engine_runs(&ports), 1, "2 つ目の対象のエンジンは回さない");
    assert_eq!(reloads(&ports), 0);
    assert_eq!(
        end,
        OrderEnd {
            ends: vec![TargetEnd::Abandoned],
            summarised: false,
            reload_requested: false,
        }
    );

    // 観測は台本の 8 件が全部流れた＝エンジンの一周は最後まで。
    assert_eq!(
        with_event(&events, "update_progress").len(),
        8,
        "{events:?}"
    );
    // 捨てた進捗は 1 件ごとに `warn!`（a の照合 2・b の取得 1・b の照合 2）。
    let dropped = with_event(&events, "update_event_dropped");
    assert!(
        dropped.iter().all(|e| e.level == Level::WARN),
        "{dropped:?}"
    );
    let ids: Vec<&str> = dropped.iter().map(|e| e.field_str("id").unwrap()).collect();
    assert_eq!(
        ids,
        [
            "OnUpdate.OnMD5CompareBegin",
            "OnUpdate.OnMD5CompareComplete",
            "OnUpdate.OnDownloadBegin",
            "OnUpdate.OnMD5CompareBegin",
            "OnUpdate.OnMD5CompareComplete",
        ]
    );
    let abandoned = with_event(&events, "update_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(abandoned[0].field_str("at"), Some("target"));
}

/// 始まり・締め（その場で送るシェル・バルーンの締め）で「閉じた」が返っても、残りの対象・列・読み直しは
/// 0 件（要件 5.7・7.4）。始まりで閉じればエンジンは回さない。
#[test]
fn closed_at_begin_or_closing_sends_nothing_further() {
    let cases: [(&str, &[&str], usize); 2] = [
        ("OnUpdateOtherBegin", &["OnUpdateOtherBegin"], 0),
        (
            "OnUpdateOtherComplete",
            &[
                "OnUpdateOtherBegin",
                "OnUpdateOtherReady",
                "OnUpdateOther.OnDownloadBegin",
                "OnUpdateOther.OnMD5CompareBegin",
                "OnUpdateOther.OnMD5CompareComplete",
                "OnUpdateOther.OnDownloadBegin",
                "OnUpdateOther.OnMD5CompareBegin",
                "OnUpdateOther.OnMD5CompareComplete",
                "OnUpdateOtherComplete",
            ],
            1,
        ),
    ];
    for (closed_at, expected, runs) in cases {
        let mut ports = FakePorts::new([script_changed(), script_none()]);
        ports.replies = HashMap::from([(closed_at, Raised::Closed)]);
        let order = order(
            vec![balloon(), ghost()],
            UpdateReason::Script,
            SummaryKind::Result,
        );
        let (end, events) = run(&order, &ports);

        assert_eq!(ports.raised_ids(), expected, "{closed_at}");
        assert_eq!(engine_runs(&ports), runs, "{closed_at}");
        assert_eq!(reloads(&ports), 0, "{closed_at}");
        assert_eq!(end.ends, vec![TargetEnd::Abandoned], "{closed_at}");
        assert!(!end.summarised && !end.reload_requested, "{closed_at}");
        assert!(
            with_event(&events, "update_event_dropped").is_empty(),
            "{closed_at}"
        );
    }
}

/// 最後に送る列の途中で「閉じた」が返れば残りを捨てる（総括 0・読み直し 0・`warn!(update_abandoned
/// at=tail)` 1 件・`update_tail_sent` 0 件・要件 5.7・7.4）。
#[test]
fn closed_in_the_tail_drops_the_rest() {
    let mut ports = FakePorts::new([script_none()]);
    ports.replies = HashMap::from([("OnUpdateComplete", Raised::Closed)]);
    let order = order(vec![ghost()], UpdateReason::Script, SummaryKind::Result);
    let (end, events) = run(&order, &ports);

    assert_eq!(ports.raised_ids(), ["OnUpdateBegin", "OnUpdateComplete"]);
    assert_eq!(reloads(&ports), 0, "{:?}", ports.calls());
    assert_eq!(end.ends, vec![TargetEnd::Unchanged]);
    assert!(!end.summarised);
    assert!(!end.reload_requested);
    let abandoned = with_event(&events, "update_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(abandoned[0].level, Level::WARN);
    assert_eq!(abandoned[0].field_str("at"), Some("tail"));
    assert!(with_event(&events, "update_tail_sent").is_empty());
}

/// メニューで `OnUpdateProcessExec` が「閉じた」なら、標準が始まった知らせ・照会・イベント・読み直しは
/// 0 件で、`warn!(update_abandoned at=process_exec)` を 1 件（要件 7.4）。
#[test]
fn process_exec_closed_abandons_before_the_standard_procedure() {
    let mut ports = FakePorts::new([]);
    ports.replies = HashMap::from([("OnUpdateProcessExec", Raised::Closed)]);
    let order = order(vec![ghost()], UpdateReason::Manual, SummaryKind::Result);
    let (end, events) = run(&order, &ports);

    assert_eq!(
        ports.calls(),
        vec![Call::Raise(
            "OnUpdateProcessExec",
            vec!["manual".to_owned()]
        )]
    );
    assert_eq!(end, OrderEnd::default());
    let abandoned = with_event(&events, "update_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(abandoned[0].level, Level::WARN);
    assert_eq!(abandoned[0].field_str("at"), Some("process_exec"));
    assert!(with_event(&events, "update_process_exec").is_empty());
}
