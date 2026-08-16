//! `transition_judge` の**解析・切り出し・集計**の決定論テスト（task 3.1）。
//!
//! テーマ別の姉妹モジュール:
//!
//! - `transition_judge_frame_tests` — フレーム刻印の扱い（周回・縮退・読めない値・同一フレーム性）
//! - `transition_judge_reobservation_tests` — 再観測ログの逐語再現
//!
//! ここは 1 つの判定量につき 1 つの分岐を突く粒度で並べる。

use areka_emo_present::presenter::{
    KIND_SURFACE, SURFACE_FIELD_H, SURFACE_FIELD_W, SURFACE_REASON_INVISIBLE,
    SURFACE_STAGE_SKIPPED, SURFACE_STAGE_UPLOAD, SURFACE_STAGE_VISUALIZE,
};
use wintf::ecs::window::transition_diag::{
    FIELD_NEW_DPI, FIELD_NEW_WA, FIELD_OLD_DPI, FIELD_SCOPE, FIELD_WIN_KIND, KIND_ALL,
    KIND_MONITOR, KIND_MSG, KIND_WRITE, MISSING, MSG_DPICHANGED, ORIGIN_DPI_SUGGESTED,
    ORIGIN_WINDOW_POS, STAGE_BEGIN, STAGE_END, STAGE_FLUSH, STAGE_SYNC,
};

use super::super::diag::{PlacementRoute, WindowKind};
use super::super::transition_diag::{
    CHAIN_STAGE_DEFERRED, CHAIN_STAGE_REALIGNED, HOLD_DECISION_HOLD, HOLD_DECISION_PROCEED,
    HOLD_SITE_DPI, PLACEMENT_KIND_ALL,
};
use super::test_support::{
    balloon_kind, chain, char_kind, enqueue, flush, ground, hold, monitor, msg, parse_ok, record,
    summarize_lines, surface, write,
};
use super::{
    RecordDefect, WindowKey, is_transition_origin, parse_transition_line, parse_transition_log,
    required_fields, split_transitions, summarize,
};
use crate::emo2_boot::target_map::{balloon_target, shell_target};

// ---------------------------------------------------------------------------
// 行の解析
// ---------------------------------------------------------------------------

#[test]
fn builders_produce_well_formed_records() {
    // 組立ヘルパが必須フィールドを漏らしていたら、以後の全テストの前提が崩れる。
    for line in [
        monitor(7, 192, 96, 1704, 1752),
        write(
            7,
            100,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            5,
        ),
        flush(7, 90, STAGE_BEGIN, 6, MISSING),
        flush(7, 300, STAGE_END, 6, "210"),
        msg(7, 120, MSG_DPICHANGED, "0x1"),
        enqueue(
            7,
            50,
            "0x1",
            PlacementRoute::BalloonFollow.as_str(),
            "1",
            balloon_kind(),
        ),
        surface(
            7,
            13,
            SURFACE_STAGE_UPLOAD,
            0,
            "382",
            "547",
            "true",
            MISSING,
        ),
        surface(
            7,
            14,
            SURFACE_STAGE_SKIPPED,
            3,
            MISSING,
            MISSING,
            MISSING,
            SURFACE_REASON_INVISIBLE,
        ),
        ground(7, 0, 1704, 1752, PlacementRoute::DpiReproject.as_str()),
        hold(7, 0, char_kind(), HOLD_DECISION_PROCEED, HOLD_SITE_DPI),
        chain(7, CHAIN_STAGE_REALIGNED, 2, 2),
    ] {
        parse_ok(&line);
    }
}

#[test]
fn parse_ignores_lines_without_the_record_tag() {
    for line in [
        "2026-08-15T11:54:14.327Z  INFO areka: ゴーストの起動を開始",
        "[diag.window_move] route=DpiReproject entity=6v0 kind=char scope=1 x=1560 y=1304",
        "perf apply t_total_us=12382 upload_us=8563 frame=41231",
        "",
    ] {
        assert!(
            parse_transition_line(line).is_none(),
            "観測チャネル外の行を数えてはならない: {line}"
        );
    }
}

