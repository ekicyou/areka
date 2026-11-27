//! 1 層 1 トランザクションの書きの試み・揮発の書き・平らな名前・層の付け替え。

use super::read::{alias, layer_order};
use super::schema::{self, APPLICATION_ID, Opened};
use super::{LOG_TARGET, OP_WAIT, RETRY_STEP, Store};
use crate::asker::AskerId;
use crate::persist::{
    PersistFailure, PersistFailureCause, PersistKey, PersistOutcome, PersistScope, ScopeRoots,
};
use rusqlite::{Connection, ErrorCode, OptionalExtension};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 1 回の覚える値の書き（層は 1 つ）。`typed` は失敗の中身に載せる型つきの別名。
#[derive(Clone, Debug)]
pub(crate) struct FileWrite {
    pub layer: PersistScope,
    /// 受けた時点の層の置き場（`None` は置き場の無い層）。
    pub root: Option<PathBuf>,
    pub puts: Vec<(String, String)>,
    pub removes: Vec<String>,
    pub typed: Vec<PersistKey>,
}

/// 書きの試みの結果。`Busy` は錠に当たって巻き戻した（層のファイルは変わっていない）。
#[derive(Debug)]
pub(crate) enum WriteAttempt {
    Done(PersistOutcome),
    Busy,
}

