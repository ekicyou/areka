//! talk 授受契約の物理正本クレート（[`TalkId`] / [`StartTalk`] / [`TalkDone`] / [`TalkEndReason`]）。
//!
//! kanade（③運行）と sakura（④再生）の間で交わす talk 起動〜完了契約の唯一の物理定義。
//! 依存ゼロ・`std` のみに依存する（`areka-actor` 型・host32 型・エンジン知識を持ち込まない）。
//!
//! # 対応する Requirements
//! - 1.1: talk 授受契約を kanade 正本の単一定義へ一本化する。
//! - 1.2: 中断理由（[`TalkEndReason`]）を通常終了・quit・中断の 3 値で提供する。
//! - 1.7: 変換アダプタによる二重定義を作らず、物理的に単一の型定義として実現する。

/// talk の一意識別子（kanade が単調増番で採番・再利用しない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TalkId(pub u64);

/// 台本末尾へ付加する汎用コマンド（`\!` キャリアの typed 前段。搬送のみ・解釈しない）。
///
/// name で消費側が選別し、tokens は不透明な文字列列として運ぶ（この層では解釈しない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpilogueCommand {
    pub name: String,
    pub tokens: Vec<String>,
}

/// talk 起動要求（kanade → dispatcher → sakura）。reply を同梱しない。
///
/// `epilogue` は compile 後の CueSheet 末尾（最終 offset・duration 0）へ carrier cue として
/// 付加される汎用コマンド列。既定は空（従来挙動）。空でない場合のみ末尾へ付く。
/// スクリプト文字列末尾への追記ではないため、`\e` による End 切詰めでも脱落しない。
#[derive(Debug, Clone)]
pub struct StartTalk {
    pub talk_id: TalkId,
    pub script: String,
    pub epilogue: Vec<EpilogueCommand>,
}

impl StartTalk {
    /// epilogue なしの従来形コンストラクタ（構築点の機械的追随を最小化）。
    pub fn new(talk_id: TalkId, script: impl Into<String>) -> Self {
        Self {
            talk_id,
            script: script.into(),
            epilogue: Vec::new(),
        }
    }
}

/// 終端理由 3 値（旧 kanade quit:bool を置換）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalkEndReason {
    /// `\e`／末尾到達／空列。
    Ended,
    /// `\-`（終了要求）。
    Quit,
    /// Close による中断（中断も ACK として通知される）。
    Interrupted,
}

/// 再生完了通知（sakura → dispatcher → `KanadeMsg::TalkDone`）。通算高々 1 回。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TalkDone {
    pub talk_id: TalkId,
    pub reason: TalkEndReason,
}

#[cfg(test)]
mod tests;
