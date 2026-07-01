//! areka-parsers — areka 用の純粋・std のみ・host 非依存なパーサーファミリ。
//!
//! 本クレートは外部状態や host 環境に依存しない純粋関数群を提供し、
//! 下流エンジンは型付き命令モデルのみを import して利用する。
//! 兄弟モジュール（shell / balloon / package 等）は各 spec が追加する。

pub mod sakura;
