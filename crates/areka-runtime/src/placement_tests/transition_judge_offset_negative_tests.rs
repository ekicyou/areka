//! 追随レコードの判定器の負のテスト（audit の指摘 12・13・14・27）。
//!
//! 壊れた行・読めない欄が**合格に化けない**ことを固定する。どれも既知の合格ログ
//! [`pass_lines`] を 1 箇所ずつ壊して作る。

use std::fs;

use temp_path_kit::TempPath;

use super::transition_judge_offset_signoff_tests::signoff_offset_log;
use super::transition_judge_offset_tests::{
    T0_SCOPE0, T0_SCOPE1, T1_SCOPE0, T1_SCOPE2, T2_SCOPE0, T2_SCOPE1, T3_SCOPE0, T3_SCOPE1,
    T3_SCOPE2, offset, pass_lines,
};
use super::{OffsetViolation, judge_offset_log};
use crate::placement_tests::transition_judge::parse_transition_line;
use crate::placement_tests::transition_judge::test_support::log_of;
use areka_placement::transition_diag::{
    FIELD_BASE_DPI, FIELD_BASE_OFFSET, FIELD_NEW_OFFSET, KIND_OFFSET, OFFSET_VERDICT_ANCHORED,
    OFFSET_VERDICT_KEYWORD_PENDING, OFFSET_VERDICT_RESCALED, OFFSET_VERDICT_SATURATED,
    OFFSET_VERDICT_UNCHANGED, OFFSET_VERDICT_UNRESOLVED,
};
use wintf::ecs::window::transition_diag::{FIELD_KIND, FIELD_SCOPE};

/// 必須欄 `base_offset` の名を崩した行（語彙の規約を破る＝`is_well_formed()` が偽）。
fn without_base_offset(line: &str) -> String {
    let broken = line.replace(&format!(" {FIELD_BASE_OFFSET}="), " broken_base_offset=");
    assert_ne!(broken, line, "檻の前提: 実際に必須欄が落ちていること");
    assert!(
        !parse_transition_line(&broken)
            .expect("行としては解析できる")
            .is_well_formed(),
        "檻の前提: 語彙の規約を破っていること"
    );
    broken
}

/// 合格ログの `scope=0 base_dpi=96` の行（T0）の欄を 1 つ書き換えた行。
fn t0_scope0_with(from: &str, to: &str) -> Vec<String> {
    let mut lines = pass_lines();
    assert!(
        lines[T0_SCOPE0].contains(from),
        "檻の前提: {}",
        lines[T0_SCOPE0]
    );
    lines[T0_SCOPE0] = lines[T0_SCOPE0].replace(from, to);
    // 欠陥は立たない（字面は読める）ので、赤は判定器が欄を読む所からしか来ない。
    assert!(
        parse_transition_line(&lines[T0_SCOPE0])
            .expect("行としては解析できる")
            .is_well_formed(),
        "檻の前提: 語彙の規約は破っていないこと"
    );
    lines
}

fn malformed_in_t0(lines: &[String], field: &str) -> bool {
    judge_offset_log(&log_of(lines))
        .violations
        .iter()
        .any(|violation| {
            matches!(violation, OffsetViolation::MalformedRecord { transition: 0, detail }
                if detail.contains(field))
        })
}

#[test]
fn a_malformed_row_before_the_first_origin_is_red() {
    // 12 ⑴ 最初の起点より前の門の行が壊れている。どの遷移にも属さないが、捨てずに違反にする。
    let mut lines = pass_lines();
    let gate = without_base_offset(&lines[T0_SCOPE1]);
    lines.insert(0, gate);
    let report = judge_offset_log(&log_of(&lines));
    assert!(
        report.violations.iter().any(|violation| matches!(
            violation,
            OffsetViolation::UnassignedMalformedRecord { .. }
        )),
        "{report}"
    );
    assert!(report.failed(), "{report}");
}

