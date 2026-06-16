//! # DolaDocument Validation
//!
//! ドキュメント構造の整合性を検証するバリデーショントレイト・実装。

mod rules;

pub(crate) use rules::collect_keyframe_names_from_ref;

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::transition::TransitionRef;

use rules::*;

/// DolaDocument のバリデーション
pub trait Validate {
    /// ドキュメント全体を検証し、すべてのエラーを収集して返す
    fn validate(&self) -> Result<(), Vec<DolaError>>;
}

impl Validate for DolaDocument {
    fn validate(&self) -> Result<(), Vec<DolaError>> {
        let mut errors = Vec::new();

        // V1: スキーマバージョン検証
        validate_schema_version(self, &mut errors);

        // V12: 変数初期値の値域検証
        validate_variable_ranges(self, &mut errors);

        // Storyboard ごとの検証
        for (sb_name, sb) in &self.storyboard {
            // V2: キーフレーム名重複検出
            validate_duplicate_keyframes(sb_name, sb, &mut errors);

            // V3: 予約キーフレーム名検証
            validate_reserved_keyframe_names(sb_name, sb, &mut errors);

            // V6: キーフレーム参照検証（前方参照許可 + 暗黙的KF追跡）
            validate_keyframe_references(sb_name, sb, &mut errors);

            // V14-V17: ループオフセットバリデーション
            validate_loop_offset(sb_name, sb, &mut errors);

            for (entry_idx, entry) in sb.entry.iter().enumerate() {
                // V4: 変数参照存在確認
                if let Some(ref var_name) = entry.variable {
                    if !self.variable.contains_key(var_name) {
                        errors.push(DolaError::UndefinedVariable {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                            name: var_name.clone(),
                        });
                    }
                }

                // V5: トランジション名前参照存在確認
                if let Some(TransitionRef::Named(ref trans_name)) = entry.transition {
                    if !self.transition.contains_key(trans_name) {
                        errors.push(DolaError::UndefinedTransition {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                            name: trans_name.clone(),
                        });
                    }
                }

                // V7: transition あり → variable 必須
                if entry.transition.is_some() && entry.variable.is_none() {
                    errors.push(DolaError::InvalidEntry {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        reason: "transition requires variable".to_string(),
                    });
                }

                // V8: at と between は排他
                if entry.at.is_some() && entry.between.is_some() {
                    errors.push(DolaError::InvalidEntry {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        reason: "at and between are mutually exclusive".to_string(),
                    });
                }

                // V9: 純粋KFエントリ（variable/transition なし）→ keyframe または trigger_storyboard 必須
                if entry.variable.is_none()
                    && entry.transition.is_none()
                    && entry.keyframe.is_none()
                    && entry.trigger_storyboard.is_none()
                {
                    errors.push(DolaError::InvalidEntry {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        reason: "entry without variable/transition must have keyframe or trigger_storyboard".to_string(),
                    });
                }

                // V16t / V14t / V18t: トリガーエントリの検証（エラー追加順を保存）
                if let Some(ref trigger_target) = entry.trigger_storyboard {
                    // V16t: trigger_storyboard と variable/transition の排他チェック
                    if entry.variable.is_some() || entry.transition.is_some() {
                        errors.push(DolaError::TriggerExclusiveViolation {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                            reason:
                                "trigger_storyboard cannot be combined with variable/transition"
                                    .to_string(),
                        });
                    }

                    // V17t: トランジション固有フィールドの追加チェックは不要。
                    // TransitionRef にはfrom/to/easing/durationが含まれるため、
                    // transition自体があればV16tで検出済み。
                    // betweenはタイミング制御なので許可とも取れるが
                    // design.mdの記述に従い、betweenはトリガーでも使用可能とする。

                    // V14t: トリガー自己参照検出
                    if trigger_target == sb_name {
                        errors.push(DolaError::TriggerSelfReference {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                        });
                    }

                    // V18t: トリガー対象ストーリーボード存在確認
                    if !self.storyboard.contains_key(trigger_target) {
                        errors.push(DolaError::TriggerTargetNotFound {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                            target: trigger_target.clone(),
                        });
                    }
                }

                // V10, V11, V13: トランジション内容の検証
                let resolved_transition = match &entry.transition {
                    Some(TransitionRef::Inline(def)) => Some(def),
                    Some(TransitionRef::Named(name)) => self.transition.get(name),
                    None => None,
                };

                if let Some(trans_def) = resolved_transition {
                    // V11: to と relative_to 排他
                    if trans_def.to.is_some() && trans_def.relative_to.is_some() {
                        errors.push(DolaError::MutuallyExclusive {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                        });
                    }

                    // V10, V13: 変数型に基づくトランジション制約
                    if let Some(ref var_name) = entry.variable {
                        if let Some(var_def) = self.variable.get(var_name) {
                            validate_transition_type_constraints(
                                sb_name,
                                entry_idx,
                                var_name,
                                var_def,
                                trans_def,
                                &mut errors,
                            );
                        }
                    }
                }
            }
        }

        // V15t: トリガー循環参照検出（ドキュメントレベル DFS）
        validate_trigger_cycles(self, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
