//! 自動の上げ下げの判断の純粋な規則（設計の PromotionRule）と、その状態の部品 [`AutoPromotion`]。
//!
//! 時刻は f64 秒のまま丸めずに比べる。前の回より戻った時刻は前の回の値として扱い（経過 0）、
//! 同じ時刻の複数の更新は 1 回に数える。ログの水準は読まない（同じ入力から同じ答え）。

// shortcut: 係から呼ぶのは後のタスク（3.x）。配線が入ったらこの許しを外す。
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use super::promotion_shape::Ineligible;

/// 上げるのに要る更新の回数。
pub(crate) const PROMOTE_COUNT: usize = 8;
/// その回数を数える直近の実時間（秒）。
pub(crate) const PROMOTE_WINDOW_SECS: f64 = 1.0;
/// 戻すのに要る更新の無い実時間（秒）。
pub(crate) const DEMOTE_QUIET_SECS: f64 = 3.0;
/// 上げた後、落ち着きで戻さない実時間（秒）。
pub(crate) const HOLD_AFTER_PROMOTE_SECS: f64 = 3.0;
/// 戻した後（即時の戻しも）、上げない実時間（秒）。
pub(crate) const HOLD_AFTER_DEMOTE_SECS: f64 = 3.0;

/// 節の上げ下げの相。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Phase {
    /// 描画面を持たない節。
    #[default]
    Node,
    /// 上げると決め、自分の描画面を用意している（画面にはまだ親の描画面で描く）。
    Preparing,
    /// 自動で上げた持ち主。
    Active,
}

/// 用意中の描画面の様子（1 回の判断が部品の様子から決める）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SurfaceReadiness {
    Pending,
    Ready,
    Failed,
}

/// 戻す理由。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DemoteReason {
    Quiet,
    Shape(Ineligible),
    SurfaceFailed,
    Absorbed,
}

/// 1 回の判断の答え。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Verdict {
    Stay,
    Prepare,
    Activate,
    Demote(DemoteReason),
    Skip(Ineligible),
}

/// 1 つの節の判断の状態（純粋・ECS に依らない）。
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct RuleState {
    phase: Phase,
    /// 今の相に入った時刻。`None` は履歴を掛けない（初めから・初期化の後）。
    since: Option<f64>,
    /// 前の回の時刻（単調にした値）。
    last_now: Option<f64>,
    /// 直近 `PROMOTE_COUNT` 回の更新の時刻（古い順）。
    updates: VecDeque<f64>,
}

impl RuleState {
    pub(crate) fn phase(&self) -> Phase {
        self.phase
    }

    /// 時刻を単調にする（前の回より戻ったら前の回の値）。
    fn monotonic(&mut self, now: f64) -> f64 {
        let t = match self.last_now {
            Some(prev) if !(now >= prev) => prev,
            _ => now,
        };
        self.last_now = Some(t);
        t
    }

    fn held(&self, t: f64, hold: f64) -> bool {
        self.since.is_some_and(|s| t - s < hold)
    }

    fn enter(&mut self, phase: Phase, t: f64) {
        self.phase = phase;
        self.since = Some(t);
    }

    /// 1 回ぶん進める。`shape` は上げる・保つ判断に要るときだけ呼ぶ（純粋な関数を渡す）。
    pub(crate) fn step(
        &mut self,
        now: f64,
        updated: bool,
        shape: impl FnOnce() -> Result<(), Ineligible>,
        surface: SurfaceReadiness,
    ) -> Verdict {
        let t = self.monotonic(now);
        if updated && self.updates.back() != Some(&t) {
            if self.updates.len() == PROMOTE_COUNT {
                self.updates.pop_front();
            }
            self.updates.push_back(t);
        }
        let verdict = match self.phase {
            Phase::Node => {
                let frequent = self.updates.len() == PROMOTE_COUNT
                    && t - self.updates[0] <= PROMOTE_WINDOW_SECS;
                if !frequent || self.held(t, HOLD_AFTER_DEMOTE_SECS) {
                    Verdict::Stay
                } else {
                    match shape() {
                        Ok(()) => Verdict::Prepare,
                        Err(e) => Verdict::Skip(e),
                    }
                }
            }
            Phase::Preparing => match (shape(), surface) {
                (Err(e), _) => Verdict::Demote(DemoteReason::Shape(e)),
                (Ok(()), SurfaceReadiness::Ready) => Verdict::Activate,
                (Ok(()), SurfaceReadiness::Failed) => Verdict::Demote(DemoteReason::SurfaceFailed),
                (Ok(()), SurfaceReadiness::Pending) => Verdict::Stay,
            },
            Phase::Active => match shape() {
                Err(e) => Verdict::Demote(DemoteReason::Shape(e)),
                Ok(()) => {
                    let quiet = self
                        .updates
                        .back()
                        .is_none_or(|&u| t - u >= DEMOTE_QUIET_SECS);
                    if quiet && !self.held(t, HOLD_AFTER_PROMOTE_SECS) {
                        Verdict::Demote(DemoteReason::Quiet)
                    } else {
                        Verdict::Stay
                    }
                }
            },
        };
        match verdict {
            Verdict::Prepare => self.enter(Phase::Preparing, t),
            Verdict::Activate => self.enter(Phase::Active, t),
            Verdict::Demote(_) => self.enter(Phase::Node, t),
            Verdict::Stay | Verdict::Skip(_) => {}
        }
        verdict
    }

    /// 祖先が上がった回の吸い込み（即時の戻し・上げの履歴を掛ける）。節のままなら何もしない。
    pub(crate) fn absorb(&mut self, now: f64) -> Verdict {
        if self.phase == Phase::Node {
            return Verdict::Stay;
        }
        let t = self.monotonic(now);
        self.enter(Phase::Node, t);
        Verdict::Demote(DemoteReason::Absorbed)
    }

    /// 手動の目印が入った回の初期化（履歴を消し、上げの履歴を掛けない）。
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 節の自動の上げ下げの状態。`WorldVisual` が連れてくる（全部の節が持つ）。
/// 係が触るので公開（中身は crate の外から触れない）。
#[derive(Component, Debug, Default, Clone)]
pub struct AutoPromotion {
    pub(crate) rule: RuleState,
    /// 最後に記録した見送りの理由（続く間は記録しない・要件 7.2）。
    pub(crate) skip_memo: Option<Ineligible>,
    /// 用意中に部分の列を描いた回が在るか（次の回に描画面の有無で Ready／Failed を決める）。
    pub(crate) built: bool,
}
