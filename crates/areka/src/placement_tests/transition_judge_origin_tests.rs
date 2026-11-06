//! 起点集合の**和集合**と、同じ変化を指す起点の**畳み込み**（task 8.4・要件 8.2〜8.5）。
//!
//! 判定器の起点は `kind=monitor`（モニタ表の値変化＝表示設定の変更）と `kind=windpi`
//! （窓の表示 DPI の書き換え＝モニタ間の移動）の 2 種別である。本ファイルは
//! [`super::split_transitions`] の 3 通り——⑴ 両方が出る ⑵ 新起点だけが出る
//! ⑶ 既存起点だけが出る——を踏み、畳み込みを
//!
//! - **水増ししない**（1 つの変化から出た複数の起点行を複数の遷移に数えない）
//! - **畳みすぎない**（別フレーム・別の拡大率変化を 1 本へ吸い込まない）
//!
//! の両側から檻に入れる。片側だけだと、畳み込みを外しても（水増し側だけが赤くなる）
//! あるいは何もかも畳んでも（畳みすぎ側だけが赤くなる）どちらか一方は静かに通る。

use wintf::ecs::window::transition_diag::{
    FIELD_KIND, FIELD_NEW_DPI, FIELD_OLD_DPI, KIND_MONITOR, KIND_WINDPI, MSG_DPICHANGED,
    RECORD_PREFIX_TAG,
};

use super::test_support::{monitor, msg, windpi};
use super::{
    is_transition_origin, parse_transition_line, parse_transition_log, split_transitions, summarize,
};

/// 遷移ごとの `(old_dpi, new_dpi)`（起点を読めない遷移は落ちる）。
fn directions(log: &str) -> Vec<(u32, u32)> {
    let records = parse_transition_log(log);
    split_transitions(&records)
        .iter()
        .filter_map(|span| summarize(span).origin)
        .map(|origin| (origin.old_dpi, origin.new_dpi))
        .collect()
}

// ---------------------------------------------------------------------------
// 起点集合の和集合
// ---------------------------------------------------------------------------

#[test]
fn the_windpi_builder_produces_a_well_formed_record() {
    // 以降の全テストの前提（必須フィールドの載せ漏れは語彙の欠陥として立つ）。
    let record = parse_transition_line(&windpi(10, "42v1", 192, 144)).expect("解析できるはず");
    assert!(record.is_well_formed(), "{:?}", record.defects);
    assert_eq!(record.kind, KIND_WINDPI);
}

#[test]
fn a_window_dpi_change_is_an_origin_on_its_own() {
    // ⑵ 新起点だけが出る場合＝モニタ間の往復。`kind=monitor` は 1 行も出ない。
    let record = parse_transition_line(&windpi(10, "42v1", 192, 144)).expect("解析できるはず");
    assert!(is_transition_origin(&record));
    assert_eq!(directions(&windpi(10, "42v1", 192, 144)), [(192, 144)]);
}

#[test]
fn a_window_dpi_record_that_did_not_change_the_scale_is_not_an_origin() {
    // 経路 1（`WM_DPICHANGED`）は component への代入が無条件なので、OS が同値の DPI を
    // 運ぶと `old_dpi == new_dpi` の行が出得る。起点に採ると遷移が水増しされる。
    let record = parse_transition_line(&windpi(10, "42v1", 192, 192)).expect("解析できるはず");
    assert!(!is_transition_origin(&record));
    assert!(split_transitions(&[record]).is_empty());
}

#[test]
fn a_monitor_table_change_is_still_an_origin_on_its_own() {
    // ⑶ 既存起点だけが出る場合（既存の規約を 1 つも変えていないことの確認）。
    assert_eq!(directions(&monitor(10, 96, 192, 1752, 1704)), [(96, 192)]);
}

// ---------------------------------------------------------------------------
// 畳み込み: 水増しさせない側
// ---------------------------------------------------------------------------

#[test]
fn the_monitor_and_window_origins_of_one_display_setting_change_form_one_transition() {
    // ⑴ 両方が出る場合。表示設定の変更では `detect_display_change_system` が `monitor` を
    // 出し、同じ処理の中で窓の DPI を引き直して `windpi` も出す——1 つの変化から起点が
    // 複数行。畳まないと遷移が本数だけ化ける。
    let lines = [
        monitor(10, 96, 192, 1752, 1704),
        windpi(10, "42v1", 96, 192),
        windpi(10, "43v1", 96, 192),
    ];
    let log = lines.join("\n");
    let records = parse_transition_log(&log);
    let transitions = split_transitions(&records);

    assert_eq!(transitions.len(), 1, "1 つの変化＝1 本の遷移");
    assert_eq!(transitions[0].len(), 3, "畳まれた起点の行も遷移に属する");
    assert_eq!(directions(&log), [(96, 192)]);
}

