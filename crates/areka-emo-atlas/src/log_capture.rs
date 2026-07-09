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
//!   （プロセス全体・並行テストを壊す）は使わない。各テストが自分のコードのみを `capture_logs`
//!   で包む限り、`cargo test` の並行実行でも他スレッドの subscriber と干渉しない。
//! - bake パイプラインは完全同期で、全ログは呼び出しスレッド上で同期的に発火するため、
//!   `capture_logs` はクロージャ復帰時点で全ログを捕捉済みである。

use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
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
    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    tracing::subscriber::with_default(subscriber, f);
    let guard = logs.lock().unwrap();
    guard.join("\n")
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
        assert!(out.contains("target=areka_emo_atlas"), "target を捕捉: {out}");
        assert!(out.contains("set=0"), "数値フィールドを捕捉: {out}");
        assert!(out.contains("rel_path=\"clear.png\""), "文字列フィールドを捕捉: {out}");
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
