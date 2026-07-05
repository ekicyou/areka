//! close 握手の Phase 分岐（Req 4・OnClose GET・期限判定・終了系列）。
//!
//! 本モジュールは close 各待ち点（[`Phase::ClosePending`] の OnClose 応答待ち・
//! [`Phase::CloseTalkWait`] の close talk 再生完了待ち／期限判定）の遷移本体を担う。
//! タスク 2.5 が本体を実装する。2.1 では骨格のみを用意する。

use super::{Action, Input, State};
use crate::msg::KanadeConfig;

/// close 握手（ClosePending / CloseTalkWait）のフェーズ分岐。
///
/// タスク 2.5 が本体（OnClose GET 応答分岐・CloseTalkWait 期限判定・quit:false 復帰・
/// 204 無言終了）を実装する。現時点では現状態を維持し副作用を返さない骨格である。
pub(crate) fn step(state: State, _input: Input, _config: &KanadeConfig) -> (State, Vec<Action>) {
    (state, Vec::new())
}
