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

#[cfg(feature = "toml")]
impl DolaDocument {
    /// TOML の文字列から読む（validate はしない）
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = dola::DolaDocument::from_toml_str("schema_version = \"1.0\"")?;
    /// assert_eq!(doc.schema_version, "1.0");
    /// assert!(dola::DolaDocument::from_toml_str("schema_version = 1.0").is_err());
    /// assert_eq!(dola::DolaDocument::from_toml_str(&doc.to_toml_string()?)?, doc);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_toml_str(s: &str) -> Result<DolaDocument, crate::TomlReadError> {
        toml::from_str(s)
    }

    /// TOML の文字列へ書く（`toml::to_string_pretty`）
    pub fn to_toml_string(&self) -> Result<String, crate::TomlWriteError> {
        toml::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[serde(default)] による省略フィールドの空コレクション化
    // （フルラウンドトリップは tests/general/core_types_test/document_variable.rs が担う）

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
