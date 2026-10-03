//! ツールの登録口（rmcp の型を含まない）。
//!
//! 後続 spec はツールの定義（MCP のツール定義を逐語）と非同期の実装の対を
//! [`ToolRegistry::register`] で積み、`start` へ渡す。rmcp の `ToolRouter` への写しは
//! `handler` が 1 度だけ行う。

// 使い手（handler・lib の公開面）が繋がるのは task 3.3。それまでは dead_code を期待として置く
// （テストのビルドでは registry_tests が使うので期待しない）。
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "handler・公開面からの使用（task 3.3）までは使い手が居ない"
    )
)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tracing::warn;

/// MCP のツール定義（要件 7.4: name・title・description・inputSchema を逐語で持つ）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    /// JSON Schema の object（逐語）。
    pub input_schema: serde_json::Map<String, serde_json::Value>,
}

/// 結果の 1 片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolContent {
    Text(String),
    /// base64 の PNG など（符号化は呼び出し側が済ませた文字列を渡す）。
    Image {
        data: String,
        mime_type: String,
    },
}

/// 実装の返す結果（`content` と `isError` へそのまま写す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutcome {
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}

pub type ToolFuture = Pin<Box<dyn Future<Output = ToolOutcome> + Send>>;
/// 実装（設計 B-4: 非同期）。引数は `arguments` の JSON（無ければ `{}`）。
pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> ToolFuture + Send + Sync>;

/// 定義と実装の対の列（要件 7.1）。
#[derive(Default)]
pub struct ToolRegistry {
    entries: Vec<(ToolSpec, ToolHandler)>,
}

impl ToolRegistry {
    /// 対を 1 つ積む。同じ名前の 2 度目は後勝ち（`ToolRouter::add_route` と同じ）で `warn!` を 1 件残す。
    pub fn register(&mut self, spec: ToolSpec, handler: ToolHandler) {
        if let Some(slot) = self.entries.iter_mut().find(|(s, _)| s.name == spec.name) {
            warn!(name = %spec.name, "同じ名前のツールが再び登録された。後の登録で置き換える");
            *slot = (spec, handler);
        } else {
            self.entries.push((spec, handler));
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 登録した対を登録順で返す（`handler` が `ToolRouter` へ写すときに読む）。
    pub(crate) fn entries(&self) -> &[(ToolSpec, ToolHandler)] {
        &self.entries
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
