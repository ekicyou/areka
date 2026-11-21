//! `#[cfg(test)]` 限定: wintf のテストが共有する **tracing ログ濾過ハーネス**の窓口。
//!
//! 実体はワークスペースで唯一の定義元 `log-capture-kit` にあり、ここは
//! [`capture_under_filter`] を wintf の既存呼出へ届ける薄い再輸出である。
//! 診断手順書が指定する `RUST_LOG` 相当の directive を [`tracing_subscriber::EnvFilter`] で
//! 実際に適用し、その濾過を通過した出力だけを文字列として返す。
//!
//! # なぜ「素朴な `with_default` 捕捉」では非決定的に取りこぼすのか
//!
//! `tracing::subscriber::with_default` が差し替えるのはスレッドローカルの既定 dispatcher だけで、
//! 「そのログを評価するか」を決める **callsite の interest キャッシュはプロセス全体で 1 つ**しかない。
//! 発行点を最初に踏んだスレッドの判定が焼き付き（先着が勝つ）、捕捉窓を持たないスレッドの既定は
//! `NoSubscriber` で `Interest::never()` を返すため、先に踏まれると以後そのイベントは捕捉先を
//! 差していても捨てられる。
//!
//! 対策（probe dispatcher の常駐・捕捉窓の内側での interest 再計算・番兵イベントによる空振り検出）
//! は `log-capture-kit` が持つ。機序の詳細は `log_capture_kit` の crate doc にある。

pub(crate) use log_capture_kit::capture_under_filter;
