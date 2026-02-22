//! Shared helpers for compile tests

use dola::*;
use std::collections::BTreeMap;

pub fn make_doc_with_storyboard(
    variables: Vec<(&str, AnimationVariableDef)>,
    transitions: Vec<(&str, TransitionDef)>,
    storyboard_name: &str,
    sb: Storyboard,
) -> DolaDocument {
    let mut variable = BTreeMap::new();
    for (name, def) in variables {
        variable.insert(name.to_string(), def);
    }
    let mut transition = BTreeMap::new();
    for (name, def) in transitions {
        transition.insert(name.to_string(), def);
    }
    let mut storyboard = BTreeMap::new();
    storyboard.insert(storyboard_name.to_string(), sb);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition,
        storyboard,
    }
}