fn is_busy(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(f, _)
            if matches!(f.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

/// 書きの中の失敗（錠に当たった・それ以外の理由）。
enum Stop {
    Busy,
    Failed(String),
}

impl From<rusqlite::Error> for Stop {
    fn from(e: rusqlite::Error) -> Self {
        if is_busy(&e) {
            Stop::Busy
        } else {
            Stop::Failed(e.to_string())
        }
    }
}

impl FileWrite {
    fn key_names(&self) -> Vec<&str> {
        self.puts
            .iter()
            .map(|(k, _)| k.as_str())
            .chain(self.removes.iter().map(String::as_str))
            .collect()
    }

    fn degraded(&self, root: Option<&Path>, cause: PersistFailureCause) -> WriteAttempt {
        WriteAttempt::Done(PersistOutcome::Degraded(PersistFailure {
            scope: self.layer,
            dir: root.map(Path::to_path_buf),
            keys: self.typed.clone(),
            cause,
        }))
    }
}

impl Store {
    /// 受けた時点の置き場の層のファイルだけを付け、1 回のトランザクションで書く（待たない）。
    ///
    /// どの段で錠に当たっても巻き戻して外し `Busy` を返す。戻った時点で層を 1 つも付けていない。
    pub(crate) fn try_write(&mut self, write: &FileWrite) -> WriteAttempt {
        let layer = write.layer;
        let Some(root) = write.root.as_deref() else {
            tracing::warn!(
                target: LOG_TARGET,
                scope = ?layer,
                "persist scope root absent; save degraded (no destination)"
            );
            return write.degraded(None, PersistFailureCause::NoRoot);
        };
        let name = alias(layer);
        let backend = self.backend.clone();
        let mut opened =
            schema::open_layer(&self.conn, backend.as_ref(), root, name, &mut self.reported);
        if opened == Opened::Missing {
            if write.puts.is_empty() {
                return WriteAttempt::Done(PersistOutcome::Saved);
            }
            opened = match backend.ensure_dir(root) {
                Ok(()) => schema::create_layer(
                    &self.conn,
                    backend.as_ref(),
                    root,
                    name,
                    &mut self.reported,
                ),
                Err(e) => Opened::Io(e.to_string()),
            };
        }
        let attempt = match opened {
            Opened::Attached { fresh } if fresh && write.puts.is_empty() => {
                WriteAttempt::Done(PersistOutcome::Saved)
            }
            Opened::Attached { fresh } => match self.write_attached(write, root, fresh) {
                Ok(()) => WriteAttempt::Done(PersistOutcome::Saved),
                Err(Stop::Busy) => WriteAttempt::Busy,
                Err(Stop::Failed(reason)) => {
                    tracing::error!(
                        target: LOG_TARGET,
                        scope = ?layer,
                        path = %root.display(),
                        error = %reason,
                        keys = ?write.key_names(),
                        "persist commit failed; existing file intact (rolled back), reporting Degraded"
                    );
                    write.degraded(Some(root), PersistFailureCause::Io(reason))
                }
            },
            Opened::Busy => WriteAttempt::Busy,
            Opened::Unusable(reason) | Opened::LegacyUnusable { reason, .. } => {
                tracing::error!(
                    target: LOG_TARGET,
                    event = "persist_existing_unusable",
                    scope = ?layer,
                    path = %root.display(),
                    reason = %reason,
                    keys = ?write.key_names(),
                    "persist existing file unusable; not overwriting, reporting Degraded"
                );
                write.degraded(Some(root), PersistFailureCause::Unusable(reason))
            }
            other => {
                let reason = match other {
                    Opened::Io(reason) => reason,
                    other => format!("{other:?}"),
                };
                tracing::error!(
                    target: LOG_TARGET,
                    scope = ?layer,
                    path = %root.display(),
                    error = %reason,
                    keys = ?write.key_names(),
                    "persist layer could not be opened for writing; reporting Degraded"
                );
                write.degraded(Some(root), PersistFailureCause::Io(reason))
            }
        };
        schema::detach(&self.conn, name);
        attempt
    }

    /// 付けた層へ、遅らせる `BEGIN` → 鍵ごとの更新と消し → 確定。失敗したら巻き戻す。
    fn write_attached(&self, write: &FileWrite, root: &Path, fresh: bool) -> Result<(), Stop> {
        let result = run_transaction(&self.conn, write, alias(write.layer), fresh, || {
            self.backend
                .before_write(root)
                .map_err(|e| Stop::Failed(e.to_string()))
        });
        if result.is_err()
            && !self.conn.is_autocommit()
            && let Err(e) = self.conn.execute_batch("ROLLBACK")
        {
            tracing::error!(target: LOG_TARGET, error = %e, "persist write rollback failed");
        }
        result
    }

    /// 揮発の値を書く（`None` は消す）。アプリの層はどの問い手からも見える。
    pub(crate) fn write_volatile(
        &mut self,
        layer: PersistScope,
        asker: &AskerId,
        key: &str,
        value: Option<&str>,
    ) -> Result<(), String> {
        let order = layer_order(layer);
        self.retry_volatile(key, |conn| {
            match (layer, value) {
                (PersistScope::App, Some(v)) => conn.execute(
                    "INSERT INTO appvol.kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    [key, v],
                ),
                (PersistScope::App, None) => {
                    conn.execute("DELETE FROM appvol.kv WHERE key = ?1", [key])
                }
                (_, Some(v)) => conn.execute(
                    "INSERT INTO main.vol (layer, asker, key, value) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(layer, asker, key) DO UPDATE SET value = excluded.value",
                    rusqlite::params![order, asker.as_str(), key, v],
                ),
                (_, None) => conn.execute(
                    "DELETE FROM main.vol WHERE layer = ?1 AND asker = ?2 AND key = ?3",
                    rusqlite::params![order, asker.as_str(), key],
                ),
            }
            .map(|_| ())
        })
    }

    /// 平らな名前を書く（`None` は消す）。返すのは、行が変わったか。
    pub(crate) fn write_flat(
        &mut self,
        asker: &AskerId,
        name: &str,
        value: Option<&str>,
    ) -> Result<bool, String> {
        let changed = match value {
            Some(v) => self.conn.execute(
                "INSERT INTO main.flat (asker, name, value) VALUES (?1, ?2, ?3) ON CONFLICT(asker, name) DO UPDATE SET value = excluded.value",
                [asker.as_str(), name, v],
            ),
            None => self.conn.execute(
                "DELETE FROM main.flat WHERE asker = ?1 AND name = ?2",
                [asker.as_str(), name],
            ),
        };
        changed.map(|n| n > 0).map_err(|e| {
            tracing::error!(target: LOG_TARGET, name, error = %e, "flat name write failed");
            e.to_string()
        })
    }

    pub(crate) fn read_flat(&mut self, asker: &AskerId, name: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM main.flat WHERE asker = ?1 AND name = ?2",
                [asker.as_str(), name],
                |r| r.get(0),
            )
            .optional()
            .unwrap_or_else(|e| {
                tracing::error!(target: LOG_TARGET, name, error = %e, "flat name read failed; reading no value");
                None
            })
    }

    /// 問い手の平らな名前の全部（名前の順）。
    pub(crate) fn flat_snapshot(&mut self, asker: &AskerId) -> BTreeMap<String, String> {
        let pairs = self
            .conn
            .prepare_cached("SELECT name, value FROM main.flat WHERE asker = ?1")
            .and_then(|mut stmt| {
                stmt.query_map([asker.as_str()], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<rusqlite::Result<BTreeMap<_, _>>>()
            });
        pairs.unwrap_or_else(|e| {
            tracing::error!(target: LOG_TARGET, error = %e, "flat name snapshot failed; reading none");
            BTreeMap::new()
        })
    }

    /// 層の置き場を替え、その層の揮発の値を捨てる。
    pub(crate) fn set_root(&mut self, layer: PersistScope, root: Option<PathBuf>) {
        if layer != PersistScope::App
            && let Err(e) = self.conn.execute(
                "DELETE FROM main.vol WHERE layer = ?1",
                [layer_order(layer)],
            )
        {
            tracing::error!(
                target: LOG_TARGET,
                scope = ?layer,
                error = %e,
                "volatile values of the re-rooted layer could not be discarded"
            );
        }
        *root_slot(&mut self.roots, layer) = root;
    }

    /// 揮発の DB の書きが錠に当たったら、置き場の口の時計で期限まで取り直す。
    fn retry_volatile(
        &self,
        key: &str,
        attempt: impl Fn(&Connection) -> rusqlite::Result<()>,
    ) -> Result<(), String> {
        let start = self.backend.now();
        loop {
            match attempt(&self.conn) {
                Ok(()) => return Ok(()),
                Err(e) if is_busy(&e) && self.backend.now().saturating_sub(start) <= OP_WAIT => {
                    self.backend.pause(RETRY_STEP);
                }
                Err(e) => {
                    tracing::error!(target: LOG_TARGET, key, error = %e, "volatile value write failed");
                    return Err(e.to_string());
                }
            }
        }
    }
}

fn run_transaction(
    conn: &Connection,
    write: &FileWrite,
    name: &str,
    fresh: bool,
    before_commit: impl FnOnce() -> Result<(), Stop>,
) -> Result<(), Stop> {
    conn.execute_batch("BEGIN")?;
    if fresh {
        conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {name}.kv (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS {name}.meta (name TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
             PRAGMA {name}.application_id = {APPLICATION_ID};
             PRAGMA {name}.user_version = {};",
            schema::current_version(schema::STEPS)
        ))?;
    }
    let mut upsert = conn.prepare_cached(&format!(
        "INSERT INTO {name}.kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    ))?;
    for (key, value) in &write.puts {
        upsert.execute([key, value])?;
    }
    let mut delete = conn.prepare_cached(&format!("DELETE FROM {name}.kv WHERE key = ?1"))?;
    for key in &write.removes {
        delete.execute([key])?;
    }
    before_commit()?;
    conn.execute_batch("COMMIT")?;
    Ok(())
}

fn root_slot(roots: &mut ScopeRoots, layer: PersistScope) -> &mut Option<PathBuf> {
    match layer {
        PersistScope::App => &mut roots.app,
        PersistScope::Ghost => &mut roots.ghost,
        PersistScope::Shell => &mut roots.shell,
        PersistScope::Balloon => &mut roots.balloon,
    }
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "writers_tests.rs"]
mod writers_tests;

#[cfg(test)]
#[path = "table_rows_tests.rs"]
mod table_rows_tests;
