//! テスト専用ログ捕捉ヘルパ（固定ログイベントの検証用）。
//!
//! ghost provider（[`crate::sylphya_wiring::from_sylphya_provider`]）は talk スナップショット
//! 生成のたびに固定ログ `debug!(target: "areka_ghost", "talk snapshot from sylphya reader")` を
//! 発行する。本モジュールはその発行を実行可能なテストで捕捉し、規約どおりの `target`・レベル（DEBUG）・
//! メッセージで出ていることを決定論的に検証可能にする。
//!
//! # 硬化機構は 1 箇所にしかない
//! 捕捉窓そのものは**共有 crate [`log_capture_kit`] へ委譲**する。本モジュールが持つのは
//! 「この crate のテストが照合する形（[`CapturedEvent`]）への変換」と表明ヘルパだけで、
//! subscriber の設置も interest の管理も自前では行わない。
//!
//! 本 crate 内で独自にフィールドを抜き出したいテスト（[`crate::sink`] の構造化ログのテスト）は
//! [`capture_events`] から正準イベントを受け取り、[`label_of`] で値を読む。捕捉窓は crate 内でも
//! ここ 1 箇所に集める。
//!
//! # 機序（「スレッドローカルゆえ安全」は誤り）
//! `tracing::subscriber::with_default` が差し替えるのはスレッドローカルの既定 dispatcher だけだが、
//! callsite の **interest キャッシュはプロセス全体で 1 つ**で、その発行点を最初に踏んだスレッドの判定が
//! 焼き付く。捕捉窓を持たないスレッドが先に踏むと `never` が焼き付き、以後そのイベントは捨てられる
//! （不在の主張は静かに緑・存在の主張は確率的に赤）。[`log_capture_kit::capture`] はこれを塞ぐ。
//! 捕捉されるのは呼出スレッドで同期的に発火したイベントだけで、別スレッド（アクター）のログを見る
//! 統合テストは全スレッド窓口（[`log_capture_kit::install_global_capture_all`]）を使う。
//! 機序の詳細は `log_capture_kit` の crate doc にある。

use log_capture_kit::CapturedEvent as CanonicalEvent;
use tracing::Level;

/// 捕捉した 1 イベント（target／level／message／event）。
#[derive(Clone, Debug)]
pub(crate) struct CapturedEvent {
    pub target: String,
    pub level: Level,
    /// 構造化フィールド `message`（マクロ本文）の `Debug` 表現。未設定なら空文字。
    pub message: String,
    /// 構造化フィールド `event`（区別語彙）の値。未設定なら `None`。
    ///
    /// 語彙は message 本文ではなくこのフィールドが担うため、語彙の照合（[`assert_logged_event`]）は
    /// 本文言い回しの変更では緑のまま・語彙の改名では確実に落ちる。
    pub event: Option<String>,
}

/// 構造化フィールド `name`（語彙の `event` など）の値を取り出す。
///
/// `record_str`（文字列リテラルで渡す規約の正路）の生値が 1 つでもあれば最後の生値、無ければ最初の Debug 表現
/// （`{:?}` の囲み引用符を剥がす・`%`／`?` で渡した場合の保険）。
///
/// **[`log_capture_kit::CapturedEvent::field_str`] だけで書いてはならない**——`?expr`／`%expr` で渡された
/// フィールドは `record_str` を通らず `None` になり、その経路の値が黙って落ちて語彙の判定が空振りになる
/// （`log_capture_kit` の crate doc「注意」節）。
pub(crate) fn label_of(ev: &CanonicalEvent, name: &str) -> Option<String> {
    let mut last_str: Option<&str> = None;
    let mut first_debug: Option<&str> = None;
    for (field_name, value) in &ev.fields {
        if field_name != name {
            continue;
        }
        match &value.str_raw {
            Some(raw) => last_str = Some(raw.as_str()),
            None if first_debug.is_none() => first_debug = Some(value.debug.as_str()),
            None => {}
        }
    }
    match (last_str, first_debug) {
        (Some(raw), _) => Some(raw.to_string()),
        (None, Some(debug)) => Some(debug.trim_matches('"').to_string()),
        (None, None) => None,
    }
}

impl CapturedEvent {
    /// 共有機構の正準イベントから、本 crate のテストが照合する形へ変換する。
    ///
    /// `message` は `fmt::Arguments` ゆえ `record_debug` 経路で届き、その `{:?}` は整形済みの本文そのもの
    /// （引用符なし）になる。欠落時は空文字。
    fn from_canonical(ev: &CanonicalEvent) -> Self {
        Self {
            target: ev.target.clone(),
            level: ev.level,
            message: ev.message().to_string(),
            event: label_of(ev, "event"),
        }
    }
}

/// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを**正準形のまま**返す。
///
/// 本 crate で捕捉窓を開くのはここ 1 箇所だけ。[`CapturedEvent`] が持たないフィールドを見たいテスト
/// （[`crate::sink`] の `at`／`actor`／`command_kind`）は [`label_of`] で必要な値を取り出す。
pub(crate) fn capture_events<F: FnOnce()>(f: F) -> Vec<CanonicalEvent> {
    let ((), events) = log_capture_kit::capture(f);
    events
}

/// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
///
/// 捕捉窓は [`log_capture_kit::capture`]。捕捉が働いていなければ空の結果を静かに返さず panic する。
pub(crate) fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
    capture_events(f)
        .iter()
        .map(CapturedEvent::from_canonical)
        .collect()
}

/// 捕捉列に `target`・`level`・`message ⊇ needle` のイベントが存在することを表明する。
///
/// 固定ログが削除・語彙変更・レベル変更されると本表明は失敗する。
pub(crate) fn assert_logged(events: &[CapturedEvent], level: Level, target: &str, needle: &str) {
    let hit = events
        .iter()
        .any(|e| e.target == target && e.level == level && e.message.contains(needle));
    assert!(
        hit,
        "期待ログ未検出: target={target:?} level={level} message⊇{needle:?}。\n捕捉={:?}",
        events
            .iter()
            .map(|e| (e.target.clone(), e.level, e.message.clone()))
            .collect::<Vec<_>>()
    );
}

/// 捕捉列に `target`・`level`・構造化フィールド `event == event_name` のイベントが存在することを
/// 表明する（語彙固定）。
///
/// [`assert_logged`] が message 本文で照合するのに対し、こちらは**語彙フィールド**で照合する。
/// 語彙の削除・改名・レベル変更で落ちる（本文の言い回し変更では落ちない）。
pub(crate) fn assert_logged_event(
    events: &[CapturedEvent],
    level: Level,
    target: &str,
    event_name: &str,
) {
    let hit = events
        .iter()
        .any(|e| e.target == target && e.level == level && e.event.as_deref() == Some(event_name));
    assert!(
        hit,
        "期待ログ未検出: target={target:?} level={level} event={event_name:?}。\n捕捉={:?}",
        events
            .iter()
            .map(|e| (
                e.target.clone(),
                e.level,
                e.event.clone(),
                e.message.clone()
            ))
            .collect::<Vec<_>>()
    );
}
