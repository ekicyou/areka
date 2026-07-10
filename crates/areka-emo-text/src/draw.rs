//! # draw — DirectWrite/D2D 描画実行（COM 層）
//!
//! `DrawExecutor`（可視窓の全域再描画・フォント解決・縦書きレシピの lift）と
//! `DWriteMetrics`（測定専用 probe TextLayout 由来の `GlyphMetrics` 実装）を担う。
//!
//! **層規律**: COM 層——UI スレッド専有。`windows`（DirectWrite/D2D）を触るのは
//! 本モジュールと surface のみ。失敗は log-first（`tracing::error!`＋`Err`）で扱い panic しない。
