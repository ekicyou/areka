//! `event.rs` の自己テスト（設計 C1 Implementation Notes → Validation ⒠⒡⒢）。
//!
//! 中心にあるのは **byte 一致**である。移行対象 crate は今日、自前の Layer／Subscriber で
//! 1 イベントを 1 行へ整形しており、その行の文字列を `contains` や `starts_with` で判定する
//! テストが多数ぶら下がっている。整形が 1 バイトでも違えば、それらは移行と同時に静かに
//! 意味を変える。そこで本ファイルは 2 段で固定する。
//!
//! 1. **逐語の見本**（[`LEVEL_TARGET_FIELDS_FIXTURE`]／[`LEVEL_FIELDS_FIXTURE`]）— 移行対象の
//!    現行整形コードを 1 文字も変えずに走らせて得た**実出力**をそのまま置いたもの。
//! 2. **差分検査**（[`LegacyLineFormatting`]）— その現行整形コードの逐語写しを本ファイル内に
//!    持ち、**同一のイベント列**に対して [`format_line`] と 1 バイト単位で突き合わせる。
//!    見本が古びても差分検査は毎回走るので、両者が同時に嘘をつくことはない。
//!
//! 逐語写しの出所（`format!`／`write!` の並びは 1 文字も変えていない）:
//! - 宛先を含む形: `crates/areka-seriko/src/table.rs` の `capture_logs`（`Capture::on_event`）と
//!   `crates/areka-emo-atlas/src/log_capture.rs`・`crates/areka-emo-compose/src/log_capture.rs`
//!   の同型。
//! - 宛先を含まない形: `crates/areka/src/input_events/choice_drain.rs` の `Capture::on_event` と
//!   `crates/areka/src/input_events/balloon_test_support.rs`・`crates/areka/src/shiori_demo.rs`・
//!   `crates/wintf/src/ecs/window_proc/dpi_helpers_tests.rs` の同型。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::{
    CapturedEvent, FieldValue, LevelCounts, LineFormat, capture_lines, count_levels, format_line,
};
use crate::capture::{SENTINEL_TARGET, run_with_subscriber};

// ── 見本のイベント列 ────────────────────────────────────────────────────────────────
//
// 宛先はすべて明示する（既定の宛先は発行元のモジュールパスになり、このファイルが移動した
// だけで見本が変わってしまうため）。7 件で次を網羅する: メッセージのみ／文字列＋整数
// フィールド／浮動小数＋真偽値／**メッセージ無し**／記述順と訪問順の関係／引用符とタブの
// エスケープ／`record_str` 経路の生値。

fn emit_fixture_events() {
    tracing::info!(target: "areka_seriko::table", "table built");
    tracing::warn!(target: "areka_emo_atlas", set = 0, rel_path = "clear.png", "surface missing");
    tracing::error!(target: "areka_emo_present::presenter", scale = 1.25f64, ok = true, "boom");
    tracing::debug!(target: "wintf::transition", count = 3u32);
    tracing::info!(target: "areka::emo2_boot", b = 1i64, a = 2i64, "order probe");
    tracing::trace!(target: "t::esc", text = "quo\"te\ttab", "esc");
    tracing::warn!(target: "areka_kanade::resource", event = "shiori_load", outcome = "ok", "raw probe");
}

/// [`emit_fixture_events`] を現行の「宛先を含む形」で整形した**実出力**（逐語）。
const LEVEL_TARGET_FIELDS_FIXTURE: &[&str] = &[
    "level=INFO target=areka_seriko::table message=table built",
    "level=WARN target=areka_emo_atlas message=surface missing set=0 rel_path=\"clear.png\"",
    "level=ERROR target=areka_emo_present::presenter message=boom scale=1.25 ok=true",
    "level=DEBUG target=wintf::transition count=3",
    "level=INFO target=areka::emo2_boot message=order probe b=1 a=2",
    "level=TRACE target=t::esc message=esc text=\"quo\\\"te\\ttab\"",
    "level=WARN target=areka_kanade::resource message=raw probe event=\"shiori_load\" outcome=\"ok\"",
];

