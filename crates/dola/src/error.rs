//! Dola のバリデーション・コンパイルエラー型

/// Dola バリデーションエラー
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DolaError {
    /// スキーマバージョン不一致 (V1)
    #[error("Schema version mismatch: expected '{expected}', found '{found}'")]
    SchemaVersionMismatch { expected: String, found: String },
    /// キーフレーム名重複 (V2)
    #[error("Duplicate keyframe '{name}' in storyboard '{storyboard}'")]
    DuplicateKeyframe { storyboard: String, name: String },
    /// 予約キーフレーム名使用 (V3)
    #[error(
        "Reserved keyframe name '{name}' cannot be user-defined in storyboard '{storyboard}' entry {entry_index}"
    )]
    ReservedKeyframeName {
        storyboard: String,
        entry_index: usize,
        name: String,
    },
    /// 未定義変数参照 (V4)
    #[error("Undefined variable '{name}' in storyboard '{storyboard}' entry {entry_index}")]
    UndefinedVariable {
        storyboard: String,
        entry_index: usize,
        name: String,
    },
    /// 未定義トランジション参照 (V5)
    #[error("Undefined transition '{name}' in storyboard '{storyboard}' entry {entry_index}")]
    UndefinedTransition {
        storyboard: String,
        entry_index: usize,
        name: String,
    },
    /// 未定義キーフレーム参照 (V6)
    #[error("Undefined keyframe '{name}' in storyboard '{storyboard}'")]
    UndefinedKeyframe { storyboard: String, name: String },
    /// 無効なエントリ構成 (V7, V8, V9)
    #[error("Invalid entry in storyboard '{storyboard}' at index {entry_index}: {reason}")]
    InvalidEntry {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// Object 型トランジション制限違反 (V10)
    #[error(
        "Object transition violation in storyboard '{storyboard}' entry {entry_index}: field '{field}' not allowed"
    )]
    ObjectTransitionViolation {
        storyboard: String,
        entry_index: usize,
        field: String,
    },
    /// to/relative_to 排他違反 (V11)
    #[error(
        "Mutually exclusive fields 'to' and 'relative_to' both specified in storyboard '{storyboard}' entry {entry_index}"
    )]
    MutuallyExclusive {
        storyboard: String,
        entry_index: usize,
    },
    /// 変数の初期値の値域超過 (V12)
    #[error(
        "Value out of range for variable '{variable}': {field} = {value}, valid range [{min}, {max}]"
    )]
    ValueOutOfRange {
        variable: String,
        field: String,
        value: f64,
        min: f64,
        max: f64,
    },
    /// 遷移の from/to/relative_to の値域超過 (V12)
    #[error(
        "Value out of range for variable '{variable}' in storyboard '{storyboard}' entry {entry_index}: {field} = {value}, valid range [{min}, {max}]"
    )]
    TransitionValueOutOfRange {
        storyboard: String,
        entry_index: usize,
        variable: String,
        field: String,
        value: f64,
        min: f64,
        max: f64,
    },
    /// 変数型とトランジション値型の不整合 (V13)
    #[error("Type mismatch in storyboard '{storyboard}' entry {entry_index}: {reason}")]
    TypeMismatch {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// キーフレーム循環依存
    #[error("Keyframe cycle detected in storyboard '{storyboard}': {}", .cycle.join(" -> "))]
    KeyframeCycle {
        storyboard: String,
        /// 循環に含まれるキーフレーム名のリスト
        cycle: Vec<String>,
    },
    /// コンパイル固有エラー（汎用）
    #[error("Compile error in storyboard '{storyboard}' at entry {entry_index}: {reason}")]
    CompileError {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// loop_offset.min が負値 (V14)
    #[error("loop_offset.min is negative in storyboard '{storyboard}': {value}")]
    LoopOffsetNegativeMin { storyboard: String, value: f64 },
    /// loop_offset.max が負値 (V15)
    #[error("loop_offset.max is negative in storyboard '{storyboard}': {value}")]
    LoopOffsetNegativeMax { storyboard: String, value: f64 },
    /// loop_offset の min > max (V16)
    #[error("loop_offset range inverted in storyboard '{storyboard}': min {min} > max {max}")]
    LoopOffsetRangeInverted {
        storyboard: String,
        min: f64,
        max: f64,
    },
    /// トリガー自己参照 (V14t)
    #[error(
        "Trigger self-reference in storyboard '{storyboard}' entry {entry_index}: cannot trigger itself"
    )]
    TriggerSelfReference {
        storyboard: String,
        entry_index: usize,
    },
    /// トリガー循環参照 (V15t)
    #[error("Trigger cycle detected: {}", .cycle.join(" -> "))]
    TriggerCycle {
        /// 循環パス（例: ["A", "B", "C", "A"]）
        cycle: Vec<String>,
    },
    /// トリガーエントリの排他違反 (V16t/V17t)
    #[error(
        "Trigger exclusive violation in storyboard '{storyboard}' entry {entry_index}: {reason}"
    )]
    TriggerExclusiveViolation {
        storyboard: String,
        entry_index: usize,
        reason: String,
    },
    /// トリガー対象ストーリーボード未定義 (V18t)
    #[error(
        "Trigger target storyboard '{target}' not found in storyboard '{storyboard}' entry {entry_index}"
    )]
    TriggerTargetNotFound {
        storyboard: String,
        entry_index: usize,
        target: String,
    },
    /// 数値の規則の違反（有限・符号・範囲）
    #[error("Invalid number in {location}: {field} = {value}")]
    InvalidNumber {
        /// 例 "storyboard 'fade' entry 2"・"variable 'opacity'"・"transition 'slide'"
        location: String,
        /// 例 "duration"
        field: String,
        value: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // DolaError の Display 文字列（全22バリアント）と Error トレイトの直接検証
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
            DolaError::ReservedKeyframeName {
                storyboard: s("sb"),
                entry_index: 1,
                name: s("start"),
            }
            .to_string(),
            "Reserved keyframe name 'start' cannot be user-defined in storyboard 'sb' entry 1"
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
        assert_eq!(
            DolaError::TransitionValueOutOfRange {
                storyboard: s("sb"),
                entry_index: 2,
                variable: s("x"),
                field: s("to"),
                value: 5.0,
                min: 0.0,
                max: 1.0,
            }
            .to_string(),
            "Value out of range for variable 'x' in storyboard 'sb' entry 2: to = 5, valid range [0, 1]"
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
    fn display_invalid_number_variant() {
        assert_eq!(
            DolaError::InvalidNumber {
                location: s("storyboard 'sb' entry 0"),
                field: s("duration"),
                value: -3.0,
            }
            .to_string(),
            "Invalid number in storyboard 'sb' entry 0: duration = -3"
        );
    }

    #[test]
    fn error_trait_object_usable() {
        let e: Box<dyn std::error::Error> = Box::new(DolaError::ReservedKeyframeName {
            storyboard: s("sb"),
            entry_index: 0,
            name: s("start"),
        });
        assert_eq!(
            e.to_string(),
            "Reserved keyframe name 'start' cannot be user-defined in storyboard 'sb' entry 0"
        );
        assert!(e.source().is_none());
    }
}
