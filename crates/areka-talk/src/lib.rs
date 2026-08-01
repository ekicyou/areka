//! talk 授受契約の物理正本クレート（[`TalkId`] / [`StartTalk`] / [`TalkDone`] / [`TalkEndReason`]
//! ／[`TalkCommand`] / [`ChoiceWaiting`]）。
//!
//! kanade（③運行）と sakura（④再生）の間で交わす talk 起動〜完了契約の唯一の物理定義。
//! 依存ゼロ・`std` のみに依存する（`areka-actor` 型・host32 型・エンジン知識を持ち込まない）。
//!
//! # 対応する Requirements
//! - 1.1: talk 授受契約を kanade 正本の単一定義へ一本化する。
//! - 1.2: 中断理由（[`TalkEndReason`]）を通常終了・quit・中断の 3 値で提供する。
//! - 1.7: 変換アダプタによる二重定義を作らず、物理的に単一の型定義として実現する。
//! - 5.1 / 5.6: 選択待ちバリアの解決指示を[`TalkCommand`]（再生層の正規入力経路）で運ぶ。
//! - 7.1: 選択待ち成立の事実と表示完了時刻を[`ChoiceWaiting`]で再生層から返す。

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

/// kanade → talk 再生系への指示（`Sender<StartTalk>` の後継・**順序保存が契約**）。
///
/// # 順序保存（Ordering / delivery）
/// 3 形は**単一型**であるため単一チャンネルを流れ、送出側が投函した順序が
/// relay ＋ dispatcher の単一 inbox を経て FIFO で保存される。
/// この順序保存は本型の**契約**であり、下流はこれに依存してよい。
/// 具体的には、カスケード完了後に同一バッチで
/// `[TalkCommand::ResolveChoice, TalkCommand::Start]` の順で投函されたとき、
/// 再生層は必ず「選択待ちの解決 → 新 talk の起動」の順に観測する。
/// 解決系と起動系を別チャンネルへ分離してはならない（順序保存が壊れる）。
#[derive(Debug, Clone)]
pub enum TalkCommand {
    /// talk 起動（従来の [`StartTalk`] 素送りの包み・情報無損失）。
    Start(StartTalk),
    /// 選択待ちバリアの解決指示。
    ///
    /// `talk_id` は stale ガード用（現行 slot と不一致なら棄却される）、
    /// `id` は解決に用いる選択肢 ID。
    ResolveChoice { talk_id: TalkId, id: String },
    /// 選択待ちの解除＋トーク終了指示（下流では Close funnel へ写像される）。
    ///
    /// `talk_id` は stale ガード用。
    CancelChoice { talk_id: TalkId },
}

/// talk → kanade への選択待ち成立通知（[`TalkDone`] と同じ done ポートを流れる）。
///
/// 同一ポートを流れるため、同一 talk についての選択待ち成立と再生完了は
/// 因果順が保存される。
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceWaiting {
    /// 選択待ちに入った talk（stale ガード用）。
    pub talk_id: TalkId,
    /// 候補選択肢 ID 列（照合用・表示順を保存する）。
    pub choice_ids: Vec<String>,
    /// トーク表示完了時刻（talk 経過秒・占有 horizon ＝ duration 権威）。
    ///
    /// タイムアウト計測の起点はこの値であり、再生層以外の時間基準を導入しない。
    pub display_end_elapsed_secs: f64,
    /// バリアのタイムアウト指令（秒）。語彙は 3 値:
    ///
    /// - `None` — 未指定。下流の既定値へ委譲する。
    /// - `Some(v)`（`v <= 0.0`）— 無効化。計測せず選択待ちを無期限に継続する。
    /// - `Some(v)`（`v > 0.0`）— 明示秒指定。
    ///
    /// この層では写像も既定値の適用も行わない（指令をそのまま搬送する）。
    pub timeout_directive_secs: Option<f64>,
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
