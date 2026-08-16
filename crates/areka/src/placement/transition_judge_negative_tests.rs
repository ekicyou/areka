//! `transition_judge` の**負例**——判定語を壊した入力・フィールドの欠落・フレーム番号の
//! 周回境界（task 3.2・要件 7.4／8.5・D14）。
//!
//! ここが押さえるのは 1 点に尽きる: **壊れた入力が合格に化けない**こと。負例を集めるのは、
//! 壊れ方によって「静かに消える場所」が 3 通りに分かれるからである:
//!
//! 1. 接頭語（`frame`／`t_us`）が壊れた行——[`super::RecordDefect`] が立ち
//!    `malformed_records` に載る（この経路は `transition_judge_frame_tests` が持つ）。
//! 2. `kind` 語・必須フィールドが壊れた行——同じく [`super::RecordDefect`] が立つ。ただし
//!    起点行が壊れると**遷移そのものが切り出せない**ので、判定は遷移の中ではなくログ水準
//!    （[`super::Report::log_violations`]）で赤くなる。
//! 3. **本体側の数値**（`diff`・`total_us`・`target_id`）だけが壊れた行——`is_well_formed()` は
//!    真のままで `malformed_records` は 0 であり、当該の量**だけ**が `None`／空へ落ちる。
//!    上限（`≤ 0`・`≤ 1`）はこれを素通しにするので、[`super::judge`] が
//!    [`super::Violation::Unmeasured`] を返すことが唯一の防波堤である。
//!
//! 対照（壊していない入力）は [`compliant_transition_lines`] であり、
//! `transition_judge_verdict_tests::the_compliant_transition_passes_both_families` が緑で
//! あることが本モジュールの前提になる。

use areka_emo_present::presenter::SURFACE_FIELD_TARGET_ID;
use wintf::ecs::window::transition_diag::{
    FIELD_KIND, FIELD_SEQ, FIELD_STAGE, FIELD_TOTAL_US, FIELD_WIN_KIND, KIND_MONITOR, KIND_WRITE,
    MISSING, STAGE_FLUSH,
};

use super::super::diag::WindowKind;
use super::super::transition_diag::FIELD_DIFF;
use super::test_support::{
    compliant_transition_lines, compliant_transition_lines_at, replace_once, summarize_lines,
};
use super::{Bounds, Quantity, Violation, WindowKey, judge, judge_transition_log};

/// 行の列を 1 本のログ文字列へ。
fn log_of(lines: &[String]) -> String {
    lines.join("\n")
}

/// 決定論の上限で判定して違反の列を得る。
fn deterministic_violations(lines: &[String]) -> Vec<Violation> {
    judge(&summarize_lines(lines), &Bounds::deterministic()).expect_err("違反があるはず")
}

// ---------------------------------------------------------------------------
// 判定語を壊した入力
// ---------------------------------------------------------------------------

#[test]
fn a_broken_origin_kind_word_leaves_no_transition_and_that_is_not_a_pass() {
    // 起点語（`kind=monitor`）が変われば遷移は 1 本も切り出せない。ここで空の Report を
    // 「違反 0 件＝合格」と読むと、語彙の退行がそのまま緑になる。
    let broken = replace_once(
        &compliant_transition_lines(),
        &format!("{FIELD_KIND}={KIND_MONITOR}"),
        &format!("{FIELD_KIND}={KIND_MONITOR}X"),
    );
    let report = judge_transition_log(&log_of(&broken));

    assert!(report.transitions.is_empty(), "遷移は切り出せない");
    assert_eq!(
        report.log_violations(),
        vec![
            Violation::NoTransitionsFound,
            Violation::MalformedRecords { count: 1 },
        ],
        "遷移が無いことと、遷移の外に壊れた行があることは別の違反"
    );
    assert!(report.failed());
    assert_eq!(report.unassigned_records, broken.len());
}