#[test]
fn parse_reads_the_prefix_and_the_body_of_a_monitor_line() {
    let record = parse_ok(&monitor(41231, 192, 96, 1704, 1752));
    assert_eq!(record.stamp.frame, 41231);
    assert_eq!(record.kind, KIND_MONITOR);
    assert_eq!(record.field(FIELD_OLD_DPI), Some("192"));
    assert_eq!(record.int_field::<u32>(FIELD_NEW_DPI), Some(96));
    assert_eq!(record.field(FIELD_NEW_WA), Some("0,0,2880,1752"));
}

#[test]
fn parse_accepts_a_subscriber_prefix_before_the_record_tag() {
    // 実機ログは行頭に時刻・水準・target が付く。タグ以降だけを読む。
    let line = format!(
        "2026-08-15T11:54:14.327123Z DEBUG wintf::transition: {}",
        monitor(41231, 192, 96, 1704, 1752)
    );
    let record = parse_ok(&line);
    assert_eq!(record.stamp.frame, 41231);
    assert_eq!(record.kind, KIND_MONITOR);
}

#[test]
fn parse_reads_the_sentinel_as_an_absent_value() {
    let record = parse_ok(&surface(
        7,
        14,
        SURFACE_STAGE_SKIPPED,
        3,
        MISSING,
        MISSING,
        MISSING,
        SURFACE_REASON_INVISIBLE,
    ));
    assert_eq!(record.raw_field(SURFACE_FIELD_W), Some(MISSING));
    assert_eq!(
        record.field(SURFACE_FIELD_W),
        None,
        "番兵は「値が無い」として読む"
    );
    assert_eq!(record.int_field::<u32>(SURFACE_FIELD_W), None);
    assert_eq!(record.field(SURFACE_FIELD_H), None);
}

#[test]
fn parse_treats_a_repeated_field_name_as_malformed_instead_of_last_wins() {
    // judge-perf.py の辞書化は後勝ちだが、判定器は「1 行に同名を 2 度出さない」を
    // 語彙の不変条件として扱う。後勝ちで飲み込むと接頭語の kind= が潰される退行が消える。
    let line = format!(
        "{} {FIELD_SCOPE}=9",
        write(
            7,
            100,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            5
        )
    );
    let record = parse_transition_line(&line).expect("タグを含む行は解析結果を返す");
    assert!(
        record
            .defects
            .contains(&RecordDefect::DuplicateField(FIELD_SCOPE.to_owned())),
        "同名フィールドの重複が欠陥として出るはず: {:?}",
        record.defects
    );
    assert_eq!(
        record.field(FIELD_SCOPE),
        Some("0"),
        "先着の値が後勝ちで消えてはならない"
    );
}

#[test]
fn parse_reports_a_missing_required_field() {
    // `win_kind=` を落とした書込行（窓種別が判定側から消える形）。
    let full = write(
        7,
        100,
        STAGE_FLUSH,
        0,
        "0x1",
        ORIGIN_WINDOW_POS,
        "0",
        char_kind(),
        5,
    );
    let broken = full.replace(&format!("{FIELD_WIN_KIND}={}", char_kind()), "");
    let record = parse_transition_line(&broken).expect("タグを含む行は解析結果を返す");
    assert!(
        record
            .defects
            .contains(&RecordDefect::MissingField(FIELD_WIN_KIND.to_owned())),
        "必須フィールドの欠落が欠陥として出るはず: {:?}",
        record.defects
    );
}

#[test]
fn parse_reports_an_unknown_kind_word() {
    let line = record(7, 0, "monitr", "entity=2v0");
    let parsed = parse_transition_line(&line).expect("タグを含む行は解析結果を返す");
    assert!(
        parsed
            .defects
            .contains(&RecordDefect::UnknownKind("monitr".to_owned())),
        "未知の種別語が欠陥として出るはず: {:?}",
        parsed.defects
    );
}

