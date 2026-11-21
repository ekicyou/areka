//! テスト専用の `tracing` ログ捕捉ハーネス。
//!
//! bake が出す 0 寸/全透明 element の `warn!` が **実際に発火したこと**まで、テストで確かめるための土台。
//!
//! 捕捉層は `log-capture-kit` に委譲し、ここは戻り値の形を合わせるだけ。出力は 1 イベント 1 行
//! （`level=…` から始めてフィールドを訪問順に ` name=value` で連ね、改行連結した 1 本の `String`）。
//!
//! # なぜ共有機構が要るのか
//!
//! 「`with_default` はスレッドローカルだから並行実行でも干渉しない」は誤り。差し替わるのは
//! スレッドローカルの既定 dispatcher だけで、callsite の interest キャッシュは**プロセス全体で 1 つ**しかなく、
//! その発行点を最初に踏んだスレッドの判定が焼き付く。捕捉窓を持たないスレッド（既定は `NoSubscriber`）が
//! 先に踏むと `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす
//! （`cargo test --workspace` の並行負荷下でのみ捕捉が空文字列になる間欠失敗を 2026-08-14 に実測）。
//!
//! 共有機構は プロセス寿命の probe 常駐・捕捉窓の内側での interest 再計算・番兵イベントによる
//! 空振り検出でこれを塞ぐ。詳細は `log_capture_kit` の crate doc と `src/probe.rs`。

use log_capture_kit::{LineFormat, capture_lines};

/// クロージャ `f` を実行し、その最中に**現在のスレッド**で発火した `tracing` イベントを
/// 1 行 1 イベントの文字列へ整形し（改行連結）、`f` の戻り値と組にして返す。
///
/// 捕捉と硬化は `log-capture-kit` の [`capture_lines`] が行う。組の 2 つ目が捕捉テキストで、呼び手は
/// `contains` で level（`level=WARN`）・target（`target=areka_emo_atlas`）・discriminating field
/// （`rel_path=…`／`set=…`／`original_w=…` 等）を検証する。
pub(crate) fn capture_logs<R>(f: impl FnOnce() -> R) -> (R, String) {
    let (out, lines) = capture_lines(LineFormat::LevelTargetFields, f);
    (out, lines.join("\n"))
}
