//! Dola のバリデーション・コンパイルエラー型

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
    LoopOffsetNegativeMin { storyboard: String, value: f64 },
    /// loop_offset.max が負値 (V15)
    LoopOffsetNegativeMax { storyboard: String, value: f64 },
    /// loop_offset の min > max (V16)
    LoopOffsetRangeInverted {
        storyboard: String,
        min: f64,
        max: f64,
    },
    /// トリガー自己参照 (V14t)
    TriggerSelfReference {
        storyboard: String,
        entry_index: usize,
    },
    /// トリガー循環参照 (V15t)
    TriggerCycle {
        /// 循環パス（例: ["A", "B", "C", "A"]）
        cycle: Vec<String>,
    },
    /// トリガーエントリの排他違反 (V16t/V17t)
    TriggerExclusiveViolation {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// トリガー対象ストーリーボード未定義 (V18t)
    TriggerTargetNotFound {
        storyboard: String,
        entry_index: usize,
        target: String,
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
            DolaError::TriggerSelfReference {
                storyboard,
                entry_index,
            } => {
                write!(
                    f,
                    "Trigger self-reference in storyboard '{}' entry {}: cannot trigger itself",
                    storyboard, entry_index
                )
            }
            DolaError::TriggerCycle { cycle } => {
                write!(f, "Trigger cycle detected: {}", cycle.join(" -> "))
            }
            DolaError::TriggerExclusiveViolation {
                storyboard,
                entry_index,
                reason,
            } => {
                write!(
                    f,
                    "Trigger exclusive violation in storyboard '{}' entry {}: {}",
                    storyboard, entry_index, reason
                )
            }
            DolaError::TriggerTargetNotFound {
                storyboard,
                entry_index,
                target,
            } => {
                write!(
                    f,
                    "Trigger target storyboard '{}' not found in storyboard '{}' entry {}",
                    target, storyboard, entry_index
                )
            }
        }
    }
}

impl std::error::Error for DolaError {}

#[cfg(test)]
mod tests {
    use super::*;

    // D2-T gap tests: DolaError の Display 文字列（全21バリアント）と Error トレイトの直接検証
    // （診断メッセージはユーザー向け挙動の一部であり、フォーマット崩れを回帰検知する）

    fn s(v: &str) -> String {
        v.to_string()
    }

    #[test]
    fn display_schema_and_keyframe_variants() {
        assert_eq!(
            DolaError::SchemaVersionMismatch {
                expected: s("1.0"),
                found: s("0.5"),
            }
            .to_string(),
            "Schema version mismatch: expected '1.0', found '0.5'"
        );
        assert_eq!(
            DolaError::DuplicateKeyframe {
                storyboard: s("sb"),
                name: s("kf"),
            }
            .to_string(),
            "Duplicate keyframe 'kf' in storyboard 'sb'"
        );
        assert_eq!(
            DolaError::ReservedKeyframeName { name: s("start") }.to_string(),
            "Reserved keyframe name 'start' cannot be user-defined"
        );
        assert_eq!(
            DolaError::UndefinedKeyframe {
                storyboard: s("sb"),
                name: s("kf"),
            }
            .to_string(),
            "Undefined keyframe 'kf' in storyboard 'sb'"
        );
        assert_eq!(
            DolaError::KeyframeCycle {
                storyboard: s("sb"),
                cycle: vec![s("a"), s("b"), s("a")],
            }
            .to_string(),
            "Keyframe cycle detected in storyboard 'sb': a -> b -> a"
        );
    }

