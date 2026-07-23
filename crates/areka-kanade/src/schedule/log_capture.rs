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
//!
//! ## 並列負荷下の確率欠陥と interest-keeper による根治
//! `with_default` は呼出ごとに transient dispatcher を登録/破棄する。`cargo test --workspace`
//! の並列負荷下では **live dispatcher が 0 個になる瞬間**が生じ、その窓で別スレッドの
//! callsite が初回 `register()` されると tracing-core が Interest を `Never` に**焼き付ける**
//! （tracing-core 0.1.36 `callsite.rs:505` の `unwrap_or_else(Interest::never)`・sticky で
//! 次の dispatcher 登録まで復活しない）。焼き付いた callsite のイベントは以降マクロの静的
//! チェックで捨てられ、対象檻が ~1/10〜1/20 の確率で偽赤する。これは areka-P0-kanade 時代から
//! main に存在した欠陥で、並列度が上がると露呈する。
//!
//! 根治は [`install_interest_keeper`] が初回 [`capture`] で一度だけ確立する
//! **プロセスグローバル interest-keeper**（[`INTEREST_KEEPER`]）。素の
//! `tracing_subscriber::registry()`（per-layer filter 無し・`register_callsite=always`・
//! `event` は no-op＝`sharded.rs:222-235` の bare registry 挙動）を
//! `set_global_default` で常駐させると、その subscriber Arc は leak され
//! （`dispatcher.rs:314-319`）**プロセス生存期間ずっと registrar が生き続ける**。以後は
//! live dispatcher 数が 0 になる瞬間が構造的に存在しなくなり、`callsite.rs:505` の
//! `Interest::never` 焼き付きが**到達不能**になる。確立の瞬間に全 callsite が rebuild され
//! （`callsite.rs:484-488`）、確立以前に焼き付いた callsite も治癒される。
//!
//! ## 不変条件
//! **本モジュール（interest-keeper）より先に、別のグローバル subscriber を設定してはならない。**
//! フィルタ付きの global を先に置くと callsite Interest がそちらの評価で `Never` へ焼き付き直され
//! capture が壊れる。違反時は [`install_interest_keeper`] が `set_global_default` の `Err` を
//! `.expect()` で受けて**大声で panic** する（silent 縮退なし・log-first）。
//!
//! bare registry を global 常駐させても capture の意味論は不変: capture 外のイベントは Registry の
//! `event` no-op へ落ち（従来の `NoSubscriber` と観測差なし）、capture 内のイベントは
//! `with_default` の thread-local subscriber が global を shadow する（`dispatcher.rs:379-398`）
//! ため thread-local のみへ配送され、捕捉列が相互に混在しない。

use std::sync::{Arc, Mutex, OnceLock};

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

/// プロセスグローバル interest-keeper（一度だけ確立・leak されて永久生存）。
///
/// 確立済みか否かの二値のみを保持する（中身なし）。`OnceLock` が初期化競合を直列化するため、
/// 並行初回呼出でも `set_global_default` はプロセスで高々 1 回しか走らない。
static INTEREST_KEEPER: OnceLock<()> = OnceLock::new();

/// tracing callsite Interest の `Never` 焼き付きを根絶する（機構の詳細はモジュール doc「決定性の要」）。
///
/// [`capture`] の先頭から呼ぶ。初回のみ素の `registry()` を global default として常駐させ、
/// 全 callsite の Interest を再評価する。2 回目以降は `OnceLock` の no-op 参照で即返る。
fn install_interest_keeper() {
    INTEREST_KEEPER.get_or_init(|| {
        // bare registry（per-layer filter 無し）を global default として leak・常駐させる。
        // 以降 live dispatcher 数が 0 になる瞬間が消え、Interest::never 焼き付きが到達不能になる。
        tracing::subscriber::set_global_default(tracing_subscriber::registry()).expect(
            "log_capture の interest-keeper より先に別のグローバル subscriber を設定してはならない: \
             フィルタ付き global は callsite Interest を Never へ焼き付け直し capture を壊す。\
             対処: テストプロセスで log_capture 以外の set_global_default / init を行わないこと",
        );
        // 登録時 rebuild で理論上は十分だが、確立以前に焼き付いた callsite の再評価保険＋意図の
        // 自己文書化として明示呼出する（コスト 0）。
        tracing::callsite::rebuild_interest_cache();
    });
}

/// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
pub(crate) fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
    install_interest_keeper();
    let sink: Arc<Mutex<Vec<CapturedEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let layer = CaptureLayer { sink: sink.clone() };
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, f);
    // NOTE: `with_default` 終了後も tracing は Dispatch（subscriber＝sink を内包する Layer を
    // 保持）を一時的に clone し得るため、sink の strong_count が 1 に戻る保証はない。
    // よって `Arc::try_unwrap` は並列テスト下で稀に失敗し panic する（タスク 6.2 レビューで
    // 発覚した flaky race・~1/10〜1/20）。参照数に依存せず、ロック下で捕捉列を取り出して返す。
    // f 完了後に本スレッドで新規イベントは発行されないため take で安全に確定できる。
    std::mem::take(&mut *sink.lock().expect("capture sink mutex は毒化しない"))
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
