//! 遷移観測チャネル（`transition_diag`）の**語彙**に対する決定論的テスト。
//!
//! 本ファイルが固定するのは 4 つである。
//!
//! - **行の逐語書式**——5 種のレコードが design.md「Data Models > Logical Data Model
//!   （レコード語彙）」の表どおりの `kind` 語・`stage` 語・フィールド名・並びで 1 行に載ること。
//!   判定側（別 crate の `transition_judge`・サインオフ手順書の grep 語）は**この行の字面**を
//!   直接読むため、字面が黙って変わると手順と判定が同時に嘘になる。
//! - **欠損の扱い**——値が無いフィールドは番兵 `-` になり、**フィールドごと消えない**。
//!   消すと「記録が出ていない」と「その経路にはその値が無い」の区別が事後に付かない。
//! - **辞書化可能性**——`tools/perf/judge-perf.py::parse_fields` と同じ規則（`名前=値` の
//!   並び・値は次の名前の直前まで）で読めること、および**同じ名前が 1 行に 2 度出ない**こと。
//!   接頭語の `kind=`（レコード種別）と書込タグの窓種別が同名だと後者が前者を上書きして
//!   しまうため、行に載る窓種別は `win_kind=` である（本ファイルがその決定を固定する）。
//! - **既定 OFF**——既定のログ設定では 1 行も出ず、専用 target を指定したときだけ出ること。
//!   定数の目視ではなく [`capture_under_filter`] による**実濾過**で確かめる。
//!
//! # 「出ないこと」を主張するときの作法
//!
//! 「この設定では出ない」の主張は、捕捉そのものが死んでいても成立してしまう。よって
//! 出ないことを見るテストには、**同じ捕捉窓の中で確かに拾える記録**（info 水準の対照）を
//! 併置し、捕捉が生きている証拠を毎回同じ出力から取る。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::Instant;

use bevy_ecs::prelude::*;

/// 本番の `World` 構築の逐語。`Update` が既定の実行器のまま挿入されていることを字面で
/// 見張るために読む（bevy 0.19 で `Schedule::get_executor_kind` が撤去されたため）。
const WORLD_CONSTRUCTION_SRC: &str = include_str!("../world/mod.rs");
use windows::Win32::Foundation::{HWND, RECT};

use super::{
    ENQUEUE_FIELDS, EnqueueRecord, FIELD_ENTITY, FIELD_KIND, FIELD_NEW_DPI, FIELD_OLD_DPI,
    FIELD_OLD_WA, FLUSH_FIELDS, FlushRecord, FlushStage, KIND_ALL, KIND_ENQUEUE, KIND_FLUSH,
    KIND_MONITOR, KIND_MSG, KIND_WINDPI, KIND_WRITE, MONITOR_FIELDS, MSG_DPICHANGED, MSG_FIELDS,
    MonitorRecord, MsgRecord, STAGE_ALL, Stamp, TRANSITION_TARGET, TickStart, WINDPI_FIELDS,
    WRITE_FIELDS, WRITE_OPTIONAL_FIELDS, WindowDpiRecord, WriteRecord, WriteStage, WriteTag,
    begin_flush, begin_tick, current_frame, emit_line, enqueue_line, flush_line, is_enabled,
    monitor_line, msg_line, record_prefix, reset_for_test, since_flush_us, since_tick_start_us,
    stamp, stamp_from_world, windpi_line, write_line,
};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::world::{EcsWorld, FrameCount, Update};

/// 実機サインオフが用いる `RUST_LOG` 相当のうち、本チャネルを点灯させる directive。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::transition=debug";

/// 既定水準（観測チャネルを有効化していない通常運転）。
const DEFAULT_DIRECTIVES: &str = "info";

/// 逐語比較に使う固定の刻印（純関数は時刻を読まないので任意の値で組める）。
const STAMP: Stamp = Stamp {
    frame: 7,
    t_us: 1234,
};

fn entity(index: u32) -> Entity {
    Entity::from_raw_u32(index).expect("valid test entity index")
}

