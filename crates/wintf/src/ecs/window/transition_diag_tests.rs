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

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HWND, RECT};

use super::{
    ENQUEUE_FIELDS, EnqueueRecord, FIELD_KIND, FLUSH_FIELDS, FlushRecord, FlushStage, KIND_ALL,
    KIND_ENQUEUE, KIND_FLUSH, KIND_MONITOR, KIND_MSG, KIND_WRITE, MONITOR_FIELDS, MSG_DPICHANGED,
    MSG_FIELDS, MonitorRecord, MsgRecord, STAGE_ALL, Stamp, TRANSITION_TARGET, WRITE_FIELDS,
    WriteRecord, WriteStage, WriteTag, begin_flush, emit_line, enqueue_line, flush_line,
    is_enabled, monitor_line, msg_line, record_prefix, since_flush_us, write_line,
};
use crate::ecs::test_support::capture_under_filter;

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
    };
    assert_eq!(
        write_line(&record),
        "[transition] frame=7 t_us=1234 kind=write stage=flush seq=3 hwnd=0x1234 \
         origin=DpiReproject scope=0 win_kind=shell x=10 y=20 cx=300 cy=400 flags=0x14 \
         ax=10 ay=20 aw=300 ah=400 call_us=61000 ok=true"
    );
    assert!(matches(&write_line(&record), KIND_WRITE, WRITE_FIELDS));
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
    for kind in [KIND_MONITOR, KIND_WRITE, KIND_FLUSH, KIND_MSG, KIND_ENQUEUE] {
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
