//! テスト専用ログ捕捉ヘルパ（Task 4.1 flaky 根治・R9.1／R8.1）。
//!
//! 永続 format 層（[`crate::persist::format`]）は寛容読取の縮退アーム（parse 失敗・未知
//! バージョン）ごとに `tracing::warn!` を発行する（steering:
//! areka-log-first-no-silent-failure・「ログ無しの失敗経路を持たない」）。本モジュールは
//! その発行を**実行可能なテストで**捕捉し、各アームが規約どおりの `target`・レベル（WARN）・
//! メッセージでログを出していることを決定論的に検証可能にする（R9.1「テスト可能領域は全て
//! 実行テスト」）。
//!
//! # 硬化機構は 1 箇所にしかない
//! 捕捉窓そのものは**共有 crate [`log_capture_kit`] へ委譲**する（spec:
//! areka-P0-test-cage-determinism・要件 1.5／2.2）。本モジュールが持つのは
//! 「この crate の檻が照合する形（[`CapturedEvent`]）への変換」と表明ヘルパだけで、
//! subscriber の設置も interest の管理も自前では行わない。以前ここにあった
//! interest-keeper（プロセス全体の既定 subscriber を常駐させる方式）は、
//! 同じ機構が crate ごとに写し取られて写し損ねた側だけが静かに嘘をつく形だったため、
//! 共有 crate へ一本化して撤去した。
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
//! 番兵は返却前に取り除かれるので、呼出側の件数・主張は変わらない。機序の逐条解説と
//! `tracing-core` の実コード引用は `log_capture_kit` の crate doc および同 crate の
//! `src/probe.rs` にある。

use log_capture_kit::CapturedEvent as CanonicalEvent;
use tracing::Level;

/// 捕捉した 1 イベント（format 層の檻が照合するのは target／level／message）。
#[derive(Clone, Debug)]
pub(crate) struct CapturedEvent {
    pub target: String,
    pub level: Level,
    /// 構造化フィールド `message`（マクロ本文）の `Debug` 表現。未設定なら空文字。
    pub message: String,
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
        }
    }
}

/// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
///
/// 捕捉窓は [`log_capture_kit::capture`]（硬化機構の唯一の定義元）。捕捉が働いていなければ
/// 空の結果を静かに返さず panic する。
pub(crate) fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
    let ((), events) = log_capture_kit::capture(f);
    events.iter().map(CapturedEvent::from_canonical).collect()
}

/// 捕捉列に `target`・`level`・`message ⊇ needle` のイベントが存在することを表明する。
///
/// アームのログが削除・語彙変更・レベル変更されると本表明は失敗する（R9.1 の回帰檻）。
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
