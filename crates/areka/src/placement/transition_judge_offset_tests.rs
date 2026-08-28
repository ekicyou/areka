//! 追随レコードの機械判定器の檻（task 8.1・要件 8.2／8.3／8.4／8.5）。
//!
//! # 緑と赤の両方を作る
//!
//! 判定器は「合格ログで緑・不合格ログで赤」の**両方**を示さないかぎり価値が無い——緑だけを
//! 示す判定器は、判定そのものが空振りしていても緑を出す（記憶〈検証の道具そのものが壊れる・
//! 較正せよ〉）。本ファイルは既知の合格ログ [`pass_lines`] を 1 本組み、そこから **1 箇所ずつ
//! 壊した**不合格ログを作って、⑴〜⑷ の各判定が**その判定の理由で**赤になることを固定する。
//!
//! 合格ログが「解析できずに 0 件で緑」になっていないことも同じ場所で確かめる
//! （[`the_passing_fixture_is_actually_parsed`]）。
//!
//! # 観測行は発行側の純関数が組む
//!
//! `kind=offset` の行は [`crate::placement::transition_diag::offset_line`] が、`kind=monitor` の
//! 行は判定器の共有ヘルパが組む。テストが自前で字面を組むと、発行側が欄を変えたときに檻だけが
//! 古い形のまま緑になる。
//!
//! # 台本（[`pass_lines`]）
//!
//! 3 つのスコープが 4 遷移（96↔192 の往復 2 回）に現れる。
//!
//! | scope | 役 | 台本 |
//! |---|---|---|
//! | 0 | 素の追従 | 基準 `(10,20)@96`。高い側で `(20,40)`・低い側で `(10,20)` を往復 |
//! | 1 | キーワード指定 | 素材が残るあいだは門で見送り、T2 で消費、T3 で追随 |
//! | 2 | 保存値から復元 | T0 で係留（値は不変）、以後は基準 `(7,7)@192` から引き直す |
//!
//! scope 2 の T2 が**恒等比の腕で値が動く**場（`base` と現在値が離れている）であり、
//! 2026-08-28 に是正した欠陥（恒等比で現在値を据え置いてしまう）が再発したときに
//! [`an_identity_ratio_that_does_not_re_derive_is_red`] が赤にする所である。

use crate::placement::resolver::PointPx;
use crate::placement::transition_diag::{
    FIELD_BASE_DPI, FIELD_VERDICT, OFFSET_VERDICT_ALL, OFFSET_VERDICT_ANCHORED,
    OFFSET_VERDICT_KEYWORD_PENDING, OFFSET_VERDICT_RESCALED, OFFSET_VERDICT_UNCHANGED,
    OFFSET_VERDICT_UNRESOLVED, OffsetRecord, offset_line,
};
use crate::placement::transition_judge::parse_transition_line;
use crate::placement::transition_judge::test_support::monitor;
use wintf::ecs::window::transition_diag::{FIELD_SCOPE, Stamp};

use super::{ALIGNMENT_RESIDUAL_MAX_PX, OffsetViolation, judge_offset_log};

/// 追随レコードの行を組む（発行側の純関数を通す）。
#[expect(
    clippy::too_many_arguments,
    reason = "欄がそのまま引数＝束ねると檻が読めない"
)]
fn offset(
    frame: u32,
    scope: u32,
    base_dpi: Option<u32>,
    new_dpi: u32,
    base_offset: (i32, i32),
    old_offset: (i32, i32),
    new_offset: (i32, i32),
    verdict: &'static str,
) -> String {
    offset_line(&OffsetRecord {
        stamp: Stamp { frame, t_us: 0 },
        scope: Some(scope),
        base_dpi,
        new_dpi,
        base_offset: point(base_offset),
        old_offset: point(old_offset),
        new_offset: point(new_offset),
        verdict,
    })
}

fn point((x, y): (i32, i32)) -> PointPx {
    PointPx { x, y }
}

