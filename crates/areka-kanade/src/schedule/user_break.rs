//! user_break — 利用者の中断（バルーンの左ダブルクリック）の受理規則。
//!
//! [`super::step`] の横断の腕 `Input::UserBreak` から、場面を問わず呼ばれる。
//! いまは入口の宣言だけで、受け取っても状態も指示も変えない。「再生中なら止める／二重に
//! 止めない／再生中でなければ何もしない」の規則と、終了の予約の取り出しはここへ入る。

use super::{Action, State};

/// 利用者の中断の要求を受ける。いまは何もせず、状態をそのまま返す。
pub(super) fn on_user_break(state: State, _scope: u32) -> (State, Vec<Action>) {
    (state, Vec::new())
}

#[cfg(test)]
#[path = "user_break_tests.rs"]
mod tests;
