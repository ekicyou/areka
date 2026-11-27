//! 層の記憶のファイルの形・形の版・層を付けたときの検査・直しの段。

use super::{Access, Located, StoreBackend};
use crate::persist::format;
use rusqlite::{Connection, ErrorCode, ffi};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const LOG_TARGET: &str = "areka_sylphya::store";

/// areka の記憶のファイルの印（'arka'）。
pub(crate) const APPLICATION_ID: i32 = 0x6172_6B61;

/// 直しの段の 1 つ（版 n → n+1）。`alias` は付けた層の DB の名前。
pub(crate) type Step = fn(&Connection, &str) -> rusqlite::Result<()>;

/// 版 1 から順の直しの段。今の形の版は `STEPS.len() + 1`。
pub(crate) const STEPS: &[Step] = &[];

/// 層を付けたときの結果。`Attached` と、付けた `LegacyUnusable` のほかは、何も付けていない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Opened {
    /// 記憶のファイルが無い（読み＝空の層・書き＝フォルダとファイルを作る・消しだけなら何も作らない）。
    Missing,
    /// 付けた。`fresh` は表と番号がまだ無い（0 バイト）。
    Attached { fresh: bool },
    /// 読み取りだけの走りの古い形の版（直さずに飛ばす・読みは値なし）。
    NeedsUpgrade,
    /// 読み取りだけの走りの、落ちた後のジャーナルが残る層（開けずに飛ばす・読みは値なし）。
    NeedsRecovery,
    /// 使えない（読みは値なし・覚える値の書きは理由つきで使えない・上書きしない）。
    Unusable(String),
    /// 隣の `sylphya.toml` が使えないので、覚える値の書きは理由つきで使えない。
    /// 読みは新しいファイルの値（`attached` は付けたまま）。
    LegacyUnusable { attached: bool, reason: String },
    /// 開けない（OS の理由。読みは値なし）。
    Io(String),
    /// 錠に当たった（巻き戻して外した）。
    Busy,
}

/// 層を付ける接続を開く。一時の表はメモリーに置き、SQLite の中では待たない。
pub(crate) fn open_connection() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.busy_timeout(Duration::ZERO)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(conn)
}

/// 層の置き場 `root` の記憶のファイルを `alias` として付け、使えるかを決める。
///
/// 控える範囲の中は `mode=ro` で開き、ファイルを変えない。無いファイルは作らない。
/// 読みを飛ばす記録は、`reported` に無い置き場でだけ出す（出したら加える）。
pub(crate) fn open_layer(
    conn: &Connection,
    backend: &dyn StoreBackend,
    root: &Path,
    alias: &str,
    reported: &mut BTreeSet<PathBuf>,
) -> Opened {
    open_layer_with(conn, backend, root, alias, reported, STEPS)
}

pub(crate) fn open_layer_with(
    conn: &Connection,
    backend: &dyn StoreBackend,
    root: &Path,
    alias: &str,
    reported: &mut BTreeSet<PathBuf>,
    steps: &[Step],
) -> Opened {
    let access = if backend.withholds(root) {
        Access::ReadOnly
    } else {
        Access::ReadWrite
    };
    attach_as(conn, backend, root, alias, reported, steps, access)
}

/// 層の置き場 `root` の記憶のファイルを、無ければ作って `alias` として付ける（初めての書き）。
pub(crate) fn create_layer(
    conn: &Connection,
    backend: &dyn StoreBackend,
    root: &Path,
    alias: &str,
    reported: &mut BTreeSet<PathBuf>,
) -> Opened {
    attach_as(conn, backend, root, alias, reported, STEPS, Access::Create)
}

fn attach_as(
    conn: &Connection,
    backend: &dyn StoreBackend,
    root: &Path,
    alias: &str,
    reported: &mut BTreeSet<PathBuf>,
    steps: &[Step],
    access: Access,
) -> Opened {
    let readonly = access == Access::ReadOnly;
    let uri = match backend.locate(root, access) {
        Ok(Located::Uri(uri)) => uri,
        Ok(Located::Missing) => return Opened::Missing,
        Err(e) => return report(root, reported, Opened::Io(e.to_string())),
    };
    let opened = match conn.execute("ATTACH ?1 AS ?2", [uri.as_str(), alias]) {
        Ok(_) => check(conn, alias, readonly, steps),
        Err(e) => classify(&e),
    };
    if !matches!(opened, Opened::Attached { .. }) {
        detach(conn, alias);
    }
    report(root, reported, opened)
}

/// 付けた層の DB の番号と形の版を読み、古い版なら直す。
fn check(conn: &Connection, alias: &str, readonly: bool, steps: &[Step]) -> Opened {
    let header = (|| -> rusqlite::Result<(i64, i32, i32)> {
        let pages = conn.query_row(&format!("PRAGMA {alias}.page_count"), [], |r| r.get(0))?;
        let id = conn.query_row(&format!("PRAGMA {alias}.application_id"), [], |r| r.get(0))?;
        let version = conn.query_row(&format!("PRAGMA {alias}.user_version"), [], |r| r.get(0))?;
        Ok((pages, id, version))
    })();
    let (pages, id, version) = match header {
        Ok(header) => header,
        Err(e) => return classify(&e),
    };
    if pages == 0 {
        return Opened::Attached { fresh: true };
    }
    if id != APPLICATION_ID {
        return Opened::Unusable(format!(
            "記憶のファイルの application_id が {id} で、areka の印 {APPLICATION_ID} と違います（areka のファイルでない）"
        ));
    }
    let current = current_version(steps);
    if !(1..=current).contains(&version) {
        return Opened::Unusable(format!(
            "記憶のファイルの形の版が {version} で、この areka が読める 1〜{current} の外です（新しい areka で書いたファイルなど）"
        ));
    }
    if version == current {
        return Opened::Attached { fresh: false };
    }
    if readonly {
        return Opened::NeedsUpgrade;
    }
    upgrade(conn, alias, steps)
}