#[test]
fn a_log_whose_every_row_is_malformed_carries_the_violations_in_the_error() {
    // 12 ⑵ 追随レコードが全部壊れたログ。走り手の `Err` の文に違反の一覧が載る。
    let kind_offset = format!("{FIELD_KIND}={KIND_OFFSET}");
    let mut lines: Vec<String> = pass_lines()
        .into_iter()
        .map(|line| {
            if line.contains(&kind_offset) {
                without_base_offset(&line)
            } else {
                line
            }
        })
        .collect();
    let before_origin = lines[T0_SCOPE1].clone();
    lines.insert(0, before_origin);
    let log = log_of(&lines);
    let report = judge_offset_log(&log);
    assert_eq!(report.rows, 0, "{report}");
    for wanted in [
        |v: &OffsetViolation| matches!(v, OffsetViolation::NoOffsetRecords),
        |v: &OffsetViolation| matches!(v, OffsetViolation::MalformedRecord { .. }),
        |v: &OffsetViolation| matches!(v, OffsetViolation::UnassignedMalformedRecord { .. }),
    ] {
        assert!(report.violations.iter().any(wanted), "{report}");
    }

    let temp = TempPath::new("offset-negative-all-malformed");
    let path = temp.child("all-malformed.log");
    fs::write(&path, &log).expect("一時ファイルを書けるはず");
    let error = signoff_offset_log(path.to_str()).expect_err("追随レコード 0 行は失敗");
    assert!(error.contains(&report.to_string()), "{error}");
}

#[test]
fn an_unreadable_base_dpi_is_a_malformed_record() {
    // 27 `base_dpi=19x` は無い（`-`）とも数とも読めない＝形の壊れた行。
    let lines = t0_scope0_with(
        &format!(" {FIELD_BASE_DPI}=96 "),
        &format!(" {FIELD_BASE_DPI}=19x "),
    );
    assert!(malformed_in_t0(&lines, FIELD_BASE_DPI));
}

#[test]
fn an_unreadable_scope_is_a_malformed_record() {
    // 27 `scope=x` は番兵の `-` でも数でもない＝形の壊れた行。
    let lines = t0_scope0_with(&format!(" {FIELD_SCOPE}=0 "), &format!(" {FIELD_SCOPE}=x "));
    assert!(malformed_in_t0(&lines, FIELD_SCOPE));
}

/// キーワード指定スコープ `scope` の門の行（素材未消費・`new_dpi=0`）。
fn gate(frame: u32, scope: u32) -> String {
    offset(
        frame,
        scope,
        Some(192),
        0,
        (25, -11),
        (25, -11),
        (25, -11),
        OFFSET_VERDICT_KEYWORD_PENDING,
    )
}

#[test]
fn a_keyword_scope_left_unmeasured_is_red_even_when_another_is_measured() {
    // 13 ⑴ scope 1 は T3 で測れているが、起点の前に門の行だけを出した scope 3 は測れていない。
    let mut lines = pass_lines();
    lines.insert(0, gate(1, 3));
    let report = judge_offset_log(&log_of(&lines));
    assert_eq!(
        report.violations,
        vec![OffsetViolation::KeywordScopeUnmeasured { scope: Some(3) }],
        "{report}"
    );
}

#[test]
fn a_doubled_row_in_one_transition_is_not_a_round_trip() {
    // 13 ⑵ T0・T1 だけ（往復なし）のログで、T1 の scope 0 の行を同じ遷移の中で 2 度出す。
    let mut lines = pass_lines()[..=T1_SCOPE2].to_vec();
    let doubled = lines[T1_SCOPE0].clone();
    lines.insert(T1_SCOPE0 + 1, doubled);
    let report = judge_offset_log(&log_of(&lines));
    assert!(
        report
            .violations
            .contains(&OffsetViolation::NoRoundTripObserved),
        "{report}"
    );
}

