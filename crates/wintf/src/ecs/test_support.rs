//! `#[cfg(test)]` 限定: wintf のテストが共有する **tracing ログ濾過ハーネス**の窓口。
//!
//! 本モジュールは `#[cfg(test)]` でのみコンパイルされ、本番バイナリには一切含まれない。
//! 実体はワークスペースで唯一の定義元である `log-capture-kit` にあり、ここはその
//! [`capture_under_filter`] を wintf の既存呼出（96 箇所）へ届けるだけの薄い再輸出である。
//!
//! 診断手順書が指定する `RUST_LOG` 相当の directive を [`tracing_subscriber::EnvFilter`] で
//! 実際に適用し、その濾過を通過した出力だけを文字列として返す、という契約は変わらない。
//! 「観測点が手順で有効化される水準に置かれている」（要件 1.5）を、定数の目視ではなく
//! **実濾過**で機械化するための土台である。
//!
//! # なぜ「素朴な `with_default` 捕捉」では非決定的に取りこぼすのか
//!
//! 「`tracing::subscriber::with_default` はスレッドローカルゆえ並行実行でも干渉しない」は
//! **誤り**である。差し替わるのはスレッドローカルの既定 dispatcher だけで、「そのログを
//! 評価するか」を決める **callsite の interest キャッシュはプロセス全体で 1 つ**しかなく、
//! その発行点をプロセス内で最初に踏んだスレッドの判定が焼き付く（先着が勝つ）。捕捉窓を
//! 持たないスレッドの既定は `NoSubscriber` で、その `register_callsite` は
//! `Interest::never()` を返すため、先に踏まれると `never` が大域キャッシュへ焼き付き、
//! 以後そのイベントは自分のスレッドへ捕捉先を差していても捨てられる。
//!
//! # 対策の所在
//!
//! 対策——⑴ プロセス寿命の probe dispatcher 常駐 ⑵ 捕捉窓の内側での interest 再計算
//! ⑶ 番兵イベントによる空振り検出——はすべて `log-capture-kit` が持つ。かつては wintf が
//! 同型の実装を自前で抱えていたが、写し間違えた側だけが静かに嘘をつくため、定義箇所を
//! 1 箇所へ寄せた。`log-capture-kit` はワークスペース内のどの crate にも依存しない leaf の
//! テスト専用 crate（`[dev-dependencies]` 限定）なので、wintf が上位の本番 crate へ依存を
//! 持つことなく同じ機構を共有できる。
//!
//! 機序の逐条解説と根拠となる `tracing-core` の実コード引用は `log_capture_kit` の crate doc
//! および同 crate の `src/probe.rs`・`src/filter.rs` にある。

pub(crate) use log_capture_kit::capture_under_filter;
