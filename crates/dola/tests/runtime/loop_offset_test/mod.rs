#![allow(deprecated)]
//! LoopOffset serde round-trip tests
//! Task 1.2: serde ラウンドトリップテスト
//! Task 5.1-5.3: バリデーションテスト
//! Task 2.2: コンパイル統合テスト
//!
//! Task 6.4: 巨大テストファイルの分割（挙動非破壊）
//! serde / validation / compile の 3 群へ分割。リーフ名・内部モジュール名は不変。

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