/// 作業領域の下端（モニタ行の必須欄・判定には使わない）。
const WA_LOW: i32 = 1752;
/// 作業領域の下端（高い拡大率側）。
const WA_HIGH: i32 = 1704;

// 合格ログの行番号（壊すときに名指しできるようにする）。
const T0_SCOPE0: usize = 1;
const T0_SCOPE1: usize = 2;
const T2_SCOPE0: usize = 9;
const T2_SCOPE2: usize = 11;
const T3_SCOPE0: usize = 13;
const T3_SCOPE1: usize = 14;
const T3_SCOPE2: usize = 15;
const T1_SCOPE0: usize = 5;
const T1_SCOPE1: usize = 6;
const T1_SCOPE2: usize = 7;

/// 既知の**合格**ログ（module doc の台本）。
pub(super) fn pass_lines() -> Vec<String> {
    vec![
        // T0: 96 → 192。
        monitor(10, 96, 192, WA_LOW, WA_HIGH),
        offset(
            10,
            0,
            Some(96),
            192,
            (10, 20),
            (10, 20),
            (20, 40),
            OFFSET_VERDICT_RESCALED,
        ),
        offset(
            10,
            1,
            Some(192),
            0,
            (25, -11),
            (25, -11),
            (25, -11),
            OFFSET_VERDICT_KEYWORD_PENDING,
        ),
        offset(
            10,
            2,
            None,
            192,
            (7, 7),
            (7, 7),
            (7, 7),
            OFFSET_VERDICT_ANCHORED,
        ),
        // T1: 192 → 96（低い拡大率側）。
        monitor(20, 192, 96, WA_HIGH, WA_LOW),
        offset(
            20,
            0,
            Some(96),
            96,
            (10, 20),
            (20, 40),
            (10, 20),
            OFFSET_VERDICT_RESCALED,
        ),
        offset(
            20,
            1,
            Some(192),
            0,
            (25, -11),
            (25, -11),
            (25, -11),
            OFFSET_VERDICT_KEYWORD_PENDING,
        ),
        offset(
            20,
            2,
            Some(192),
            96,
            (7, 7),
            (7, 7),
            (4, 4),
            OFFSET_VERDICT_RESCALED,
        ),
        // T2: 96 → 192（往復の戻り・scope 1 は素材を消費済み）。
        monitor(30, 96, 192, WA_LOW, WA_HIGH),
        offset(
            30,
            0,
            Some(96),
            192,
            (10, 20),
            (10, 20),
            (20, 40),
            OFFSET_VERDICT_RESCALED,
        ),
        offset(
            30,
            1,
            Some(192),
            192,
            (25, -11),
            (25, -11),
            (25, -11),
            OFFSET_VERDICT_UNCHANGED,
        ),
        offset(
            30,
            2,
            Some(192),
            192,
            (7, 7),
            (4, 4),
            (7, 7),
            OFFSET_VERDICT_RESCALED,
        ),
        // T3: 192 → 96（低い拡大率側・素材消費後）。
        monitor(40, 192, 96, WA_HIGH, WA_LOW),
        offset(
            40,
            0,
            Some(96),
            96,
            (10, 20),
            (20, 40),
            (10, 20),
            OFFSET_VERDICT_RESCALED,
        ),
        offset(
            40,
            1,
            Some(192),
            96,
            (25, -11),
            (25, -11),
            (13, -6),
            OFFSET_VERDICT_RESCALED,
        ),
        offset(
            40,
            2,
            Some(192),
            96,
            (7, 7),
            (7, 7),
            (4, 4),
            OFFSET_VERDICT_RESCALED,
        ),
    ]
}

/// 行の並びをログ本文へ。
pub(super) fn log_of(lines: &[String]) -> String {
    lines.join("\n")
}

/// 1 行だけ差し替えた不合格ログ。
fn broken(index: usize, replacement: String) -> String {
    let mut lines = pass_lines();
    lines[index] = replacement;
    log_of(&lines)
}

// ---------------------------------------------------------------------------
// 合格側（既知の合格ログが緑になること・かつ空振りでないこと）
// ---------------------------------------------------------------------------