/// [`emit_fixture_events`] を現行の「宛先を含まない形」で整形した**実出力**（逐語）。
const LEVEL_FIELDS_FIXTURE: &[&str] = &[
    "level=INFO message=table built",
    "level=WARN message=surface missing set=0 rel_path=\"clear.png\"",
    "level=ERROR message=boom scale=1.25 ok=true",
    "level=DEBUG count=3",
    "level=INFO message=order probe b=1 a=2",
    "level=TRACE message=esc text=\"quo\\\"te\\ttab\"",
    "level=WARN message=raw probe event=\"shiori_load\" outcome=\"ok\"",
];

// ── 現行整形コードの逐語写し ────────────────────────────────────────────────────────

/// 正準型と**同じイベント列**から、現行整形コードの逐語写しで行を組む subscriber。
///
/// 逐語写しから唯一外したのは「番兵イベントを行に積まない」ことだけで、整形そのもの
/// （`format!`／`write!` の並び）は 1 文字も変えていない。番兵は [`crate::capture`] が窓の
/// 生存を示すために内側で 1 件発火するもので、移行対象 crate には存在しない。
#[derive(Clone, Default)]
struct LegacyLineFormatting {
    events: Arc<Mutex<Vec<CapturedEvent>>>,
    with_target: Arc<Mutex<Vec<String>>>,
    without_target: Arc<Mutex<Vec<String>>>,
}

impl tracing::Subscriber for LegacyLineFormatting {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, ev: &tracing::Event<'_>) {
        self.events
            .lock()
            .expect("捕捉バッファは毒化していない")
            .push(CapturedEvent::from_event(ev));

        if ev.metadata().target() == SENTINEL_TARGET {
            return;
        }

        // ↓ `areka-seriko/src/table.rs`（＝`areka-emo-atlas`／`areka-emo-compose` も同一）逐語
        {
            use tracing::field::{Field, Visit};
            let meta = ev.metadata();
            let mut line = format!("level={} target={}", meta.level(), meta.target());
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut line));
            self.with_target
                .lock()
                .expect("捕捉バッファは毒化していない")
                .push(line);
        }

        // ↓ `areka/src/input_events/choice_drain.rs`（＝他 3 ファイルも同一）逐語
        {
            use tracing::field::{Field, Visit};
            let meta = ev.metadata();
            let mut line = format!("level={}", meta.level());
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut line));
            self.without_target
                .lock()
                .expect("捕捉バッファは毒化していない")
                .push(line);
        }
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// 見本イベント列を 1 度だけ発火し、正準型・現行整形（2 形）を同時に取る。
fn run_fixture() -> (Vec<CapturedEvent>, Vec<String>, Vec<String>) {
    let sub = LegacyLineFormatting::default();
    let sink = Arc::clone(&sub.events);
    let with_target = Arc::clone(&sub.with_target);
    let without_target = Arc::clone(&sub.without_target);

    let ((), events) = run_with_subscriber(sub, sink, emit_fixture_events);

    let with_target = std::mem::take(&mut *with_target.lock().expect("毒化なし"));
    let without_target = std::mem::take(&mut *without_target.lock().expect("毒化なし"));
    (events, with_target, without_target)
}

// ── ⒠ 行整形 2 形の byte 一致 ───────────────────────────────────────────────────────

#[test]
fn level_target_fields_matches_verbatim_fixture() {
    let (events, legacy, _) = run_fixture();

    assert_eq!(
        legacy,
        LEVEL_TARGET_FIELDS_FIXTURE.to_vec(),
        "現行整形コードの逐語写しが見本と違う（見本が古い）"
    );

    let formatted: Vec<String> = events
        .iter()
        .map(|e| format_line(e, LineFormat::LevelTargetFields))
        .collect();
    assert_eq!(
        formatted,
        LEVEL_TARGET_FIELDS_FIXTURE.to_vec(),
        "整形結果が現行出力の逐語見本と 1 バイト違う"
    );
}

#[test]
fn level_fields_matches_verbatim_fixture() {
    let (events, _, legacy) = run_fixture();

    assert_eq!(
        legacy,
        LEVEL_FIELDS_FIXTURE.to_vec(),
        "現行整形コードの逐語写しが見本と違う（見本が古い）"
    );

    let formatted: Vec<String> = events
        .iter()
        .map(|e| format_line(e, LineFormat::LevelFields))
        .collect();
    assert_eq!(
        formatted,
        LEVEL_FIELDS_FIXTURE.to_vec(),
        "整形結果が現行出力の逐語見本と 1 バイト違う"
    );
}