#[test]
fn required_fields_cover_every_kind_word_the_three_crates_define() {
    // 語彙の定義元は 3 箇所（wintf・emo-present・areka）。どれか 1 語でも表から漏れると
    // その種別の行が「未知の種別」として全件欠陥になり、判定が静かに空振りする。
    for kind in KIND_ALL
        .iter()
        .chain(std::iter::once(&KIND_SURFACE))
        .chain(PLACEMENT_KIND_ALL.iter())
    {
        assert!(
            required_fields(kind).is_some(),
            "必須フィールド表に載っていない種別語: {kind}"
        );
    }
}

// ---------------------------------------------------------------------------
// 遷移の切り出し
// ---------------------------------------------------------------------------

#[test]
fn split_starts_a_transition_at_each_dpi_changing_monitor_record() {
    let lines = [
        monitor(10, 192, 96, 1704, 1752),
        write(
            10,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
        monitor(20, 96, 192, 1752, 1704),
        write(
            20,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
    ];
    let records = parse_transition_log(&lines.join("\n"));
    let transitions = split_transitions(&records);
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].len(), 2);
    assert_eq!(transitions[1].len(), 2);
    assert_eq!(transitions[0][0].stamp.frame, 10);
    assert_eq!(transitions[1][0].stamp.frame, 20);
}

#[test]
fn split_does_not_start_a_transition_when_only_the_work_area_changed() {
    // 作業領域だけの更新を起点にすると遷移が水増しされ、充足判定（各方向 3 回以上）が甘くなる。
    let work_area_only = monitor(10, 96, 96, 1704, 1752);
    let record = parse_ok(&work_area_only);
    assert!(!is_transition_origin(&record));
    assert!(split_transitions(&[record]).is_empty());
}

#[test]
fn split_drops_records_before_the_first_origin() {
    let lines = [
        write(
            1,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
        monitor(10, 192, 96, 1704, 1752),
        write(
            10,
            2,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
    ];
    let records = parse_transition_log(&lines.join("\n"));
    let transitions = split_transitions(&records);
    assert_eq!(transitions.len(), 1);
    assert_eq!(
        transitions[0].len(),
        2,
        "起点より前の起動時書込は遷移に入らない"
    );
    assert_eq!(summarize(&transitions[0]).writes, 1);
}

// ---------------------------------------------------------------------------
// 判定量の集計
// ---------------------------------------------------------------------------

#[test]
fn summarize_keys_writes_by_the_win_kind_field_not_the_record_kind() {
    // 行の上では `kind=write` と `win_kind=char` が別のフィールドである（同名にすると
    // レコード種別が消える）。窓ごとの回数は後者で引く。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(
            10,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            1,
        ),
        write(
            10,
            2,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::KeepPositionResize.as_str(),
            "0",
            balloon_kind(),
            1,
        ),
        write(
            10,
            3,
            STAGE_FLUSH,
            2,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            balloon_kind(),
            1,
        ),
    ]);
    assert_eq!(summary.writes, 3);
    assert_eq!(
        summary
            .writes_per_window
            .get(&WindowKey::of(0, WindowKind::Char)),
        Some(&1)
    );
    assert_eq!(
        summary
            .writes_per_window
            .get(&WindowKey::of(0, WindowKind::Balloon)),
        Some(&2)
    );
    assert!(
        !summary
            .writes_per_window
            .keys()
            .any(|key| key.kind == KIND_WRITE),
        "レコード種別語を窓種別として拾ってはならない"
    );
}

#[test]
fn summarize_counts_path_a_writes_by_origin_and_corroborates_with_the_stage() {
    // 経路 A は `origin=dpi-suggested` で数える（`stage=sync` は裏取り）。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(
            10,
            1,
            STAGE_SYNC,
            0,
            "0x1",
            ORIGIN_DPI_SUGGESTED,
            "0",
            char_kind(),
            1,
        ),
        write(
            10,
            2,
            STAGE_FLUSH,
            1,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            1,
        ),
    ]);
    assert_eq!(summary.path_a_writes, 1);
    assert_eq!(summary.sync_stage_writes, 1);

    // 札と段が食い違う行は、両方の量に別々に現れる（片方だけ見て 0 と結論しないため）。
    let diverged = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(
            10,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_DPI_SUGGESTED,
            "0",
            char_kind(),
            1,
        ),
    ]);
    assert_eq!(diverged.path_a_writes, 1);
    assert_eq!(diverged.sync_stage_writes, 0);
}

