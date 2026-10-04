//! `get_expression_table` の本物の処理（spec: areka-P0-mcp-expression-table）。
//! 今のシェルのフォルダの `surfacetable.txt` を読み、SSP の日本語の既定の名前と重ねた表を素の値で答える。

use std::collections::HashSet;
use std::path::Path;

use areka_mcp::tools::get_expression_table::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use areka_parsers::charset::{self, DefaultEncoding};
use areka_parsers::shell::{SurfaceTable, parse_surfacetable};
use bevy_ecs::world::World;

use super::resolve::ActiveGhost;
use crate::ghost_session::GhostSlot;

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

/// シェルのフォルダの `surfacetable.txt` だけを読んで転記を返す（`surfaces.txt`・画像・
/// `descript.txt` は読まない）。`charset` を見て読み、無ければ Shift_JIS。無ければ記録せず空の転記、
/// 開けなければ `warn!` して空の転記、読めない行があれば行番号の列を持つ `warn!` を 1 回だけ出す
/// （要件 1.6・2.10・2.12・3.5・5.1・5.2・5.7・5.8）。
fn load(shell_dir: &Path) -> SurfaceTable {
    let path = shell_dir.join("surfacetable.txt");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return SurfaceTable::default(),
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "[get_expression_table] surfacetable.txt を開けない: 既定の名前だけで答える"
            );
            return SurfaceTable::default();
        }
    };
    let table = parse_surfacetable(&charset::decode(&bytes, DefaultEncoding::Ansi));
    if !table.unreadable.is_empty() {
        tracing::warn!(
            path = %path.display(),
            count = table.unreadable.len(),
            lines = ?table.unreadable,
            "[get_expression_table] surfacetable.txt に読めない行: 読めた行だけで答える"
        );
    }
    table
}

/// 呼ばれるたびに稼働中のゴーストの今のシェルのフォルダを引き、表を素の値で 1 回だけ答える。
/// 引けなければ（置き場が空・実行系が無い）`warn!` して既定の 15 件で答える。World は読むだけで、
/// 送り口・SHIORI・サーフェスに触れず、ファイルを書かない。`ghost`・`args` は入口で解決済みなので
/// 使わない（要件 1.7・3.5・5.9・6.1・6.2）。
pub(super) fn handle(world: &mut World, _ghost: &ActiveGhost, _args: Args, reply: ReplyTo) {
    let shell_dir = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime())
        .map(|runtime| runtime.mount().shell.dir.as_path());
    let table = match shell_dir {
        Some(dir) => load(dir),
        None => {
            tracing::warn!(
                "[get_expression_table] シェルのフォルダを引けない: 既定の名前だけで答える"
            );
            SurfaceTable::default()
        }
    };
    reply.send(outcome::value(render(&table)));
}

#[cfg(test)]
#[path = "get_expression_table_tests.rs"]
mod get_expression_table_tests;
