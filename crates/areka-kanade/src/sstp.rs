//! SSTP の入口の外向けの型と検査（結果 2 種・`NotifyArgError`・`validate_notify`）。
//!
//! MCP の側の答えと送出の檻の両方が使う正本。`std` だけに依存する。

/// 外からの台本（`KanadeMsg::SstpSend`）の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SstpSendOutcome {
    /// 新しいトークとして再生を始めた（翻訳を済ませ、再生の開始を送った）。
    Accepted,
    /// 起動・終了・切替の手順の途中で受け付けなかった（何もしていない）。
    NotAccepted,
    /// この依頼の処理の中で kanade が終了系列へ進み止まった（翻訳の輸送路の失敗など）。
    Failed,
}

/// 外から頼まれたイベント（`KanadeMsg::SstpNotify`）の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SstpNotifyOutcome {
    /// SHIORI が空でない台本を返し、再生を始めた。中身は SHIORI が返したまま（翻訳の前）。
    Script(String),
    /// 返り値なし・空か空白だけ・エラー応答（何も再生していない・今の再生に触れていない）。
    NoScript,
    /// 起動・終了・切替の手順の途中で送らなかった。
    NotAccepted,
    /// 往復が失敗した、またはこの依頼の処理の中で kanade が止まった。
    Failed,
}

/// `validate_notify` が断る理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyArgError {
    /// イベントの名前が空か空白だけ。
    EmptyEvent,
    /// イベントの名前か Reference のどれかに CR・LF・NUL が在る。
    ControlCharacter,
}

/// 名前が空か空白だけ（`str::trim` が空）なら `EmptyEvent`、名前か Reference に `'\r'`・`'\n'`・`'\0'`
/// が在れば `ControlCharacter`。それ以外（バイト値 1 などほかの制御文字を含む）は `Ok`。判定は EmptyEvent が先。
pub fn validate_notify(event: &str, references: &[String]) -> Result<(), NotifyArgError> {
    if event.trim().is_empty() {
        return Err(NotifyArgError::EmptyEvent);
    }
    let breaks = |s: &str| s.contains(['\r', '\n', '\0']);
    if breaks(event) || references.iter().any(|r| breaks(r)) {
        return Err(NotifyArgError::ControlCharacter);
    }
    Ok(())
}

#[cfg(test)]
#[path = "sstp_tests.rs"]
mod tests;
