//! 常駐 probe。発行点の interest を `sometimes` に合成し、他スレッドの先着で `never` が
//! 焼き付く経路を閉じる（プロセス寿命で冪等に確立する）。
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
//! したがって「`with_default` はスレッドローカルだから並行実行でも干渉しない」は**誤り**で
//! ある。干渉しないのは *dispatcher* だけで、*interest* は全スレッドで 1 つしかない。
//!
//! # 構造的対策
//!
//! **プロセス寿命の probe dispatcher を 2 個常駐させ、`has_just_one` を恒久的に偽にする。**
//!
//! - `Dispatch::new`（`dispatcher.rs:472-481`）は `callsite::register_dispatch` を呼び、
//!   `LOCKED_DISPATCHERS` へ自身の `Registrar`（`Weak`）を push したうえで
//!   **登録済み全 callsite の interest を再計算**する（`callsite.rs:484-488`）。
//! - `Dispatchers::register_dispatch`（同 :551-558）は `retain(upgrade().is_some())` の後
//!   `has_just_one = (len <= 1)` を書く。probe を **2 個**作れば 2 個目の登録時点で
//!   `len == 2` となり `has_just_one` は**偽**になる。probe の `Arc` は
//!   [`std::sync::OnceLock`] がプロセス寿命で保持するので `retain` で落ちず、
//!   以後どの `register_dispatch` でも `len >= 2` ＝ **`has_just_one` は二度と真にならない**。
//! - `has_just_one` が偽なら `rebuilder()` は `Rebuilder::Read` を返し、interest は
//!   **生存する登録済み dispatcher 全体の `Interest::and`** で決まる。`get_default`
//!   （＝毒の入口）は二度と参照されない。
//! - probe の `register_callsite` は常に [`tracing::subscriber::Interest::sometimes`] を返す。
//!   `Interest::and` は「両者が異なれば必ず `sometimes`」（`subscriber.rs:652-658`）ゆえ、
//!   probe が混ざる限り**合成結果が `never` になることはない**。`sometimes` は
//!   「毎回 `enabled()` を訊く」＝ interest キャッシュが実質無効化された状態であり、
//!   判定は現スレッドの dispatcher（＝捕捉 subscriber）へ委ねられる。
//! - probe 導入**前**（プロセス起動〜最初の捕捉呼び出し）に焼かれた `never` は、
//!   捕捉窓の内側で [`tracing::callsite::rebuild_interest_cache`] を 1 回叩いて解消する
//!   （この時点で `has_just_one` は偽なので `Read` 経路＝必ず非 `never` へ再計算される）。
//!
//! probe は `enabled()` が偽・`event()` が no-op ゆえ、他テストの観測へ副作用を与えない
//! （interest が `always` から `sometimes` へ下がることで `enabled()` が毎回訊かれるように
//! なるが、これは各 subscriber 本来のフィルタ判定が働く方向の変化である）。
//!
//! # 測定のための切替（要件 13.1・設計 C9）
//!
//! 硬化の代償として支払っている実行時間を測るため、**常駐の確立だけ**を環境変数
//! `AREKA_LOG_CAPTURE_PROBES` で無効にできる（測定専用の口）。判定は
//! [`std::sync::OnceLock`] の初期化の中でプロセス寿命に 1 度だけ行うので、測定対象の
//! 実行時間へ上乗せしない。変数が無いときの挙動は本切替の導入前と 1 ビットも変わらない。
//!
//! 無効側では `tracing::Dispatch::new` を**1 度も呼ばない**。`Dispatch::new` 自身が
//! 登録（`callsite::register_dispatch`）だからで、作って**保持し続ける**形にすると
//! 「無効にしたつもりで実は常駐している」＝測定が無意味になる。
//!
//! 「保持し続ける」の限定は実測に基づく（タスク 11.1 のレビューが変異で確かめた）。無効側で
//! 2 個作って**即座に捨てる**変異では硬化は残らなかった——捕捉窓が次の `register_dispatch` を
//! 起こし、死んだ弱参照が掃かれて `has_just_one` が真へ戻るためである。硬化が生き残るのは
//! `std::mem::forget` などで**生かしたまま**にした場合だった。**そもそも作らない**本実装は
//! どちらの場合よりも安全である。

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

/// 常駐の確立を無効にする環境変数（測定専用・要件 13.1／設計 C9）。
///
/// 本ワークスペースの実行時環境変数は `AREKA_` 名前空間であり、既存の較正用変数
/// `AREKA_LOG_CAPTURE_CALIBRATION`（`tests/capture_calibration_test.rs`）と同じ流儀
/// ——**値は逐語の語**で、綴りが合わなければ黙って既定へ落ちない——に揃えてある。
const PROBES_ENV: &str = "AREKA_LOG_CAPTURE_PROBES";