#[test]
fn the_known_passing_log_is_green() {
    let report = judge_offset_log(&log_of(&pass_lines()));
    assert!(!report.failed(), "既知の合格ログが赤になった:\n{report}");
}

#[test]
fn the_passing_fixture_is_actually_parsed() {
    // 「違反 0 件」が**解析できずに 0 件**から出ていないことを確かめる。行数と遷移本数を
    // 逐語で固定しないと、行の形を壊した瞬間に判定器が静かに空振りして緑になる。
    let report = judge_offset_log(&log_of(&pass_lines()));
    assert_eq!(report.transitions.len(), 4, "{report}");
    assert_eq!(report.rows, 12, "{report}");
    let low_side: Vec<u32> = report
        .transitions
        .iter()
        .filter(|transition| transition.new_dpi < transition.old_dpi)
        .map(|transition| transition.new_dpi)
        .collect();
    assert_eq!(low_side, vec![96, 96], "低い拡大率側の遷移が 2 本あるはず");
}

#[test]
fn an_empty_log_is_red_not_an_empty_pass() {
    let report = judge_offset_log("");
    assert_eq!(report.violations, vec![OffsetViolation::NoOffsetRecords]);
}

#[test]
fn a_log_without_a_round_trip_is_red() {
    // 往復を 1 度も観測していないログは「違反 0 件」ではなく**判定できていない**。
    let lines = pass_lines();
    let log = log_of(&lines[..8]);
    let report = judge_offset_log(&log);
    assert!(
        report
            .violations
            .contains(&OffsetViolation::NoRoundTripObserved),
        "{report}"
    );
}

// ---------------------------------------------------------------------------
// ⑴ 往復の前後で bit 同一（要件 8.2）
// ---------------------------------------------------------------------------

#[test]
fn a_round_trip_that_does_not_return_the_same_bits_is_red() {
    // T2 で 96 → 192 へ戻ったのに、T0 の (20,40) と 1 px ずれた値を反映した。
    let log = broken(
        T2_SCOPE0,
        offset(
            30,
            0,
            Some(96),
            192,
            (10, 20),
            (10, 20),
            (21, 40),
            OFFSET_VERDICT_RESCALED,
        ),
    );
    let report = judge_offset_log(&log);
    assert_eq!(
        report.violations,
        vec![OffsetViolation::RoundTripDrift {
            scope: Some(0),
            dpi: 192,
            first_transition: 0,
            first: (20, 40),
            again_transition: 2,
            again: (21, 40),
        }],
        "{report}"
    );
}

#[test]
fn the_round_trip_check_reads_values_not_verdict_words() {
    // 是正後の往復は `rescaled → rescaled` を出す（`rescaled → unchanged` ではない）。
    // 判定語の並びを鍵にしていたらこの合格ログは赤になる——値だけを鍵にしている証拠として、
    // 往復の 2 本がどちらも `rescaled` であることを逐語で押さえる。
    let lines = pass_lines();
    for index in [T0_SCOPE0, T2_SCOPE0] {
        assert!(
            lines[index].contains(&format!("verdict={OFFSET_VERDICT_RESCALED}")),
            "{}",
            lines[index]
        );
    }
    assert!(!judge_offset_log(&log_of(&lines)).failed());
}

// ---------------------------------------------------------------------------
// ⑵ 判定語が期待の腕であること（要件 8.3）
// ---------------------------------------------------------------------------

#[test]
fn an_identity_ratio_that_does_not_re_derive_is_red() {
    // 2026-08-28 に是正した欠陥の再発形: 恒等比の腕で**基準から引き直さず**現在値 (4,4) を
    // 据え置き、`unchanged` と記録した。基準 (7,7) と現在値が離れているので期待の腕は
    // `rescaled` である。
    let log = broken(
        T2_SCOPE2,
        offset(
            30,
            2,
            Some(192),
            192,
            (7, 7),
            (4, 4),
            (4, 4),
            OFFSET_VERDICT_UNCHANGED,
        ),
    );
    let report = judge_offset_log(&log);
    assert_eq!(
        report.violations,
        vec![OffsetViolation::UnexpectedVerdict {
            transition: 2,
            scope: Some(2),
            verdict: OFFSET_VERDICT_UNCHANGED.to_owned(),
            expected: &[OFFSET_VERDICT_RESCALED],
        }],
        "{report}"
    );
}

