//! pilot quarantine crate（先進坑・検疫所）.
//!
//! このクレートは先進坑（pilot）の使い捨て探索コードと一次記録を集約する
//! 葉ノード（leaf node, `publish = false`）である。
//!
//! # 命綱の構造的担保（structural guarantee）
//!
//! この lib は**意図的に空**であり、公開 API（`pub` item）を一切持たない。
//! 探索コードはすべて `examples/` 配下にのみ置く。Cargo の `examples/` は
//! 他クレートから `[dependencies]` で参照できず、空 lib は依存しても意味のある
//! API を露出しないため、先進坑コードへの被依存（inbound edge）は
//! **構造的に発生し得ない**。これが命綱（葉ノード隔離）の構造的担保そのものである。
//!
//! 唯一の inbound 経路（他クレートの `Cargo.toml` への一行依存追加）は
//! 人手レビューで捕捉する。規律の正本は `.kiro/steering/two-tunnel.md`。