/// 見本が古びても効く側の検査。**同一のイベント列**に対して、現行整形コードの逐語写しと
/// [`format_line`] を毎回突き合わせる。
#[test]
fn format_line_is_byte_identical_to_current_formatting_code() {
    let (events, legacy_with, legacy_without) = run_fixture();

    assert!(
        !events.is_empty(),
        "イベントが 1 件も捕れていない（この検査は空列に対して恒真になる）"
    );
    assert_eq!(events.len(), legacy_with.len());
    assert_eq!(events.len(), legacy_without.len());

    for (i, ev) in events.iter().enumerate() {
        assert_eq!(
            format_line(ev, LineFormat::LevelTargetFields),
            legacy_with[i],
            "{i} 件目（宛先を含む形）が現行整形コードと違う"
        );
        assert_eq!(
            format_line(ev, LineFormat::LevelFields),
            legacy_without[i],
            "{i} 件目（宛先を含まない形）が現行整形コードと違う"
        );
    }
}

/// レベルは `Display`（`INFO`）で載る。`Debug`（`Level(Info)`）ではない。
#[test]
fn level_is_rendered_with_display_not_debug() {
    let (events, _, _) = run_fixture();
    let line = format_line(&events[0], LineFormat::LevelFields);
    assert!(line.starts_with("level=INFO"), "実際: {line}");
    assert!(
        !line.contains("Level("),
        "レベルが Debug 表現で載っている: {line}"
    );
}

/// フィールドが 1 個も無いイベントは、末尾に空白を残さない。
#[test]
fn event_without_fields_has_no_trailing_space() {
    let ev = CapturedEvent {
        level: tracing::Level::INFO,
        target: "t".to_string(),
        fields: Vec::new(),
    };
    assert_eq!(
        format_line(&ev, LineFormat::LevelTargetFields),
        "level=INFO target=t"
    );
    assert_eq!(format_line(&ev, LineFormat::LevelFields), "level=INFO");
}

// ── `capture_lines` ─────────────────────────────────────────────────────────────────

#[test]
fn capture_lines_returns_formatted_lines_and_the_closure_result() {
    let (ret, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        tracing::warn!(target: "areka_emo_atlas", set = 0, rel_path = "clear.png", "surface missing");
        41 + 1
    });
    assert_eq!(ret, 42);
    assert_eq!(
        lines,
        vec![
            "level=WARN target=areka_emo_atlas message=surface missing set=0 rel_path=\"clear.png\""
                .to_string()
        ]
    );
}

#[test]
fn capture_lines_does_not_leak_the_sentinel() {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, || {
        tracing::info!(target: "areka::emo2_boot", "only mine");
    });
    // 陽性対照（自分のイベントは 1 件捕れている）と対で見る。
    assert_eq!(lines.len(), 1, "実際: {lines:?}");
    assert!(
        !lines.iter().any(|l| l.contains(SENTINEL_TARGET)),
        "番兵が漏れている: {lines:?}"
    );
}

// ── ⒡ レベル別件数 ─────────────────────────────────────────────────────────────────

#[test]
fn count_levels_counts_each_level() {
    let (ret, counts) = count_levels(|| {
        tracing::error!(target: "t", "e1");
        tracing::warn!(target: "t", "w1");
        tracing::warn!(target: "t", "w2");
        tracing::info!(target: "t", "i1");
        tracing::info!(target: "t", "i2");
        tracing::info!(target: "t", "i3");
        tracing::debug!(target: "t", "d1");
        tracing::trace!(target: "t", "t1");
        "done"
    });
    assert_eq!(ret, "done");
    assert_eq!(
        counts,
        LevelCounts {
            error: 1,
            warn: 2,
            info: 3,
            debug: 1,
            trace: 1,
        }
    );
}

#[test]
fn count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live() {
    // 陰性主張（0 件）は、同じ形で陽性が数えられることと対にして初めて意味を持つ。
    let ((), quiet) = count_levels(|| {});
    assert_eq!(quiet, LevelCounts::default());

    let ((), loud) = count_levels(|| tracing::warn!(target: "t", "w"));
    assert_eq!(loud.warn, 1, "陽性対照が数えられていない: {loud:?}");
}

// ── ⒢ フィールドの取り出し ──────────────────────────────────────────────────────────