#[test]
fn several_windows_crossing_together_form_one_transition() {
    // ⑵ の水増し側。モニタ間の移動ではキャラ窓とバルーン窓がそれぞれ `windpi` を出す。
    // 畳まないと**窓の数だけ**遷移が増える。
    let log = [
        windpi(10, "42v1", 192, 144),
        windpi(10, "43v1", 192, 144),
        windpi(10, "44v1", 192, 144),
        windpi(10, "45v1", 192, 144),
    ]
    .join("\n");
    assert_eq!(directions(&log), [(192, 144)]);
}

#[test]
fn other_records_between_two_origins_of_the_same_change_do_not_break_the_coalescing() {
    // 同一フレームの内側では、起点のあいだに別種別の行（メッセージ受理など）が挟まる。
    // 挟まっても同じ変化であることは変わらない。
    let log = [
        monitor(10, 96, 192, 1752, 1704),
        msg(10, 5, MSG_DPICHANGED, "0x1"),
        windpi(10, "42v1", 96, 192),
    ]
    .join("\n");
    assert_eq!(directions(&log), [(96, 192)]);
}

// ---------------------------------------------------------------------------
// 畳み込み: 畳みすぎさせない側
// ---------------------------------------------------------------------------

#[test]
fn the_same_scale_change_in_a_later_frame_is_a_new_transition() {
    // 対が同じでもフレームが違えば別の変化である。ここを畳むと、往復を繰り返したログが
    // 1 本の遷移に潰れて「往復が 1 度も観測されていない」の偽の赤になる。
    let log = [windpi(10, "42v1", 192, 144), windpi(11, "42v1", 192, 144)].join("\n");
    assert_eq!(directions(&log), [(192, 144), (192, 144)]);
}

#[test]
fn the_opposite_direction_in_the_same_frame_is_a_new_transition() {
    // 同一フレームでも対が違えば別の変化である（畳むと往復の片道が消える）。
    let log = [windpi(10, "42v1", 192, 144), windpi(10, "42v1", 144, 192)].join("\n");
    assert_eq!(directions(&log), [(192, 144), (144, 192)]);
}

#[test]
fn frames_are_compared_by_difference_so_the_counter_may_wrap() {
    // D14: `frame` は u32 で周回する。差分で見るかぎり `u32::MAX` の次の `0` は
    // 「同一フレーム」ではない（絶対値の大小比較を判定語に使っていないことの裏取り）。
    let log = [
        windpi(u32::MAX, "42v1", 192, 144),
        windpi(0, "42v1", 192, 144),
    ]
    .join("\n");
    assert_eq!(directions(&log), [(192, 144), (192, 144)]);
}

// ---------------------------------------------------------------------------
// 充足判定が畳み込みの前後で水増しされない（逐語ログ）
// ---------------------------------------------------------------------------