fn hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

/// 本仕様が採る書込タグ（origin＝要求経路語・scope＝キャラ番号・kind＝窓種別）。
fn tag() -> WriteTag {
    WriteTag {
        origin: "DpiReproject",
        scope: Some(0),
        kind: "shell",
    }
}

// ---------------------------------------------------------------------------
// judge-perf.py と同じ辞書化規則の再実装（判定側が読めることの検査に使う）
// ---------------------------------------------------------------------------

/// `tools/perf/judge-perf.py::parse_fields` と同じ規則で `名前=値` を切り出す。
///
/// 正規表現 `(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=` に相当する走査を手で書いてある。
/// 判定側と**同じ規則**で読めることを見るのが目的なので、規則を緩めてはならない。
fn field_keys(line: &str) -> Vec<(String, usize, usize)> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let at_boundary = i == 0 || bytes[i - 1].is_ascii_whitespace();
        let starts_ident = bytes[i].is_ascii_alphabetic() || bytes[i] == b'_';
        if at_boundary && starts_ident {
            let key_start = i;
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                out.push((line[key_start..j].to_string(), key_start, j + 1));
                i = j + 1;
                continue;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

/// 行を辞書へ（後勝ち＝judge-perf.py と同じ）。
fn parse_fields(line: &str) -> BTreeMap<String, String> {
    let keys = field_keys(line);
    let mut fields = BTreeMap::new();
    for (idx, (name, _, value_start)) in keys.iter().enumerate() {
        let end = keys.get(idx + 1).map_or(line.len(), |next| next.1);
        fields.insert(name.clone(), line[*value_start..end].trim().to_string());
    }
    fields
}

/// 判定側が行う照合の最小形——`kind` が期待の語で、必須フィールドがすべて在ること。
fn matches(line: &str, kind: &str, required: &[&str]) -> bool {
    let fields = parse_fields(line);
    if fields.get(FIELD_KIND).map(String::as_str) != Some(kind) {
        return false;
    }
    required.iter().all(|name| fields.contains_key(*name))
}

// ---------------------------------------------------------------------------
// 正例（逐語一致）
// ---------------------------------------------------------------------------

#[test]
fn record_prefix_is_verbatim() {
    assert_eq!(
        record_prefix(STAMP, KIND_MONITOR),
        "[transition] frame=7 t_us=1234 kind=monitor"
    );
}

#[test]
fn monitor_line_is_verbatim() {
    let record = MonitorRecord {
        stamp: STAMP,
        entity: entity(5),
        old_dpi: 96,
        new_dpi: 192,
        old_work_area: rect(0, 0, 1920, 1032),
        new_work_area: rect(0, 0, 3840, 2064),
    };
    assert_eq!(
        monitor_line(&record),
        "[transition] frame=7 t_us=1234 kind=monitor entity=5v0 old_dpi=96 new_dpi=192 \
         old_wa=0,0,1920,1032 new_wa=0,0,3840,2064"
    );
    assert!(matches(
        &monitor_line(&record),
        KIND_MONITOR,
        MONITOR_FIELDS
    ));
}

#[test]
fn windpi_line_is_verbatim() {
    let record = WindowDpiRecord {
        stamp: STAMP,
        entity: entity(9),
        old_dpi: 120,
        new_dpi: 192,
    };
    assert_eq!(
        windpi_line(&record),
        "[transition] frame=7 t_us=1234 kind=windpi entity=9v0 old_dpi=120 new_dpi=192"
    );
    assert!(matches(&windpi_line(&record), KIND_WINDPI, WINDPI_FIELDS));
}

/// 新起点（`kind=windpi`）は既存の起点（`kind=monitor`）と**同名・同義・同値形**の欄で
/// 読めること。判定器は両者を同じ読み方で起点に採る（task 8.4 の前提）ので、欄名が
/// 1 文字でもずれたら起点集合の拡張が静かに空振りする。
#[test]
fn windpi_reads_like_the_monitor_origin() {
    let windpi = windpi_line(&WindowDpiRecord {
        stamp: STAMP,
        entity: entity(9),
        old_dpi: 120,
        new_dpi: 192,
    });
    let monitor = monitor_line(&MonitorRecord {
        stamp: STAMP,
        entity: entity(9),
        old_dpi: 120,
        new_dpi: 192,
        old_work_area: rect(0, 0, 10, 10),
        new_work_area: rect(0, 0, 20, 20),
    });
    let from_windpi = parse_fields(&windpi);
    let from_monitor = parse_fields(&monitor);
    for name in [FIELD_ENTITY, FIELD_OLD_DPI, FIELD_NEW_DPI] {
        assert_eq!(
            from_windpi.get(name),
            from_monitor.get(name),
            "起点 2 種で {name} の読み方が違う: {windpi} / {monitor}"
        );
    }
    // 窓は作業領域を持たない——番兵で埋めるのではなく**欄ごと持たない**（欄の意味が無い）。
    assert!(
        !from_windpi.contains_key(FIELD_OLD_WA),
        "窓の起点に作業領域の欄が生えている: {windpi}"
    );
}

#[test]
fn write_line_is_verbatim() {
    let record = WriteRecord {
        stamp: STAMP,
        stage: WriteStage::Flush,
        seq: 3,
        hwnd: hwnd(0x1234),
        tag: tag(),
        x: 10,
        y: 20,
        cx: 300,
        cy: 400,
        flags: 0x14,
        after: Some(rect(10, 20, 310, 420)),
        call_us: 61_000,
        ok: true,
        in_batch: true,
    };
    assert_eq!(
        write_line(&record),
        "[transition] frame=7 t_us=1234 kind=write stage=flush seq=3 hwnd=0x1234 \
         origin=DpiReproject scope=0 win_kind=shell x=10 y=20 cx=300 cy=400 flags=0x14 \
         ax=10 ay=20 aw=300 ah=400 call_us=61000 ok=true in_batch=true"
    );
    assert!(matches(&write_line(&record), KIND_WRITE, WRITE_FIELDS));
    // 任意フィールドも**発行側は必ず載せる**（読む側が古いログを受け入れるだけ）。
    assert!(
        matches(&write_line(&record), KIND_WRITE, WRITE_OPTIONAL_FIELDS),
        "`in_batch` が行から落ちている: {}",
        write_line(&record)
    );

    // 縮退した書込は同じ位置に偽で載る（フィールドごと消えない）。
    let degraded = WriteRecord {
        in_batch: false,
        ..record
    };
    assert!(
        write_line(&degraded).ends_with("call_us=61000 ok=true in_batch=false"),
        "縮退した書込の札: {}",
        write_line(&degraded)
    );
}

#[test]
fn write_line_marks_the_synchronous_path() {
    let record = WriteRecord {
        stamp: STAMP,
        stage: WriteStage::Sync,
        seq: 0,
        hwnd: hwnd(0x20),
        tag: tag(),
        x: 0,
        y: 0,
        cx: 0,
        cy: 0,
        flags: 0,
        after: None,
        call_us: 0,
        ok: true,
        in_batch: false,
    };
    let line = write_line(&record);
    assert!(
        line.contains(" stage=sync "),
        "経路 A（メッセージ受理時の同期書込）は stage=sync で数える: {line}"
    );
}

#[test]
fn flush_begin_and_end_lines_are_verbatim() {
    let begin = FlushRecord {
        stamp: STAMP,
        stage: FlushStage::Begin,
        count: 4,
        since_tick_us: 1200,
        total_us: None,
    };
    assert_eq!(
        flush_line(&begin),
        "[transition] frame=7 t_us=1234 kind=flush stage=begin count=4 since_tick_us=1200 \
         total_us=-"
    );

    let end = FlushRecord {
        stamp: Stamp {
            frame: 7,
            t_us: 1500,
        },
        stage: FlushStage::End,
        count: 4,
        since_tick_us: 1200,
        total_us: Some(266),
    };
    assert_eq!(
        flush_line(&end),
        "[transition] frame=7 t_us=1500 kind=flush stage=end count=4 since_tick_us=1200 \
         total_us=266"
    );
    assert!(matches(&flush_line(&begin), KIND_FLUSH, FLUSH_FIELDS));
    assert!(matches(&flush_line(&end), KIND_FLUSH, FLUSH_FIELDS));
}

#[test]
fn msg_line_is_verbatim() {
    let record = MsgRecord {
        stamp: STAMP,
        msg: MSG_DPICHANGED,
        hwnd: hwnd(0x1234),
        in_swp: true,
        since_flush_us: Some(100),
    };
    assert_eq!(
        msg_line(&record),
        "[transition] frame=7 t_us=1234 kind=msg msg=WM_DPICHANGED hwnd=0x1234 in_swp=true \
         since_flush_us=100"
    );
    assert!(matches(&msg_line(&record), KIND_MSG, MSG_FIELDS));
}

#[test]
fn enqueue_line_is_verbatim() {
    let record = EnqueueRecord {
        stamp: STAMP,
        hwnd: hwnd(0x1234),
        tag: tag(),
        merged_into_seq: Some(3),
    };
    assert_eq!(
        enqueue_line(&record),
        "[transition] frame=7 t_us=1234 kind=enqueue hwnd=0x1234 origin=DpiReproject scope=0 \
         win_kind=shell merged_into_seq=3"
    );
    assert!(matches(
        &enqueue_line(&record),
        KIND_ENQUEUE,
        ENQUEUE_FIELDS
    ));
}

// ---------------------------------------------------------------------------
// 欠損の扱い（番兵・フィールドを落とさない）
// ---------------------------------------------------------------------------

#[test]
fn missing_values_render_as_sentinel_and_keep_their_fields() {
    let record = WriteRecord {
        stamp: STAMP,
        stage: WriteStage::Flush,
        seq: 0,
        hwnd: hwnd(0x1234),
        tag: WriteTag::UNTAGGED,
        x: 0,
        y: 0,
        cx: 0,
        cy: 0,
        flags: 0,
        after: None,
        call_us: 0,
        ok: false,
        in_batch: false,
    };
    let line = write_line(&record);
    let fields = parse_fields(&line);

    // 書込後矩形が読み戻せなかった場合も 4 フィールドは行に残る。
    for name in ["ax", "ay", "aw", "ah", "origin", "scope", "win_kind"] {
        assert_eq!(
            fields.get(name).map(String::as_str),
            Some("-"),
            "欠損は番兵で埋める（フィールドは落とさない）: {name} / {line}"
        );
    }
    // 必須フィールドの集合は欠損時も変わらない。
    assert!(matches(&line, KIND_WRITE, WRITE_FIELDS));
}

#[test]
fn optional_scalars_render_as_sentinel() {
    let enqueue = EnqueueRecord {
        stamp: STAMP,
        hwnd: hwnd(0x1),
        tag: WriteTag::UNTAGGED,
        merged_into_seq: None,
    };
    assert_eq!(
        parse_fields(&enqueue_line(&enqueue))
            .get("merged_into_seq")
            .map(String::as_str),
        Some("-")
    );

    let msg = MsgRecord {
        stamp: STAMP,
        msg: MSG_DPICHANGED,
        hwnd: hwnd(0x1),
        in_swp: false,
        since_flush_us: None,
    };
    assert_eq!(
        parse_fields(&msg_line(&msg))
            .get("since_flush_us")
            .map(String::as_str),
        Some("-")
    );
}

// ---------------------------------------------------------------------------
// 辞書化可能性（同名フィールドの禁止）
// ---------------------------------------------------------------------------

#[test]
fn no_line_repeats_a_field_name() {
    // judge-perf.py の辞書化は後勝ちなので、同名フィールドが 2 度出ると先の値が消える。
    // とくに接頭語の `kind=`（レコード種別）は窓種別と同名にしてはならない。
    let lines = [
        monitor_line(&MonitorRecord {
            stamp: STAMP,
            entity: entity(1),
            old_dpi: 96,
            new_dpi: 120,
            old_work_area: rect(0, 0, 10, 10),
            new_work_area: rect(0, 0, 20, 20),
        }),
        windpi_line(&WindowDpiRecord {
            stamp: STAMP,
            entity: entity(1),
            old_dpi: 96,
            new_dpi: 120,
        }),
        write_line(&WriteRecord {
            stamp: STAMP,
            stage: WriteStage::Flush,
            seq: 0,
            hwnd: hwnd(0x1),
            tag: tag(),
            x: 0,
            y: 0,
            cx: 1,
            cy: 1,
            flags: 0,
            after: Some(rect(0, 0, 1, 1)),
            call_us: 1,
            ok: true,
            in_batch: false,
        }),
        flush_line(&FlushRecord {
            stamp: STAMP,
            stage: FlushStage::End,
            count: 1,
            since_tick_us: 1,
            total_us: Some(1),
        }),
        msg_line(&MsgRecord {
            stamp: STAMP,
            msg: MSG_DPICHANGED,
            hwnd: hwnd(0x1),
            in_swp: false,
            since_flush_us: None,
        }),
        enqueue_line(&EnqueueRecord {
            stamp: STAMP,
            hwnd: hwnd(0x1),
            tag: tag(),
            merged_into_seq: None,
        }),
    ];
    for line in lines {
        let keys: Vec<String> = field_keys(&line).into_iter().map(|(k, _, _)| k).collect();
        let mut unique = keys.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            keys.len(),
            unique.len(),
            "1 行に同じフィールド名が 2 度出てはならない（後勝ちで値が消える）: {line}"
        );
        // レコード種別は接頭語のものが残る。
        assert!(
            parse_fields(&line).contains_key(FIELD_KIND),
            "接頭語の kind= が生き残ること: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// 負例（判定語・フィールド名を壊した入力）
// ---------------------------------------------------------------------------

#[test]
fn broken_kind_word_is_not_matched() {
    let record = MonitorRecord {
        stamp: STAMP,
        entity: entity(5),
        old_dpi: 96,
        new_dpi: 192,
        old_work_area: rect(0, 0, 1920, 1032),
        new_work_area: rect(0, 0, 3840, 2064),
    };
    let good = monitor_line(&record);
    assert!(matches(&good, KIND_MONITOR, MONITOR_FIELDS));

    let broken = good.replace("kind=monitor", "kind=moniter");
    assert!(
        !matches(&broken, KIND_MONITOR, MONITOR_FIELDS),
        "kind 語を壊した入力は照合に失敗しなければならない: {broken}"
    );
}

#[test]
fn broken_field_name_is_not_matched() {
    let record = MonitorRecord {
        stamp: STAMP,
        entity: entity(5),
        old_dpi: 96,
        new_dpi: 192,
        old_work_area: rect(0, 0, 1920, 1032),
        new_work_area: rect(0, 0, 3840, 2064),
    };
    let good = monitor_line(&record);

    let renamed = good.replace("new_wa=", "newwa=");
    assert!(
        !matches(&renamed, KIND_MONITOR, MONITOR_FIELDS),
        "フィールド名を壊した入力は照合に失敗しなければならない: {renamed}"
    );
}

#[test]
fn dropped_field_is_not_matched() {
    let record = MsgRecord {
        stamp: STAMP,
        msg: MSG_DPICHANGED,
        hwnd: hwnd(0x1234),
        in_swp: true,
        since_flush_us: None,
    };
    let good = msg_line(&record);
    assert!(matches(&good, KIND_MSG, MSG_FIELDS));

    let dropped = good.replace(" since_flush_us=-", "");
    assert!(
        !matches(&dropped, KIND_MSG, MSG_FIELDS),
        "フィールドを落とした入力は照合に失敗しなければならない: {dropped}"
    );
}

#[test]
fn broken_stage_word_is_not_matched() {
    let record = FlushRecord {
        stamp: STAMP,
        stage: FlushStage::Begin,
        count: 2,
        since_tick_us: 5,
        total_us: None,
    };
    let good = flush_line(&record);
    assert_eq!(
        parse_fields(&good).get("stage").map(String::as_str),
        Some("begin")
    );

    let broken = good.replace("stage=begin", "stage=start");
    assert_ne!(
        parse_fields(&broken).get("stage").map(String::as_str),
        Some("begin"),
        "stage 語を壊した入力は照合に失敗しなければならない: {broken}"
    );
}

// ---------------------------------------------------------------------------
// 語彙の一意性
// ---------------------------------------------------------------------------

#[test]
fn kind_words_are_unique_and_complete() {
    let mut sorted = KIND_ALL.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), KIND_ALL.len(), "kind 語は互いに異なる");
    for kind in [
        KIND_MONITOR,
        KIND_WINDPI,
        KIND_WRITE,
        KIND_FLUSH,
        KIND_MSG,
        KIND_ENQUEUE,
    ] {
        assert!(KIND_ALL.contains(&kind), "{kind} が KIND_ALL に無い");
    }
}

#[test]
fn stage_words_are_unique() {
    let mut sorted = STAGE_ALL.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), STAGE_ALL.len(), "stage 語は互いに異なる");
}

