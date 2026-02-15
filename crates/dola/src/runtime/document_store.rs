//! 指示書（DolaDocument）の保持・バリデーション・差し替え。

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::storyboard::Storyboard;
use crate::validate::Validate;

/// 指示書の保持・バリデーション・差し替えを行う内部コンポーネント。
///
/// # Invariants
/// - `document` は `store()` 成功時のみ更新される
/// - バリデーション失敗時は既存の document を保持する
pub(crate) struct DocumentStore {
    document: Option<DolaDocument>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self { document: None }
    }

    /// バリデーション実行 + 成功時のみ保持。
    ///
    /// # Preconditions
    /// - `doc` はデシリアライズ済み `DolaDocument`
    ///
    /// # Postconditions
    /// - 成功: 新 doc を保持（旧 doc を上書き）
    /// - 失敗: 既存 doc を保持 + エラー返却
    pub fn store(&mut self, doc: DolaDocument) -> Result<(), Vec<DolaError>> {
        doc.validate()?;
        self.document = Some(doc);
        Ok(())
    }

    /// 現在の document への参照。
    pub fn document(&self) -> Option<&DolaDocument> {
        self.document.as_ref()
    }

    /// ストーリーボード定義の名前検索。
    #[allow(dead_code)]
    pub fn get_storyboard(&self, name: &str) -> Option<&Storyboard> {
        self.document
            .as_ref()
            .and_then(|doc| doc.storyboard.get(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::storyboard::{InterruptionPolicy, Storyboard, StoryboardEntry};
    use crate::transition::{TransitionDef, TransitionRef, TransitionValue};
    use crate::variable::AnimationVariableDef;

    fn make_valid_doc() -> DolaDocument {
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "fade".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                entry: vec![StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: None,
                }],
            },
        );

        DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    fn make_invalid_doc() -> DolaDocument {
        DolaDocument {
            schema_version: "99.0".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        }
    }

    #[test]
    fn initial_state_is_none() {
        let store = DocumentStore::new();
        assert!(store.document().is_none());
    }

    #[test]
    fn store_valid_document() {
        let mut store = DocumentStore::new();
        let doc = make_valid_doc();
        assert!(store.store(doc).is_ok());
        assert!(store.document().is_some());
    }

    #[test]
    fn store_invalid_document_preserves_existing() {
        let mut store = DocumentStore::new();
        let valid = make_valid_doc();
        store.store(valid).unwrap();
        assert!(store.document().is_some());

        let invalid = make_invalid_doc();
        assert!(store.store(invalid).is_err());
        // 既存ドキュメントが保持されている
        assert!(store.document().is_some());
        assert_eq!(store.document().unwrap().schema_version, "1.0");
    }

    #[test]
    fn get_storyboard_found() {
        let mut store = DocumentStore::new();
        store.store(make_valid_doc()).unwrap();
        assert!(store.get_storyboard("fade").is_some());
    }

    #[test]
    fn get_storyboard_not_found() {
        let mut store = DocumentStore::new();
        store.store(make_valid_doc()).unwrap();
        assert!(store.get_storyboard("nonexistent").is_none());
    }

    #[test]
    fn overwrite_document() {
        let mut store = DocumentStore::new();
        store.store(make_valid_doc()).unwrap();

        // 新しい doc で差し替え
        let mut doc2 = make_valid_doc();
        doc2.variable.insert(
            "y".to_string(),
            AnimationVariableDef::Float {
                initial: 5.0,
                min: None,
                max: None,
            },
        );
        store.store(doc2).unwrap();
        assert!(store.document().unwrap().variable.contains_key("y"));
    }
}
