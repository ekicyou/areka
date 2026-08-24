//! テスト専用の `tracing` ログ捕捉ハーネス（本番影響ゼロ・`#[cfg(test)]` 限定）。
//!
//! 決定論的テスト網羅の要請（開発者マンデート）に従い、bake 経路が発火する 0 寸/全透明
//! element の `warn!`（ゴースト制作者ミスの可能性が高いための早期警告・設計ディスカッション #1）
//! を、観測可能な副作用ではなく **ログが実際に発火したこと**まで実行テストで檻に入れるための土台。
//! `crates/areka-emo-compose/src/log_capture.rs` の実証済みパターンを踏襲する（target のみ差）。
//!
//! # 設計（本番非侵襲・スレッドローカル）
//!
//! - `fmt`＋`MakeWriter` ではなく最小の [`tracing_subscriber::Layer`] 実装（[`Capture`]）で
//!   イベントの **level／target／フィールド**を文字列化して共有 `Arc<Mutex<Vec<String>>>` へ push する。
//!   フィールド値（`set=...`／`rel_path=...`／`original_w=...`）も `record_debug` で載せるため、
//!   個別ログを判別する discriminating field をテストで突ける。
//! - 導入は `tracing::subscriber::with_default`（**スレッドローカル**）で行い、`set_global_default`
//!   （プロセス全体・並行テストを壊す）は使わない。
//! - bake パイプラインは完全同期で、全ログは呼び出しスレッド上で同期的に発火するため、
//!   `capture_logs` はクロージャ復帰時点で全ログを捕捉済みである。
//!
//! # `with_default` だけでは並行実行で取りこぼす（実測で判明・2026-08-14）
//!
//! 「各テストが自分のコードのみを包む限り並行実行でも干渉しない」というのは**誤り**だった。
//! `with_default` が差し替えるのはスレッドローカルの既定 dispatcher だが、
//! **callsite の interest キャッシュはプロセス大域**で、その callsite をプロセス内で
//! 最初に踏んだスレッドが勝つ。subscriber を持たないスレッドの既定は `NoSubscriber` で、
//! その `register_callsite` は `Interest::never()` を返すため、`never` が大域キャッシュへ
//! 焼き付き、以後そのイベントは早期 return で捨てられる。
//!
//! 症状: `cargo test --workspace` の並行負荷下でのみ `capture_logs` が**空文字列**を返し、
//! `warn_fires_on_all_transparent_element` が約 1/3 の頻度で落ちる（単独実行では常に緑）。
//!
//! 対策は `crates/areka/src/placement/test_support.rs` で確立済みの構造的対策の移植——
//! **プロセス寿命の probe dispatcher を 2 個常駐させて `has_just_one` を恒久的に偽にし**、
//! 加えて捕捉窓の内側で `rebuild_interest_cache` を 1 回叩いて probe 導入前の毒を解く。
//! 根因の逐条解説（`tracing-core-0.1.36` の実コード行番号つき）は上記 `test_support.rs`
//! のモジュール doc を参照。

use std::sync::{Arc, Mutex, OnceLock};

use tracing::field::{Field, Visit};
use tracing::subscriber::Interest;
use tracing_subscriber::prelude::*;

/// イベントの `level`＋`target`＋各フィールドを 1 行文字列へ整形して捕捉する最小 Layer。
///
/// 出力形式（1 イベント 1 行）: `level=WARN target=areka_emo_atlas set=0 rel_path="clear.png" …`。
/// `record_debug` で全フィールドを `name=value` として載せるため、テストは level（WARN/ERROR）と
/// discriminating field（set/rel_path/original_w/original_h）を substring で検証できる。
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
    fn on_event(&self, ev: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let meta = ev.metadata();
        // level と target を先頭に固定で載せる（level 判定・target フィルタの土台）。
        let mut line = format!("level={} target={}", meta.level(), meta.target());
        struct V<'a>(&'a mut String);
        impl Visit for V<'_> {
            fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                // 文字列フィールドは `key="value"` の Debug 表記になる（例 rel_path="clear.png"）。
                let _ = write!(self.0, " {}={:?}", f.name(), v);
            }
        }
        ev.record(&mut V(&mut line));
        // Mutex 汚染時もテストは失敗させたいので unwrap（本番経路ではない）。
        self.0.lock().unwrap().push(line);
    }
}

/// クロージャ `f` を実行し、その最中に**現在のスレッド**で発火した `tracing` イベントを
/// 1 行 1 イベントの文字列へ整形して返す（改行連結）。
///
/// スレッドローカルな `with_default` で subscriber を差し込むため、並行テストでも安全
/// （各テストは自身のコードのみを包む）。返り値は捕捉テキストで、呼び手は `contains` で
/// level（`level=WARN`）・target（`target=areka_emo_atlas`）・discriminating field
/// （`rel_path=…` 等）を検証する。
pub(crate) fn capture_logs<F: FnOnce()>(f: F) -> String {
    ensure_interest_probes();

    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    // `with_default` は内部で `Dispatch::new`（＝register_dispatch＋全 callsite 再計算）を
    // 行うため、この時点で既存の `never` は解毒されている。
    tracing::subscriber::with_default(subscriber, || {
        // probe 常駐前（プロセス起動〜初回捕捉）に焼かれた `never` の掃き残しを、
        // 窓が開いた**後**の時点でもう一度確定的に潰す。
        tracing::callsite::rebuild_interest_cache();
        f()
    });
    let guard = logs.lock().unwrap();
    guard.join("\n")
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

#[cfg(test)]
mod tests {
    use super::capture_logs;

    /// ハーネス自体の健全性: 捕捉クロージャ内で発火した warn/error を level・target・field つきで拾う。
    #[test]
    fn harness_captures_level_target_and_fields() {
        let out = capture_logs(|| {
            tracing::warn!(target: "areka_emo_atlas", set = 0u32, rel_path = "clear.png", "テスト warn");
            tracing::error!(target: "areka_emo_atlas", key = "k", "テスト error");
        });
        assert!(out.contains("level=WARN"), "WARN level を捕捉: {out}");
        assert!(out.contains("level=ERROR"), "ERROR level を捕捉: {out}");
        assert!(
            out.contains("target=areka_emo_atlas"),
            "target を捕捉: {out}"
        );
        assert!(out.contains("set=0"), "数値フィールドを捕捉: {out}");
        assert!(
            out.contains("rel_path=\"clear.png\""),
            "文字列フィールドを捕捉: {out}"
        );
    }

    /// クロージャ外（with_default スコープ外）のログは捕捉されない（スレッドローカル境界の実証）。
    #[test]
    fn harness_does_not_capture_outside_scope() {
        let out = capture_logs(|| {
            tracing::warn!(target: "areka_emo_atlas", "inside");
        });
        // スコープ後のログは別 subscriber（=none）へ流れ、捕捉テキストへ混ざらない。
        tracing::warn!(target: "areka_emo_atlas", "outside");
        assert!(out.contains("inside"), "スコープ内は捕捉: {out}");
        assert!(!out.contains("outside"), "スコープ外は捕捉しない: {out}");
    }

    /// 空クロージャ（何もログしない）は空文字列を返す（非発火の陰性確認＝非空虚性の土台）。
    #[test]
    fn harness_returns_empty_when_no_logs() {
        let out = capture_logs(|| {});
        assert!(out.is_empty(), "ログ皆無なら空文字列: {out:?}");
    }
}