#[test]
fn an_unanchored_base_that_is_not_anchored_is_red() {
    // 未係留（`base_dpi` が番兵）の腕は係留の語しか出せない。
    let log = broken(
        T0_SCOPE1,
        offset(
            10,
            1,
            None,
            192,
            (25, -11),
            (25, -11),
            (25, -11),
            OFFSET_VERDICT_UNCHANGED,
        ),
    );
    let report = judge_offset_log(&log);
    assert!(
        report.violations.iter().any(|violation| matches!(
            violation,
            OffsetViolation::UnexpectedVerdict { expected, .. }
                if *expected == [OFFSET_VERDICT_ANCHORED]
        )),
        "{report}"
    );
}

#[test]
fn a_stable_arm_that_moved_the_value_is_red() {
    let log = broken(
        T3_SCOPE2,
        offset(
            40,
            2,
            Some(192),
            96,
            (7, 7),
            (7, 7),
            (4, 4),
            OFFSET_VERDICT_UNRESOLVED,
        ),
    );
    let report = judge_offset_log(&log);
    assert!(
        report.violations.iter().any(|violation| matches!(
            violation,
            OffsetViolation::ValueMoved {
                transition: 3,
                scope: Some(2),
                ..
            }
        )),
        "{report}"
    );
}

#[test]
fn a_verdict_outside_the_vocabulary_is_red() {
    // 語彙表に無い語（発行側が語を変えたのに判定側が追随していない形）。負例なので字面を
    // 書くが、これは**判定語ではない**字面である。
    let log = broken(
        T0_SCOPE0,
        offset(
            10,
            0,
            Some(96),
            192,
            (10, 20),
            (10, 20),
            (20, 40),
            "not-a-verdict",
        ),
    );
    let report = judge_offset_log(&log);
    assert!(
        report.violations.iter().any(|violation| matches!(
            violation,
            OffsetViolation::UnknownVerdict {
                transition: 0,
                scope: Some(0),
                ..
            }
        )),
        "{report}"
    );
}

#[test]
fn a_record_with_a_missing_field_is_red() {
    // 欄が落ちた行は「読めなかった」として立てる（黙って落とすと 0 件が合格に化ける）。
    let mut lines = pass_lines();
    let intact = lines[T0_SCOPE0].clone();
    let cut = intact
        .split_once(&format!(" verdict={OFFSET_VERDICT_RESCALED}"))
        .expect("合格ログの行は判定語を持つ")
        .0
        .to_owned();
    assert_ne!(cut, intact, "檻の前提: 実際に欄が落ちていること");
    lines[T0_SCOPE0] = cut;
    let report = judge_offset_log(&log_of(&lines));
    assert!(
        report
            .violations
            .iter()
            .any(|violation| matches!(violation, OffsetViolation::MalformedRecord { .. })),
        "{report}"
    );
}

// ---------------------------------------------------------------------------
// 門の判定語は表示 DPI を運ばない（要件 4.3・design D7・腕は verdict だけで見分ける）
// ---------------------------------------------------------------------------

#[test]
fn a_keyword_pending_row_that_carries_a_display_dpi_is_red() {
    let log = broken(
        T0_SCOPE1,
        offset(
            10,
            1,
            Some(192),
            192,
            (25, -11),
            (25, -11),
            (25, -11),
            OFFSET_VERDICT_KEYWORD_PENDING,
        ),
    );
    let report = judge_offset_log(&log);
    assert_eq!(
        report.violations,
        vec![OffsetViolation::KeywordPendingCarriesDisplayDpi {
            transition: 0,
            scope: Some(1),
            new_dpi: 192,
        }],
        "{report}"
    );
}

