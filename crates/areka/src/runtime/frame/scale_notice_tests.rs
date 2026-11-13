//! 計画の檻（純粋・shell-scale-linkage 要件 5.1〜5.4・5.6〜5.9・7.3）。

use super::*;
use areka_emo_present::{ScaleRatio, TargetId};

fn scale(num: u32, den: u32, dpi: u16) -> AppliedScale {
    AppliedScale {
        ratio: ScaleRatio::new(num, den).unwrap(),
        dpi_x: Some(dpi),
    }
}

fn change(target: u32, before: Option<AppliedScale>, after: AppliedScale) -> ScaleChange {
    ScaleChange {
        target: TargetId(target),
        before,
        after,
    }
}

fn refs(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn first_determined_record_writes_only() {
    let after = scale(3, 2, 144);
    let plan = plan_scale_notices(&[change(0, None, after)]);
    assert_eq!(plan.writes, vec![(0, TargetWindow::Shell, after)]);
    assert!(plan.events.is_empty());
}

#[test]
fn body_scale_change_raises_on_shell_scaling() {
    let after = scale(2, 1, 192);
    let plan = plan_scale_notices(&[change(0, Some(scale(3, 2, 144)), after)]);
    assert_eq!(plan.writes, vec![(0, TargetWindow::Shell, after)]);
    assert_eq!(
        plan.events,
        vec![ScaleEvent {
            id: "OnShellScaling",
            scope: 0,
            references: refs(&["200", "150", "200", "150", "0"]),
        }]
    );
}

#[test]
fn scope_one_only() {
    let after = scale(2, 1, 192);
    let plan = plan_scale_notices(&[change(2, Some(scale(1, 1, 96)), after)]);
    assert_eq!(plan.writes, vec![(1, TargetWindow::Shell, after)]);
    assert_eq!(plan.events.len(), 1);
    assert_eq!(plan.events[0].id, "OnShellScaling");
    assert_eq!(plan.events[0].scope, 1);
}

#[test]
fn balloon_only_raises_on_balloon_scaling() {
    let after = scale(3, 2, 144);
    let plan = plan_scale_notices(&[change(1, Some(scale(1, 1, 96)), after)]);
    assert_eq!(plan.writes, vec![(0, TargetWindow::Balloon, after)]);
    assert_eq!(
        plan.events,
        vec![ScaleEvent {
            id: "OnBalloonScaling",
            scope: 0,
            references: refs(&["150", "100", "150", "100", "0"]),
        }]
    );
}

#[test]
fn same_percent_with_only_dpi_differing_writes_only() {
    let after = scale(1, 1, 192);
    let plan = plan_scale_notices(&[change(0, Some(scale(1, 1, 96)), after)]);
    assert_eq!(plan.writes, vec![(0, TargetWindow::Shell, after)]);
    assert!(plan.events.is_empty());
}

#[test]
fn two_changes_in_one_frame_fold_to_one() {
    let last = scale(2, 1, 192);
    let plan = plan_scale_notices(&[
        change(0, Some(scale(1, 1, 96)), scale(3, 2, 144)),
        change(0, Some(scale(3, 2, 144)), last),
    ]);
    assert_eq!(plan.writes, vec![(0, TargetWindow::Shell, last)]);
    assert_eq!(plan.events.len(), 1);
    assert_eq!(
        plan.events[0].references,
        refs(&["200", "100", "200", "100", "0"])
    );
}

#[test]
fn returned_to_original_writes_only() {
    let original = scale(1, 1, 96);
    let plan = plan_scale_notices(&[
        change(0, Some(original), scale(3, 2, 144)),
        change(0, Some(scale(3, 2, 144)), original),
    ]);
    assert_eq!(plan.writes, vec![(0, TargetWindow::Shell, original)]);
    assert!(plan.events.is_empty());
}

#[test]
fn multiple_windows_follow_target_order() {
    let before = Some(scale(1, 1, 96));
    let after = scale(2, 1, 192);
    // 積まれた順はばらばら・計画は TargetId の昇順（スコープの若い順・シェル → バルーン）。
    let plan = plan_scale_notices(&[
        change(3, before, after),
        change(1, before, after),
        change(2, before, after),
        change(0, before, after),
    ]);
    let writes: Vec<_> = plan.writes.iter().map(|(s, w, _)| (*s, *w)).collect();
    assert_eq!(
        writes,
        vec![
            (0, TargetWindow::Shell),
            (0, TargetWindow::Balloon),
            (1, TargetWindow::Shell),
            (1, TargetWindow::Balloon),
        ]
    );
    let events: Vec<_> = plan.events.iter().map(|e| (e.id, e.scope)).collect();
    assert_eq!(
        events,
        vec![
            ("OnShellScaling", 0),
            ("OnBalloonScaling", 0),
            ("OnShellScaling", 1),
            ("OnBalloonScaling", 1),
        ]
    );
}

#[test]
fn five_references_are_after_before_after_before_scope() {
    // 120/144 → 83.33・135/120 → 112.5（前後が別の数で並びの取り違えを見分ける）。
    let plan = plan_scale_notices(&[change(5, Some(scale(120, 144, 96)), scale(135, 120, 144))]);
    assert_eq!(
        plan.events,
        vec![ScaleEvent {
            id: "OnBalloonScaling",
            scope: 2,
            references: refs(&["112.5", "83.33", "112.5", "83.33", "2"]),
        }]
    );
}