    #[test]
    fn display_reference_and_entry_variants() {
        assert_eq!(
            DolaError::UndefinedVariable {
                storyboard: s("sb"),
                entry_index: 2,
                name: s("x"),
            }
            .to_string(),
            "Undefined variable 'x' in storyboard 'sb' entry 2"
        );
        assert_eq!(
            DolaError::UndefinedTransition {
                storyboard: s("sb"),
                entry_index: 1,
                name: s("fade"),
            }
            .to_string(),
            "Undefined transition 'fade' in storyboard 'sb' entry 1"
        );
        assert_eq!(
            DolaError::InvalidEntry {
                storyboard: s("sb"),
                entry_index: 3,
                reason: s("bad entry"),
            }
            .to_string(),
            "Invalid entry in storyboard 'sb' at index 3: bad entry"
        );
        assert_eq!(
            DolaError::ObjectTransitionViolation {
                storyboard: s("sb"),
                entry_index: 0,
                field: s("easing"),
            }
            .to_string(),
            "Object transition violation in storyboard 'sb' entry 0: field 'easing' not allowed"
        );
        assert_eq!(
            DolaError::MutuallyExclusive {
                storyboard: s("sb"),
                entry_index: 4,
            }
            .to_string(),
            "Mutually exclusive fields 'to' and 'relative_to' both specified in storyboard 'sb' entry 4"
        );
        assert_eq!(
            DolaError::TypeMismatch {
                storyboard: s("sb"),
                entry_index: 1,
                reason: s("scalar expected"),
            }
            .to_string(),
            "Type mismatch in storyboard 'sb' entry 1: scalar expected"
        );
        assert_eq!(
            DolaError::CompileError {
                storyboard: s("sb"),
                entry_index: 2,
                reason: s("boom"),
            }
            .to_string(),
            "Compile error in storyboard 'sb' at entry 2: boom"
        );
    }

    #[test]
    fn display_value_out_of_range_variant() {
        assert_eq!(
            DolaError::ValueOutOfRange {
                variable: s("x"),
                field: s("initial"),
                value: 5.0,
                min: 0.0,
                max: 1.0,
            }
            .to_string(),
            "Value out of range for variable 'x': initial = 5, valid range [0, 1]"
        );
    }

    #[test]
    fn display_loop_offset_variants() {
        assert_eq!(
            DolaError::LoopOffsetNegativeMin {
                storyboard: s("sb"),
                value: -1.0,
            }
            .to_string(),
            "loop_offset.min is negative in storyboard 'sb': -1"
        );
        assert_eq!(
            DolaError::LoopOffsetNegativeMax {
                storyboard: s("sb"),
                value: -2.5,
            }
            .to_string(),
            "loop_offset.max is negative in storyboard 'sb': -2.5"
        );
        assert_eq!(
            DolaError::LoopOffsetRangeInverted {
                storyboard: s("sb"),
                min: 3.0,
                max: 1.0,
            }
            .to_string(),
            "loop_offset range inverted in storyboard 'sb': min 3 > max 1"
        );
    }

    #[test]
    fn display_trigger_variants() {
        assert_eq!(
            DolaError::TriggerSelfReference {
                storyboard: s("sb"),
                entry_index: 0,
            }
            .to_string(),
            "Trigger self-reference in storyboard 'sb' entry 0: cannot trigger itself"
        );
        assert_eq!(
            DolaError::TriggerCycle {
                cycle: vec![s("A"), s("B"), s("A")],
            }
            .to_string(),
            "Trigger cycle detected: A -> B -> A"
        );
        assert_eq!(
            DolaError::TriggerExclusiveViolation {
                storyboard: s("sb"),
                entry_index: 1,
                reason: s("variable not allowed"),
            }
            .to_string(),
            "Trigger exclusive violation in storyboard 'sb' entry 1: variable not allowed"
        );
        assert_eq!(
            DolaError::TriggerTargetNotFound {
                storyboard: s("sb"),
                entry_index: 2,
                target: s("child"),
            }
            .to_string(),
            "Trigger target storyboard 'child' not found in storyboard 'sb' entry 2"
        );
    }

    #[test]
    fn error_trait_object_usable() {
        let e: Box<dyn std::error::Error> =
            Box::new(DolaError::ReservedKeyframeName { name: s("start") });
        assert_eq!(
            e.to_string(),
            "Reserved keyframe name 'start' cannot be user-defined"
        );
        assert!(e.source().is_none());
    }
}
