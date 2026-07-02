//! areka-parsers — areka 用の純粋・std のみ・host 非依存なパーサーファミリ。
//!
//! 本クレートは外部状態や host 環境に依存しない純粋関数群を提供し、
//! 下流エンジンは型付き命令モデルのみを import して利用する。
//! 兄弟モジュール（shell / balloon / package 等）は各 spec が追加する。
//!
//! 例外は `package` module のみ: ローカルディレクトリツリー走査（`std::fs`
//! 読取）を行い、`Result` を返す。それ以外の module（`charset` / `kv` /
//! `sakura` 等）は従来どおり純粋関数群であり I/O を持たない。`package` の
//! I/O 許容は「マウントは物理不在という現実の失敗を持つ」ための意図的逸脱で、
//! `resolve` サブモジュール内に局所化されている（詳細は `package` module doc）。

pub mod balloon;
pub mod charset;
pub mod kv;
pub mod package;
pub mod sakura;
pub mod shell;
