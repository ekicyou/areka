//! LoopOffset のテスト群（serde / validation / compile の 3 群）

use crate::support::{linear, ms};
use dola::*;
use std::collections::BTreeMap;

// =============================================================
// ヘルパー: バリデーション・コンパイル用（群をまたいで共有）
// =============================================================

fn doc_with_storyboard(sb: Storyboard) -> DolaDocument {
    let mut storyboard = BTreeMap::new();
    storyboard.insert("test_sb".to_string(), sb);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable: BTreeMap::new(),
        transition: BTreeMap::new(),
        storyboard,
    }
}

fn make_storyboard_with_offset(loop_offset: Option<LoopOffset>) -> Storyboard {
    Storyboard {
        time_scale: 1.0,
        loop_count: -1,
        interruption_policy: InterruptionPolicy::Conclude,
        loop_offset,
        entry: vec![],
    }
}

mod loop_offset_compile;
mod loop_offset_serde;
mod loop_offset_validation;