#[test]
fn summarize_maps_the_surface_target_id_to_a_scope_and_a_kind() {
    // 採番の正本は `emo2_boot::target_map`（shell = 2·scope／balloon = 2·scope+1）であり、
    // 判定器が持つのはその**逆写像**である。数値をここへ書き直すと、正本が
    // 採番を変えたときに判定器だけが反転したまま静かに緑になるので、正本の関数を呼んで往復させる。
    for scope in 0..3 {
        assert_eq!(
            WindowKey::from_target_id(shell_target(scope).0),
            WindowKey::of(scope, WindowKind::Char),
            "シェル表示対象（偶数側）はキャラ窓へ対応する"
        );
        assert_eq!(
            WindowKey::from_target_id(balloon_target(scope).0),
            WindowKey::of(scope, WindowKind::Balloon)
        );
    }
}

#[test]
fn summarize_reports_the_visualize_to_write_gap_in_frames_and_in_microseconds() {
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        surface(
            10,
            19_300,
            SURFACE_STAGE_VISUALIZE,
            2,
            "336",
            "400",
            MISSING,
            MISSING,
        ),
        write(
            10,
            181_000,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "1",
            char_kind(),
            57_200,
        ),
        flush(10, 271_500, STAGE_END, 6, "171000"),
    ]);
    let char1 = WindowKey::of(1, WindowKind::Char);
    assert_eq!(
        summary.mismatch_frames_per_window.get(&char1),
        Some(&0),
        "同一フレームなのでフレーム差は 0（是正前でも 0＝実機専用量が要る理由）"
    );
    assert_eq!(
        summary.visualize_to_write_us.get(&char1),
        Some(&161_700),
        "同一フレーム内の食い違いは µs でしか見えない"
    );
    assert_eq!(summary.flush_total_us_max, Some(171_000));
}

#[test]
fn summarize_excludes_windows_whose_redisplay_was_skipped() {
    // 要件 4.6: 寸の再導出結果が得られない窓は現状維持であり、合否から除外する。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        surface(
            10,
            5,
            SURFACE_STAGE_VISUALIZE,
            2,
            "336",
            "400",
            MISSING,
            MISSING,
        ),
        surface(
            10,
            6,
            SURFACE_STAGE_SKIPPED,
            3,
            MISSING,
            MISSING,
            MISSING,
            SURFACE_REASON_INVISIBLE,
        ),
        write(
            10,
            100,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "1",
            char_kind(),
            1,
        ),
        // 見送られた窓が後のフレームで表示され、そこで初めて新寸になる形。可視化も書込も
        // **在る**ので、除外していなければ食い違いの表へ載ってしまう。
        //
        // ここで遅れているのは**随伴の位置（`BalloonFollow`）そのもの**である＝要件 4.3 の
        // 欠陥形。再観測 §3.2 の遷移 2 とは**別物**なので混同しないこと——あちらは位置が
        // 定刻に届き寸（`KeepPositionResize`）だけが +660ms に届いた形で、レポートが
        // 「Requirement 4.6 の現状維持挙動＝欠陥ではない」と明記している。両者を分けるのは
        // 遅れた書込の**経路語**だけであり、§3.2 の形は
        // `transition_judge_verdict_tests::a_deferred_resize_after_a_punctual_follow_is_not_a_companion_defect`
        // が持つ。
        surface(
            20,
            5,
            SURFACE_STAGE_VISUALIZE,
            3,
            "288",
            "203",
            MISSING,
            MISSING,
        ),
        write(
            20,
            200,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "1",
            balloon_kind(),
            1,
        ),
    ]);
    let balloon1 = WindowKey::of(1, WindowKind::Balloon);
    assert!(summary.skipped_windows.contains(&balloon1));
    assert_eq!(
        summary
            .writes_per_window
            .get(&balloon1)
            .copied()
            .unwrap_or_default(),
        1,
        "書込そのものは数える（除外するのは食い違いと随伴の判定だけ）"
    );
    assert!(
        !summary.mismatch_frames_per_window.contains_key(&balloon1),
        "見送り窓を食い違いの判定へ数えてはならない"
    );
    assert!(
        !summary.visualize_to_write_us.contains_key(&balloon1),
        "実機専用の量も見送り窓では測らない"
    );
    assert!(
        summary
            .mismatch_frames_per_window
            .contains_key(&WindowKey::of(1, WindowKind::Char)),
        "見送られていない窓は従来どおり測る（除外が広がりすぎていないこと）"
    );
    // 随伴の同一フレーム性だけは**バルーン側の見送りでは降りない**（task 3.2 のレビューで
    // 是正）。見送りは「サーフェスの再表示をしなかった」という事実であって、随伴の窓書込は
    // 不可視のバルーンにも出る——ここで降りると、実機サインオフの定常の形（発話させないので
    // バルーンは常に `stage=skipped reason=invisible`）で要件 4.3 が 1 度も検査されない。
    // 本例はまさにその欠陥形（キャラは frame 10・随伴は frame 20）であり、検査される。
    assert_eq!(
        summary.balloon_pairs_checked, 1,
        "両窓とも書込があるので対は検査できる（キャラ側は `last_write_frame_per_window`・バルーン側は `last_follow_frame_per_window` 由来）"
    );
    assert!(
        !summary.balloon_same_frame,
        "随伴が 10 フレーム遅れて動いたことは要件 4.3 の違反として見えなければならない"
    );
}