#[test]
fn field_str_returns_the_raw_value_and_field_returns_the_debug_representation() {
    let (events, _, _) = run_fixture();
    let ev = events
        .iter()
        .find(|e| e.target == "areka_kanade::resource")
        .expect("見本の raw probe イベント");

    // `areka-kanade`／`areka-ghost`／`areka-sylphya` の `assert_logged` は生値の完全一致で
    // 判定するため、引用符を剥がす仕事はアダプタ側ではなく kit の `field_str` が担う。
    assert_eq!(ev.field_str("event"), Some("shiori_load"));
    assert_eq!(ev.field_str("outcome"), Some("ok"));
    assert_eq!(ev.field("event"), Some("\"shiori_load\""));
    assert_eq!(ev.field("outcome"), Some("\"ok\""));
}

#[test]
fn field_str_is_none_for_values_that_did_not_come_through_record_str() {
    let (events, _, _) = run_fixture();
    let ev = events
        .iter()
        .find(|e| e.target == "wintf::transition")
        .expect("見本の count イベント");

    // 陽性対照: Debug 表現では取り出せる。
    assert_eq!(ev.field("count"), Some("3"));
    assert_eq!(ev.field_str("count"), None);
}

#[test]
fn message_is_the_body_and_is_empty_when_absent() {
    let (events, _, _) = run_fixture();

    let with_message = events
        .iter()
        .find(|e| e.target == "areka_seriko::table")
        .expect("見本の table イベント");
    assert_eq!(with_message.message(), "table built");

    let without_message = events
        .iter()
        .find(|e| e.target == "wintf::transition")
        .expect("見本の count イベント");
    assert_eq!(without_message.message(), "");
}

#[test]
fn missing_field_is_none() {
    let (events, _, _) = run_fixture();
    let ev = &events[0];
    assert_eq!(ev.field("no_such_field"), None);
    assert_eq!(ev.field_str("no_such_field"), None);
    // 陽性対照。
    assert_eq!(ev.field("message"), Some("table built"));
}

#[test]
fn field_names_sorted_is_ascending_and_includes_message() {
    let (events, _, _) = run_fixture();
    let ev = events
        .iter()
        .find(|e| e.target == "areka::emo2_boot")
        .expect("見本の order probe イベント");

    // 訪問順は message → b → a。整列列は昇順（`emo-present` の `field_names()` 互換）。
    assert_eq!(
        ev.fields
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>(),
        vec!["message", "b", "a"]
    );
    assert_eq!(ev.field_names_sorted(), vec!["a", "b", "message"]);
}

#[test]
fn fields_map_is_name_to_debug_representation() {
    let (events, _, _) = run_fixture();
    let ev = events
        .iter()
        .find(|e| e.target == "areka::emo2_boot")
        .expect("見本の order probe イベント");

    let expected: BTreeMap<&str, &str> = [("a", "2"), ("b", "1"), ("message", "order probe")]
        .into_iter()
        .collect();
    assert_eq!(ev.fields_map(), expected);
}

/// 同名フィールドが 2 度現れたら**後勝ち**（`placement` の `BTreeMap::insert`・`emo-present` の
/// `HashMap::insert` と同じ）。整形だけは訪問順どおり 2 度出す。
#[test]
fn duplicate_field_names_resolve_to_the_last_occurrence() {
    let ev = CapturedEvent {
        level: tracing::Level::WARN,
        target: "t".to_string(),
        fields: vec![
            (
                "k".to_string(),
                FieldValue {
                    debug: "\"first\"".to_string(),
                    str_raw: Some("first".to_string()),
                },
            ),
            (
                "k".to_string(),
                FieldValue {
                    debug: "\"last\"".to_string(),
                    str_raw: Some("last".to_string()),
                },
            ),
        ],
    };

    assert_eq!(ev.field("k"), Some("\"last\""));
    assert_eq!(ev.field_str("k"), Some("last"));
    assert_eq!(ev.field_names_sorted(), vec!["k"]);
    assert_eq!(
        ev.fields_map(),
        [("k", "\"last\"")].into_iter().collect::<BTreeMap<_, _>>()
    );
    assert_eq!(
        format_line(&ev, LineFormat::LevelFields),
        "level=WARN k=\"first\" k=\"last\""
    );
}
