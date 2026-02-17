// TODO: Implement DolaError
use std::fmt;

/// Dola バリデーションエラー
#[derive(Debug, Clone, PartialEq)]
pub enum DolaError {
    /// スキーマバージョン不一致 (V1)
    SchemaVersionMismatch { expected: String, found: String },
    /// キーフレーム名重複 (V2)
    DuplicateKeyframe { storyboard: String, name: String },
    /// 予約キーフレーム名使用 (V3)
    ReservedKeyframeName { name: String },
    /// 未定義変数参照 (V4)
    UndefinedVariable {
        storyboard: String,
        entry_index: usize,
        name: String,
    },
    /// 未定義トランジション参照 (V5)
    UndefinedTransition {
        storyboard: String,
        entry_index: usize,
        name: String,
    },
    /// 未定義キーフレーム参照 (V6)
    UndefinedKeyframe { storyboard: String, name: String },
    /// 無効なエントリ構成 (V7, V8, V9)
    InvalidEntry {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// Object 型トランジション制限違反 (V10)
    ObjectTransitionViolation {
        storyboard: String,
        entry_index: usize,
        field: String,
    },
    /// to/relative_to 排他違反 (V11)
    MutuallyExclusive {
        storyboard: String,
        entry_index: usize,
    },
    /// 値域超過 (V12)
    ValueOutOfRange {
        variable: String,
        field: String,
        value: f64,
        min: f64,
        max: f64,
    },
    /// 変数型とトランジション値型の不整合 (V13)
    TypeMismatch {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// キーフレーム循環依存
    KeyframeCycle {
        storyboard: String,
        /// 循環に含まれるキーフレーム名のリスト
        cycle: Vec<String>,
    },
    /// コンパイル固有エラー（汎用）
    CompileError {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// loop_offset.min が負値 (V14)
    LoopOffsetNegativeMin {
        storyboard: String,
        value: f64,
    },
    /// loop_offset.max が負値 (V15)
    LoopOffsetNegativeMax {
        storyboard: String,
        value: f64,
    },
    /// loop_offset の min > max (V16)
    LoopOffsetRangeInverted {
        storyboard: String,
        min: f64,
        max: f64,
    },
}

impl fmt::Display for DolaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DolaError::SchemaVersionMismatch { expected, found } => {
                write!(
                    f,
                    "Schema version mismatch: expected '{}', found '{}'",
                    expected, found
                )
            }
            DolaError::DuplicateKeyframe { storyboard, name } => {
                write!(
                    f,
                    "Duplicate keyframe '{}' in storyboard '{}'",
                    name, storyboard
                )
            }
            DolaError::ReservedKeyframeName { name } => {
                write!(
                    f,
                    "Reserved keyframe name '{}' cannot be user-defined",
                    name
                )
            }
            DolaError::UndefinedVariable {
                storyboard,
                entry_index,
                name,
            } => {
                write!(
                    f,
                    "Undefined variable '{}' in storyboard '{}' entry {}",
                    name, storyboard, entry_index
                )
            }
            DolaError::UndefinedTransition {
                storyboard,
                entry_index,
                name,
            } => {
                write!(
                    f,
                    "Undefined transition '{}' in storyboard '{}' entry {}",
                    name, storyboard, entry_index
                )
            }
            DolaError::UndefinedKeyframe { storyboard, name } => {
                write!(
                    f,
                    "Undefined keyframe '{}' in storyboard '{}'",
                    name, storyboard
                )
            }
            DolaError::InvalidEntry {
                storyboard,
                entry_index,
                reason,
            } => {
                write!(
                    f,
                    "Invalid entry in storyboard '{}' at index {}: {}",
                    storyboard, entry_index, reason
                )
            }
            DolaError::ObjectTransitionViolation {
                storyboard,
                entry_index,
                field,
            } => {
                write!(
                    f,
                    "Object transition violation in storyboard '{}' entry {}: field '{}' not allowed",
                    storyboard, entry_index, field
                )
            }
            DolaError::MutuallyExclusive {
                storyboard,
                entry_index,
            } => {
                write!(
                    f,
                    "Mutually exclusive fields 'to' and 'relative_to' both specified in storyboard '{}' entry {}",
                    storyboard, entry_index
                )
            }
            DolaError::ValueOutOfRange {
                variable,
                field,
                value,
                min,
                max,
            } => {
                write!(
                    f,
                    "Value out of range for variable '{}': {} = {}, valid range [{}, {}]",
                    variable, field, value, min, max
                )
            }
            DolaError::TypeMismatch {
                storyboard,
                entry_index,
                reason,
            } => {
                write!(
                    f,
                    "Type mismatch in storyboard '{}' entry {}: {}",
                    storyboard, entry_index, reason
                )
            }
            DolaError::KeyframeCycle { storyboard, cycle } => {
                write!(
                    f,
                    "Keyframe cycle detected in storyboard '{}': {}",
                    storyboard,
                    cycle.join(" -> ")
                )
            }
            DolaError::CompileError {
                storyboard,
                entry_index,
                reason,
            } => {
                write!(
                    f,
                    "Compile error in storyboard '{}' at entry {}: {}",
                    storyboard, entry_index, reason
                )
            }
            DolaError::LoopOffsetNegativeMin { storyboard, value } => {
                write!(
                    f,
                    "loop_offset.min is negative in storyboard '{}': {}",
                    storyboard, value
                )
            }
            DolaError::LoopOffsetNegativeMax { storyboard, value } => {
                write!(
                    f,
                    "loop_offset.max is negative in storyboard '{}': {}",
                    storyboard, value
                )
            }
            DolaError::LoopOffsetRangeInverted {
                storyboard,
                min,
                max,
            } => {
                write!(
                    f,
                    "loop_offset range inverted in storyboard '{}': min {} > max {}",
                    storyboard, min, max
                )
            }
        }
    }
}

impl std::error::Error for DolaError {}