// ---------------------------------------------------------------------------
// flush 起点（RAII）
// ---------------------------------------------------------------------------

#[test]
fn since_flush_us_is_none_outside_a_flush() {
    assert_eq!(
        since_flush_us(),
        None,
        "flush の外では起点が無い（0 へ潰さない）"
    );
    {
        let _epoch = begin_flush();
        assert!(
            since_flush_us().is_some(),
            "flush の内側では起点からの経過が読める"
        );
    }
    assert_eq!(
        since_flush_us(),
        None,
        "flush を抜けたら起点は消える（他テストへ残さない）"
    );
}

// ---------------------------------------------------------------------------
// 実濾過（既定 OFF・専用指定で ON）
// ---------------------------------------------------------------------------

#[test]
fn default_directives_emit_nothing() {
    let captured = capture_under_filter(DEFAULT_DIRECTIVES, || {
        // 捕捉が生きている証拠（同じ窓の中で確かに拾える対照）。
        tracing::info!(target: TRANSITION_TARGET, "[transition-probe] alive");
        emit_line(&record_prefix(STAMP, KIND_MONITOR));
    });

    assert!(
        captured.contains("[transition-probe] alive"),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );
    assert!(
        !captured.contains("kind=monitor"),
        "既定のログ設定では観測チャネルは 1 行も出てはならない: {captured}"
    );
}

