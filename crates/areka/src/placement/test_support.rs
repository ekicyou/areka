//! `#[cfg(test)]` 限定: placement 配下のテストが共有する **tracing ログ捕捉ハーネス**。
//!
//! 本モジュールは `#[cfg(test)]` でのみコンパイルされ、本番バイナリには一切含まれない。
//! `source.rs`／`measure.rs` の各 `mod tests` が `use super::test_support::*;` で消費する。
//!
//! # なぜ「素朴な `with_default` 捕捉」では非決定的に取りこぼすのか
//!
//! `tracing::subscriber::with_default` が差し替えるのは**スレッドローカルの既定 dispatcher**
//! だが、**callsite の interest キャッシュはプロセス大域**であり、しかも
//! 「その callsite をプロセス内で最初に踏んだスレッドが勝つ」。
//! （以下 `tracing-core-0.1.36` の実コードに基づく）
//!
//! - `DefaultCallsite::interest()` はキャッシュ未設定（`0xFF`）のとき `register()` を呼ぶ。
//! - `DefaultCallsite::register()`（`callsite.rs:307-318`）は
//!   `rebuild_callsite_interest(self, &DISPATCHERS.rebuilder())` を実行する。
//! - `Dispatchers::rebuilder()`（同 :544-549）は `has_just_one` が真のとき
//!   `Rebuilder::JustOne` を返し、`Rebuilder::for_each`（同 :562-567）はこれを
//!   **`dispatcher::get_default(f)`＝登録したスレッドの既定 dispatcher**で評価する。
//! - subscriber を持たないスレッドの既定は `NoSubscriber` であり、その
//!   `register_callsite` は **`Interest::never()`**（`subscriber.rs:676-678`）を返す。
//! - 結果、`never` が**大域キャッシュへ焼き付く**。以後その callsite のイベントは
//!   `interest.is_never()` の早期 return で**捨てられ**、次に interest 全走査が起きるまで
//!   復旧しない。捕捉テストが自分のスレッドへ subscriber を差していても、
//!   **窓の内側で他スレッドが初回登録すれば取りこぼす**。
//!
//! # 本ハーネスの構造的対策
//!
//! **プロセス寿命の probe dispatcher を 2 個常駐させ、`has_just_one` を恒久的に偽にする。**
//!
//! - `Dispatch::new`（`dispatcher.rs:472-481`）は `callsite::register_dispatch` を呼び、
//!   `LOCKED_DISPATCHERS` へ自身の `Registrar`（`Weak`）を push したうえで
//!   **登録済み全 callsite の interest を再計算**する（`callsite.rs:484-488`）。
//! - `Dispatchers::register_dispatch`（同 :551-558）は `retain(upgrade().is_some())` の後
//!   `has_just_one = (len <= 1)` を書く。probe を **2 個**作れば 2 個目の登録時点で
//!   `len == 2` となり `has_just_one` は**偽**になる。probe の `Arc` は
//!   [`OnceLock`] がプロセス寿命で保持するので `retain` で落ちず、
//!   以後どの `register_dispatch` でも `len >= 2` ＝ **`has_just_one` は二度と真にならない**。
//! - `has_just_one` が偽なら `rebuilder()` は `Rebuilder::Read` を返し、interest は
//!   **生存する登録済み dispatcher 全体の `Interest::and`** で決まる。`get_default`
//!   （＝毒の入口）は二度と参照されない。
//! - probe の `register_callsite` は常に [`Interest::sometimes`] を返す。`Interest::and` は
//!   「両者が異なれば必ず `sometimes`」（`subscriber.rs:652-658`）ゆえ、probe が混ざる限り
//!   **合成結果が `never` になることはない**。`sometimes` は「毎回 `enabled()` を訊く」＝
//!   interest キャッシュが実質無効化された状態であり、判定は現スレッドの dispatcher
//!   （＝捕捉 subscriber）へ委ねられる。
//! - probe 導入**前**（プロセス起動〜最初の捕捉呼び出し）に焼かれた `never` は、
//!   捕捉窓の内側で [`tracing::callsite::rebuild_interest_cache`] を 1 回叩いて解毒する
//!   （この時点で `has_just_one` は偽なので `Read` 経路＝必ず非 `never` へ再計算される）。
//!
//! probe は `enabled()` が偽・`event()` が no-op ゆえ、他テストの観測へ副作用を与えない
//! （interest が `always` から `sometimes` へ下がることで `enabled()` が毎回訊かれるように
//! なるが、これは各 subscriber 本来のフィルタ判定が働く方向の変化である）。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use tracing::subscriber::Interest;

