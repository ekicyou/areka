//! 利用者の中断（バルーンの左ダブルクリック）の判定と送り出し（areka-P0-balloon-break）。
//!
//! 中断を禁じる旗・直前の押下の記憶・送出端を持ち、押下のたびに「中断にするか」を決めて、
//! 表示の側へ「隠せ」、kanade へ `KanadeMsg::UserBreak` を 1 件ずつ送る層がここへ入る。

#[cfg(test)]
#[path = "user_break_tests.rs"]
mod tests;