#[test]
fn dedicated_directive_emits_the_line() {
    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_line(&record_prefix(STAMP, KIND_WRITE));
    });

    assert!(
        captured.contains("[transition] frame=7 t_us=1234 kind=write"),
        "専用 target を指定したときは行が出る: {captured}"
    );
}

#[test]
fn is_enabled_follows_the_directive() {
    let mut under_default = true;
    let mut under_signoff = false;

    let control_default = capture_under_filter(DEFAULT_DIRECTIVES, || {
        tracing::info!(target: TRANSITION_TARGET, "[transition-probe] alive");
        under_default = is_enabled();
    });
    let control_signoff = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        tracing::info!(target: TRANSITION_TARGET, "[transition-probe] alive");
        under_signoff = is_enabled();
    });

    assert!(control_default.contains("[transition-probe] alive"));
    assert!(control_signoff.contains("[transition-probe] alive"));
    assert!(
        !under_default,
        "既定では前置ガードが偽＝呼び出し側は組立の費用を払わない"
    );
    assert!(under_signoff, "専用指定では前置ガードが真");
}

// ---------------------------------------------------------------------------
// 刻印の権威（D1）——資源が正・スレッド局所ミラーは World 外専用
// ---------------------------------------------------------------------------

#[test]
fn reset_for_test_clears_the_mirror_and_the_time_bases() {
    begin_tick(4_242, Instant::now());
    assert_eq!(current_frame(), 4_242, "写しは tick 開始で更新される");

    {
        let _epoch = begin_flush();
        assert!(since_flush_us().is_some());
        reset_for_test();
        assert_eq!(
            current_frame(),
            0,
            "初期化後の写しは 0（前のテストのフレーム番号を持ち越さない・要件 7.7）"
        );
        assert_eq!(
            since_tick_start_us(),
            0,
            "初期化後は tick 開始の時刻基準も無い"
        );
        assert_eq!(
            since_flush_us(),
            None,
            "初期化は flush 起点も消す（生存札より先に効く）"
        );
    }
}

