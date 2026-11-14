//! render_surface: 続く失敗の記録の間引き（設計討議 議題 1・要件 6.6）

use super::render::{FailureLog, forget_failures_on_device_lost, note_failure};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};

fn entities() -> (Entity, Entity) {
    let mut world = World::new();
    (world.spawn_empty().id(), world.spawn_empty().id())
}

#[test]
fn same_stage_and_hresult_is_reported_once() {
    let (a, _) = entities();
    let mut log = FailureLog::default();
    assert!(
        note_failure(&mut log, a, "composite", E_FAIL),
        "1 回目は error! の番"
    );
    assert!(
        !note_failure(&mut log, a, "composite", E_FAIL),
        "2 回目は debug! の番"
    );
    assert!(
        !note_failure(&mut log, a, "composite", E_FAIL),
        "3 回目も debug! の番"
    );
}

#[test]
fn different_stage_or_hresult_is_reported_again() {
    let (a, _) = entities();
    let mut log = FailureLog::default();
    assert!(note_failure(&mut log, a, "composite", E_FAIL));
    assert!(
        note_failure(&mut log, a, "EndDraw", E_FAIL),
        "段が違えば error!"
    );
    assert!(
        note_failure(&mut log, a, "EndDraw", E_INVALIDARG),
        "HRESULT が違えば error!"
    );
    assert!(
        note_failure(&mut log, a, "EndDraw", E_FAIL),
        "別の失敗で控えが消えた後はまた error!"
    );
}

#[test]
fn success_clears_the_memo() {
    let (a, _) = entities();
    let mut log = FailureLog::default();
    assert!(note_failure(&mut log, a, "composite create", E_FAIL));
    log.remove(&a);
    assert!(
        note_failure(&mut log, a, "composite create", E_FAIL),
        "成功の後はまた error!"
    );
}

#[test]
fn memo_is_per_surface() {
    let (a, b) = entities();
    let mut log = FailureLog::default();
    assert!(note_failure(&mut log, a, "composite", E_FAIL));
    assert!(
        note_failure(&mut log, b, "composite", E_FAIL),
        "別の描画面は別の控え"
    );
    log.remove(&b);
    assert!(
        !note_failure(&mut log, a, "composite", E_FAIL),
        "別の描画面の成功は控えを消さない"
    );
}

#[test]
fn device_loss_clears_the_memo() {
    let (a, b) = entities();
    let mut log = FailureLog::default();
    assert!(note_failure(&mut log, a, "composite", E_FAIL));
    assert!(note_failure(&mut log, b, "EndDraw", E_FAIL));
    forget_failures_on_device_lost(&mut log);
    assert!(
        note_failure(&mut log, a, "composite", E_FAIL),
        "装置を作り直した後の最初の失敗はまた error!"
    );
    assert!(note_failure(&mut log, b, "EndDraw", E_FAIL));
}

// ========== 描き直しの量の記録（5.3・3.4・5.4・6.1〜6.4） ==========

use super::render::{DamageRecord, damage_record, log_damage};
use crate::ecs::graphics::damage::{ComposePlan, FullReason, PartialPlan, RegionShift};
use crate::ecs::graphics::visual::PxRect;
use crate::ecs::test_support::capture_under_filter;

fn px(left: i32, top: i32, right: i32, bottom: i32) -> PxRect {
    PxRect {
        left,
        top,
        right,
        bottom,
    }
}

fn partial_plan() -> PartialPlan {
    PartialPlan {
        rects: vec![px(0, 0, 4, 2), px(0, 8, 4, 10)],
        shifts: vec![RegionShift {
            src: px(0, 2, 4, 10),
            dst: (0, 0),
        }],
        items_redrawn: 3,
        area: 16,
        shift_area: 32,
        exposed_area: 8,
        present_rect: px(0, 0, 4, 10),
    }
}

/// 記録の 1 行を捕まえる（target `wintf::damage` の debug だけを開ける）。
fn line(rec: &DamageRecord) -> String {
    capture_under_filter("wintf::damage=debug", || {
        log_damage("Surface#1", rec, 12, 34)
    })
}

