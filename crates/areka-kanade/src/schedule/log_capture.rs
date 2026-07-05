//! テスト専用ログ捕捉ヘルパ（タスク 6.1・Req 6.1／6.3）。
//!
//! 純粋関数 [`crate::schedule::step`]（および各サブモジュール `step`）は、失敗・防御
//! アームごとに `tracing::{error,warn}!` を発行する（steering:
//! areka-log-first-no-silent-failure・「ログ無しの失敗経路を持たない」）。本モジュールは
//! その発行を**実行可能なテストで**捕捉し、各アームが規約どおりの `target="kanade"`・
//! `event=<語彙>`・レベル（ERROR/WARN）でログを出していることを検証可能にする。
//!
//! # 決定性の要（PITFALL）
//! [`capture`] は [`tracing::subscriber::with_default`] でスレッドローカルに subscriber を
//! 差し込み、クロージャ内で発行されたイベントのみを捕える。`step()` はテストスレッド上で
//! 同期的に走る純粋関数ゆえ、そのイベントは確実に同一スレッドで捕捉される（spawn した
//! アクタースレッドのログは捕えない——それはタスク 6.2 の担当）。

use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

/// 捕捉した 1 イベント（本タスクが照合するのは target／event／level のみ）。
#[derive(Debug, Clone)]
pub(crate) struct CapturedEvent {
    pub target: String,
    /// 構造化フィールド `event`（区別語彙）の値。未設定なら `None`。
    pub event: Option<String>,
    pub level: Level,
}

/// `event` フィールド（文字列リテラル）を取り出す訪問子。
struct EventFieldVisitor {
    event: Option<String>,
}

impl Visit for EventFieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "event" {
            self.event = Some(value.to_string());
        }
    }

    // `event` は常に文字列リテラルで渡す規約だが、Debug 経路でも拾えるよう保険を掛ける。
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "event" && self.event.is_none() {
            self.event = Some(format!("{value:?}").trim_matches('"').to_string());
        }
    }
}

/// 捕捉先へイベントを積む Layer。
struct CaptureLayer {
    sink: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = EventFieldVisitor { event: None };
        event.record(&mut visitor);
        let meta = event.metadata();
        self.sink.lock().unwrap().push(CapturedEvent {
            target: meta.target().to_string(),
            event: visitor.event,
            level: *meta.level(),
        });
    }
}

/// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
pub(crate) fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
    let sink: Arc<Mutex<Vec<CapturedEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let layer = CaptureLayer { sink: sink.clone() };
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, f);
    Arc::try_unwrap(sink)
        .expect("capture sink は with_default 終了後に唯一参照へ戻る")
        .into_inner()
        .expect("capture sink mutex は毒化しない")
}

/// 捕捉列に `target="kanade"`・`event=event_name`・`level` のイベントが存在することを表明する。
///
/// アームのログが削除・語彙変更・レベル変更されると本表明は失敗する（Req 6.1／6.3 の回帰檻）。
pub(crate) fn assert_logged(events: &[CapturedEvent], level: Level, event_name: &str) {
    let hit = events.iter().any(|e| {
        e.target == "kanade" && e.level == level && e.event.as_deref() == Some(event_name)
    });
    assert!(
        hit,
        "期待ログ未検出: target=\"kanade\" level={level} event=\"{event_name}\"。\n捕捉={events:#?}"
    );
}
