//! compile の統合テスト

use dola::*;

use crate::common::{
    delta_ms, float_var, make_doc_with_storyboard as make_doc, ms, transition_entry,
};

mod compilation_flow;
mod keyframe_and_trigger;