#[test]
fn full_record_has_reason_and_whole_surface() {
    let rec = damage_record(&ComposePlan::Full(FullReason::Scale), 5, (10, 20), None);
    assert_eq!(
        rec,
        DamageRecord {
            mode: "full",
            reason: Some("scale"),
            items_redrawn: 5,
            items: 5,
            area: 200,
            surface_area: 200,
            shift_area: 0,
            exposed_area: 0,
            present_area: 200,
        }
    );
    let log = line(&rec);
    for field in [
        "[render_surface] damage",
        "entity=Surface#1",
        "mode=\"full\"",
        "reason=\"scale\"",
        "items_redrawn=5",
        "items=5",
        "area=200",
        "surface_area=200",
        "shift_area=0",
        "exposed_area=0",
        "present_area=200",
        "compose_us=12",
        "present_us=34",
    ] {
        assert!(log.contains(field), "欄 {field} が無い: {log}");
    }
}

#[test]
fn every_full_reason_is_spelled_in_the_record() {
    for (reason, word) in [
        (FullReason::First, "first"),
        (FullReason::Recreated, "recreated"),
        (FullReason::RootList, "root_list"),
        (FullReason::Scale, "scale"),
        (FullReason::NoPrev, "no_prev"),
        (FullReason::Undecidable, "undecidable"),
        (FullReason::CoversSurface, "covers_surface"),
    ] {
        let log = line(&damage_record(&ComposePlan::Full(reason), 1, (4, 4), None));
        assert!(
            log.contains(&format!("reason=\"{word}\"")),
            "理由 {word} が無い: {log}"
        );
    }
}

#[test]
fn partial_record_splits_shift_and_exposed_area() {
    let plan = partial_plan();
    let rec = damage_record(
        &ComposePlan::Partial(plan.clone()),
        7,
        (4, 10),
        Some(plan.present_rect),
    );
    assert_eq!(
        rec,
        DamageRecord {
            mode: "partial",
            reason: None,
            items_redrawn: 3,
            items: 7,
            area: 16,
            surface_area: 40,
            shift_area: 32,
            exposed_area: 8,
            present_area: 40,
        }
    );
    let log = line(&rec);
    for field in [
        "mode=\"partial\"",
        "items_redrawn=3",
        "items=7",
        "area=16",
        "shift_area=32",
        "exposed_area=8",
    ] {
        assert!(log.contains(field), "欄 {field} が無い: {log}");
    }
    assert!(!log.contains("reason="), "範囲だけに理由は無い: {log}");
    // 前の写しを終えていない回は全面を写す（present_area は描画面の面積）。
    let whole = damage_record(&ComposePlan::Partial(plan), 7, (8, 10), None);
    assert_eq!(whole.present_area, 80);
}

#[test]
fn present_only_record_copies_without_redraw() {
    let rec = damage_record(&ComposePlan::PresentOnly, 4, (10, 10), None);
    assert_eq!(
        (
            rec.mode,
            rec.reason,
            rec.items_redrawn,
            rec.area,
            rec.present_area
        ),
        ("present", None, 0, 0, 100)
    );
    let log = line(&rec);
    assert!(log.contains("mode=\"present\""), "{log}");
    assert!(log.contains("present_area=100"), "{log}");
}

#[test]
fn skip_record_counts_an_undrawn_round() {
    let rec = damage_record(&ComposePlan::Skip, 4, (10, 10), None);
    assert_eq!(
        (rec.mode, rec.items_redrawn, rec.area, rec.present_area),
        ("skip", 0, 0, 0)
    );
    let log = line(&rec);
    assert!(log.contains("mode=\"skip\""), "{log}");
    assert!(log.contains("items=4"), "{log}");
}

#[test]
fn record_is_silent_unless_damage_debug_is_open() {
    let rec = damage_record(&ComposePlan::Skip, 0, (1, 1), None);
    let log = capture_under_filter("info", || log_damage("Surface#1", &rec, 0, 0));
    assert!(
        !log.contains("[render_surface] damage"),
        "info では出ない: {log}"
    );
}