/// 表示設定の変更で 3 往復したログの逐語（起点行だけを抜き出した形）。
///
/// 1 つの変化につき `monitor` 1 行 ＋ 窓 4 枚ぶんの `windpi` 4 行＝**5 行**が出る。
/// 起点行を素朴に数えると 30 本だが、拡大率の変化は 6 回である。
const VERBATIM_ROUND_TRIP_LOG: &str = "\
DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=monitor entity=2v0 old_dpi=192 new_dpi=144 old_wa=0,0,2880,1704 new_wa=0,0,2880,1704
DEBUG wintf::transition: [transition] frame=10 t_us=1 kind=windpi entity=42v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=10 t_us=2 kind=windpi entity=43v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=10 t_us=3 kind=windpi entity=44v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=10 t_us=4 kind=windpi entity=45v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=20 t_us=0 kind=monitor entity=2v0 old_dpi=144 new_dpi=192 old_wa=0,0,2880,1704 new_wa=0,0,2880,1704
DEBUG wintf::transition: [transition] frame=20 t_us=1 kind=windpi entity=42v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=20 t_us=2 kind=windpi entity=43v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=20 t_us=3 kind=windpi entity=44v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=20 t_us=4 kind=windpi entity=45v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=30 t_us=0 kind=monitor entity=2v0 old_dpi=192 new_dpi=144 old_wa=0,0,2880,1704 new_wa=0,0,2880,1704
DEBUG wintf::transition: [transition] frame=30 t_us=1 kind=windpi entity=42v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=30 t_us=2 kind=windpi entity=43v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=30 t_us=3 kind=windpi entity=44v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=30 t_us=4 kind=windpi entity=45v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=40 t_us=0 kind=monitor entity=2v0 old_dpi=144 new_dpi=192 old_wa=0,0,2880,1704 new_wa=0,0,2880,1704
DEBUG wintf::transition: [transition] frame=40 t_us=1 kind=windpi entity=42v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=40 t_us=2 kind=windpi entity=43v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=40 t_us=3 kind=windpi entity=44v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=40 t_us=4 kind=windpi entity=45v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=50 t_us=0 kind=monitor entity=2v0 old_dpi=192 new_dpi=144 old_wa=0,0,2880,1704 new_wa=0,0,2880,1704
DEBUG wintf::transition: [transition] frame=50 t_us=1 kind=windpi entity=42v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=50 t_us=2 kind=windpi entity=43v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=50 t_us=3 kind=windpi entity=44v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=50 t_us=4 kind=windpi entity=45v1 old_dpi=192 new_dpi=144
DEBUG wintf::transition: [transition] frame=60 t_us=0 kind=monitor entity=2v0 old_dpi=144 new_dpi=192 old_wa=0,0,2880,1704 new_wa=0,0,2880,1704
DEBUG wintf::transition: [transition] frame=60 t_us=1 kind=windpi entity=42v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=60 t_us=2 kind=windpi entity=43v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=60 t_us=3 kind=windpi entity=44v1 old_dpi=144 new_dpi=192
DEBUG wintf::transition: [transition] frame=60 t_us=4 kind=windpi entity=45v1 old_dpi=144 new_dpi=192
";

#[test]
fn the_verbatim_fixture_uses_the_current_vocabulary() {
    // 逐語の字面は発行側の語彙から離れ得る。離れたら以下の檻は「起点が 1 つも無いログ」を
    // 測ることになり、水増しの検査が空振りする。
    for word in [KIND_MONITOR, KIND_WINDPI] {
        assert!(
            VERBATIM_ROUND_TRIP_LOG.contains(&format!("{FIELD_KIND}={word}")),
            "{word} の行が逐語ログから消えている"
        );
    }
    for name in [FIELD_OLD_DPI, FIELD_NEW_DPI] {
        assert!(VERBATIM_ROUND_TRIP_LOG.contains(&format!("{name}=")));
    }
    assert!(VERBATIM_ROUND_TRIP_LOG.contains(RECORD_PREFIX_TAG));
    let records = parse_transition_log(VERBATIM_ROUND_TRIP_LOG);
    assert_eq!(records.len(), 30, "起点行 30 本（変化 6 回 × 5 行）");
    assert!(records.iter().all(|record| record.is_well_formed()));
}

#[test]
fn the_round_trip_count_is_not_inflated_by_the_duplicated_origins() {
    // 起点行は 30 本あるが変化は 6 回である。畳まないと各方向 15 回と数えられ、報告される
    // 遷移本数と、手順書が運用者に目で確かめさせる往復の本数が実態の 5 倍で刷られる。
    let observed = directions(VERBATIM_ROUND_TRIP_LOG);
    assert_eq!(
        observed,
        [
            (192, 144),
            (144, 192),
            (192, 144),
            (144, 192),
            (192, 144),
            (144, 192)
        ],
        "遷移は 6 本（起点行 30 本ではない）"
    );

    let down = observed.iter().filter(|d| **d == (192, 144)).count();
    let up = observed.iter().filter(|d| **d == (144, 192)).count();
    assert_eq!(
        (down, up),
        (3, 3),
        "各方向 3 回（起点行を数えた 15 回ではない）"
    );
}

#[test]
fn every_origin_line_of_the_verbatim_log_is_recognized_as_an_origin_candidate() {
    // 畳み込みを「`windpi` を起点に採らない」で実現していたら、遷移の本数は 6 本のまま
    // 緑になる。候補として認められていること自体を別に測り、その抜け道を塞ぐ。
    let records = parse_transition_log(VERBATIM_ROUND_TRIP_LOG);
    assert_eq!(
        records.iter().filter(|r| is_transition_origin(r)).count(),
        30
    );
    assert_eq!(
        records
            .iter()
            .filter(|r| r.kind == KIND_WINDPI && is_transition_origin(r))
            .count(),
        24
    );
}
