//! テスト専用: `tracing` の callsite interest キャッシュ毒化を防ぐ常駐 probe（`#[cfg(test)]` 限定）。
//!
//! 本クレートには `capture_logs` 系のログ捕捉ヘルパーが 3 箇所ある
//! （`actor_test_support.rs`／`looper_tests.rs`／`state_test_support.rs`）。
//! いずれも `tracing::subscriber::with_default` でスレッドローカルに subscriber を差すが、
//! **それだけでは並行実行で取りこぼす**——本モジュールはその対策を 1 箇所へ集約する。
//!
//! # なぜ `with_default` だけでは足りないのか
//!
//! `with_default` が差し替えるのはスレッドローカルの既定 dispatcher だが、
//! **callsite の interest キャッシュはプロセス大域**で、その callsite をプロセス内で
//! 最初に踏んだスレッドが勝つ。subscriber を持たないスレッドの既定は `NoSubscriber` で、
//! その `register_callsite` は `Interest::never()` を返すため、`never` が大域キャッシュへ
//! 焼き付き、以後そのイベントは早期 return で捨てられる。捕捉テストが自分のスレッドへ
//! subscriber を差していても、窓の内側で他スレッドが初回登録すれば取りこぼす。
//!
//! 症状は「単独実行では常に緑・`cargo test --workspace` の並行負荷下でのみ捕捉が空になる」
//! という間欠失敗で、`areka-emo-atlas` と本クレートの双方で実測された（2026-08-14）。
//!
//! # 対策
//!
//! **プロセス寿命の probe dispatcher を 2 個常駐させ、`has_just_one` を恒久的に偽にする。**
//! 2 個必要なのは `has_just_one = (dispatchers.len() <= 1)` ゆえ——1 個では登録直後に
//! 真のままとなり、次の `register_dispatch` までの隙間で毒の経路が生き残る。
//! probe の `register_callsite` は常に `Interest::sometimes` を返し、`Interest::and` は
//! 「両者が異なれば必ず `sometimes`」ゆえ合成結果が `never` へ落ちない。
//! probe は `enabled()` が偽・`event()` が no-op なので、他テストの観測へ副作用を与えない。
//!
//! 加えて捕捉窓の内側で [`tracing::callsite::rebuild_interest_cache`] を 1 回叩き、
//! probe 常駐前（プロセス起動〜初回捕捉）に焼かれた `never` を解毒する。
//!
//! 根因の逐条解説（`tracing-core-0.1.36` の実コード行番号つき）は
//! `crates/areka/src/placement/test_support.rs` のモジュール doc を参照。

use std::sync::OnceLock;

use tracing::subscriber::Interest;

/// interest キャッシュへ `never` を焼かせないための常駐 dispatcher。
///
/// `register_callsite` が常に [`Interest::sometimes`] を返すことだけが仕事で、
/// `enabled()` は偽・`event()` は no-op（観測への副作用なし）。
struct InterestProbe;

impl tracing::Subscriber for InterestProbe {
    fn register_callsite(&self, _meta: &'static tracing::Metadata<'static>) -> Interest {
        // 既定実装は `enabled()` が偽なら `never` を返してしまう。ここを `sometimes` に
        // 固定することが本 probe の唯一の存在理由。
        Interest::sometimes()
    }
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        false
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, _event: &tracing::Event<'_>) {}
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// probe dispatcher を 2 個プロセス寿命で常駐させる（冪等）。
///
/// 本クレートの全 `capture_logs` 系ヘルパーは、subscriber を差す**前**に必ずこれを呼ぶ。
pub(crate) fn ensure_interest_probes() {
    static PROBES: OnceLock<(tracing::Dispatch, tracing::Dispatch)> = OnceLock::new();
    PROBES.get_or_init(|| {
        // `Dispatch::new` が `callsite::register_dispatch` を呼ぶ（＝登録＋全走査再計算）。
        let first = tracing::Dispatch::new(InterestProbe);
        let second = tracing::Dispatch::new(InterestProbe);
        (first, second)
    });
}
