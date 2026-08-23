//! テスト専用ログ捕捉ヘルパ（Task 10.1・固定ログイベント檻・R9.3／R8.1）。
//!
//! ghost provider（[`crate::sylphya_wiring::from_sylphya_provider`]）は talk スナップショット
//! 生成のたびに固定ログ `debug!(target: "areka_ghost", "talk snapshot from sylphya reader")` を
//! 発行する（design Monitoring・provider 差替の R9.3 サインオフ証跡）。本モジュールはその発行を
//! **実行可能なテストで**捕捉し、規約どおりの `target`・レベル（DEBUG）・メッセージでログが
//! 出ていることを決定論的に検証可能にする（R9.1「テスト可能領域は全て実行テスト」）。
//!
//! # 硬化機構は 1 箇所にしかない
//! 捕捉窓そのものは**共有 crate [`log_capture_kit`] へ委譲**する（spec:
//! areka-P0-test-cage-determinism・要件 1.5／2.2）。本モジュールが持つのは
//! 「この crate の檻が照合する形（[`CapturedEvent`]）への変換」と表明ヘルパだけで、
//! subscriber の設置も interest の管理も自前では行わない。以前ここにあった
//! interest-keeper（プロセス全体の既定 subscriber を常駐させる方式）は、`areka-sylphya`／
//! `areka-kanade` にも同じものが写し取られており、写し損ねた側だけが静かに嘘をつく形だった
//! ため、共有 crate へ一本化して撤去した。
//!
//! 本 crate 内で独自にフィールドを抜き出したいテスト（[`crate::sink`] の構造化ログ檻）は
//! [`capture_events`] から正準イベントを受け取る。捕捉窓は crate 内でもここ 1 箇所に集める。
//!
//! # 機序（「スレッドローカルゆえ安全」は誤り）
//! `tracing::subscriber::with_default` が差し替えるのはスレッドローカルの既定 dispatcher
//! だけで、そこは確かにスレッドごとに独立している。しかし「そのログを評価するか」を決める
//! callsite の **interest キャッシュはプロセス全体で 1 つ**であり、その発行点をプロセス内で
//! 最初に踏んだスレッドの判定が焼き付く。捕捉窓を持たないスレッドの既定は `NoSubscriber` で
//! 判定は「不要」なので、先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を
//! 差していても以後そのイベントは早期 return で捨てられる。結果、不在の主張は捕捉 0 件のまま
//! 静かに緑になり（偽陰性）、存在の主張は捕捉 0 件で確率的に赤になる（偽陽性）。
//!
//! [`log_capture_kit::capture`] はこれを ⑴ プロセス寿命の probe 常駐 ⑵ 窓の内側での
//! interest 再計算 ⑶ 窓の内側で発火する対照イベント（番兵）による空振り検出、の 3 点で塞ぐ。
//! 番兵は返却前に取り除かれるので、呼出側の件数・主張は変わらない。捕捉されるのは呼出
//! スレッドで同期的に発火したイベントだけである点は移行前と同じで、別スレッド（アクター）の
//! ログを見る必要がある統合テストは全スレッド窓口
//! （[`log_capture_kit::install_global_capture_all`]）を使う。機序の逐条解説と `tracing-core`
//! の実コード引用は `log_capture_kit` の crate doc および同 crate の `src/probe.rs` にある。

use log_capture_kit::CapturedEvent as CanonicalEvent;
use tracing::Level;

/// 捕捉した 1 イベント（固定ログ檻が照合するのは target／level／message／event）。
#[derive(Clone, Debug)]
pub(crate) struct CapturedEvent {
    pub target: String,
    pub level: Level,
    /// 構造化フィールド `message`（マクロ本文）の `Debug` 表現。未設定なら空文字。
    pub message: String,
    /// 構造化フィールド `event`（design「ログ語彙表」の区別語彙）の値。未設定なら `None`。
    ///
    /// 語彙は message 本文ではなくこのフィールドが担うため、語彙の檻（[`assert_logged_event`]）は
    /// 本文言い回しの変更では緑のまま・語彙の改名では確実に落ちる。
    pub event: Option<String>,
}

/// 語彙フィールド（`event`）の値を、移行前の visitor と同じ規則で取り出す。
///
/// 移行前は `record_str`（文字列リテラルで渡す規約の正路）を無条件に採り、`record_debug` は
/// 生値がまだ無いときだけ採っていた。すなわち**生値が 1 つでもあれば最後の生値、無ければ
/// 最初の Debug 表現**で、後者は `{:?}` の囲み引用符を剥がす（`%`／`?` で渡した場合の保険）。
///
/// **[`log_capture_kit::CapturedEvent::field_str`] だけで書いてはならない**——`?expr`／`%expr`
/// で渡されたフィールドは `record_str` を通らないため `None` になり、その経路の値が黙って
/// 落ちて語彙の判定が空振りになる（`log_capture_kit` の crate doc「注意」節）。
fn label_of(ev: &CanonicalEvent, name: &str) -> Option<String> {
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
    /// 共有機構の正準イベントから、本 crate の檻が照合する形へ変換する。
    ///
    /// `message` は `fmt::Arguments` ゆえ `record_debug` 経路で届き、その `{:?}` は整形済みの
    /// 本文そのもの（引用符なし）になる。欠落時に空文字を返すのも移行前と同じ
    /// （[`log_capture_kit::CapturedEvent::message`] が `unwrap_or("")`）。
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
/// 本 crate で捕捉窓を開くのはここ 1 箇所だけである。[`CapturedEvent`] が持たないフィールドを
/// 見たいテスト（[`crate::sink`] の `at`／`actor`／`command_kind` 檻）はこちらを使い、
/// [`log_capture_kit::CapturedEvent`] から必要な値を自分で取り出す。
pub(crate) fn capture_events<F: FnOnce()>(f: F) -> Vec<CanonicalEvent> {
    let ((), events) = log_capture_kit::capture(f);
    events
}

/// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
///
/// 捕捉窓は [`log_capture_kit::capture`]（硬化機構の唯一の定義元）。捕捉が働いていなければ
/// 空の結果を静かに返さず panic する。
pub(crate) fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
    capture_events(f)
        .iter()
        .map(CapturedEvent::from_canonical)
        .collect()
}

/// 捕捉列に `target`・`level`・`message ⊇ needle` のイベントが存在することを表明する。
///
/// 固定ログが削除・語彙変更・レベル変更されると本表明は失敗する（R9.3 の回帰檻）。
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
/// 表明する（design「ログ語彙表」の語彙固定・棄却/防御アームの回帰檻）。
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
