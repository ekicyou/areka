//! 切替の相（`src/schedule/change.rs`）。
//!
//! 運行表の帳簿のうち、ゴーストの切替を受理してから止まるまでの控えを置く。

use crate::change::ChangeRequest;

/// 受理した切替の帳簿（[`super::State::change`]）。
///
/// 受理で立ち、中止・取りやめでだけ消え、`Unloading` まで残る。止まった時点の値が
/// 停止通知の切替の中身（`KanadeStopped.handoff`）の源になる。
pub(crate) struct ChangeState {
    /// 受理した切替の要求。
    pub req: ChangeRequest,
    /// `OnGhostChanging` が返した台本（送らなかった・204・失敗は `None`）。
    pub script: Option<String>,
}
