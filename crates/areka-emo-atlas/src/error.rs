//! 診断可能なエラー型（`BakeError` / `DecodeError`）と継続方針。
//!
//! 要件 **R2**。
//!
//! bake パイプラインのエラー型を定義する。デコード失敗（`DecodeError`）は記録して
//! 継続し（1 element の失敗が全体を止めない）、パイプライン全体の失敗は `BakeError`。
//! 失敗は error! ログ ＋ Err 戻り値で扱い、安易な panic を避ける
//! （記憶 areka-log-first-no-silent-failure）。
//!
//! （本タスクは雛形。型定義は後続タスクで追加する。）
