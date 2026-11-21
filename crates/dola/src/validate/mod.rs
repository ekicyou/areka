//! ドキュメント構造の整合性を検証する `DolaDocument::validate` の実装。

mod rules;

pub(crate) use rules::{IMPLICIT_KEYFRAME_PREFIX, collect_keyframe_names_from_ref};

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::transition::TransitionRef;

use rules::*;

impl DolaDocument {
    /// ドキュメント全体を検証し、すべてのエラーを収集して返す
    pub fn validate(&self) -> Result<(), Vec<DolaError>> {
        let mut errors = Vec::new();

        validate_schema_version(self, &mut errors);

        validate_variable_ranges(self, &mut errors);

        validate_numbers(self, &mut errors);

        for (sb_name, sb) in &self.storyboard {
            validate_duplicate_keyframes(sb_name, sb, &mut errors);

            validate_reserved_keyframe_names(sb_name, sb, &mut errors);

            // 前方参照は許可する
            validate_keyframe_references(sb_name, sb, &mut errors);

            validate_loop_offset(sb_name, sb, &mut errors);

            for (entry_idx, entry) in sb.entry.iter().enumerate() {
                if let Some(ref var_name) = entry.variable
                    && !self.variable.contains_key(var_name)
                {
                    errors.push(DolaError::UndefinedVariable {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        name: var_name.clone(),
                    });
                }

                if let Some(TransitionRef::Named(ref trans_name)) = entry.transition
                    && !self.transition.contains_key(trans_name)
                {
                    errors.push(DolaError::UndefinedTransition {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        name: trans_name.clone(),
                    });
                }

                let v7 = match (entry.variable.is_some(), entry.transition.is_some()) {
                    (false, true) => Some("transition requires variable"),
                    (true, false) => Some("variable requires transition"),
                    _ => None,
                };
                if let Some(reason) = v7 {
                    errors.push(DolaError::InvalidEntry {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        reason: reason.to_string(),
                    });
                }

                // トリガー以外の trigger_start_offset は compile が黙って捨てる
                if entry.trigger_start_offset.is_some() && entry.trigger_storyboard.is_none() {
                    errors.push(DolaError::InvalidEntry {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        reason: "trigger_start_offset requires trigger_storyboard".to_string(),
                    });
                }

                if entry.at.is_some() && entry.between.is_some() {
                    errors.push(DolaError::InvalidEntry {
                        storyboard: sb_name.clone(),
                        entry_index: entry_idx,
                        reason: "at and between are mutually exclusive".to_string(),
                    });
                }

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

                // エラーの追加順を保つ: 排他 → 自己参照 → 対象の存在
                if let Some(ref trigger_target) = entry.trigger_storyboard {
                    if entry.variable.is_some() || entry.transition.is_some() {
                        errors.push(DolaError::TriggerExclusiveViolation {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                            reason:
                                "trigger_storyboard cannot be combined with variable/transition"
                                    .to_string(),
                        });
                    }

                    // between はタイミング制御なので、トリガーでも使える。
                    // トランジション固有フィールドは transition ごと上の排他で拒まれている

                    if trigger_target == sb_name {
                        errors.push(DolaError::TriggerSelfReference {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                        });
                    }

                    if !self.storyboard.contains_key(trigger_target) {
                        errors.push(DolaError::TriggerTargetNotFound {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                            target: trigger_target.clone(),
                        });
                    }
                }

                let resolved_transition = match &entry.transition {
                    Some(TransitionRef::Inline(def)) => Some(def),
                    Some(TransitionRef::Named(name)) => self.transition.get(name),
                    None => None,
                };

                if let Some(trans_def) = resolved_transition {
                    if trans_def.to.is_some() && trans_def.relative_to.is_some() {
                        errors.push(DolaError::MutuallyExclusive {
                            storyboard: sb_name.clone(),
                            entry_index: entry_idx,
                        });
                    }

                    if let Some(ref var_name) = entry.variable
                        && let Some(var_def) = self.variable.get(var_name)
                    {
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

        validate_trigger_cycles(self, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