#[test]
fn the_accepted_residual_is_not_read_as_a_broken_follow() {
    // 開発者裁定（2026-08-27）が受容した残余＝素材未消費のまま寸据え置きの遷移を迎えた腕は、
    // `keyword-pending` かつ前後の値が bit 同一という形で現れる。合格ログの T0／T1 が
    // まさにその形であり、違反にしてはならない。
    let lines = pass_lines();
    assert!(
        lines[T0_SCOPE1].contains(&format!("verdict={OFFSET_VERDICT_KEYWORD_PENDING}")),
        "{}",
        lines[T0_SCOPE1]
    );
    assert!(!judge_offset_log(&log_of(&lines)).failed());
}

// ---------------------------------------------------------------------------
// ⑶ 低い拡大率側で追随が出ていること（要件 8.4）
// ---------------------------------------------------------------------------

#[test]
fn a_log_without_any_rescale_on_the_low_scale_side_is_red() {
    // 先行仕様の残所見（低い拡大率側でバルーンがずれたまま）の形: 低い側の遷移では
    // 縮退しか出ておらず、追随が 1 度も効いていない。scope 1 の T3 は追随だが、後続の
    // T4 で素材が再び未消費になる（＝素材消費後の遷移ではない）ので母数に数えない。
    let mut lines = pass_lines();
    lines[T1_SCOPE0] = offset(
        20,
        0,
        Some(96),
        96,
        (10, 20),
        (20, 40),
        (20, 40),
        OFFSET_VERDICT_UNRESOLVED,
    );
    lines[T1_SCOPE2] = offset(
        20,
        2,
        Some(192),
        96,
        (7, 7),
        (7, 7),
        (7, 7),
        OFFSET_VERDICT_UNRESOLVED,
    );
    lines[T3_SCOPE0] = offset(
        40,
        0,
        Some(96),
        96,
        (10, 20),
        (20, 40),
        (20, 40),
        OFFSET_VERDICT_UNRESOLVED,
    );
    lines[T3_SCOPE2] = offset(
        40,
        2,
        Some(192),
        96,
        (7, 7),
        (7, 7),
        (7, 7),
        OFFSET_VERDICT_UNRESOLVED,
    );
    lines.push(monitor(50, 96, 192, WA_LOW, WA_HIGH));
    lines.push(offset(
        50,
        1,
        Some(192),
        0,
        (25, -11),
        (25, -11),
        (25, -11),
        OFFSET_VERDICT_KEYWORD_PENDING,
    ));

    let report = judge_offset_log(&log_of(&lines));
    assert_eq!(
        report.violations,
        vec![OffsetViolation::NoLowScaleRescale {
            low_side_transitions: 2,
        }],
        "{report}"
    );
}

#[test]
fn a_keyword_scope_still_holding_its_material_does_not_make_a_false_red() {
    // 逆向きの較正: 素材が残るあいだの遷移（T0／T1 の門）が母数に入っていたら、合格ログの
    // 側が赤になる。実際には scope 0 の T1 の追随が母数を満たすので緑である。
    let report = judge_offset_log(&log_of(&pass_lines()));
    assert!(
        !report
            .violations
            .iter()
            .any(|violation| matches!(violation, OffsetViolation::NoLowScaleRescale { .. })),
        "{report}"
    );
}

// ---------------------------------------------------------------------------
// ⑷ キーワード指定スコープの揃えの残差（要件 8.5・design D8）
// ---------------------------------------------------------------------------

#[test]
fn an_alignment_residual_beyond_the_allowance_is_red() {
    // 基準 (25,-11)@192 を 96 へ引き直した厳密値は (12.5,-5.5)。x を 17 にすると残差は
    // 4.5px であり、許容量 3px を超える。
    let log = broken(
        T3_SCOPE1,
        offset(
            40,
            1,
            Some(192),
            96,
            (25, -11),
            (25, -11),
            (17, -6),
            OFFSET_VERDICT_RESCALED,
        ),
    );
    let report = judge_offset_log(&log);
    assert_eq!(
        report.violations,
        vec![OffsetViolation::AlignmentResidual {
            transition: 3,
            scope: Some(1),
            axis: "x",
            residual_hundredths: 450,
            max_px: ALIGNMENT_RESIDUAL_MAX_PX,
        }],
        "{report}"
    );
}

