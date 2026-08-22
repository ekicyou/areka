//! `transition_judge` の**フレーム刻印の扱い**の決定論テスト（task 3.1）。
//!
//! 1 箇所へ集めるのは、フレーム番号にまつわる規約が 3 つとも「静かに壊れる」たぐいだから
//! である——周回（D14）は差分でしか扱えず、縮退値 `frame=0` は「1 フレームで完了」に
//! 化け（要件 8.5）、値が読めなかった行は 0 として通ってしまう。どれも壊れた側が
//! **良い数字**を返すので、赤にする檻を明示的に持たないと合格が捏造される。
//!
//! 集計の本体（回数・窓ごとの内訳・接地点差など）は姉妹モジュール
//! `transition_judge_tests` が持つ。

use areka_emo_present::presenter::{SURFACE_STAGE_UPLOAD, SURFACE_STAGE_VISUALIZE};
use wintf::ecs::window::transition_diag::{
    FIELD_FRAME, FIELD_KIND, FIELD_NEW_DPI, FIELD_NEW_WA, FIELD_OLD_DPI, FIELD_OLD_WA, FIELD_T_US,
    KIND_MONITOR, MISSING, ORIGIN_WINDOW_POS, RECORD_PREFIX_TAG, STAGE_FLUSH,
};

use super::super::diag::WindowKind;
use super::test_support::{char_kind, monitor, summarize_lines, surface, write};
use super::{RecordDefect, WindowKey, parse_transition_line, parse_transition_log};

/// 接頭語を差し替えたモニタ行（`frame=` を任意の字面に置ける）。
fn monitor_with_raw_frame(frame_field: &str) -> String {
    format!(
        "{RECORD_PREFIX_TAG} {frame_field} {FIELD_T_US}=0 {FIELD_KIND}={KIND_MONITOR} \
         entity=2v0 {FIELD_OLD_DPI}=192 {FIELD_NEW_DPI}=96 {FIELD_OLD_WA}=0,0,1,1 \
         {FIELD_NEW_WA}=0,0,1,2"
    )
}

/// 組立ヘルパが出した行の `frame=<数>` を任意の字面へ差し替える。
fn with_raw_frame(line: &str, frame: u32, raw: &str) -> String {
    let from = format!("{FIELD_FRAME}={frame} ");
    assert!(line.contains(&from), "差し替え対象が見つからない: {line}");
    line.replacen(&from, &format!("{FIELD_FRAME}={raw} "), 1)
}

// ---------------------------------------------------------------------------
// 刻印が読めたか（欠落と壊れた値は別の欠陥）
// ---------------------------------------------------------------------------

#[test]
fn parse_reports_a_missing_prefix_field() {
    let line = monitor_with_raw_frame("");
    let parsed = parse_transition_line(&line).expect("タグを含む行は解析結果を返す");
    assert!(
        parsed
            .defects
            .contains(&RecordDefect::MissingField(FIELD_FRAME.to_owned())),
        "接頭語の欠落が欠陥として出るはず: {:?}",
        parsed.defects
    );
    assert!(
        !parsed.has_frame_stamp(),
        "読めなかった frame を縮退値 0 と同一視してはならない"
    );
}

#[test]
fn parse_reports_an_unreadable_prefix_value_separately_from_a_missing_one() {
    // フィールドは在るが値が数として読めない場合（欠落とは別の欠陥）。ここを黙って
    // 0 にすると、「frame が読めなかった」と「frame は本当に 0 だった」が同じ値になり、
    // 要件 8.5 の判定不能ガードが根拠を失う。
    let line = monitor_with_raw_frame(&format!("{FIELD_FRAME}=4x2y"));
    let parsed = parse_transition_line(&line).expect("タグを含む行は解析結果を返す");
    assert!(
        parsed.defects.contains(&RecordDefect::UnreadableField {
            name: FIELD_FRAME.to_owned(),
            value: "4x2y".to_owned(),
        }),
        "読めない値が欠陥として出るはず: {:?}",
        parsed.defects
    );
    assert!(
        !parsed
            .defects
            .contains(&RecordDefect::MissingField(FIELD_FRAME.to_owned())),
        "欠落とは別の欠陥として区別されるはず: {:?}",
        parsed.defects
    );
    assert!(
        !parsed.has_frame_stamp(),
        "読めなかった frame を縮退値 0 と同一視してはならない"
    );
    assert_eq!(parsed.stamp.frame, 0, "値は捏造せず既定値のまま持つ");
}

#[test]
fn summarize_refuses_a_series_whose_frames_could_not_be_read() {
    // 全行の frame が壊れている系列。差分は 0 に見えるが、根拠は 1 つも無い。
    let broken = [
        monitor_with_raw_frame(&format!("{FIELD_FRAME}=4x2y")),
        with_raw_frame(
            &write(
                0,
                1,
                STAGE_FLUSH,
                0,
                "0x1",
                ORIGIN_WINDOW_POS,
                "0",
                char_kind(),
                1,
            ),
            0,
            "zz",
        ),
    ];
    let records = parse_transition_log(&broken.join("\n"));
    let transitions = super::split_transitions(&records);
    assert_eq!(transitions.len(), 1);
    let summary = super::summarize(&transitions[0]);
    assert_eq!(summary.malformed_records, 2, "壊れた行は数える");
    assert!(
        summary.frames_indeterminate,
        "読めた frame が 1 つも無い系列を確定量として通してはならない"
    );
}

