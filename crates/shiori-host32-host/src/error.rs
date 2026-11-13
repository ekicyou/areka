//! host-32 x64/arm64 ホスト側の構造化エラー型（`thiserror`）。
//!
//! 本ファイルは複数タスクが単一責務ごとに追記する共存ファイルである。
//! - `SpawnError`（task 4.1）: helper プロセス spawn の I/O 失敗を包む。
//! - `HandshakeError`（task 4.2）: HELLO ハンドシェイクの失敗を包む。
//! - `ShioriError`（task 1.3）: codec / SHIORI 応答由来の失敗（transport とは別クラス）。
//! - `RequestError`（task 1.3）: 出口 API（get/notify）の統合失敗語彙。

use shiori_host32_ipc::IpcError;

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
    /// 返すのは送信パスのゲート（未確定 helper HWND での `send_request` を拒否する
    /// `ParentMessageWindow::send_request`）である。
    #[error("handshake incomplete: helper HWND not yet established")]
    Incomplete,
}

/// codec / SHIORI 応答由来の失敗（transport とは別クラス・要件 5.2・design.md §392-400）。
///
/// [`IpcError`] が担う transport（wire timeout・送出失敗・framing 破損）とは別クラスの、
/// SHIORI/3.0 応答そのものの意味論的失敗を表す。malformed な応答（status 行欠落など）と
/// SHIORI エラーステータス（400 / 500・`ErrorLevel` / `ErrorDescription` 付き）を区別保持する。
#[derive(thiserror::Error, Debug)]
pub enum ShioriError {
    /// SHIORI/3.0 応答が規約に整合しない（status 行欠落等・要件 2.x malformed）。
    #[error("malformed SHIORI/3.0 response")]
    Parse,
    /// SHIORI が非成功ステータスを返した（400 / 500・要件 2.4 / 2.5 / 5.2）。
    ///
    /// `error_level` / `error_description` は応答ヘッダ由来の付随情報で、
    /// 欠落し得るため `Option`。表示はあるものだけを括弧に添える（無ければ出さない）。
    #[error("SHIORI error status {status}{}", status_detail(.error_level, .error_description))]
    Status {
        /// SHIORI/3.0 応答のステータスコード（例: 400 / 500）。
        status: u16,
        /// `ErrorLevel` ヘッダ（存在する場合）。
        error_level: Option<String>,
        /// `ErrorDescription` ヘッダ（存在する場合）。
        error_description: Option<String>,
    },
}

