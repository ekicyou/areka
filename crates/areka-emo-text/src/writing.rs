//! # writing — writing_mode 宣言の解釈（純粋層）
//!
//! balloon descript の `writing_mode` 転記値（2層マージ後勝ち解決済み・生文字列）を
//! `WritingMode`（`HorizontalTb`／`VerticalRl`／`VerticalLr`）へ解決し、
//! 方向写像と M2 予約キー名（`text_orientation`／`text_combine_upright`）の記録を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
