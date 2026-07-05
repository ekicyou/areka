//! helper ライフサイクル管理（要件 R5〜R7）。
//!
//! 本モジュールは helper プロセスの正常終了要求（unload → ループ正常終了 →
//! `exit(0)`）と、その結果の観測・分類を担う。将来的に以下を提供する予定
//! （いずれも後続タスクで実装）:
//! - `HelperLifecycle` — ライフサイクルの状態機械。
//! - `classify_failure` — 終了コード/シグナルから `FailureClass` への分類。
//! - `request_clean_shutdown` — 正常終了（Clean shutdown）要求の発行。
//!
//! 本タスク（1.2）では統一報告語彙・shutdown 失敗語彙の型骨格と定数を定義する:
//! - [`HelperStatus`] — 現在の生存/終了ステータス。
//! - [`FailureClass`] — 死活・request 失敗の突合分類。
//! - [`LifecycleReport`] — 突合結果と原因を束ねる統一報告。
//! - [`ShutdownError`] — shutdown 経路専用の失敗語彙。
//! - [`UNLOAD_ACK_TIMEOUT`] / [`EXIT_OBSERVE_TIMEOUT`] / [`EXIT_POLL_INTERVAL`] — 関連定数。

use std::time::Duration;

use crate::error::RequestError;
use crate::parent_window::SendError;
use crate::process_host::ExitKind;

/// 死活監視の観測結果（Send・Copy な所有データ・R2.6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperStatus {
    /// 稼働中（終了未検出・R1.1）。
    Running,
    /// 終了検出済み（種別付き・R1.2〜1.5）。sticky（以後不変）。
    Exited(ExitKind),
}

/// 統一失敗分類（R2.1〜2.3・Send・Copy）。処分判断は含まない（R2.7）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// helper 死亡起因（失敗時点で終了が観測された・R2.1）。保持値は観測された終了種別。
    HelperDown(ExitKind),
    /// helper 生存（終了未検出）だが上限時間内に応答なし（wire timeout・R2.2）。
    Unresponsive,
    /// SHIORI エラー応答（400/500/ErrorLevel・helper は生存・R2.3）。
    ShioriFailure,
    /// helper 生存だが transport 送出失敗（Ipc・死亡未検出の境界異常）。
    Transport,
    /// 未ハンドシェイク（構造上通常起きない・区別のため潰さない・R2.4）。
    Handshake,
}

/// 統一報告（本仕様が正本・kanade は再定義しない・R2.4/2.6）。
/// class（突合結果）と error（原因の RequestError・凍結型を所有内包）の二軸を保持し、
/// 単一の不透明失敗へ潰さない。
#[derive(Debug)]
pub struct LifecycleReport {
    pub class: FailureClass,
    pub error: RequestError,
}

/// UNLOAD ack（unload 完了確認）待機の上限（LOAD_ACK_TIMEOUT と同値 30s）。
pub const UNLOAD_ACK_TIMEOUT: Duration = Duration::from_secs(30);
/// ack 受領後、プロセス終了（ExitKind 観測）までの bounded poll 上限。
pub const EXIT_OBSERVE_TIMEOUT: Duration = Duration::from_secs(10);
/// bounded poll の刻み（実時間 sleep はこれのみ・R7.5）。
pub const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// shutdown 経路の失敗語彙（R5.5・error! と対で surface）。
#[derive(thiserror::Error, Debug)]
pub enum ShutdownError {
    /// UNLOAD の送出／ack 往復が失敗（SendError を所有内包・凍結型不改変）。
    #[error("unload round-trip failed: {0}")]
    Unload(#[from] SendError),
    /// ack が厳密 1 byte [1] でない（契約違反の観測）。
    #[error("unexpected unload ack: {0:?}")]
    UnexpectedAck(Vec<u8>),
    /// ack 受領後、上限時間内にプロセス終了が観測できない。
    #[error("helper did not exit within bound after unload ack")]
    ExitTimeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `T: Send` を静的に固定する（R2.6/7.7・報告データは Send な所有データ）。
    /// 本関数が対象型で単相化できること自体が Send の証明であり、
    /// コンパイルが通れば当該型は Send（通らなければ型が Send でない）。
    const fn assert_send<T: Send>() {}

    #[test]
    fn lifecycle_report_types_are_send() {
        assert_send::<HelperStatus>();
        assert_send::<FailureClass>();
        assert_send::<LifecycleReport>();
        assert_send::<ShutdownError>();
    }
}