/// `Status` の表示に添える ` (ErrorLevel: x, ErrorDescription: y)`（どちらも無ければ空）。
fn status_detail(level: &Option<String>, description: &Option<String>) -> String {
    let parts: Vec<String> = [("ErrorLevel", level), ("ErrorDescription", description)]
        .into_iter()
        .filter_map(|(name, value)| value.as_ref().map(|v| format!("{name}: {v}")))
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

/// 出口 API（get / notify）の統合失敗（区別保持・単一不透明へ潰さない・要件 5.4・design.md §402-413）。
///
/// wire timeout（要件 5.1）・SHIORI エラー応答（要件 5.2）・helper 死活の一態様である
/// transport 送出失敗（要件 5.3）・未ハンドシェイク（要件 3.3）を、単一の不透明エラーへ
/// 潰さずに区別保持する（要件 5.4）。helper の**処分判断**は本語彙に含めず
/// `host32-lifecycle`（[`crate::ExitKind`]）へ委ねる。本 API は `Ipc`（送出失敗）で
/// 「helper 応答不能」を区別可能に surface するに留める。
///
/// **写像の注意**: [`IpcError::Timeout`] は `Timeout` へ、それ以外の [`IpcError`]
/// （`SendFailed` 等）は `Ipc` へ振り分ける（task 3 が担う）。この振り分けを成立させるため
/// `Ipc` バリアントには意図的に `#[from]` を付けない（付けると素の [`IpcError`] が
/// 自動変換で一律 `Ipc` へ流れ、timeout 経路を殺すため）。`#[from]` を持つのは
/// `Handshake` と `Shiori` のみ。
#[derive(thiserror::Error, Debug)]
pub enum RequestError {
    /// ハンドシェイク未完了・タイムアウト（要件 3.3 / 3.4）。構造上通常は起きない。
    #[error("handshake incomplete or timed out")]
    Handshake(#[from] HandshakeError),
    /// wire タイムアウト（[`IpcError::Timeout`] 由来・要件 5.1）。
    #[error("wire timeout")]
    Timeout,
    /// transport 送出失敗（[`IpcError::SendFailed`] 等・helper 死活の一態様・要件 5.3）。
    ///
    /// 意図的に `#[from]` を持たない（型 doc の写像注意を参照）。
    #[error("ipc transport failure: {0}")]
    Ipc(#[source] IpcError),
    /// SHIORI エラー応答（[`ShioriError`] 由来・要件 5.2）。
    #[error("shiori error: {0}")]
    Shiori(#[from] ShioriError),
}

#[cfg(test)]
mod request_error_tests {
    use super::*;

    // ShioriError の Display 文字列（要件 5.2）
    #[test]
    fn shiori_error_parse_display() {
        assert_eq!(
            format!("{}", ShioriError::Parse),
            "malformed SHIORI/3.0 response"
        );
    }

    #[test]
    fn shiori_error_status_display() {
        let err = ShioriError::Status {
            status: 500,
            error_level: Some("critical".to_string()),
            error_description: Some("boom".to_string()),
        };
        assert_eq!(
            format!("{err}"),
            "SHIORI error status 500 (ErrorLevel: critical, ErrorDescription: boom)"
        );
    }

    #[test]
    fn shiori_error_status_display_omits_absent_headers() {
        let bare = ShioriError::Status {
            status: 400,
            error_level: None,
            error_description: None,
        };
        assert_eq!(format!("{bare}"), "SHIORI error status 400");
        let level_only = ShioriError::Status {
            status: 500,
            error_level: Some("warning".to_string()),
            error_description: None,
        };
        assert_eq!(
            format!("{level_only}"),
            "SHIORI error status 500 (ErrorLevel: warning)"
        );
    }

    // RequestError 各 variant の Display 文字列（要件 5.1 / 5.2 / 5.3 / 5.4）
    #[test]
    fn request_error_handshake_display() {
        let err = RequestError::Handshake(HandshakeError::Incomplete);
        assert_eq!(format!("{err}"), "handshake incomplete or timed out");
    }

    #[test]
    fn request_error_timeout_display() {
        assert_eq!(format!("{}", RequestError::Timeout), "wire timeout");
    }

    #[test]
    fn request_error_ipc_display() {
        let err = RequestError::Ipc(IpcError::SendFailed);
        assert_eq!(format!("{err}"), "ipc transport failure: ipc send failed");
    }

    #[test]
    fn request_error_ipc_keeps_the_inner_error_as_source() {
        use std::error::Error as _;
        let err = RequestError::Ipc(IpcError::SendFailed);
        let source = err.source().expect("中の IpcError が原因として返る");
        assert!(matches!(
            source.downcast_ref::<IpcError>(),
            Some(IpcError::SendFailed)
        ));
    }

    #[test]
    fn request_error_shiori_display() {
        let err = RequestError::Shiori(ShioriError::Parse);
        assert_eq!(
            format!("{err}"),
            "shiori error: malformed SHIORI/3.0 response"
        );
    }

    // #[from] 変換（HandshakeError / ShioriError のみ・要件 5.4 区別保持）
    #[test]
    fn handshake_error_lifts_into_request_error() {
        let err: RequestError = HandshakeError::Incomplete.into();
        assert!(matches!(err, RequestError::Handshake(_)));
    }

    #[test]
    fn shiori_error_lifts_into_request_error() {
        let err: RequestError = ShioriError::Parse.into();
        assert!(matches!(err, RequestError::Shiori(_)));
    }
}
