//! ストーリーボード実行インスタンスの状態管理。

use crate::InterruptionPolicy;

/// 実行インスタンスの状態（ランタイム内部専用）
///
/// 7バリアント: Created → Playing ⇄ Paused → 終了状態（4種）。
/// 終了状態は `InterruptionPolicy` の同名戦略に1対1対応する（Never 除く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    /// コンパイル直後、再生開始前
    Created,
    /// 再生中
    Playing,
    /// 一時停止中
    Paused,
    /// Conclude 戦略で終了（最終値ジャンプ / 自然終了）
    Concluded,
    /// Cancel 戦略で終了（現在値凍結）
    Cancelled,
    /// Trim 戦略で終了（割り込み時点で切断）
    Trimmed,
    /// Compress 戦略で終了（全トランジション最終値ジャンプ）
    Compressed,
}

impl InstanceState {
    /// この状態が終了状態（遷移不可）かどうかを返す。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Concluded | Self::Cancelled | Self::Trimmed | Self::Compressed
        )
    }

    /// 状態遷移を試行する。
    ///
    /// 遷移が不正な場合は `Err(current_state)` を返す。
    pub fn try_transition(&self, target: InstanceState) -> Result<InstanceState, InstanceState> {
        match (self, &target) {
            // Created → Playing（コンパイル完了）
            (Self::Created, InstanceState::Playing) => Ok(target),

            // Playing → Paused
            (Self::Playing, InstanceState::Paused) => Ok(target),

            // Paused → Playing（Resume）
            (Self::Paused, InstanceState::Playing) => Ok(target),

            // Playing / Paused → 任意の終了状態
            (Self::Playing | Self::Paused, t) if t.is_terminal() => Ok(target),

            // それ以外は不正遷移
            _ => Err(*self),
        }
    }

    /// `InterruptionPolicy` に対応する終了状態を返す。
    ///
    /// `Never` ポリシーは終了状態に直接対応しないため `None` を返す。
    ///
    /// 不変条件: 返却される `Some(state)` は常に `is_terminal() == true`
    /// （`conflict_resolver` の expect / debug_assert はこの不変条件に依存する。
    /// tests/runtime/core_types_test.rs::from_policy_results_are_terminal で検証済み）。
    pub fn from_policy(policy: InterruptionPolicy) -> Option<InstanceState> {
        match policy {
            InterruptionPolicy::Cancel => Some(Self::Cancelled),
            InterruptionPolicy::Conclude => Some(Self::Concluded),
            InterruptionPolicy::Trim => Some(Self::Trimmed),
            InterruptionPolicy::Compress => Some(Self::Compressed),
            InterruptionPolicy::Never => None,
        }
    }
}
