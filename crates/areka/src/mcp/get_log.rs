//! `get_log` の処理（spec: areka-P0-mcp-tool-entrances・areka-P0-mcp-log-history）。

use areka_mcp::ToolOutcome;
use areka_mcp::tools::get_log::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::ActiveGhost;
use crate::log_history::{Kind, Record};

/// 未知・空の `log_type` の理由（SSP と同じ本文）。
pub(crate) const UNKNOWN_LOG_TYPE: &str =
    "Unknown log_type (error / script / network / update / status)";
/// 0 件の本文（失敗ではない）。
pub(crate) const NO_ENTRIES: &str = "(no log entries)";

/// まだ中身が無い。World・引数は使わず（ゴーストに何もさせず）`NG:` で答える（要件 5.1・5.2）。
pub(super) fn handle(_world: &mut World, _args: Args, reply: ReplyTo) {
    reply.send(outcome::ng("not implemented yet"));
}

/// 純粋な答え。`rows_of` は種別の記録（古い順）を返す口で、種別が決まった後に 1 度だけ呼ぶ
/// （未知の `log_type` では呼ばない）。`active` は起動中のゴースト（`ghost_name` の解決に使う）。
// 本番の呼び手は get_log の入口（areka-P0-mcp-log-history task 3.2）。生えるまで未使用の警告を抑え、3.2 で外す。
#[allow(dead_code)]
fn answer(
    rows_of: impl FnOnce(Kind) -> Vec<Record>,
    _active: Option<&ActiveGhost>,
    args: &Args,
) -> ToolOutcome {
    // 1. 種別（省略は error・照合は大文字小文字を区別せず前後の空白は削らない）。
    let kind = match args.log_type.as_deref() {
        None => Kind::Error,
        Some(word) => match Kind::from_log_type(word) {
            Some(kind) => kind,
            None => return outcome::ng(UNKNOWN_LOG_TYPE),
        },
    };
    let rows = rows_of(kind);
    // 5. 書式（先頭に OK: を付けない・末尾に区切りを付けない）。
    if rows.is_empty() {
        return outcome::value(NO_ENTRIES);
    }
    outcome::value(rows.iter().map(render).collect::<Vec<_>>().join("\r\n"))
}

/// 記録 1 件を 1 行（継続行を含む）にする。本文の `\r\n`・`\n`・`\r` はどれも `\r\n` とタブ 1 つへ。
// 本番の呼び手は answer（上と同じく 3.2 で外す）。
#[allow(dead_code)]
fn render(record: &Record) -> String {
    let t = record.at;
    let body = record
        .body
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n\t");
    format!(
        "#{} {:04}/{:02}/{:02} {:02}:{:02} [{}] {} : {}",
        record.id, t.year, t.month, t.day, t.hour, t.minute, record.label, record.name, body
    )
}

#[cfg(test)]
#[path = "get_log_tests.rs"]
mod get_log_tests;
