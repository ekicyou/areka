//! boot 系列の Phase 分岐（Req 1・状態機械図の Idle→…→Steady）。
//!
//! 本モジュールは boot 各待ち点（[`Phase::Idle`] からの起動・BootInit / BootType /
//! BootMain / BootVersion の応答進行）の遷移本体を担う。タスク 2.3 が本体を実装する。
//! 2.1 では骨格（呼出面）のみを用意し、[`crate::schedule::mod`] の `step` から
//! フェーズ分岐として呼び出せるようにする。

use super::{Action, Input, State};
use crate::msg::KanadeConfig;

/// boot 系列（Idle / BootInit / BootType / BootMain / BootVersion）のフェーズ分岐。
///
/// タスク 2.3 が本体（OnInitialize NOTIFY 起動・応答進行・OnFirstBoot Value 分岐）を
/// 実装する。現時点では現状態を維持し副作用を返さない骨格である。
pub(crate) fn step(state: State, _input: Input, _config: &KanadeConfig) -> (State, Vec<Action>) {
    (state, Vec::new())
}
