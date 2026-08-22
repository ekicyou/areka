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

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

/// 捕捉した 1 イベント（照合対象は target／event／outcome／message／level）。
#[derive(Debug, Clone)]
pub(crate) struct CapturedEvent {
    pub target: String,
    /// 構造化フィールド `event`（区別語彙）の値。未設定なら `None`。
    pub event: Option<String>,
    /// 構造化フィールド `outcome`（リソース照会 prefetch 完了固定ログの分類値・R9.3）。未設定なら `None`。
    pub outcome: Option<String>,
    /// イベントメッセージ本文（固定ログ `"shiori resource prefetch done"` の照合に使う）。未設定なら `None`。
    pub message: Option<String>,
    pub level: Level,
    /// 構造化フィールドの全記録（フィールド名 → 値の文字列表現）。
    ///
    /// 文字列フィールドは素の値、それ以外（数値・`?expr` の Debug 記録）は `Debug` 表現が入る。
    /// 「ログに載っていること」自体が要求である値（例: 選択待ち帳簿確立の候補数・期限）を
    /// 檻から突合するために使う。
    pub fields: BTreeMap<String, String>,
}

/// 照合対象フィールド（`event`・`outcome`）とメッセージ本文（`message`）を取り出しつつ、
/// 全フィールドを [`CapturedEvent::fields`] へ記録する訪問子。
struct EventFieldVisitor {
    event: Option<String>,
    outcome: Option<String>,
    message: Option<String>,
    fields: BTreeMap<String, String>,
}

impl Visit for EventFieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
        match field.name() {
            "event" => self.event = Some(value.to_string()),
            "outcome" => self.outcome = Some(value.to_string()),
            "message" => self.message = Some(value.to_string()),
            _ => {}
        }
    }

    // `event`／`outcome` は文字列リテラルで渡す規約だが Debug 経路でも拾えるよう保険を掛ける。
    // メッセージ本文（tracing の `message` フィールド）は fmt::Arguments ゆえ Debug 経路で拾う。
    // 数値・`?expr` など非文字列のフィールドも `Visit` の既定実装が本経路へ落ちるため、
    // ここで全フィールドを記録すれば取りこぼしがない。
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .entry(field.name().to_string())
            .or_insert_with(|| format!("{value:?}"));
        match field.name() {
            "event" if self.event.is_none() => {
                self.event = Some(format!("{value:?}").trim_matches('"').to_string());
            }
            "outcome" if self.outcome.is_none() => {
                self.outcome = Some(format!("{value:?}").trim_matches('"').to_string());
            }
            "message" if self.message.is_none() => {
                self.message = Some(format!("{value:?}"));
            }
            _ => {}
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
        let mut visitor = EventFieldVisitor {
            event: None,
            outcome: None,
            message: None,
            fields: BTreeMap::new(),
        };
        event.record(&mut visitor);
        let meta = event.metadata();
        self.sink.lock().unwrap().push(CapturedEvent {
            target: meta.target().to_string(),
            event: visitor.event,
            outcome: visitor.outcome,
            message: visitor.message,
            level: *meta.level(),
            fields: visitor.fields,
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

/// 捕捉列に `target="kanade"`・`event=event_name` のイベントが**存在しない**ことを表明する。
///
/// [`assert_logged`] の否定側。「この経路では当該語彙が発火してはならない」ことが要求である檻
/// （例: 1 世代 stale 防御が効いているとき `unknown_talk_done` は発火しない・C4 規則 9）で使う。
pub(crate) fn assert_not_logged(events: &[CapturedEvent], event_name: &str) {
    assert!(
        !events
            .iter()
            .any(|e| e.target == "kanade" && e.event.as_deref() == Some(event_name)),
        "発火してはならないログを検出: target=\"kanade\" event=\"{event_name}\"。\n捕捉={events:#?}"
    );
}

/// 捕捉列に ERROR レベルのイベントが 1 件も無いことを表明する。
///
/// 「正常系のユーザー操作で error レベルのログが出ない」ことが完了状態そのものである檻
/// （選択の happy path・タスク 4.6）で使う。target は限定しない——正常系で error を出す層は
/// 存在してはならないためである。
pub(crate) fn assert_no_error_logs(events: &[CapturedEvent]) {
    let errors: Vec<_> = events.iter().filter(|e| e.level == Level::ERROR).collect();
    assert!(
        errors.is_empty(),
        "正常系の操作で error レベルのログが出てはならない。\n検出={errors:#?}"
    );
}

/// 捕捉列から `target="kanade"`・`event=event_name`・`level` のイベントを 1 件取り出す。
///
/// [`assert_logged`] が「発火したこと」だけを固定するのに対し、本関数は**フィールド値まで
/// 突合する檻**（ログに載っていること自体が要求である値の検証）のために本体を返す。
/// 該当が 0 件、または複数件のときは panic する（1 発火であることも同時に固定する）。
pub(crate) fn logged_once<'a>(
    events: &'a [CapturedEvent],
    level: Level,
    event_name: &str,
) -> &'a CapturedEvent {
    let mut hits = events.iter().filter(|e| {
        e.target == "kanade" && e.level == level && e.event.as_deref() == Some(event_name)
    });
    let first = hits.next().unwrap_or_else(|| {
        panic!(
            "期待ログ未検出: target=\"kanade\" level={level} event=\"{event_name}\"。\n捕捉={events:#?}"
        )
    });
    assert!(
        hits.next().is_none(),
        "期待ログは 1 回だけ発火すべき: target=\"kanade\" level={level} event=\"{event_name}\"。\n捕捉={events:#?}"
    );
    first
}

/// リソース照会 prefetch の完了固定ログ（R9.3 grep 証跡）が**ちょうど 1 回**発火したことを表明する。
///
/// 固定ログは `info!(target: "areka_kanade::resource", id = "username", outcome = <outcome_label>,
/// "shiori resource prefetch done")`（design Postconditions・研究 §12-10）。target・level(INFO)・
/// `outcome` フィールド値・message 本文の全一致で照合し、発火回数が 1 であることまで固定する
/// （target が `kanade` でなく `areka_kanade::resource` ゆえ [`assert_logged`] は使えない・専用檻）。
pub(crate) fn assert_resource_prefetch_logged_once(events: &[CapturedEvent], outcome_label: &str) {
    let hits = events
        .iter()
        .filter(|e| {
            e.target == "areka_kanade::resource"
                && e.level == Level::INFO
                && e.outcome.as_deref() == Some(outcome_label)
                && e.message.as_deref() == Some("shiori resource prefetch done")
        })
        .count();
    assert_eq!(
        hits, 1,
        "prefetch 完了固定ログ（target=\"areka_kanade::resource\" level=INFO \
         outcome=\"{outcome_label}\" message=\"shiori resource prefetch done\"）は\
         ちょうど 1 回発火すべき（実際={hits}）。\n捕捉={events:#?}"
    );
}
