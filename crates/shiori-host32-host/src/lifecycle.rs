//! helper ライフサイクル管理（要件 R5〜R7）。
//!
//! 本モジュールは helper プロセスの正常終了要求（unload → ループ正常終了 →
//! `exit(0)`）と、その結果の観測・分類を担う。将来的に以下を提供する予定
//! （いずれも後続タスクで実装）:
//! - `HelperLifecycle` — ライフサイクルの状態機械。
//! - `HelperStatus` — 現在の生存/終了ステータス。
//! - `FailureClass` — 異常終了の分類。
//! - `LifecycleReport` — 終了経路の観測レポート。
//! - `classify_failure` — 終了コード/シグナルから `FailureClass` への分類。
//! - `request_clean_shutdown` — 正常終了（Clean shutdown）要求の発行。
//! - `ShutdownError` — 正常終了要求の失敗を表す構造化エラー。
//! - 関連する定数群。
//!
//! 現時点では skeleton（空モジュール）であり、上記の型・関数は未実装。