#[test]
fn a_keyword_scope_measured_only_at_an_identity_ratio_is_red() {
    // 13 ⑶ scope 1 の `rescaled` は戻りの T2（192 → 192・恒等比）だけ。作りから残差 0 の行は
    // 測った数に入れないので、T3 を門の行にすると scope 1 は 1 度も測れていない。
    let mut lines = pass_lines();
    lines[T2_SCOPE1] = offset(
        30,
        1,
        Some(192),
        192,
        (25, -11),
        (13, -6),
        (25, -11),
        OFFSET_VERDICT_RESCALED,
    );
    lines[T3_SCOPE1] = gate(40, 1);
    let report = judge_offset_log(&log_of(&lines));
    assert_eq!(
        report.violations,
        vec![OffsetViolation::KeywordScopeUnmeasured { scope: Some(1) }],
        "{report}"
    );
}

// ---------------------------------------------------------------------------
// 判定器の分岐（audit の指摘 14 ⑵⑶⑷）
// ---------------------------------------------------------------------------

/// 合格ログの scope 0 の T2・T3（往復の戻り）を、基準を据え直した行へ替える。
fn scope0_rebased(t2: String, t3: String) -> Vec<String> {
    let mut lines = pass_lines();
    lines[T2_SCOPE0] = t2;
    lines[T3_SCOPE0] = t3;
    lines
}

#[test]
fn a_rebased_base_offset_splits_the_round_trip_key() {
    // 14 ⑵ T2 の前にドラッグで基準が (12,20)@96 へ据え直された。T0 の (20,40) と T2 の
    // (24,40) は別の基準の値なので、往復の破れに数えない。
    let lines = scope0_rebased(
        offset(
            30,
            0,
            Some(96),
            192,
            (12, 20),
            (10, 20),
            (24, 40),
            OFFSET_VERDICT_RESCALED,
        ),
        offset(
            40,
            0,
            Some(96),
            96,
            (12, 20),
            (24, 40),
            (12, 20),
            OFFSET_VERDICT_RESCALED,
        ),
    );
    let report = judge_offset_log(&log_of(&lines));
    assert!(!report.failed(), "{report}");
}

#[test]
fn a_rebased_base_dpi_splits_the_round_trip_key() {
    // 14 ⑵ 基準の値は (10,20) のまま、属する DPI だけが 192 へ据え直された。T0 の (20,40) と
    // T2 の (10,20) は別の基準の値なので、往復の破れに数えない。
    let lines = scope0_rebased(
        offset(
            30,
            0,
            Some(192),
            192,
            (10, 20),
            (10, 20),
            (10, 20),
            OFFSET_VERDICT_UNCHANGED,
        ),
        offset(
            40,
            0,
            Some(192),
            96,
            (10, 20),
            (10, 20),
            (5, 10),
            OFFSET_VERDICT_RESCALED,
        ),
    );
    let report = judge_offset_log(&log_of(&lines));
    assert!(!report.failed(), "{report}");
}

#[test]
fn a_residual_of_exactly_the_allowance_is_green() {
    // 14 ⑶ 基準 (24,-12)@192 を 96 へ引き直した厳密値は (12,-6)。x の 15 は残差ちょうど 3.0px
    // ＝許容量「以内」なので緑。
    let mut lines = pass_lines();
    lines[T3_SCOPE1] = offset(
        40,
        1,
        Some(192),
        96,
        (24, -12),
        (24, -12),
        (15, -6),
        OFFSET_VERDICT_RESCALED,
    );
    let report = judge_offset_log(&log_of(&lines));
    assert!(!report.failed(), "{report}");
}