#[test]
fn a_broken_kind_word_inside_a_transition_is_counted_as_a_malformed_record() {
    // 対照は書込を 2 行持つので、壊すのは先頭（`seq=0`）の 1 行だけと明示する。
    let broken = replace_once(
        &compliant_transition_lines(),
        &format!("{FIELD_KIND}={KIND_WRITE} {FIELD_STAGE}={STAGE_FLUSH} {FIELD_SEQ}=0"),
        &format!("{FIELD_KIND}={KIND_WRITE}X {FIELD_STAGE}={STAGE_FLUSH} {FIELD_SEQ}=0"),
    );
    let summary = summarize_lines(&broken);
    assert_eq!(summary.malformed_records, 1);
    assert!(
        deterministic_violations(&broken).contains(&Violation::MalformedRecords { count: 1 }),
        "壊れた行があるだけで合格にしない"
    );
}

// ---------------------------------------------------------------------------
// フィールドの欠落
// ---------------------------------------------------------------------------

#[test]
fn dropping_a_required_field_is_reported_and_takes_the_window_out_of_its_measurements() {
    // `win_kind=` を落とすと、その行は欠落として赤くなる（1）だけでなく、窓キーの種別語が
    // 番兵へ落ちて可視化との突合から外れる（3）。両方が別の違反として出ることを固定する。
    let broken = replace_once(
        &compliant_transition_lines(),
        &format!(" {FIELD_WIN_KIND}={} ", WindowKind::Char.as_str()),
        " ",
    );
    let summary = summarize_lines(&broken);
    assert_eq!(summary.malformed_records, 1);

    let sentinel_window = WindowKey {
        scope: Some(0),
        kind: MISSING.to_owned(),
    };
    assert_eq!(
        summary.writes_per_window.get(&sentinel_window),
        Some(&1),
        "種別語を失った書込は番兵のキーへ落ちる"
    );

    let violations = deterministic_violations(&broken);
    assert!(violations.contains(&Violation::MalformedRecords { count: 1 }));
    assert!(
        violations.contains(&Violation::Unmeasured(Quantity::MismatchFrames(
            sentinel_window
        ))),
        "可視化と突き合わせられない窓が居ることを黙らせない: {violations:?}"
    );
    assert!(
        !violations.iter().any(|violation| matches!(
            violation,
            Violation::Unmeasured(Quantity::BalloonSameFrame(_))
        )),
        "随伴の対はキャラ窓を起点に数えるので、種別語を失った窓は対の起点にならない: {violations:?}"
    );
}

// ---------------------------------------------------------------------------
// 本体側の数値だけが壊れた行（`malformed_records` は 0 のまま）
// ---------------------------------------------------------------------------

#[test]
fn an_unreadable_ground_difference_is_a_violation_not_a_silent_pass() {
    let broken = replace_once(
        &compliant_transition_lines(),
        &format!("{FIELD_DIFF}=0"),
        &format!("{FIELD_DIFF}=??"),
    );
    let summary = summarize_lines(&broken);
    assert_eq!(
        summary.malformed_records, 0,
        "`diff` は接頭語ではないので行は健全と判定される（だから判定器が要る）"
    );
    assert_eq!(summary.ground_diff_max, None);
    assert_eq!(
        deterministic_violations(&broken),
        vec![Violation::Unmeasured(Quantity::GroundDiff)],
        "接地点差 `≤ 0` は「量が無い」を素通しにする"
    );
}

#[test]
fn an_unreadable_flush_total_is_a_violation_of_the_signoff_family_only() {
    let broken = replace_once(
        &compliant_transition_lines(),
        &format!("{FIELD_TOTAL_US}=300"),
        &format!("{FIELD_TOTAL_US}=??"),
    );
    let summary = summarize_lines(&broken);
    assert_eq!(summary.malformed_records, 0);
    assert_eq!(summary.flush_total_us_max, None);
    assert_eq!(
        judge(&summary, &Bounds::signoff()),
        Err(vec![Violation::Unmeasured(Quantity::FlushTotalUs)])
    );
    assert_eq!(
        judge(&summary, &Bounds::deterministic()),
        Ok(()),
        "決定論側はこの量を当てないので合否が動かない"
    );
}

