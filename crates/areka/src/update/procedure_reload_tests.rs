//! 読み直しの頼みの時機と、途中で閉じたときの捨て方の兄弟テスト（design「Testing Strategy /
//! 手続き」の読み直し・要件 5.2・5.4・5.7・7.4・9.9・裁定 13・16）。
//!
//! 偽の口の `raise` に「閉じた」を返させ、送ったイベント・読み直しの頼み・記録を判定する。

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
        .filter(|c| matches!(c, Call::Reload(_)))
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

/// `changed` が 1 つでもあれば、総括の応えを受けた後に読み直しを 1 回だけ頼む（要件 5.2・5.3・9.9）。
/// `changed` はバルーン（2 つ目）でも同じ＝対象の種別を問わない。
#[test]
fn reload_is_requested_once_after_the_summary_when_any_target_changed() {
    for summary in [SummaryKind::Result, SummaryKind::ResultEx] {
        let ports = FakePorts::new([script_none(), script_changed()]);
        let order = order(vec![ghost(), balloon()], UpdateReason::Script, summary);
        let (end, events) = run(&order, &ports);

        let calls = ports.calls();
        let id = match summary {
            SummaryKind::Result => "OnUpdateResult",
            SummaryKind::ResultEx => "OnUpdateResultEx",
        };
        let summary_at = calls
            .iter()
            .position(|c| matches!(c, Call::Raise(i, _) if *i == id))
            .expect("総括を送った");
        assert_eq!(reloads(&ports), 1, "{calls:?}");
        // 読み直しの頼みは総括の後の最後の 1 件。
        assert_eq!(
            calls.last(),
            Some(&Call::Reload(order.ghost_dir.clone())),
            "{calls:?}"
        );
        assert_eq!(summary_at, calls.len() - 2, "{calls:?}");
        assert_eq!(end.ends, vec![TargetEnd::Unchanged, TargetEnd::Changed(2)]);
        assert!(end.summarised);
        assert!(end.reload_requested);
        let asked = with_event(&events, "update_reload_requested");
        assert_eq!(asked.len(), 1, "{events:?}");
        assert_eq!(asked[0].level, Level::INFO);
    }
}

/// 全部 `none`・失敗・その混ざりなら、総括は送るが読み直しは 0 回（要件 5.4・9.9）。
#[test]
fn no_reload_when_every_target_is_none_or_failed() {
    let g = ghost();
    let unavailable = || EngineScript {
        progress: Vec::new(),
        result: EngineRun::Unavailable(FetchError::Other { code: 12007 }),
    };
    let cases: [(&str, Vec<TargetSpec>, Vec<EngineScript>); 5] = [
        ("none", vec![g.clone()], vec![script_none()]),
        ("fetch", vec![g.clone()], vec![script_fetch_failed(&g.dir)]),
        ("md5", vec![g.clone()], vec![script_md5_mismatch(&g.dir)]),
        ("connect", vec![g.clone()], vec![unavailable()]),
        (
            "none+md5",
            vec![balloon(), g.clone()],
            vec![script_none(), script_md5_mismatch(&g.dir)],
        ),
    ];
    for (label, targets, scripts) in cases {
        let ports = FakePorts::new(scripts);
        let order = order(targets, UpdateReason::Manual, SummaryKind::Result);
        let (end, events) = run(&order, &ports);

        assert_eq!(reloads(&ports), 0, "{label}: {:?}", ports.calls());
        assert_eq!(
            ports.raised_ids().last(),
            Some(&"OnUpdateResult"),
            "{label}"
        );
        assert!(end.summarised, "{label}");
        assert!(!end.reload_requested, "{label}");
        assert!(
            with_event(&events, "update_reload_requested").is_empty(),
            "{label}"
        );
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

/// 始まり・締めで「閉じた」が返っても、残りの対象・総括・読み直しは 0 件（要件 5.7・7.4）。
/// 始まりで閉じればエンジンは回さない。
#[test]
fn closed_at_begin_or_closing_sends_nothing_further() {
    let cases: [(&str, &[&str], usize); 2] = [
        ("OnUpdateBegin", &["OnUpdateBegin"], 0),
        (
            "OnUpdateComplete",
            &[
                "OnUpdateBegin",
                "OnUpdateReady",
                "OnUpdate.OnDownloadBegin",
                "OnUpdate.OnMD5CompareBegin",
                "OnUpdate.OnMD5CompareComplete",
                "OnUpdate.OnDownloadBegin",
                "OnUpdate.OnMD5CompareBegin",
                "OnUpdate.OnMD5CompareComplete",
                "OnUpdateComplete",
            ],
            1,
        ),
    ];
    for (closed_at, expected, runs) in cases {
        let mut ports = FakePorts::new([script_changed(), script_none()]);
        ports.replies = HashMap::from([(closed_at, Raised::Closed)]);
        let order = order(
            vec![ghost(), balloon()],
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

/// 総括が「閉じた」なら、`changed` が在っても読み直さない（要件 5.7・7.4・裁定 13）。
#[test]
fn closed_summary_means_no_reload() {
    let mut ports = FakePorts::new([script_changed()]);
    ports.replies = HashMap::from([("OnUpdateResult", Raised::Closed)]);
    let order = order(vec![ghost()], UpdateReason::Script, SummaryKind::Result);
    let (end, events) = run(&order, &ports);

    assert_eq!(ports.raised_ids().last(), Some(&"OnUpdateResult"));
    assert_eq!(reloads(&ports), 0, "{:?}", ports.calls());
    assert_eq!(end.ends, vec![TargetEnd::Changed(2)]);
    assert!(!end.summarised);
    assert!(!end.reload_requested);
    let abandoned = with_event(&events, "update_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(abandoned[0].level, Level::WARN);
    assert_eq!(abandoned[0].field_str("at"), Some("summary"));
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
