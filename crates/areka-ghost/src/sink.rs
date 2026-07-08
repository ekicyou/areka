//! `LogSink` — 本番既定の記録先コンポーネント。
//!
//! 発火内容を `tracing` へ構造化出力するだけの、複製可能（`Clone`）な既定実装。
//! M-boot 統合はこの位置に seriko／emo-text-layer の実 sink を挿す
//! （design.md「ghost::sink」）。task 2.4 で実装する。