#[test]
fn an_unreadable_surface_target_hides_both_visualize_derived_quantities() {
    // `target_id` が読めないと可視化の行は窓へ写像できず、フレーム差（決定論）と
    // 可視化から書込までの経過（実機専用）の**両方**が静かに消える。
    let mut broken = replace_once(
        &compliant_transition_lines(),
        &format!("{SURFACE_FIELD_TARGET_ID}=0"),
        &format!("{SURFACE_FIELD_TARGET_ID}=??"),
    );
    broken = replace_once(
        &broken,
        &format!("{SURFACE_FIELD_TARGET_ID}=1"),
        &format!("{SURFACE_FIELD_TARGET_ID}=??"),
    );
    let summary = summarize_lines(&broken);
    assert_eq!(summary.malformed_records, 0);
    assert!(summary.mismatch_frames_per_window.is_empty());
    assert!(summary.visualize_to_write_us.is_empty());

    let windows = [
        WindowKey::of(0, WindowKind::Char),
        WindowKey::of(0, WindowKind::Balloon),
    ];
    let deterministic = deterministic_violations(&broken);
    let signoff = judge(&summary, &Bounds::signoff()).expect_err("実機側も量が無い");
    for window in windows {
        assert!(
            deterministic.contains(&Violation::Unmeasured(Quantity::MismatchFrames(
                window.clone()
            ))),
            "{window} のフレーム差が黙って消えている: {deterministic:?}"
        );
        assert!(
            signoff.contains(&Violation::Unmeasured(Quantity::VisualizeToWriteUs(window))),
            "実機専用の量も黙って消えている: {signoff:?}"
        );
    }
}

#[test]
fn a_transition_without_any_window_write_is_not_a_pass() {
    // 上限が `≤ 1`・`≤ 0` である以上、「1 回も書かなかった」は全ての回数の上限を満たす。
    // 満たしたことを合格にしないのは `frames_to_last_write` が `None` になるからである。
    let no_writes: Vec<String> = compliant_transition_lines()
        .into_iter()
        .filter(|line| !line.contains(&format!("{FIELD_KIND}={KIND_WRITE} ")))
        .collect();
    let summary = summarize_lines(&no_writes);
    assert!(summary.writes_per_window.is_empty());
    assert_eq!(
        deterministic_violations(&no_writes),
        vec![Violation::Unmeasured(Quantity::FramesToLastWrite)]
    );
}

// ---------------------------------------------------------------------------
// フレーム番号の周回境界（D14）
// ---------------------------------------------------------------------------

#[test]
fn a_transition_that_completes_across_the_wraparound_boundary_measures_one_frame_not_four_billion()
{
    // 起点 `u32::MAX` → 書込 `0` は **1 フレーム差**であって `u32::MAX` ではない。
    // 絶対値で比べていれば「4,294,967,295 フレーム遅れ」か、逆に「早い」に化ける。
    let wrapped = compliant_transition_lines_at(u32::MAX, 0);
    assert_eq!(
        deterministic_violations(&wrapped),
        vec![Violation::FramesToLastWrite {
            frames: 1,
            max: super::TRANSITION_FRAME_BOUND,
        }]
    );
}

#[test]
fn the_wraparound_boundary_itself_still_passes_when_everything_lands_in_one_frame() {
    // 周回の直前のフレームで全部が終わる遷移は合格でなければならない（境界の反対側）。
    let at_boundary = compliant_transition_lines_at(u32::MAX, u32::MAX);
    assert_eq!(
        judge(&summarize_lines(&at_boundary), &Bounds::deterministic()),
        Ok(())
    );
}

#[test]
fn an_all_zero_frame_series_is_indeterminate_and_never_a_pass() {
    // `frame=0` は tick 前の World が返す縮退値であり、「1 フレームで完了」の根拠にしない
    // （要件 8.5）。判定量としては `frames_to_last_write = Some(0)` が立つので、
    // `frames_indeterminate` を別の違反として返さないとそのまま合格になる。
    let all_zero = compliant_transition_lines_at(0, 0);
    let summary = summarize_lines(&all_zero);
    assert_eq!(summary.frames_to_last_write, Some(0));
    assert!(summary.frames_indeterminate);
    assert_eq!(
        deterministic_violations(&all_zero),
        vec![Violation::FramesIndeterminate]
    );
}
