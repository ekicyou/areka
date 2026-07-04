//! デコードポート定義（`ElementDecoder` trait ＋ `DecodedImage` / `DecodeError`）。
//!
//! 設計決定 **D4**（要件 **R2**）。
//!
//! デコード手段を差し替え可能にするための trait ポート。既定腕＝WIC（COM 必要・
//! [`wic_arm`] に隔離）／テスト腕＝メモリ PBGRA。正規化以降のコアは COM 非依存で、
//! 既定手段を上位へ露出しない（R2.3）。デコード出力は premultiplied BGRA を
//! 想定し、変換前フレームのピクセルフォーマット由来の α 有無を保持する。
//!
//! （本タスクは雛形。trait 署名・型は後続タスクで定義する。）

pub mod wic_arm;
