//! 切替の語彙（areka-P0-ghost-shell-balloon-switch）。
//!
//! ゴーストの切替の要求・停止通知に載せる切替の中身・運行の通知・起動の由来の型だけを置く。
//! 判断は持たない。どの型も `Debug, Clone, PartialEq, Eq` を持ち、決定論テストで丸ごと
//! 突き合わせられる。

use crate::msg::KanadeStopped;

/// 切替の要求（UI → kanade）。名前の突き合わせは UI 側で済んでいる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRequest {
    pub target: ChangeTarget,
    pub origin: ChangeOrigin,
    /// 真なら送り出す側へ `OnGhostChanging` を送る。
    pub raise_event: bool,
}

/// 切替先（`OnGhostChanging` の Ref0〜3 の材料）。`sakura_name` は無ければ空。`dir` は絶対パスの文字列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeTarget {
    pub sakura_name: String,
    pub name: String,
    pub dir: String,
}

/// 切替の出どころ（`OnGhostChanging` の Ref1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeOrigin {
    /// 利用者がメニューから選んだ。
    Manual,
    /// 台本（`\![change,ghost,..]`）から求められた。
    Automatic,
}

impl ChangeOrigin {
    /// Reference に載せる綴り（`manual`／`automatic`）。
    pub fn as_ref_str(self) -> &'static str {
        match self {
            ChangeOrigin::Manual => "manual",
            ChangeOrigin::Automatic => "automatic",
        }
    }
}

/// 停止通知に載せる切替の中身（kanade が切替の相を経て止まったことの証）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeHandoff {
    /// `OnGhostChanging` が返した台本（送らなかった・204・失敗は `None`）。
    pub script: Option<String>,
}

/// 切替の中止の理由（記録の語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    /// 利用者が中断した。
    UserBreak,
    /// 終了要求が来て切替を取りやめた。
    CloseRequest,
    /// kanade が受理しなかった（終了要求と競合）。
    Rejected,
}

/// 運行の通知（kanade → UI の 1 本の線）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KanadeNotice {
    /// 定常に入った。
    Steady,
    /// 切替を中止した（定常へ戻る。ただし終了要求・`\-` の予約が勝った場合は、このあと今日どおり停止へ進む）。
    ChangeCancelled { reason: CancelReason },
    /// kanade が止まった。
    Stopped(KanadeStopped),
}

/// 起動の由来（`KanadeConfig::boot_origin`）。起動の根を選ぶ表の入力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootOrigin {
    /// ふつうの起動。
    Plain,
    /// 切替で来た。
    ChangedFrom(ChangedFrom),
    /// 前回のゴーストが落ちて既定へ戻った（`OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴースト名）。
    Halted { ghost_name: String },
    /// ネットワーク更新で読み直した。`id`（許可表の綴り）と Reference を起動の知らせとして送る
    /// （`OnGhostChanged`・`OnBoot` の代わり・204 でも `OnBoot` へ続けない・areka-P0-network-update 要件 5.5）。
    Updated {
        id: &'static str,
        references: Vec<String>,
    },
}

/// 直前のゴーストの情報（`OnGhostChanged` の Ref0〜3）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFrom {
    pub sakura_name: String,
    pub script: String,
    pub name: String,
    pub dir: String,
}

/// 汎用の入口の GET／NOTIFY の別（`ShioriCall` の `Get`／`Notify` に写す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShioriMethod {
    Get,
    Notify,
}

/// 汎用の通知の入口で送ったイベントの結果（kanade → 送り手・`KanadeMsg::RaiseEvent` の返信端）。
///
/// 写すのは最初の往復の結果だけで、運行表の判断（許可表・定常・置き換え）は変えない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaiseOutcome {
    /// 許可表に無い名前（送っていない）。
    NotAllowed,
    /// 定常でない（送っていない・積んでもいない）。
    NotSteady,
    /// 空でない台本が返った。
    Script,
    /// 返事なし（204・空か空白だけの台本・エラー応答・NOTIFY の完了）。
    NoReply,
    /// 往復が失敗した（kanade は今日どおり終了系列へ進む）。
    Failed,
}

/// 台詞の切れ目の口に添える印のイベント（本仕様では `OnShellChanging` だけ）。
///
/// 口（`KanadeMsg::AwaitTalkGap`）は印の有無で振る舞いを分ける。印が無ければ切れ目を待つだけで、
/// 印があれば先にこのイベントを送り、その応答の台詞を追う（areka-P0-shell-balloon-switch 要件 8.11）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapRaise {
    /// イベント名（許可表 `ALLOWED_EVENT_IDS` の要素）。
    pub id: String,
    /// Ref0〜Ref n（欠番は空文字列・詰めない）。
    pub references: Vec<String>,
    /// GET か NOTIFY か。
    pub method: ShioriMethod,
}

/// 台詞の切れ目の口の結果（kanade → 依頼した側・ちょうど 1 回）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalkGap {
    /// 定常・再生中のトーク無し・終了とゴースト切替の保留無しに達した。
    Reached { marked: Option<MarkedEnd> },
    /// 印の台詞を利用者が中断した（終了系列へは進んでいない）。
    CancelledByUser,
    /// 切れ目に達しないと決まった。
    Left { reason: GapLeft },
    /// 印のイベントを送らなかった（許可表に無い）。
    NotSent { outcome: RaiseOutcome },
}

/// 印の台詞の終わり方（要件 8.11 ⑴）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkedEnd {
    /// 印の台詞が最後まで流れた（選択肢の時間切れの解除を含む）。
    Completed,
    /// 別のトークに置き換わった。
    Replaced,
    /// 印のイベントが台詞を返さなかった。
    NoTalk,
}

/// 切れ目に達しないと決まった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapLeft {
    /// 依頼が届いたとき定常でなかった（起動系列・終了系列・終了の保留あり）。
    NotSteady,
    /// 待っている間に終了系列へ入った・終了の保留が立った。
    Closing,
    /// 待っている間にゴースト切替の相へ入った・ゴースト切替の保留が立った。
    GhostChange,
}

#[cfg(test)]
#[path = "change_tests.rs"]
mod tests;
