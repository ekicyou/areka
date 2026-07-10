//! # layout — 折返し・行送り・スクロール可視窓の決定（純粋層）
//!
//! `GlyphMetrics` trait（グリフ送り幅・行高さの唯一の注入口）を通じて metrics 依存を
//! 外部化し、折返し位置・行送り・あふれ判定・可視窓決定のアルゴリズム自体は純粋に保つ
//! `LayoutEngine`／`FixedMetrics`／`PositionedLine`／`VisibleWindow` を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//! 実測 metrics（DWriteMetrics）は COM 層（draw）が本 trait を実装して注入する。