/// 捕捉した 1 イベント（level ＋ フィールド名 → Debug 表現）。
#[derive(Debug, Clone)]
pub(crate) struct LogEvent {
    /// イベントのレベル（`error!`／`warn!`／`debug!` の別＝縮退梯子の契約）。
    pub(crate) level: tracing::Level,
    /// 構造化フィールド（`message` を含む）の Debug 表現。
    pub(crate) fields: BTreeMap<String, String>,
}

impl LogEvent {
    /// `message` フィールド（本文）。無ければ空文字（panic しない）。
    pub(crate) fn message(&self) -> &str {
        self.fields.get("message").map(String::as_str).unwrap_or("")
    }

    /// 構造化フィールドの Debug 表現。欠落は失敗（フィールド名も契約のうち）。
    pub(crate) fn field(&self, name: &str) -> &str {
        self.fields
            .get(name)
            .unwrap_or_else(|| panic!("ログフィールド `{name}` が無い: {:?}", self.fields))
    }
}

/// 全フィールドを Debug 表現で拾う visitor。
///
/// [`tracing::field::Visit`] の `record_u64`／`record_f64`／`record_str` 等はすべて既定実装が
/// `record_debug` へ転送するため、`record_debug` 1 本で型を問わず全フィールドを捕捉できる。
struct FieldGrab<'a>(&'a mut BTreeMap<String, String>);

impl tracing::field::Visit for FieldGrab<'_> {
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        self.0.insert(f.name().to_string(), format!("{v:?}"));
    }
}

/// イベントを溜めるだけの最小 subscriber（span は使わないので `new_span` は固定 id を返す）。
#[derive(Clone, Default)]
struct CaptureSubscriber(Arc<Mutex<Vec<LogEvent>>>);

impl tracing::Subscriber for CaptureSubscriber {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = BTreeMap::new();
        event.record(&mut FieldGrab(&mut fields));
        self.0
            .lock()
            .expect("捕捉バッファの毒化なし")
            .push(LogEvent {
                level: *event.metadata().level(),
                fields,
            });
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

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
/// 登録直後に `has_just_one` が真のままとなり、次の `register_dispatch` までの隙間で
/// `Rebuilder::JustOne`（毒の経路）が生き残る。2 個目の登録で確定的に偽へ落とす。
fn ensure_interest_probes() {
    static PROBES: OnceLock<(tracing::Dispatch, tracing::Dispatch)> = OnceLock::new();
    PROBES.get_or_init(|| {
        // `Dispatch::new` が `callsite::register_dispatch` を呼ぶ（＝登録＋全走査再計算）。
        let first = tracing::Dispatch::new(InterestProbe);
        let second = tracing::Dispatch::new(InterestProbe);
        (first, second)
    });
}

/// クロージャ実行中に**現在のスレッド**で発火した tracing イベントを戻り値と共に返す。
///
/// callsite interest 毒化への対策はモジュール doc を参照（probe 常駐＋窓内 rebuild）。
pub(crate) fn capture_logs<R, F: FnOnce() -> R>(f: F) -> (R, Vec<LogEvent>) {
    ensure_interest_probes();

    let cap = CaptureSubscriber::default();
    let sink = cap.0.clone();
    // `with_default` は内部で `Dispatch::new`（＝register_dispatch＋全 callsite 再計算）を
    // 行うため、この時点で既存の `never` は解毒されている。
    let out = tracing::subscriber::with_default(cap, || {
        // probe 常駐前（プロセス起動〜初回捕捉）に焼かれた `never` の掃き残しを、
        // 窓が開いた**後**の時点でもう一度確定的に潰す。
        tracing::callsite::rebuild_interest_cache();
        f()
    });
    let events = sink.lock().expect("捕捉バッファの毒化なし").clone();
    (out, events)
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
