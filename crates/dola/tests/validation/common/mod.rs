//! Shared helpers for validation tests

use dola::*;
use std::collections::BTreeMap;

/// ヘルパー: 最小の有効なドキュメントを作成
pub fn minimal_valid_doc() -> DolaDocument {
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable: BTreeMap::new(),
        transition: BTreeMap::new(),
        storyboard: BTreeMap::new(),
    }
}