#[test]
fn summarize_reads_the_flush_total_only_from_the_end_of_the_interval() {
    // 判定器の入力は外から来る文字列であり、発行側の規約（`stage=begin` の総所要は番兵）を
    // 満たさない行も届き得る。段で選り分けていなければ、開始行の値が総所要として通る。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        flush(10, 90, STAGE_BEGIN, 6, "999999"),
        flush(10, 300, STAGE_END, 6, "171000"),
    ]);
    assert_eq!(
        summary.flush_total_us_max,
        Some(171_000),
        "開始行の値を総所要として拾ってはならない"
    );
}

#[test]
fn summarize_keeps_the_first_write_time_instead_of_the_latest() {
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(
            10,
            103_000,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            1,
        ),
        write(
            10,
            255_000,
            STAGE_FLUSH,
            1,
            "0x2",
            ORIGIN_WINDOW_POS,
            "0",
            balloon_kind(),
            1,
        ),
        write(
            10,
            271_000,
            STAGE_FLUSH,
            2,
            "0x2",
            ORIGIN_WINDOW_POS,
            "0",
            balloon_kind(),
            1,
        ),
    ]);
    assert_eq!(summary.wall.first_write_t_us, Some(103_000));
    assert_eq!(summary.wall.last_write_t_us, Some(271_000));
}

#[test]
fn summarize_excludes_a_skipped_char_window_from_the_companion_check() {
    // 見送りはキャラ窓側にも起こり得る（不可視のキャラ窓）。対の片側が除外なら
    // 随伴の同一フレーム性は検査しない——検査した上で「一致」と答えると、
    // 測っていない対を合格の根拠にすることになる。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        surface(
            10,
            6,
            SURFACE_STAGE_SKIPPED,
            0,
            MISSING,
            MISSING,
            MISSING,
            SURFACE_REASON_INVISIBLE,
        ),
        write(
            10,
            100,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            1,
        ),
        write(
            11,
            200,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            balloon_kind(),
            1,
        ),
    ]);
    assert!(
        summary
            .skipped_windows
            .contains(&WindowKey::of(0, WindowKind::Char))
    );
    assert_eq!(summary.balloon_pairs_checked, 0);
    assert!(summary.balloon_same_frame);
}

#[test]
fn summarize_flags_a_companion_balloon_landing_in_another_frame() {
    let same = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(
            11,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            1,
        ),
        write(
            11,
            2,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            balloon_kind(),
            1,
        ),
    ]);
    assert_eq!(same.balloon_pairs_checked, 1);
    assert!(same.balloon_same_frame);

    let split_frame = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(
            11,
            1,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            char_kind(),
            1,
        ),
        write(
            12,
            2,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            balloon_kind(),
            1,
        ),
    ]);
    assert_eq!(split_frame.balloon_pairs_checked, 1);
    assert!(!split_frame.balloon_same_frame);
    assert_eq!(split_frame.frames_to_last_write, Some(2));
}

