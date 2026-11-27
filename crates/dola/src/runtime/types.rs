//! ランタイムの公開型定義（EvaluatedValue, RuntimeError, StartResult）。

use std::rc::Rc;

use crate::DolaError;
use crate::time::{TimePoint, TimeSpan};
use crate::value::DynamicValue;

/// 無限ループ許可の最小の 1 周の長さ（100 ms）。
/// 時刻が飛んだ tick で進める周回の数に上限を作る（トリガーを持つループの重さは
/// `loop_controller::process_loops` の注）。
pub(crate) const MIN_LOOP_DURATION: TimeSpan = TimeSpan::from_millis(100);

/// 評価済み変数値（補間計算の出力）。
///
/// `Object` バリアントは `Rc<DynamicValue>` を保持し、
/// `PartialEq` では `Rc::ptr_eq()` による O(1) ポインタ比較を行う。
#[derive(Debug, Clone)]
pub enum EvaluatedValue {
    /// 浮動小数点（f64 直接値）
    Float(f64),
    /// 整数（f64 補間 → i64 丸め）
    Integer(i64),
    /// オブジェクト型（即時切替、Rc 共有による O(1) 比較）
    Object(Rc<DynamicValue>),
}

impl PartialEq for EvaluatedValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // Float(NaN) は IEEE 754 により自分自身と不等。NaN が補間結果に混入すると、
            // 差分検出は当該変数を毎フレーム「変化あり」と判定し続ける。文書の数値は validate が
            // 有限に限るが、極端に大きい値の補間はまだ NaN を出しうる
            // （`tests/runtime/core_types_test.rs` の `float_nan_is_never_equal_to_itself`）。
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// ランタイムエラー。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RuntimeError {
    /// 指定ストーリーボード名が未定義
    #[error("storyboard not found: '{0}'")]
    StoryboardNotFound(String),
    /// 指定 group_id が存在しない（終了済みインスタンスへの操作を含む）
    #[error("invalid group_id: {0}")]
    InvalidGroupId(u64),
    /// duration=0 かつ loop_count 設定
    #[error("storyboard '{storyboard}' has zero duration with loop_count")]
    ZeroDurationWithLoop { storyboard: String },
    /// 無限ループ時の周期が短すぎる（MIN_LOOP_DURATION 未満）
    #[error(
        "storyboard '{storyboard}' has too short duration ({duration}) for infinite loop (min: {MIN_LOOP_DURATION})"
    )]
    TooShortDurationWithInfiniteLoop {
        storyboard: String,
        duration: TimeSpan,
    },
    /// コンパイルエラー
    #[error("compile error: {0:?}")]
    CompileError(Vec<DolaError>),
    /// Never 戦略を持つインスタンスとの競合により start() が拒否された
    #[error("conflict with never-interrupt instances: {conflicting_group_ids:?}")]
    Conflict { conflicting_group_ids: Vec<u64> },
    /// 無効な variable_id（存在しない or 購読解除済み）
    #[error("invalid variable_id: {0}")]
    InvalidVariableId(i64),
}

impl From<Vec<DolaError>> for RuntimeError {
    fn from(errors: Vec<DolaError>) -> Self {
        Self::CompileError(errors)
    }
}

/// Start コマンドの返却値。
#[derive(Debug, Clone, PartialEq)]
pub struct StartResult {
    /// 実行インスタンスの一意識別子
    pub group_id: u64,
    /// 再生の全体の終わりの時刻。その時に決まっていなければ None
    /// （終わらないループ・周回の間の待ちを、これから周回ごとに引くループ）。
    pub end_time: Option<TimePoint>,
    /// 競合解決で影響を受けた既存 group_id のリスト。
    /// 競合がなければ空。
    pub affected_group_ids: Vec<u64>,
}

/// `tick()` の結果（`last_result()` が返す）。
///
/// 変数の差分変化とトリガー実行結果を含む。
#[derive(Debug, Clone)]
pub struct UpdateResult {
    /// 変数の差分変化（variable_id ベース）
    pub changes: Vec<(i64, EvaluatedValue)>,
    /// トリガー実行結果のリスト
    pub triggered: Vec<TriggerResult>,
}

/// 個別トリガーの実行結果。
#[derive(Debug, Clone)]
pub enum TriggerResult {
    /// トリガー成功
    Started {
        /// 起動元ストーリーボード名
        source_storyboard: String,
        /// 起動先ストーリーボード名
        target_storyboard: String,
        /// 起動結果
        start_result: StartResult,
    },
    /// トリガー失敗（競合等）
    Error {
        /// 起動元ストーリーボード名
        source_storyboard: String,
        /// 起動先ストーリーボード名
        target_storyboard: String,
        /// エラー内容
        error: RuntimeError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_error_contains_group_ids() {
        let err = RuntimeError::Conflict {
            conflicting_group_ids: vec![1, 2, 3],
        };
        match &err {
            RuntimeError::Conflict {
                conflicting_group_ids,
            } => {
                assert_eq!(conflicting_group_ids, &vec![1, 2, 3]);
            }
            _ => panic!("expected Conflict variant"),
        }
    }

    #[test]
    fn conflict_error_display() {
        let err = RuntimeError::Conflict {
            conflicting_group_ids: vec![5, 10],
        };
        let msg = format!("{err}");
        assert!(
            msg.contains("conflict"),
            "Display should mention conflict: {msg}"
        );
        assert!(
            msg.contains("5"),
            "Display should contain group_id 5: {msg}"
        );
        assert!(
            msg.contains("10"),
            "Display should contain group_id 10: {msg}"
        );
    }

    #[test]
    fn start_result_has_affected_group_ids() {
        let result = StartResult {
            group_id: 1,
            end_time: Some(TimePoint::from_ticks(50_000_000)),
            affected_group_ids: vec![2, 3],
        };
        assert_eq!(result.affected_group_ids, vec![2, 3]);
    }
}
