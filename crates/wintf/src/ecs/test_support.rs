//! `#[cfg(test)]` 限定: wintf のテストが共有する **tracing ログ濾過ハーネス**。
//!
//! 本モジュールは `#[cfg(test)]` でのみコンパイルされ、本番バイナリには一切含まれない。
//! 診断手順書が指定する `RUST_LOG` 相当の directive を [`tracing_subscriber::EnvFilter`] で
//! 実際に適用し、その濾過を通過した出力だけを文字列として返す。
//! 「観測点が手順で有効化される水準に置かれている」（要件 1.5）を、定数の目視ではなく
//! **実濾過**で機械化するための土台である。
//!
//! # なぜ「素朴な `with_default` 捕捉」では非決定的に取りこぼすのか
//!
//! `tracing::subscriber::with_default` が差し替えるのは**スレッドローカルの既定 dispatcher**
//! だが、**callsite の interest キャッシュはプロセス大域**であり、しかも
//! 「その callsite をプロセス内で最初に踏んだスレッドが勝つ」。subscriber を持たない
//! スレッドの既定は `NoSubscriber` で、その `register_callsite` は `Interest::never()` を
//! 返すため、`never` が大域キャッシュへ焼き付き、以後そのイベントは捨てられる。
//!
//! # 本ハーネスの構造的対策
//!
//! **プロセス寿命の probe dispatcher を 2 個常駐させ、`has_just_one` を恒久的に偽にする。**
//! `Dispatchers::register_dispatch` は `has_just_one = (len <= 1)` を書くため、probe を 2 個
//! 常駐させると interest 再計算が「登録済み dispatcher 全体の `Interest::and`」経路へ移り、
//! `NoSubscriber` の `never` が焼き付く入口（`Rebuilder::JustOne`）が使われなくなる。
//! probe 導入前に焼かれた `never` は、捕捉窓の内側で
//! [`tracing::callsite::rebuild_interest_cache`] を 1 回叩いて解毒する。
//!
//! （対策の根拠と詳細は areka `placement/test_support.rs` の module doc と同一。
//! wintf は areka に依存できないため、同型の対策を本クレート内に持つ。）

use std::sync::{Arc, Mutex, OnceLock};

use tracing::subscriber::Interest;
use tracing_subscriber::EnvFilter;

/// interest キャッシュへ `never` を焼かせないための常駐 dispatcher。
///
/// `register_callsite` が常に [`Interest::sometimes`] を返すことだけが仕事で、
/// `enabled()` は偽・`event()` は no-op（観測への副作用なし）。
struct InterestProbe;

impl tracing::Subscriber for InterestProbe {
    fn register_callsite(&self, _meta: &'static tracing::Metadata<'static>) -> Interest {
        // 既定実装は `enabled()` が偽なら `never` を返してしまう。ここを `sometimes` に
        // 固定することが本 probe の唯一の存在理由（`Interest::and` は差異があれば
        // 必ず `sometimes` ＝ 合成結果が `never` へ落ちない）。
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

/// probe dispatcher を**2 個**プロセス寿命で常駐させる（冪等）。
///
/// 2 個必要なのは `has_just_one = (dispatchers.len() <= 1)` ゆえ——1 個では
/// 登録直後に `has_just_one` が真のままとなり、毒の経路が生き残る。
pub(crate) fn ensure_interest_probes() {
    static PROBES: OnceLock<(tracing::Dispatch, tracing::Dispatch)> = OnceLock::new();
    PROBES.get_or_init(|| {
        // `Dispatch::new` が `callsite::register_dispatch` を呼ぶ（＝登録＋全走査再計算）。
        let first = tracing::Dispatch::new(InterestProbe);
        let second = tracing::Dispatch::new(InterestProbe);
        (first, second)
    });
}

/// 共有バッファへ書き出す `io::Write`（`fmt` subscriber の writer）。
#[derive(Clone)]
struct VecWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("捕捉バッファの毒化なし")
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `RUST_LOG` 相当の `directives` を実際に適用し、`f` の実行中に**現在のスレッド**で
/// 濾過を通過した tracing 出力を文字列で返す。
///
/// 診断手順書の directive をそのまま渡せる（例:
/// `"info,wintf::ecs::window_proc=debug"`）。通過しなかった観測点は 1 文字も現れない。
pub(crate) fn capture_under_filter<F: FnOnce()>(directives: &str, f: F) -> String {
    ensure_interest_probes();

    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(directives))
        .with_ansi(false)
        .with_writer(move || VecWriter(sink.clone()))
        .finish();

    // `with_default` は内部で `Dispatch::new`（＝register_dispatch＋全 callsite 再計算）を
    // 行うが、probe 導入前に焼かれた `never` の掃き残しを窓の内側でもう一度潰す。
    tracing::subscriber::with_default(subscriber, || {
        tracing::callsite::rebuild_interest_cache();
        f();
    });

    String::from_utf8(buf.lock().expect("捕捉バッファの毒化なし").clone()).expect("UTF-8")
}
