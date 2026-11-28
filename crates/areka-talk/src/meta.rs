//! 台本の付帯情報（実行するゴーストと置き場・送り手）の型。

use std::path::PathBuf;

/// 台本の付帯情報（トークの始まりに決まり、終わりまで変わらない）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TalkMeta {
    pub runner: TalkRunner,
    pub sender: TalkSender,
}

/// 台本を実行する（喋る）ゴースト。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TalkRunner {
    /// 一覧と script の記録が使う名（descript の name、無い・空なら最上位の絶対パス）。
    pub name: String,
    /// 置き場。`None` は置き場を渡さずに組んだ kanade（試験の支え）。
    pub place: Option<GhostPlace>,
}

/// 実行するゴーストの置き場（どちらも絶対パス）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostPlace {
    /// ghost/master（SHIORI の DLL がある所）。
    pub master: PathBuf,
    /// ゴーストの最上位（ghost/master の 2 つ上）。
    pub root: PathBuf,
}

/// 台本の送り手（今の出どころの種類と信用度は `ScriptOrigin` のまま別に持つ）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TalkSender {
    /// 実行するゴースト自身（名は `TalkRunner::name`）。
    #[default]
    Runner,
    /// 実行するゴーストと別のゴースト。
    Ghost(String),
    /// 道具（MCP の `sakurascript`・`raise_event`）。
    Tool(String),
}

impl TalkMeta {
    /// 送り手の名（実行するゴースト自身なら実行するゴーストの名）。
    pub fn sender_name(&self) -> &str {
        match &self.sender {
            TalkSender::Runner => &self.runner.name,
            TalkSender::Ghost(n) | TalkSender::Tool(n) => n,
        }
    }
}

#[cfg(test)]
#[path = "meta_tests.rs"]
mod tests;
