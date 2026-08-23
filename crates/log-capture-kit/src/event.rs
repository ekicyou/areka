//! 捕捉した 1 イベントの正準表現と行整形。フィールドは `record()` の訪問順を保持する
//! （行整形の byte 一致に必要）。

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// フィールド 1 個の値。
///
/// `debug` は `record_debug` 経路の `{:?}` 表現で、行整形はこちらを使う。`str_raw` は
/// `record_str` 経路で渡された生文字列（引用符・エスケープ無し）で、値の完全一致で
/// 判定する消費側はこちらを使う。文字列として渡されたフィールドは両方が埋まる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValue {
    /// `{:?}` 表現（文字列値なら引用符つき）。
    pub debug: String,
    /// `record_str` 経路で渡された生値。`record_debug` 経路のフィールドでは `None`。
    pub str_raw: Option<String>,
}

/// 捕捉した 1 イベント。
///
/// `fields` は [`tracing::Event::record`] の**訪問順**（＝マクロでの記述順）を保持する。
/// 行整形が現行の文字列形を 1 バイトも違わずに再現するために順序が要る。
#[derive(Debug, Clone)]
pub struct CapturedEvent {
    /// イベントのレベル（`error!`／`warn!`／`info!`／`debug!`／`trace!` の別）。
    pub level: tracing::Level,
    /// イベントの宛先（`target:` 指定が無ければ発行元モジュールパス）。
    pub target: String,
    /// 構造化フィールド（`message` を含む）を訪問順で並べたもの。
    pub fields: Vec<(String, FieldValue)>,
}

/// 全フィールドを訪問順で拾う visitor。
///
/// [`tracing::field::Visit`] の `record_u64`／`record_f64`／`record_bool` 等はすべて既定実装が
/// `record_debug` へ転送するため、`record_debug` と `record_str` の 2 本で型を問わず全
/// フィールドを捕捉できる。`record_str` を別に持つのは生値（引用符なし）を残すためで、
/// `debug` 側は `record_debug` が同じ値に対して作る表現と一致させる。
pub(crate) struct FieldCollector<'a>(pub(crate) &'a mut Vec<(String, FieldValue)>);

impl tracing::field::Visit for FieldCollector<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push((
            field.name().to_string(),
            FieldValue {
                debug: format!("{value:?}"),
                str_raw: Some(value.to_string()),
            },
        ));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push((
            field.name().to_string(),
            FieldValue {
                debug: format!("{value:?}"),
                str_raw: None,
            },
        ));
    }
}

impl CapturedEvent {
    /// `tracing` のイベントから正準表現を組み立てる。
    pub(crate) fn from_event(event: &tracing::Event<'_>) -> Self {
        let mut fields: Vec<(String, FieldValue)> = Vec::new();
        event.record(&mut FieldCollector(&mut fields));
        Self {
            level: *event.metadata().level(),
            target: event.metadata().target().to_string(),
            fields,
        }
    }
}

impl CapturedEvent {
    /// メッセージ本文（`message` フィールドの Debug 表現）。無ければ空文字を返す。
    ///
    /// `message` の値は `fmt::Arguments` で渡るため、その `{:?}` は整形済みの本文
    /// そのもの（引用符なし）になる。移行元の `placement::LogEvent::message`／
    /// `emo-present::CapturedEvent::message` と同じく、欠落は panic ではなく `""`。
    pub fn message(&self) -> &str {
        self.field("message").unwrap_or("")
    }

    /// フィールドの **Debug 表現**（文字列値なら引用符つき）。無ければ `None`。
    ///
    /// 行整形に載るのと同じ文字列で、`placement::LogEvent::field` と同じ値を返す
    /// （欠落時に panic するかどうかだけが違い、判定はアダプタ側に委ねる）。
    pub fn field(&self, name: &str) -> Option<&str> {
        self.last(name).map(|v| v.debug.as_str())
    }

    /// フィールドの **生値**（`record_str` 経路で渡された文字列。引用符なし）。
    ///
    /// `record_debug` 経路で載ったフィールド（数値・真偽値・`Debug` で渡した値）と
    /// 欠落は `None`。`areka-kanade`／`areka-ghost`／`areka-sylphya` の `assert_logged` は
    /// `event`／`outcome` を生値の完全一致で判定するため、引用符を剥がす仕事はここに置く
    /// （アダプタ側で `trim_matches('"')` を再実装すると、値そのものが引用符を含む場合に
    /// 静かに壊れる）。
    pub fn field_str(&self, name: &str) -> Option<&str> {
        self.last(name).and_then(|v| v.str_raw.as_deref())
    }