pub(crate) fn current_version(steps: &[Step]) -> i32 {
    i32::try_from(steps.len()).map_or(i32::MAX, |n| n.saturating_add(1))
}

/// 直しの段を 1 つのトランザクションで順に当てる。
fn upgrade(conn: &Connection, alias: &str, steps: &[Step]) -> Opened {
    let current = current_version(steps);
    let run = || -> rusqlite::Result<i32> {
        conn.execute_batch("BEGIN")?;
        let from: i32 =
            conn.query_row(&format!("PRAGMA {alias}.user_version"), [], |r| r.get(0))?;
        if from >= current {
            conn.execute_batch("COMMIT")?;
            return Ok(from);
        }
        let first = usize::try_from(from.max(1) - 1).unwrap_or(0);
        for step in steps.iter().skip(first) {
            step(conn, alias)?;
        }
        conn.execute_batch(&format!("PRAGMA {alias}.user_version = {current}; COMMIT"))?;
        Ok(from)
    };
    match run() {
        Ok(from) if from > current => Opened::Unusable(format!(
            "記憶のファイルの形の版が {from} で、この areka が読める 1〜{current} の外です（新しい areka で書いたファイルなど）"
        )),
        Ok(from) => {
            if from < current {
                tracing::info!(
                    target: LOG_TARGET,
                    event = "persist_schema_upgraded",
                    from,
                    to = current,
                    "persist layer file upgraded to the current schema version"
                );
            }
            Opened::Attached { fresh: false }
        }
        Err(e) => {
            if !conn.is_autocommit()
                && let Err(rollback) = conn.execute_batch("ROLLBACK")
            {
                tracing::error!(
                    target: LOG_TARGET,
                    error = %rollback,
                    "persist schema upgrade rollback failed"
                );
            }
            match classify(&e) {
                Opened::Busy => Opened::Busy,
                _ => Opened::Io(format!("記憶のファイルを今の形の版へ直せませんでした: {e}")),
            }
        }
    }
}

/// 付けた層の隣の `sylphya.toml`（`legacy`・無ければ `None`）が使えなければ、書きを断る形にする。
///
/// 使えない新しいファイルの隣の `sylphya.toml` は取り込まないので、ここでも見ない。
pub(crate) fn with_legacy(opened: Opened, legacy: Option<&str>) -> Opened {
    match (opened, legacy) {
        (opened @ (Opened::Missing | Opened::Attached { .. }), Some(content)) => {
            match format::parse_toml_str(content) {
                Ok(_) => opened,
                Err(reason) => Opened::LegacyUnusable {
                    attached: opened != Opened::Missing,
                    reason,
                },
            }
        }
        (opened, _) => opened,
    }
}

fn classify(e: &rusqlite::Error) -> Opened {
    let rusqlite::Error::SqliteFailure(failure, _) = e else {
        return Opened::Io(e.to_string());
    };
    match failure.code {
        ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => Opened::Busy,
        ErrorCode::ReadOnly if failure.extended_code == ffi::SQLITE_READONLY_ROLLBACK => {
            Opened::NeedsRecovery
        }
        ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt => Opened::Unusable(format!(
            "記憶のファイルが壊れているか、SQLite のファイルではありません: {e}"
        )),
        _ => Opened::Io(e.to_string()),
    }
}

pub(crate) fn detach(conn: &Connection, alias: &str) {
    let attached = conn
        .query_row(
            "SELECT 1 FROM pragma_database_list WHERE name = ?1",
            [alias],
            |_| Ok(()),
        )
        .is_ok();
    if attached && let Err(e) = conn.execute("DETACH ?1", [alias]) {
        tracing::error!(
            target: LOG_TARGET,
            alias,
            error = %e,
            "persist layer detach failed"
        );
    }
}

/// 読みを飛ばす結果を、置き場ごとに 1 回だけ記録する。
fn report(root: &Path, reported: &mut BTreeSet<PathBuf>, opened: Opened) -> Opened {
    let mut first = || reported.insert(root.to_path_buf());
    match &opened {
        Opened::NeedsUpgrade if first() => tracing::error!(
            target: LOG_TARGET,
            event = "persist_readonly_needs_upgrade",
            root = %root.display(),
            "persist layer has an old schema version; a read-only run does not upgrade it, skipping this layer"
        ),
        Opened::NeedsRecovery if first() => tracing::error!(
            target: LOG_TARGET,
            event = "persist_readonly_needs_recovery",
            root = %root.display(),
            "persist layer has a hot journal; a read-only run cannot roll it back, skipping this layer"
        ),
        Opened::Unusable(reason) | Opened::Io(reason) if first() => tracing::warn!(
            target: LOG_TARGET,
            root = %root.display(),
            reason = %reason,
            "persist layer file unusable; skipping this layer for reads (not overwriting)"
        ),
        _ => {}
    }
    opened
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
