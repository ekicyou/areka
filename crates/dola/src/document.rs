//! DolaDocument — Dola ドキュメントのルートコンテナ定義。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::storyboard::Storyboard;
use crate::transition::TransitionDef;
use crate::variable::AnimationVariableDef;

/// Dola ドキュメントのルートコンテナ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DolaDocument {
    /// スキーマバージョン（例: "1.0"）
    pub schema_version: String,
    /// 名前付きアニメーション変数（グローバルスコープ）
    #[serde(default)]
    pub variable: BTreeMap<String, AnimationVariableDef>,
    /// 名前付きトランジションテンプレート（グローバルスコープ）
    #[serde(default)]
    pub transition: BTreeMap<String, TransitionDef>,
    /// 名前付きストーリーボード
    #[serde(default)]
    pub storyboard: BTreeMap<String, Storyboard>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // D3-T ギャップテスト: #[serde(default)] による省略フィールドの空コレクション化
    // （フルラウンドトリップは tests/general/core_types_test.rs が担う）

    #[test]
    fn deserialize_with_only_schema_version_defaults_to_empty_maps() {
        let doc: DolaDocument = serde_json::from_str(r#"{"schema_version":"1.0"}"#).unwrap();
        assert_eq!(doc.schema_version, "1.0");
        assert!(doc.variable.is_empty());
        assert!(doc.transition.is_empty());
        assert!(doc.storyboard.is_empty());
    }

    #[test]
    fn deserialize_without_schema_version_fails() {
        // schema_version は #[serde(default)] なしの必須フィールド
        let result = serde_json::from_str::<DolaDocument>("{}");
        assert!(result.is_err());
    }
}