#[test]
fn summarize_keeps_the_sign_of_the_largest_ground_difference() {
    // 浮き（負）と沈み（正）が混ざっても、絶対値が最大のものを符号つきで持つ。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        ground(10, 0, 1704, 1752, PlacementRoute::DpiReproject.as_str()),
        ground(10, 1, 1760, 1752, PlacementRoute::DpiReproject.as_str()),
    ]);
    assert_eq!(summary.ground_diff_max, Some(-48));
}

#[test]
fn summarize_counts_holds_and_chain_realignments_by_their_own_words() {
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        hold(10, 0, char_kind(), HOLD_DECISION_HOLD, HOLD_SITE_DPI),
        hold(10, 1, char_kind(), HOLD_DECISION_PROCEED, HOLD_SITE_DPI),
        chain(11, CHAIN_STAGE_DEFERRED, 2, 0),
        chain(12, CHAIN_STAGE_REALIGNED, 2, 2),
    ]);
    assert_eq!(summary.holds, 1, "整合待ちは decision=hold だけを数える");
    assert_eq!(
        summary.chain_realigned, 1,
        "解き直しは stage=realigned だけを数える"
    );
}

#[test]
fn summarize_collects_wall_clock_values_as_reference_only() {
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(
            10,
            103_000,
            STAGE_FLUSH,
            0,
            "0x1",
            ORIGIN_WINDOW_POS,
            "0",
            char_kind(),
            20_500,
        ),
        write(
            10,
            255_000,
            STAGE_FLUSH,
            1,
            "0x2",
            ORIGIN_WINDOW_POS,
            "0",
            balloon_kind(),
            16_200,
        ),
    ]);
    assert_eq!(summary.wall.first_write_t_us, Some(103_000));
    assert_eq!(summary.wall.last_write_t_us, Some(255_000));
    assert_eq!(summary.wall.sum_call_us, 36_700);
}

#[test]
fn summarize_counts_malformed_records_instead_of_dropping_them() {
    let mut lines = vec![monitor(10, 192, 96, 1704, 1752)];
    lines.push(record(10, 1, "surfac", "stage=upload"));
    let summary = summarize_lines(&lines);
    assert_eq!(summary.records, 2);
    assert_eq!(
        summary.malformed_records, 1,
        "壊れた行を落とすと「判定語を壊した」が 0 件として合格に見える"
    );
}

#[test]
fn summarize_keeps_an_untagged_write_visible_under_the_sentinel_key() {
    // 札を付け忘れた経路の書込を落とすと、回数の判定が静かに緩む。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        write(10, 1, STAGE_FLUSH, 0, "0x1", MISSING, MISSING, MISSING, 1),
    ]);
    assert_eq!(summary.writes, 1);
    assert_eq!(
        summary.writes_per_window.get(&WindowKey {
            scope: None,
            kind: MISSING.to_owned(),
        }),
        Some(&1)
    );
}

#[test]
fn summarize_ignores_records_that_are_not_part_of_the_judged_quantities() {
    // msg／enqueue は台帳の裏取り用であり、判定量そのものは動かさない。
    let summary = summarize_lines(&[
        monitor(10, 192, 96, 1704, 1752),
        msg(10, 124_000, MSG_DPICHANGED, "0x1"),
        enqueue(
            10,
            13,
            "0x1",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            balloon_kind(),
        ),
    ]);
    assert_eq!(summary.records, 3);
    assert_eq!(summary.writes, 0);
    assert_eq!(summary.frames_to_last_write, None);
    assert_eq!(summary.malformed_records, 0);
    assert!(
        summary
            .origin
            .is_some_and(|origin| origin.old_dpi == 192 && origin.new_dpi == 96),
        "起点の新旧 DPI は要約が持つ"
    );
    assert_ne!(KIND_MSG, KIND_MONITOR);
}