    /// フィールド名の昇順・重複なしの一覧（`message` を含む）。
    ///
    /// `emo-present` の `CapturedEvent::field_names()`（`HashMap` の鍵を `sort_unstable`）と
    /// 同じ列を返す。「フィールド集合をちょうどで固定する」判定に使う。
    pub fn field_names_sorted(&self) -> Vec<&str> {
        self.fields_map().into_keys().collect()
    }

    /// フィールド名 → Debug 表現の写像（`message` を含む）。
    ///
    /// `placement` の `LogEvent.fields`（`BTreeMap<String, String>` を `insert` で組む）と
    /// 同じ内容。同名が 2 度現れた場合は**後勝ち**で、これも移行元と同じ。
    pub fn fields_map(&self) -> BTreeMap<&str, &str> {
        self.fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.debug.as_str()))
            .collect()
    }

    /// 同名フィールドの**最後**の出現。移行元がいずれも写像（`insert` は後勝ち）で
    /// 組んでいるため、取り出し系はすべてこれに揃える（行整形だけは訪問順どおり全件出す）。
    fn last(&self, name: &str) -> Option<&FieldValue> {
        self.fields
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value)
    }
}

/// 行整形の 2 形。移行対象 crate が今日出している文字列を 1 バイトも違わずに再現する。
///
/// どちらも「先頭の固定部 ＋ フィールドを訪問順で ` {name}={value:?}` として連結」であり、
/// フィールドが 1 個も無ければ固定部だけ（末尾に空白は付かない）。レベルは `Display`
/// （`INFO`／`WARN`／`ERROR`／`DEBUG`／`TRACE`）で載る——`Debug`（`Level(Info)`）ではない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineFormat {
    /// `level={level} target={target}` から始める形。
    ///
    /// 出所: `areka-seriko/src/table.rs`・`areka-emo-atlas/src/log_capture.rs`・
    /// `areka-emo-compose/src/log_capture.rs`・`areka/src/emo2_boot/*`。
    LevelTargetFields,
    /// `level={level}` から始める形（宛先を載せない）。
    ///
    /// 出所: `areka/src/input_events/{choice_drain.rs, balloon_test_support.rs}`・
    /// `areka/src/shiori_demo.rs`・`wintf/src/ecs/window_proc/dpi_helpers_tests.rs`。
    LevelFields,
}

/// 1 イベントを 1 行へ整形する（[`LineFormat`] の逐語再現）。
pub fn format_line(ev: &CapturedEvent, fmt: LineFormat) -> String {
    let mut line = match fmt {
        LineFormat::LevelTargetFields => format!("level={} target={}", ev.level, ev.target),
        LineFormat::LevelFields => format!("level={}", ev.level),
    };
    for (name, value) in &ev.fields {
        // 移行元は ` {}={:?}` と書いており、`{:?}` の結果は `FieldValue::debug` に入っている。
        let _ = write!(line, " {}={}", name, value.debug);
    }
    line
}

/// [`crate::capture`] の結果をその場で行へ整形して返す。
///
/// 移行対象の `capture_logs` は「行の `Vec`」か「改行連結した 1 本の `String`」を返して
/// いる。前者はこれをそのまま、後者は `.join("\n")` を被せて置き換える。
pub fn capture_lines<R>(fmt: LineFormat, f: impl FnOnce() -> R) -> (R, Vec<String>) {
    let (out, events) = crate::capture::capture(f);
    let lines = events.iter().map(|e| format_line(e, fmt)).collect();
    (out, lines)
}

/// レベル別の件数。
///
/// `areka-emo-text` の `with_log_cage`（`(T, warns, errors)`）・`count_warns`／
/// `resolve_counting_warns`（`(T, warns)`）が数えているものの上位集合で、
/// 呼出側の戻り値の形は各 crate 側のアダプタで維持する。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelCounts {
    /// `error!` の件数。
    pub error: usize,
    /// `warn!` の件数。
    pub warn: usize,
    /// `info!` の件数。
    pub info: usize,
    /// `debug!` の件数。
    pub debug: usize,
    /// `trace!` の件数。
    pub trace: usize,
}

/// [`crate::capture`] の結果をレベル別に数えて返す。
///
/// 番兵イベントは [`crate::capture`] が既に取り除いているので `trace` に混ざらない。
pub fn count_levels<R>(f: impl FnOnce() -> R) -> (R, LevelCounts) {
    let (out, events) = crate::capture::capture(f);
    let mut counts = LevelCounts::default();
    for ev in &events {
        match ev.level {
            tracing::Level::ERROR => counts.error += 1,
            tracing::Level::WARN => counts.warn += 1,
            tracing::Level::INFO => counts.info += 1,
            tracing::Level::DEBUG => counts.debug += 1,
            tracing::Level::TRACE => counts.trace += 1,
        }
    }
    (out, counts)
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
