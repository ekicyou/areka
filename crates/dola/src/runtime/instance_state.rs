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
    /// このファイルの tests::instance_state_properties::from_policy_results_are_terminal で検証済み）。
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

// 公開の口がこの型を返さないので、状態遷移の表はクレートの中で見る
#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Task 1.1: InstanceState 状態遷移テスト
    // =============================================================================

    mod instance_state_transitions {
        use super::*;
        use InstanceState::*;

        const STATES: [InstanceState; 7] = [
            Created, Playing, Paused, Concluded, Cancelled, Trimmed, Compressed,
        ];

        /// 7×7 の 49 組すべての期待（行＝遷移元・列＝遷移先、並びは STATES）。
        /// 許す組は Ok(遷移先)、ほかは Err(遷移元)。
        #[test]
        fn try_transition_table() {
            #[rustfmt::skip]
            const ALLOWED: [[bool; 7]; 7] = [
                //       Created Playing Paused Concluded Cancelled Trimmed Compressed
                /* Created    */ [false, true,  false, false, false, false, false],
                /* Playing    */ [false, false, true,  true,  true,  true,  true ],
                /* Paused     */ [false, true,  false, true,  true,  true,  true ],
                /* Concluded  */ [false, false, false, false, false, false, false],
                /* Cancelled  */ [false, false, false, false, false, false, false],
                /* Trimmed    */ [false, false, false, false, false, false, false],
                /* Compressed */ [false, false, false, false, false, false, false],
            ];
            let mut checked = 0;
            for (from, row) in STATES.iter().zip(ALLOWED) {
                for (to, allowed) in STATES.iter().zip(row) {
                    let expected = if allowed { Ok(*to) } else { Err(*from) };
                    assert_eq!(from.try_transition(*to), expected, "{from:?} -> {to:?}");
                    checked += 1;
                }
            }
            assert_eq!(checked, 49);
        }
    }

    mod instance_state_properties {
        use super::*;

        #[test]
        fn terminal_states() {
            assert!(!InstanceState::Created.is_terminal());
            assert!(!InstanceState::Playing.is_terminal());
            assert!(!InstanceState::Paused.is_terminal());
            assert!(InstanceState::Concluded.is_terminal());
            assert!(InstanceState::Cancelled.is_terminal());
            assert!(InstanceState::Trimmed.is_terminal());
            assert!(InstanceState::Compressed.is_terminal());
        }

        #[test]
        fn from_policy_mapping() {
            assert_eq!(
                InstanceState::from_policy(InterruptionPolicy::Cancel),
                Some(InstanceState::Cancelled)
            );
            assert_eq!(
                InstanceState::from_policy(InterruptionPolicy::Conclude),
                Some(InstanceState::Concluded)
            );
            assert_eq!(
                InstanceState::from_policy(InterruptionPolicy::Trim),
                Some(InstanceState::Trimmed)
            );
            assert_eq!(
                InstanceState::from_policy(InterruptionPolicy::Compress),
                Some(InstanceState::Compressed)
            );
            assert_eq!(InstanceState::from_policy(InterruptionPolicy::Never), None);
        }

        /// from_policy が返す Some(state) は全て終了状態である
        /// （conflict_resolver の expect / debug_assert が依存する不変条件）
        #[test]
        fn from_policy_results_are_terminal() {
            let policies = [
                InterruptionPolicy::Cancel,
                InterruptionPolicy::Conclude,
                InterruptionPolicy::Trim,
                InterruptionPolicy::Compress,
                InterruptionPolicy::Never,
            ];
            for policy in policies {
                if let Some(state) = InstanceState::from_policy(policy) {
                    assert!(
                        state.is_terminal(),
                        "from_policy({policy:?}) returned non-terminal state {state:?}"
                    );
                }
            }
        }
    }
}
