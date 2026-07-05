//! 定常運転の Phase 分岐（Req 2・3・pump ゲート・talk 調停・保留 close）。
//!
//! 本モジュールは [`Phase::Steady`] における Tick pump（OnSecondChange GET/NOTIFY の
//! 使い分け）・talk 調停（Value→StartTalk）・boot 中／active talk 中に受領した close
//! 指示の保留処理を担う。タスク 2.4 が本体を実装する。2.1 では骨格のみを用意する。

use super::{Action, Input, State};
use crate::msg::KanadeConfig;

/// 定常運転（Steady）のフェーズ分岐。
///
/// タスク 2.4 が本体（Tick→OnSecondChange pump・Value→StartTalk・保留 close 消化）を
/// 実装する。現時点では現状態を維持し副作用を返さない骨格である。
pub(crate) fn step(state: State, _input: Input, _config: &KanadeConfig) -> (State, Vec<Action>) {
    (state, Vec::new())
}
