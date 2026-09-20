//! 利用者の中断（バルーンの左ダブルクリック）の判定と送り出し（areka-P0-balloon-break）。
//!
//! 中断を禁じる旗・直前の押下の記憶・送出端を持ち、押下のたびに「中断にするか」を決めて、
//! 表示の側へ「隠せ」、kanade へ `KanadeMsg::UserBreak` を 1 件ずつ送る層がここへ入る。
//!
//! 本 mod は現状、判断だけを担う純関数 2 本（[`judge_press`]・[`fold_no_user_break`]）を収める。
//! 持ち物（`UserBreakWiring`）・押下の入口・旗の取り出し・結線は後続の task で本 mod へ増設される。

use wintf::ecs::pointer::DoubleClick;

use crate::emo2_boot::user_break_cue::NoUserBreakSignal;

/// 押下 1 回の結論（design「UserBreakWiring と judge_press」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressVerdict {
    /// 左ダブルクリックではない——何もしない（要件 1.2）。
    NotDoubleClick,
    /// この押下か直前の押下が選択の確定——合図を作らない（要件 1.5）。
    ConsumedBySelection,
    /// バルーンが出ていない・観測できない——作らない（要件 1.9）。
    BalloonHidden,
    /// 中断を禁じる区間——止めず隠さず記録だけ（要件 2.3）。
    Disabled,
    /// 受け入れ——隠して、止める要求を送る（要件 1.1・1.6）。
    Break,
}

/// 押下 1 回を結論へ写す（純関数）。上から順に、最初に当たった結論を返す。
///
/// 順は ⑴ 左ダブルクリックでない ⑵ 選択の確定に消費された ⑶ バルーンが出ていない
/// ⑷ 無効化の区間 ⑸ 受理（design「判定の順」）。`balloon_visible` の `None` は
/// 「表示層に相手が居ない＝観測できない」で、作らない側へ倒す（要件 1.9）。
///
/// SSTP に関する判定は 1 つも置かない（要件 5.8）。
pub(crate) fn judge_press(
    double_click: DoubleClick,
    selected_now: bool,
    prev_press_selected: bool,
    balloon_visible: Option<bool>,
    no_user_break: bool,
) -> PressVerdict {
    if double_click != DoubleClick::Left {
        PressVerdict::NotDoubleClick
    } else if selected_now || prev_press_selected {
        PressVerdict::ConsumedBySelection
    } else if balloon_visible != Some(true) {
        PressVerdict::BalloonHidden
    } else if no_user_break {
        PressVerdict::Disabled
    } else {
        PressVerdict::Break
    }
}

/// 中断を禁じる旗の畳み込み（純関数）。返り値は（次の旗, 区間外の「出る」だったか）。
///
/// 入れ子は数えない——「入る」が重なっても旗は 1 本で、「出る」1 回で区間から出る
/// （要件 5.5）。トークの始まりは閉じ忘れた区間を解く契機である（要件 5.4）。
/// 区間外の「出る」は旗を変えず、呼び手が記録できるように第 2 の返り値で伝える（要件 5.6）。
pub(crate) fn fold_no_user_break(flag: bool, signal: NoUserBreakSignal) -> (bool, bool) {
    match signal {
        NoUserBreakSignal::Enter => (true, false),
        NoUserBreakSignal::Leave => (false, !flag),
        NoUserBreakSignal::TalkStarted => (false, false),
    }
}

#[cfg(test)]
#[path = "user_break_tests.rs"]
mod tests;