#[test]
fn a_residual_inside_the_allowance_is_green() {
    // 許容量ちょうど（3px）は合格側に置く——契約の上限は「以内」である。
    // 基準 (25,-11)@192 → 96 の厳密値 12.5 に対し 15.5 は 3px。整数の欄なので 15 を採り、
    // 残差 2.5px が緑であることを固定する（3px 超で初めて赤になることは上の檻が示す）。
    let log = broken(
        T3_SCOPE1,
        offset(
            40,
            1,
            Some(192),
            96,
            (25, -11),
            (25, -11),
            (15, -6),
            OFFSET_VERDICT_RESCALED,
        ),
    );
    assert!(!judge_offset_log(&log).failed());
}

#[test]
fn a_log_that_never_measures_a_keyword_alignment_is_red() {
    // キーワード指定スコープが門の語しか出していない（＝揃えを 1 度も測れていない）ログ。
    let log = broken(
        T3_SCOPE1,
        offset(
            40,
            1,
            Some(192),
            0,
            (25, -11),
            (25, -11),
            (25, -11),
            OFFSET_VERDICT_KEYWORD_PENDING,
        ),
    );
    let report = judge_offset_log(&log);
    assert!(
        report
            .violations
            .contains(&OffsetViolation::NoKeywordAlignmentMeasured { keyword_scopes: 1 }),
        "{report}"
    );
}

// ---------------------------------------------------------------------------
// ⑷ の母数は観測行の全体から作る（task 8.6・要件 8.5）
// ---------------------------------------------------------------------------

/// キーワード指定スコープの門の行（合格ログの scope 1 と同じ値・素材未消費）。
fn keyword_gate_line(frame: u32) -> String {
    offset(
        frame,
        1,
        Some(192),
        0,
        (25, -11),
        (25, -11),
        (25, -11),
        OFFSET_VERDICT_KEYWORD_PENDING,
    )
}

/// 合格ログから**遷移の内側**の門の行 2 件（T0・T1 の scope 1）を落とした並び。
fn lines_without_gate_rows() -> Vec<String> {
    let mut lines = pass_lines();
    lines.remove(T1_SCOPE1);
    lines.remove(T0_SCOPE1);
    lines
}

#[test]
fn a_gate_row_before_the_first_origin_still_forms_the_denominator() {
    // ⑴ 門の行が**最初の起点より前にしか無い**ログ。実機はこの形しか採れない——素材は
    // 起動から 0.73〜5.0 秒で自動的に消費され、最初の起点は利用者のドラッグ由来ゆえ必ず
    // それより後に出る（2026-08-28 の実機ログ 3 本すべて）。
    let mut lines = lines_without_gate_rows();
    lines.insert(0, keyword_gate_line(1));
    let report = judge_offset_log(&log_of(&lines));
    // 門の行はどの遷移にも属さない（`split_transitions` が捨てる）＝母数は遷移の外から来た。
    assert_eq!(report.rows, 10, "{report}");
    assert!(
        !report
            .transitions
            .iter()
            .flat_map(|transition| transition.rows.iter())
            .any(|row| row.verdict == OFFSET_VERDICT_KEYWORD_PENDING),
        "遷移の内側に門の行が残っている＝この檻が ⑴ の形を踏んでいない:
{report}"
    );
    assert!(!report.failed(), "{report}");
}

#[test]
fn a_gate_row_inside_a_transition_still_forms_the_denominator() {
    // ⑵ 従来どおり遷移の内側に門の行があるログ（既知の合格ログ）。母数の作り方を広げても
    // この形が壊れないことを固定する。
    let report = judge_offset_log(&log_of(&pass_lines()));
    assert!(
        report
            .transitions
            .iter()
            .flat_map(|transition| transition.rows.iter())
            .any(|row| row.verdict == OFFSET_VERDICT_KEYWORD_PENDING),
        "既知の合格ログが遷移の内側に門の行を持たなくなった:
{report}"
    );
    assert!(!report.failed(), "{report}");
}