#[test]
fn resource_and_mirror_stamps_agree_on_the_frame_within_one_tick() {
    // 同一 tick の同一値で資源と写しを更新する（`try_tick_world` が行う配管と同じ形）。
    let start = Instant::now();
    let frame = FrameCount(11);
    let tick_start = TickStart(start);
    begin_tick(frame.0, start);

    assert_eq!(
        stamp_from_world(&frame, &tick_start).frame,
        stamp().frame,
        "同一 tick では資源から組んだ刻印と写しから組んだ刻印のフレーム番号が一致する"
    );
    assert_eq!(stamp().frame, 11);

    reset_for_test();
}

#[test]
fn the_mirror_does_not_reach_other_threads_but_the_resource_does() {
    // D1 の核心——ミラーはスレッド局所ゆえ、ワーカースレッドからは読めない。
    // World を持つ観測点が `stamp()` を呼ぶと frame=0 の行が出る（＝退行）ので、
    // それらは必ず `stamp_from_world` を使う、という取り決めの根拠がこれである。
    let start = Instant::now();
    begin_tick(77, start);

    let observed = std::thread::spawn(move || {
        let frame = FrameCount(77);
        let tick_start = TickStart(start);
        (current_frame(), stamp_from_world(&frame, &tick_start).frame)
    })
    .join()
    .expect("別スレッドの観測が完了する");

    assert_eq!(
        observed.0, 0,
        "別スレッドからは写しが見えない（スレッド局所ゆえテスト間でも汚染しない・要件 7.7）"
    );
    assert_eq!(
        observed.1, 77,
        "資源から組んだ刻印は別スレッドでも正しいフレーム番号になる"
    );
    assert_eq!(current_frame(), 77, "元のスレッドの写しは影響を受けない");

    reset_for_test();
}

