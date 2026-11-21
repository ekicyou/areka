//! pilot quarantine crate（先進坑・検疫所）.
//!
//! 先進坑（pilot）の使い捨て探索コードと一次記録を集める葉ノード（`publish = false`）。
//!
//! # 命綱の構造的担保
//!
//! この lib は意図的に空で、`pub` item を持たない。探索コードは `examples/` にだけ置く。
//! `examples/` は他クレートから依存できないので、先進坑コードへの被依存は構造的に起きない。
//! 残る経路（他クレートの `Cargo.toml` への依存の追加）は人手レビューで捕まえる。
//! 規律の正本は `.kiro/steering/two-tunnel.md`。