/// 門の行から**必須欄を 1 つ落とした**行（語彙の規約を破っている＝読めない行）。
///
/// 落とすのは `base_dpi` である——`scope` と `verdict` は残るので、母数が
/// `is_well_formed()` で弾いているのか、それとも欄が読めずに弾かれただけなのかを
/// 取り違えない。値が読めない字面（`base_dpi=x` など）では欠陥が立たない
/// （`RecordDefect::UnreadableField` は `frame`／`t_us` にしか立たない・実測）。
fn malformed_keyword_gate_line() -> String {
    let intact = keyword_gate_line(1);
    let broken = intact.replace(&format!(" {FIELD_BASE_DPI}=192"), "");
    assert_ne!(broken, intact, "檻の前提: 実際に必須欄が落ちていること");
    broken
}

#[test]
fn a_malformed_gate_row_does_not_enter_the_denominator() {
    // 母数は**読める行**からしか作らない（`keyword_pending_scopes` の `is_well_formed()`）。
    // 読めない行から母数を組むと、行の形が壊れたときに母数だけが静かに増え、⑷ が
    // 「1 度も測れていない」を言えなくなる。
    let broken_gate = malformed_keyword_gate_line();
    // 檻の較正——この行は語彙の規約を破っているが、`scope` と `verdict` は**依然として
    // 読める**。ゆえにこの檻が緑に化けるとしたら、それは `is_well_formed()` の分岐が
    // 消えたときだけである（欄が読めないせいで弾かれたのではない）。
    let record = parse_transition_line(&broken_gate).expect("行としては解析できる");
    assert!(
        !record.is_well_formed(),
        "檻の前提: 語彙の規約を破っていること"
    );
    assert_eq!(record.int_field::<u32>(FIELD_SCOPE), Some(1));
    assert_eq!(
        record.field(FIELD_VERDICT),
        Some(OFFSET_VERDICT_KEYWORD_PENDING)
    );

    let mut lines = lines_without_gate_rows();
    lines.insert(0, broken_gate);
    let report = judge_offset_log(&log_of(&lines));
    assert!(
        report
            .violations
            .contains(&OffsetViolation::NoKeywordAlignmentMeasured { keyword_scopes: 0 }),
        "{report}"
    );
}

#[test]
fn a_log_with_no_gate_row_anywhere_is_still_red() {
    // ⑶ 門の行が**どこにも無い**ログ。母数を広げた結果「検査が何も要求しなくなる」形を
    // 防ぐ檻——ここが緑になったら、⑷ はキーワード指定スコープを 1 つも要求していない。
    let report = judge_offset_log(&log_of(&lines_without_gate_rows()));
    assert!(
        report
            .violations
            .contains(&OffsetViolation::NoKeywordAlignmentMeasured { keyword_scopes: 0 }),
        "{report}"
    );
}

// ---------------------------------------------------------------------------
// 判定語のリテラルを書いていないこと
// ---------------------------------------------------------------------------

#[test]
fn the_judge_writes_no_verdict_literal() {
    // 判定器の本文に判定語の**文字列リテラル**が 1 つも無いこと（発行側の `pub const` を
    // 参照するだけであること）を字面で押さえる。檻そのものが空振りしないよう、探す形が
    // 実在の字面で当たることを同じテストの中で確かめる。
    let source = include_str!("transition_judge_offset.rs");
    for verdict in OFFSET_VERDICT_ALL {
        let literal = format!("\"{verdict}\"");
        assert!(
            !source.contains(&literal),
            "判定器が判定語のリテラル {literal} を書いている"
        );
    }
    let probe = format!("\"{OFFSET_VERDICT_RESCALED}\"");
    assert!(
        format!("let x = {probe};").contains(&probe),
        "檻の探し方そのものが壊れている"
    );
}
