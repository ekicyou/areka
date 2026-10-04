//! `get_expression_table` のダミーの処理（spec: areka-P0-mcp-tool-entrances）。

use std::collections::HashSet;

use areka_mcp::tools::get_expression_table::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use areka_parsers::shell::SurfaceTable;
use bevy_ecs::world::World;

use super::resolve::ActiveGhost;

/// 既定の名前（SSP の日本語の表と同じ 15 件・ID の昇順）。言語の設定は読まない（要件 3.1・3.6）。
const DEFAULT_NAMES: [(u32, &str); 15] = [
    (0, "素"),
    (1, "照れ"),
    (2, "驚き"),
    (3, "不安"),
    (4, "落ち込み"),
    (5, "微笑み"),
    (6, "目閉じ"),
    (7, "怒り"),
    (8, "冷笑"),
    (9, "照れ怒り"),
    (10, r"\1-素"),
    (11, r"\1-刮目"),
    (19, r"\1-歌"),
    (20, "立て看板"),
    (25, "歌"),
];

/// 転記と既定の名前から返す表の文字列を組む（純粋）。
///
/// 書かれた ID の集合は全行（`__disabled`・`__parts`・名前の省略・スコープを問わない）から作り、
/// 既定はその集合に無い ID にだけ（スコープ 0・キャラクタ名は空で）載せる。シェルの行は
/// `__disabled` と `__parts` を除いて載せ、既定と 1 本にして（スコープ, ID）で安定に並べる
/// （同じ（スコープ, ID）は書かれた順のまま両方）。どの行も `\r\n` で終える
/// （要件 1.1〜1.5・2.7〜2.9・2.11・3.2〜3.4・3.7・4.1〜4.4・5.9・5.10）。
fn render(table: &SurfaceTable) -> String {
    let written: HashSet<u32> = table.rows.iter().map(|row| row.id).collect();
    let mut rows: Vec<(u32, &str, &str, u32)> = table
        .rows
        .iter()
        .filter(|row| row.group != "__disabled" && row.name != "__parts")
        .map(|row| (row.scope, row.group.as_str(), row.name.as_str(), row.id))
        .chain(
            DEFAULT_NAMES
                .iter()
                .filter(|(id, _)| !written.contains(id))
                .map(|&(id, name)| (0, "", name, id)),
        )
        .collect();
    rows.sort_by_key(|&(scope, _, _, id)| (scope, id));

    let mut out = String::from(
        "|scope \\0,\\1,\\p[2]...|character name|description|surface number : \\s[]|\r\n\
         |-----|-----|-----|-----|\r\n",
    );
    for (scope, group, name, id) in rows {
        let scope = match scope {
            0 | 1 => format!("\\{scope}"),
            n => format!("\\p[{n}]"),
        };
        out.push_str(&format!("|{scope}|{group}|{name}|\\s[{id}]|\r\n"));
    }
    out
}

/// まだ中身が無い。World・ゴースト・引数は使わず（ゴーストに何もさせず）`NG:` で答える（要件 5.1・5.2）。
pub(super) fn handle(_world: &mut World, _ghost: &ActiveGhost, _args: Args, reply: ReplyTo) {
    reply.send(outcome::ng("not implemented yet"));
}

#[cfg(test)]
#[path = "get_expression_table_tests.rs"]
mod get_expression_table_tests;