/// 常駐する（未設定と同じ）。A/B の対照側を明示して走らせるときに使う。
const PROBES_ON: &str = "on";

/// 常駐しない（測定の被験側）。
const PROBES_OFF: &str = "off";

/// probe を常駐させるかどうかの指定を読む。**プロセス寿命で 1 度だけ**呼ばれる。
///
/// 未設定なら真（＝本切替の導入前と同一の挙動）。値が `on`／`off` のどちらでもないとき、
/// および値が Unicode でないときは panic する——「立てたつもりで綴りを誤り、既定側を
/// 測っていた」を静かに通さないためである。要件 13.1 の A/B は、どちらの側を走らせたかが
/// 確定していなければ数字として意味を持たない。
fn probes_requested() -> bool {
    #[cfg(test)]
    PROBES_ENV_READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let Some(raw) = std::env::var_os(PROBES_ENV) else {
        return true; // 未設定＝既定。
    };
    let text = raw.to_str().unwrap_or_else(|| {
        panic!("{PROBES_ENV} の値が Unicode ではない: {raw:?}");
    });
    match text.trim() {
        PROBES_ON => true,
        PROBES_OFF => false,
        other => panic!(
            "{PROBES_ENV} の値が不正: {other:?}。`{PROBES_ON}`（常駐する＝未設定と同じ）か \
             `{PROBES_OFF}`（常駐しない＝測定の被験側）のいずれかを指定すること"
        ),
    }
}

/// [`probes_requested`] を呼んだ回数（テスト専用の計器）。
///
/// 「プロセス寿命で 1 度だけ読む」は、**回数を数えなければ判別できない**。毎回読む形へ変えても
/// 同一プロセス内では答えが動かないので、判定の一致だけを見る檻は恒真になる——タスク 11.1 の
/// レビューが、`probes_requested()` を毎回呼ぶ変異を当てても檻が 3 本とも緑のままであることを
/// 実測した。ここで実際に数えることで、その変異が赤になる。
#[cfg(test)]
static PROBES_ENV_READS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// 環境変数を読んだ回数（テスト専用）。
#[cfg(test)]
pub(crate) fn probes_env_read_count() -> u32 {
    PROBES_ENV_READS.load(std::sync::atomic::Ordering::Relaxed)
}

/// 常駐 probe の実体を 1 度だけ用意する（無効指定なら `None`）。
///
/// `None` の側では [`tracing::Dispatch`] を**作らない**。生成そのものが登録なので、
/// 作ってから保持しない形にすると常駐が消えず、測定の被験側が成立しない。
fn probes() -> &'static Option<(tracing::Dispatch, tracing::Dispatch)> {
    static PROBES: OnceLock<Option<(tracing::Dispatch, tracing::Dispatch)>> = OnceLock::new();
    PROBES.get_or_init(|| {
        if !probes_requested() {
            return None;
        }
        // `Dispatch::new` が `callsite::register_dispatch` を呼ぶ（＝登録＋全走査再計算）。
        let first = tracing::Dispatch::new(InterestProbe);
        let second = tracing::Dispatch::new(InterestProbe);
        Some((first, second))
    })
}

/// probe dispatcher を**2 個**プロセス寿命で常駐させる（冪等）。
///
/// 2 個必要なのは `has_just_one = (dispatchers.len() <= 1)` ゆえ——1 個では
/// 登録直後に `has_just_one` が真のままとなり、次の `register_dispatch` までの隙間で
/// `Rebuilder::JustOne`（毒の経路）が生き残る。2 個目の登録で確定的に偽へ落とす。
///
/// 通常は [`crate::capture`] が窓を開くたびに内部で呼ぶので、消費側が直接呼ぶ必要は無い。
/// 捕捉窓を開かずに常駐だけ確立したい場合（プロセス冒頭での明示的な確立など）に公開する。
///
/// 何度呼んでも、また複数スレッドから同時に呼んでも安全（[`OnceLock`] が 1 回に畳む）。
///
/// 環境変数 `AREKA_LOG_CAPTURE_PROBES=off` が立っているときだけ、登録せずに戻る
/// （測定専用の切替・要件 13.1）。立っているかどうかは [`interest_probes_enabled`] で読める。
pub fn ensure_interest_probes() {
    let _ = probes();
}

/// 常駐 probe が**実際に確立されているか**を返す（確立がまだなら、その場で判定して確立する）。
///
/// 測定（要件 13.1 の A/B）で「環境変数を立てたつもりで立っていなかった」を炙り出すための
/// 窓口である。値そのものではなく**この判定の結果**を返すので、変数が測定対象のプロセスまで
/// 届いていたかどうかを、切替と同じ経路で確かめられる。
pub fn interest_probes_enabled() -> bool {
    probes().is_some()
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod tests;