#[test]
fn every_stable_verdict_that_moved_the_value_is_red() {
    // 14 ⑷ 値を動かさない 4 語のどれで値が動いても `ValueMoved` が立つ（T3 の scope 2 に置く）。
    for (base_dpi, new_dpi, verdict) in [
        (None, 96, OFFSET_VERDICT_ANCHORED),
        (Some(96), 96, OFFSET_VERDICT_UNCHANGED),
        (Some(192), 0, OFFSET_VERDICT_KEYWORD_PENDING),
        (Some(192), 96, OFFSET_VERDICT_UNRESOLVED),
    ] {
        let mut lines = pass_lines();
        lines[T3_SCOPE2] = offset(40, 2, base_dpi, new_dpi, (7, 7), (7, 7), (4, 4), verdict);
        let report = judge_offset_log(&log_of(&lines));
        assert!(
            report.violations.iter().any(|violation| matches!(
                violation,
                OffsetViolation::ValueMoved { transition: 3, scope: Some(2), verdict: moved, .. }
                    if moved == verdict
            )),
            "{verdict}: {report}"
        );
    }
}

#[test]
fn a_non_identity_ratio_expects_rescaled_or_saturated() {
    // 14 ⑷ 比が恒等でない腕（T0 の scope 0・96 → 192）。飽和は許し、据え置きの語は赤。
    let mut lines = pass_lines();
    lines[T0_SCOPE0] = offset(
        10,
        0,
        Some(96),
        192,
        (10, 20),
        (10, 20),
        (20, 40),
        OFFSET_VERDICT_SATURATED,
    );
    let report = judge_offset_log(&log_of(&lines));
    assert!(!report.failed(), "{report}");

    lines[T0_SCOPE0] = offset(
        10,
        0,
        Some(96),
        192,
        (10, 20),
        (20, 40),
        (20, 40),
        OFFSET_VERDICT_UNCHANGED,
    );
    let report = judge_offset_log(&log_of(&lines));
    assert_eq!(
        report.violations,
        vec![OffsetViolation::UnexpectedVerdict {
            transition: 0,
            scope: Some(0),
            verdict: OFFSET_VERDICT_UNCHANGED.to_owned(),
            expected: &[OFFSET_VERDICT_RESCALED, OFFSET_VERDICT_SATURATED],
        }],
        "{report}"
    );
}

#[test]
fn an_identity_ratio_with_the_current_value_on_the_base_expects_unchanged() {
    // 14 ⑷ 恒等比の腕で現在値が基準と同じ（T2 の scope 1・192 → 192）なら、期待は据え置きの語。
    let mut lines = pass_lines();
    lines[T2_SCOPE1] = offset(
        30,
        1,
        Some(192),
        192,
        (25, -11),
        (25, -11),
        (25, -11),
        OFFSET_VERDICT_RESCALED,
    );
    let report = judge_offset_log(&log_of(&lines));
    assert_eq!(
        report.violations,
        vec![OffsetViolation::UnexpectedVerdict {
            transition: 2,
            scope: Some(1),
            verdict: OFFSET_VERDICT_RESCALED.to_owned(),
            expected: &[OFFSET_VERDICT_UNCHANGED],
        }],
        "{report}"
    );
}

#[test]
fn an_unreadable_point_field_is_a_malformed_record() {
    // 14 ⑷ `new_offset=abc` は `x,y` の形でない＝形の壊れた行。
    let lines = t0_scope0_with(
        &format!(" {FIELD_NEW_OFFSET}=20,40 "),
        &format!(" {FIELD_NEW_OFFSET}=abc "),
    );
    assert!(malformed_in_t0(&lines, FIELD_NEW_OFFSET));
}

#[test]
fn a_zero_base_dpi_is_not_divided_by() {
    // 14 ⑷ `base_dpi=0` の `rescaled` は残差を測らない（0 割りしない）。scope 1 の測れる行は
    // これだけなので、測れていないとして立つ。
    let mut lines = pass_lines();
    lines[T3_SCOPE1] = offset(
        40,
        1,
        Some(0),
        96,
        (25, -11),
        (25, -11),
        (13, -6),
        OFFSET_VERDICT_RESCALED,
    );
    let report = judge_offset_log(&log_of(&lines));
    assert_eq!(
        report.violations,
        vec![OffsetViolation::KeywordScopeUnmeasured { scope: Some(1) }],
        "{report}"
    );
}