#[test]
fn summarize_marks_an_all_zero_frame_series_as_indeterminate() {
    // frame=0 は tick 前の World が返す縮退値。要件 8.5 により「1 フレームで完了」と
    // 読んではならない。
    let summary = summarize_lines(&[
        monitor(0, 192, 96, 1704, 1752),
        write(
            0,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
    ]);
    assert_eq!(summary.frames_to_last_write, Some(0));
    assert!(
        summary.frames_indeterminate,
        "一様に 0 のフレーム系列は判定不能として立てる"
    );
}

// ---------------------------------------------------------------------------
// 周回（D14）——差分でしか扱わない
// ---------------------------------------------------------------------------

#[test]
fn summarize_takes_the_frame_difference_across_the_wraparound_boundary() {
    // `frame` は u32 で周回する。差分でしか扱わない（D14）。
    let summary = summarize_lines(&[
        monitor(u32::MAX, 192, 96, 1704, 1752),
        write(
            0,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
    ]);
    assert_eq!(summary.frames_to_last_write, Some(1));
    assert!(!summary.frames_indeterminate);
}

#[test]
fn the_visualize_to_write_frame_gap_also_wraps_around() {
    // 周回の規約は 1 つの判定量だけのものではない。可視化と書込の食い違いも差分で採る
    // （飽和引き算だと周回をまたいだ瞬間に 0＝「食い違い無し」へ化ける）。
    let summary = summarize_lines(&[
        monitor(u32::MAX, 192, 96, 1704, 1752),
        surface(
            u32::MAX,
            10,
            SURFACE_STAGE_VISUALIZE,
            0,
            "382",
            "547",
            MISSING,
            MISSING,
        ),
        write(
            0,
            20,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
    ]);
    assert_eq!(
        summary
            .mismatch_frames_per_window
            .get(&WindowKey::of(0, WindowKind::Char)),
        Some(&1),
        "周回をまたいでも食い違いは 1 フレーム"
    );
}

// ---------------------------------------------------------------------------
// 同一フレーム性（実機専用量の前提）
// ---------------------------------------------------------------------------

#[test]
fn the_visualize_to_write_microseconds_are_only_measured_inside_one_frame() {
    // C7 が測るのは「**同一 frame** の可視化から当該窓の書込まで」である。別フレームの
    // 書込まで拾うと、tick をまたいだ待ち時間が同一 tick 内の食い違いとして報告され、
    // `Bounds::signoff` が実在しない症状を測ることになる。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        surface(
            11,
            10,
            SURFACE_STAGE_VISUALIZE,
            0,
            "382",
            "547",
            MISSING,
            MISSING,
        ),
        // 可視化のフレーム（11）には当該窓の書込が無く、書込は次のフレームにある。
        write(
            12,
            9_000,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
    ]);
    let char0 = WindowKey::of(0, WindowKind::Char);
    assert!(
        !summary.visualize_to_write_us.contains_key(&char0),
        "同一フレームに書込が無い可視化は測らない: {:?}",
        summary.visualize_to_write_us
    );
    assert_eq!(
        summary.mismatch_frames_per_window.get(&char0),
        Some(&1),
        "フレーム差のほうは従来どおり出る（測れないのは µs だけ）"
    );
}

#[test]
fn the_visualize_to_write_microseconds_keep_the_widest_gap_in_the_transition() {
    // 遷移は次の起点まで伸びるので、後続フレームの表情差替などで可視化が上書きされ得る。
    // 上限判定に使う量なので、遷移自身の**最も大きい**隔たりを残す（小さく見積もると
    // 「もう是正は要らない」という誤答へ寄る）。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        surface(
            10,
            1_000,
            SURFACE_STAGE_VISUALIZE,
            0,
            "382",
            "547",
            MISSING,
            MISSING,
        ),
        write(
            10,
            201_000,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
        // 遷移の後半（表情差替）——隔たりは小さいが、こちらが後着である。
        surface(
            20,
            1_000,
            SURFACE_STAGE_UPLOAD,
            0,
            "382",
            "547",
            "false",
            MISSING,
        ),
        surface(
            20,
            2_000,
            SURFACE_STAGE_VISUALIZE,
            0,
            "382",
            "547",
            MISSING,
            MISSING,
        ),
        write(
            20,
            3_000,
            STAGE_FLUSH,
            1,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
    ]);
    assert_eq!(
        summary
            .visualize_to_write_us
            .get(&WindowKey::of(0, WindowKind::Char)),
        Some(&200_000),
        "後着の小さい隔たり（1,000µs）で上書きされてはならない"
    );
}
