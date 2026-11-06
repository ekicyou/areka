//! `#[cfg(test)]` 限定: placement 配下のテストが共有する **ログ捕捉の判定ヘルパ**。
//!
//! 本モジュールは `#[cfg(test)]` でのみコンパイルされ、本番バイナリには一切含まれない。
//! `source.rs`／`measure.rs` の各 `mod tests` が `use super::test_support::*;` で消費する。
//!
//! # 捕捉そのものはここに無い
//!
//! 捕捉窓・常駐の仕掛け・イベントの正準表現は、ワークスペースで 1 箇所だけ定義される
//! `log-capture-kit` にある。本モジュールが持つのは ⑴ 正準型への別名 ⑵ 呼出側の判定を
//! 短く書くための補助（[`expect_one`]・[`ExpectField`]）⑶ 委譲 1 行の [`capture_logs`]
//! だけである。使い方（存在／不在の主張の書き方・全スレッド捕捉との使い分け）は
//! `log_capture_kit` の crate doc が利用手順として書いている。
//!
//! # なぜ「素朴な `with_default` 捕捉」では非決定的に取りこぼすのか
//!
//! 「`tracing::subscriber::with_default` はスレッドローカルだから並行実行でも干渉しない」は
//! **誤り**である。差し替わるのはスレッドローカルの既定 dispatcher だけで、「そのログを
//! 評価するか」を決める **callsite の interest キャッシュはプロセス全体で 1 つ**しかなく、
//! その発行点をプロセス内で最初に踏んだスレッドの判定が焼き付く（先着が勝つ）。捕捉窓を
//! 持たないスレッドの既定は `NoSubscriber` で判定は「不要」ゆえ、先に踏まれると `never` が
//! 大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。結果、不在の主張は
//! 捕捉 0 件のまま静かに緑になり、存在の主張は確率的に赤になる。
//!
//! 共有機構はこれを ⑴ プロセス寿命の probe 常駐（interest が `never` へ落ちる経路を恒久的に
//! 閉じる）⑵ 捕捉窓の内側での interest 再計算 ⑶ 番兵イベントによる空振り検出（捕捉が働いて
//! いなければ panic する）の 3 点で塞ぐ。`tracing-core` の実コードを引いた逐条の機序解説は
//! `log_capture_kit` の crate doc と同 crate の `src/probe.rs` にある。

use log_capture_kit::CapturedEvent;

/// 常駐の仕掛けを捕捉窓を開かずに確立する（冪等）。
///
/// [`capture_logs`] は内部で呼ぶので、自前で捕捉先を差す側だけが直接呼ぶ。
pub(crate) use log_capture_kit::ensure_interest_probes;

/// 捕捉した 1 イベント。正準表現 [`log_capture_kit::CapturedEvent`] の型別名。
///
/// `level`（`error!`／`warn!`／`debug!` の別＝縮退梯子の契約）と、`message` を含む構造化
/// フィールドを持つ。本文は [`CapturedEvent::message`]、フィールドの Debug 表現は
/// [`CapturedEvent::field`]（欠落は `None`）で引く。欠落を失敗として扱いたい判定は
/// [`ExpectField::expect_field`] を使う。
pub(crate) type LogEvent = CapturedEvent;

/// 構造化フィールドの Debug 表現を引き、欠落を失敗として扱う（フィールド名も契約のうち）。
///
/// 正準型は同名の固有メソッド `field()` を `Option<&str>` で持つ。固有メソッドは拡張
/// トレイトより先に解決されるため、この補助は**別名**でなければ黙って無視される。
pub(crate) trait ExpectField {
    /// 構造化フィールドの Debug 表現。欠落は失敗。
    fn expect_field(&self, name: &str) -> &str;
}

impl ExpectField for LogEvent {
    fn expect_field(&self, name: &str) -> &str {
        self.field(name)
            .unwrap_or_else(|| panic!("ログフィールド `{name}` が無い: {:?}", self.fields_map()))
    }
}

/// クロージャ実行中に**現在のスレッド**で発火した tracing イベントを戻り値と共に返す。
///
/// 捕捉層は共有機構への委譲で、硬化（probe 常駐・窓内の interest 再計算・番兵）も
/// 呼出側の判定内容も共有機構側が保証する。
pub(crate) fn capture_logs<R, F: FnOnce() -> R>(f: F) -> (R, Vec<LogEvent>) {
    log_capture_kit::capture(f)
}

/// メッセージに `needle` を含むイベントが**ちょうど 1 件**在ることを主張して返す。
pub(crate) fn expect_one<'a>(events: &'a [LogEvent], needle: &str) -> &'a LogEvent {
    let hits: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "`{needle}` を含むログがちょうど 1 件ではない: {events:?}"
    );
    hits[0]
}
