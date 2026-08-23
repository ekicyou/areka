//! ログ捕捉テストの硬化機構を、ワークスペースで **1 箇所**だけ定義する crate。
//!
//! # なぜ 1 箇所なのか
//!
//! `tracing` の callsite interest キャッシュはプロセス全体で共有され、最初にその発行点を
//! 踏んだスレッドの判定が焼き付く。捕捉用の subscriber をスレッド局所に差し込んでも
//! interest は差し込みの外側にあるため、別のスレッドが先に「このログは不要」と焼き付けると
//! 後続の捕捉テストはイベントを 1 件も観測できない。結果として「このログは出ない」という
//! 主張は捕捉 0 件のまま静かに緑になり（偽陰性）、「このログが出る」という主張は捕捉 0 件で
//! 確率的に赤になる（偽陽性）。この機序への対処を crate ごとに書き写すと、写し損ねた側だけが
//! 静かに嘘をつくため、機構の定義箇所を本 crate に一本化する。
//!
//! # 依存方向の規律
//!
//! 本 crate はワークスペース内のどの crate にも依存しない leaf であり、`publish = false`。
//! 消費側は **`[dev-dependencies]` からのみ**引く。`[dependencies]`（および
//! `[build-dependencies]`）に現れた時点で規律違反であり、番人テストが赤にする。
//! この規律により、`wintf` のような下層 crate が上位の本番 crate へ依存を持つことなく
//! 同じ捕捉機構を共有できる。
//!
//! # 引き方
//!
//! 消費 crate の `Cargo.toml` に 1 行加える。
//!
//! ```toml
//! [dev-dependencies]
//! log-capture-kit = { path = "../log-capture-kit" }
//! ```
//!
//! `RUST_LOG` 相当の directive で実濾過した出力が要る場合（現状は `wintf` のみ）は
//! feature `env-filter` を有効にする。
//!
//! ```toml
//! [dev-dependencies]
//! log-capture-kit = { path = "../log-capture-kit", features = ["env-filter"] }
//! ```
//!
//! bin crate の in-crate `#[cfg(test)]` テスト・`areka-*` 各 crate・統合テスト（`tests/`）の
//! いずれからも同じ形で `use` できる。

mod capture;
mod event;
#[cfg(feature = "env-filter")]
mod filter;
mod global;
mod probe;
