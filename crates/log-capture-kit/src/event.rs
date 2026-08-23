//! 捕捉した 1 イベントの正準表現と行整形。フィールドは `record()` の訪問順を保持する
//! （行整形の byte 一致に必要）。

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
