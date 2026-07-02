//! host-32 x64/arm64 ホスト側の構造化エラー型（`thiserror`）。
//!
//! 本ファイルは複数タスクが単一責務ごとに追記する共存ファイルである。
//! - `SpawnError`（task 4.1）: helper プロセス spawn の I/O 失敗を包む。
//! - `HandshakeError`（task 4.2）: HELLO ハンドシェイクの失敗を包む。

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

/// HELLO ハンドシェイクの失敗（要件 3.3 / 3.4・design.md §486-506）。
///
/// helper が起動直後に送る HELLO（自窓 HWND の u32 LE）を親が所定の待機時間内に
/// 受領できなかった場合、または未ハンドシェイク状態で往復送信が試みられた場合を
/// 表す。distinct な PeerGone は設けず、peer の生死は別系統
/// （[`crate::ExitKind`]）で観測する（Requirement 1 と 3/5 の分離）。
#[derive(thiserror::Error, Debug)]
pub enum HandshakeError {
    /// 所定の待機時間内に HELLO が受領できず、helper HWND が確定しなかった
    /// （要件 3.4）。`pump_until_hello_or` の `None` に対応する失敗表現。
    #[error("handshake timed out before HELLO was received")]
    Timeout,
    /// ハンドシェイク未完了（helper HWND 未確定）のまま往復送信が試みられた
    /// （要件 3.3・ハンドシェイクゲート）。
    ///
    /// 本タスク（4.2）では定義のみで、実際に返すのは送信パスを実装する
    /// task 4.3 のゲート（未確定 helper HWND での `send_request` を拒否）である。
    #[error("handshake incomplete: helper HWND not yet established")]
    Incomplete,
}
