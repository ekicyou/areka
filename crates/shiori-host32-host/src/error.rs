//! host-32 x64/arm64 ホスト側の構造化エラー型（`thiserror`）。
//!
//! 本ファイルは複数タスクが単一責務ごとに追記する共存ファイルである。
//! - `SpawnError`（task 4.1・本タスク）: helper プロセス spawn の I/O 失敗を包む。
//! - `HandshakeError`（task 4.2・別タスク）: HELLO ハンドシェイクの失敗を包む（本タスクでは未定義）。

/// helper プロセスの spawn 失敗（要件 1.5）。
///
/// `std::process::Command::spawn` が返す I/O エラー（helper exe 不在・
/// 実行権限不足など）を包む。spawn 失敗時は [`crate::HelperHandle`] を
/// 返さないため、呼び出し側から見て「稼働中の helper が存在しない状態」が
/// 保たれる。
#[derive(thiserror::Error, Debug)]
pub enum SpawnError {
    /// helper プロセスの起動そのものが失敗した（`Command::spawn` の I/O 失敗）。
    #[error("failed to spawn helper process: {0}")]
    Spawn(#[from] std::io::Error),
}