// ---------------------------------------------------------------------------
// 多スレッド実行器のままの検証（design C1 Validation）
// ---------------------------------------------------------------------------

/// 観測系システムが 1 回の実行で記録する値。
#[derive(Clone, Copy, Debug)]
struct Observation {
    /// World 資源（`FrameCount`＋`TickStart`）から組んだ刻印のフレーム番号。
    from_world: u32,
    /// スレッド局所ミラーから読んだフレーム番号（World を持つ点は本来これを読まない）。
    from_mirror: u32,
    /// 記録したスレッド。
    thread: ThreadId,
}

/// 観測の集積先。共有参照＋内部可変にしてあるので、複数の観測系システムが
/// **同時に**走れる（`ResMut` にすると実行器が直列化してしまい、多スレッド実行の
/// 検証にならない）。
#[derive(Resource, Clone, Default)]
struct ObservedStamps(Arc<Mutex<Vec<Observation>>>);

/// `Update` へ複数本登録する観測系システム（`N` を変えて別の型にする）。
fn observe_stamp<const N: usize>(
    frame: Res<FrameCount>,
    tick_start: Res<TickStart>,
    out: Res<ObservedStamps>,
) {
    let observation = Observation {
        from_world: stamp_from_world(&frame, &tick_start).frame,
        from_mirror: current_frame(),
        thread: std::thread::current().id(),
    };
    out.0
        .lock()
        .expect("観測バッファの毒化なし")
        .push(observation);
}

