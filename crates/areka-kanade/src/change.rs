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
    /// 切替を中止して定常へ戻った。
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

#[cfg(test)]
#[path = "change_tests.rs"]
mod tests;
