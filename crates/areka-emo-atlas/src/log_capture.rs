//! テスト専用の `tracing` ログ捕捉ハーネス（本番影響ゼロ・`#[cfg(test)]` 限定）。
//!
//! 決定論的テスト網羅の要請（開発者マンデート）に従い、bake 経路が発火する 0 寸/全透明
//! element の `warn!`（ゴースト制作者ミスの可能性が高いための早期警告・設計ディスカッション #1）
//! を、観測可能な副作用ではなく **ログが実際に発火したこと**まで実行テストで檻に入れるための土台。
//!
//! 捕捉層そのものは本 module に持たず、硬化機構の唯一の定義元 `log-capture-kit` へ委譲する。
//! ここに残るのは戻り値の形を合わせる 2 行だけで、呼出側の判定内容も行の形（1 イベント 1 行・
//! `level=…` から始めてフィールドを訪問順に ` name=value` で連ね、改行連結した 1 本の `String`）も
//! 移行前と 1 バイトも変わらない。
//!
//! # なぜ共有機構が要るのか（「スレッドローカルゆえ安全」は誤り）
//!
//! 「`with_default` はスレッドローカルだから並行実行でも干渉しない」は**誤り**である。差し替わる
//! のはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
//! interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点をプロセス内で最初に踏んだ
//! スレッドの判定が焼き付く。捕捉窓を持たないスレッド（既定は `NoSubscriber`）が先に踏むと
//! `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす（本 crate では
//! `cargo test --workspace` の並行負荷下でのみ捕捉が**空文字列**になり
//! `warn_fires_on_all_transparent_element` が約 1/3 で落ちる間欠失敗として 2026-08-14 に実測された）。
//!
//! 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 捕捉窓の内側での interest 再計算 ⑶ 番兵イベントに
//! よる空振り検出、の 3 点でこれを塞ぐ。機序の逐条解説（`tracing-core` の実コード引用つき）は
//! `log_capture_kit` の crate doc および同 crate の `src/probe.rs` にある。

use log_capture_kit::{LineFormat, capture_lines};

/// クロージャ `f` を実行し、その最中に**現在のスレッド**で発火した `tracing` イベントを
/// 1 行 1 イベントの文字列へ整形して返す（改行連結）。
///
/// 捕捉と硬化は `log-capture-kit` の [`capture_lines`] が行う。返り値は捕捉テキストで、呼び手は
/// `contains` で level（`level=WARN`）・target（`target=areka_emo_atlas`）・discriminating field
/// （`rel_path=…`／`set=…`／`original_w=…` 等）を検証する。
pub(crate) fn capture_logs<F: FnOnce()>(f: F) -> String {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines.join("\n")
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