/// 既定の多スレッド実行器のまま `Update` を回し、World 資源から組んだ刻印のフレーム番号が
/// `FrameCount` と一致することを、**ログ捕捉に依らず**（システムが載るスレッドによっては
/// 捕捉が届かない・要件 7.6）レコード純関数へ渡す値そのもので確かめる。
#[test]
fn world_stamps_match_frame_count_under_the_default_multi_threaded_executor() {
    let mut ecs = EcsWorld::new();
    let sink = ObservedStamps::default();
    ecs.world_mut().insert_resource(sink.clone());

    ecs.add_systems(Update, observe_stamp::<0>);
    ecs.add_systems(Update, observe_stamp::<1>);
    ecs.add_systems(Update, observe_stamp::<2>);
    ecs.add_systems(Update, observe_stamp::<3>);

    // 検証が空振りしていないこと——`Update` は既定の実行器（多スレッド）のままである。
    // bevy 0.19 で `Schedule::get_executor_kind` が撤去され（実行器は `Box<dyn SystemExecutor>`
    // として私有）、実行形態を実行時に読む手段が無くなった。ゆえに**構築側の字面**で見張る——
    // `Update` だけは実行器を差し替えずに挿入されている、という形をそのまま固定する。
    // 誰かが `Update` を `set_executor` で単スレッドへ落とせば、この逐語が消えて赤になる。
    assert!(
        WORLD_CONSTRUCTION_SRC.contains("schedules.insert(Schedule::new(Update));"),
        "本番の World 構築が `Update` の実行器を差し替えている。既定の多スレッド実行器を前提とする本テストは空振りする"
    );

    assert!(
        ecs.try_tick_world(),
        "システム登録済みの World は tick する"
    );
    let expected = ecs.world().resource::<FrameCount>().0;
    assert_eq!(expected, 1, "1 周で FrameCount は 1 になる");

    let observations = sink.0.lock().expect("観測バッファの毒化なし").clone();
    assert_eq!(observations.len(), 4, "4 本の観測系システムが全て走る");
    for observation in &observations {
        assert_eq!(
            observation.from_world, expected,
            "多スレッド実行でも資源から組んだ刻印は FrameCount とずれない: {observation:?}"
        );
    }

    // ワーカースレッドに載った観測があれば、そこでは写しが見えない（frame=0）ことを
    // その場の実測で示す。載らなかった場合の恒久的な証拠は
    // `the_mirror_does_not_reach_other_threads_but_the_resource_does` が持つ。
    let ui_thread = std::thread::current().id();
    for observation in observations.iter().filter(|o| o.thread != ui_thread) {
        assert_eq!(
            observation.from_mirror, 0,
            "ワーカースレッドからは写しが読めない（World を持つ点が stamp() を呼ぶ退行の検出）: {observation:?}"
        );
    }

    // UI スレッド（tick を回した側）の写しは資源と同じフレーム番号を指す。
    assert_eq!(
        current_frame(),
        expected,
        "写しは FrameCount 増分と同一点で同じ値に更新される"
    );

    reset_for_test();
}

#[test]
fn each_tick_advances_the_resource_and_the_mirror_together() {
    let mut ecs = EcsWorld::new();
    // tick が空回りしないようにダミーのシステムを 1 本置く（`has_systems` の条件）。
    ecs.add_systems(Update, || {});

    for expected in 1..=3u32 {
        assert!(ecs.try_tick_world());
        let frame = ecs.world().resource::<FrameCount>().0;
        let tick_start = *ecs.world().resource::<TickStart>();
        assert_eq!(frame, expected, "FrameCount は tick ごとに 1 進む");
        assert_eq!(
            current_frame(),
            expected,
            "写しも同じ点で同じ値へ進む（1 系列が保たれる）"
        );
        assert_eq!(
            stamp_from_world(&FrameCount(frame), &tick_start).frame,
            stamp().frame,
            "資源から組んだ刻印と写しから組んだ刻印は同じフレーム番号になる"
        );
    }

    reset_for_test();
}
