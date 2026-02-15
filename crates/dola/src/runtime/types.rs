//! ランタイムの公開型定義（EvaluatedValue, RuntimeError, StartResult）。

use std::fmt;
use std::rc::Rc;

use crate::DolaError;
use crate::value::DynamicValue;

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
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Display for EvaluatedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(v) => write!(f, "{v:.6}"),
            Self::Integer(v) => write!(f, "{v}"),
            Self::Object(v) => write!(f, "{v:?}"),
        }
    }
}

/// ランタイムエラー。
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// 指定ストーリーボード名が未定義
    StoryboardNotFound(String),
    /// 指定 group_id が存在しない（終了済みインスタンスへの操作を含む）
    InvalidGroupId(u64),
    /// duration=0 かつ loop_count 設定 (Req 2.9)
    ZeroDurationWithLoop { storyboard: String },
    /// 無効な loop_count 値（0 以下で -1 を除く）
    InvalidLoopCount(i32),
    /// コンパイルエラー（既存 DolaError のラップ）
    CompileError(Vec<DolaError>),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoryboardNotFound(name) => {
                write!(f, "storyboard not found: '{name}'")
            }
            Self::InvalidGroupId(id) => {
                write!(f, "invalid group_id: {id}")
            }
            Self::ZeroDurationWithLoop { storyboard } => {
                write!(
                    f,
                    "storyboard '{storyboard}' has zero duration with loop_count"
                )
            }
            Self::InvalidLoopCount(count) => {
                write!(f, "invalid loop_count: {count}")
            }
            Self::CompileError(errors) => {
                write!(f, "compile error: {errors:?}")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

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
    /// 正常再生した場合の終了予定時刻（f64秒）。
    /// 無限ループ時は `f64::INFINITY`。
    pub end_time: f64,
}
