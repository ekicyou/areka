//! `get_log` の処理（spec: areka-P0-mcp-tool-entrances・areka-P0-mcp-log-history）。

use areka_mcp::ToolOutcome;
use areka_mcp::tools::get_log::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{self, ActiveGhost, CANNOT_FIND, Omitted};
use crate::log_history::{self, Kind, Record};

/// 未知・空の `log_type` の理由（SSP と同じ本文）。
pub(crate) const UNKNOWN_LOG_TYPE: &str =
    "Unknown log_type (error / script / network / update / status)";
/// 0 件の本文（失敗ではない）。
pub(crate) const NO_ENTRIES: &str = "(no log entries)";

/// 入口。World は `ghost_name` があるときに起動中のゴーストを読むだけで、ゴーストに何もさせない。
/// 置き場の写しで答え、その場で送る（要件 3.4・3.5）。
pub(super) fn handle(world: &mut World, args: Args, reply: ReplyTo) {
    let active = args
        .ghost_name
        .as_ref()
        .and_then(|_| resolve::active(world));
    reply.send(answer(log_history::snapshot, active.as_ref(), &args));
}

/// 純粋な答え。`rows_of` は種別の記録（古い順）を返す口で、種別とゴーストが決まった後に 1 度だけ呼ぶ
/// （`NG:` では呼ばない）。`active` は起動中のゴースト（`ghost_name` が無いときは見ない）。
/// 絞り込みは種別 → `ghost_name` → `since_id` → `max_count` の順（要件 5.9）。
fn answer(
    rows_of: impl FnOnce(Kind) -> Vec<Record>,
    active: Option<&ActiveGhost>,
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
    // 2. ゴースト。空は解決へ渡さない（解決は空を省略として扱う。SSP は空も Cannot find）。
    let name = match args.ghost_name.as_deref() {
        None => None,
        Some("") => return outcome::ng(CANNOT_FIND),
        Some(given) => match resolve::resolve(active, Some(given), Omitted::Reject) {
            Ok(ghost) => Some(resolve::listed_value(ghost)),
            Err(_) => return outcome::ng(CANNOT_FIND),
        },
    };
    let mut rows = rows_of(kind);
    if let Some(name) = name {
        rows.retain(|r| r.name == name);
    }
    // 3. since_id（無い・負は絞らない）。
    if let Some(since) = args.since_id.filter(|n| *n >= 0) {
        rows.retain(|r| r.id > since as u64);
    }
    // 4. max_count（0 は 0 件・無い・負は絞らない・1 以上は新しい方から N 件を古い順で）。
    if let Some(max) = args.max_count.filter(|n| *n >= 0) {
        let keep = usize::try_from(max).unwrap_or(usize::MAX);
        rows.drain(..rows.len().saturating_sub(keep));
    }
    // 5. 書式（先頭に OK: を付けない・末尾に区切りを付けない）。
    if rows.is_empty() {
        return outcome::value(NO_ENTRIES);
    }
    outcome::value(rows.iter().map(render).collect::<Vec<_>>().join("\r\n"))
}

/// 記録 1 件を 1 行（継続行を含む）にする。本文の `\r\n`・`\n`・`\r` はどれも `\r\n` とタブ 1 つへ。
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
