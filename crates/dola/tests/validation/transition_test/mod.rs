//! Validation tests for V7, V10-V13 (transition constraints, object type, value range, type mismatch)
//! Tasks 7.6, 8.6

use dola::*;
use std::collections::BTreeMap;

use super::common::minimal_valid_doc;

/// ヘルパー: f64変数付きドキュメント
fn doc_with_float_var(
    name: &str,
    initial: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        name.to_string(),
        AnimationVariableDef::Float { initial, min, max },
    );
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard: BTreeMap::new(),
    }
}

mod transition_v11_v12;
mod transition_v13_nan;
mod transition_v7_v10;
