//! 記憶の sqlite の住み場所（置き場の口・層を開いたときの検査・重ね読み）。
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the guardian and the direct KVS that call the store core land later"
    )
)]

mod backend;
mod locate;
mod read;
mod schema;
mod write;

pub use backend::{
    Access, FsBackend, Located, MemBackend, StoreBackend, UNUSABLE_APPLICATION_ID, WriterHold,
    fs_backend,
};

use crate::persist::ScopeRoots;
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const LOG_TARGET: &str = "areka_sylphya::store";

/// 1 回の操作の待ちの期限。
pub(crate) const OP_WAIT: Duration = Duration::from_secs(1);
/// 錠に当たったときの取り直しの間。
pub(crate) const RETRY_STEP: Duration = Duration::from_millis(10);

/// 接続を 1 本持ち、層のファイルを操作の間だけ付けて読み書きする同期の核。
pub(crate) struct Store {
    conn: Connection,
    roots: ScopeRoots,
    backend: Arc<dyn StoreBackend>,
    /// 読みを飛ばす記録を出した置き場（置き場ごとに 1 回）。
    reported: BTreeSet<PathBuf>,
}

impl Store {
    pub(crate) fn open(roots: ScopeRoots, backend: Arc<dyn StoreBackend>) -> Result<Store, String> {
        let setup = || -> rusqlite::Result<Connection> {
            let conn = schema::open_connection()?;
            conn.execute_batch(
                "CREATE TABLE main.vol (layer INTEGER, asker TEXT, key TEXT, value TEXT NOT NULL, PRIMARY KEY (layer, asker, key)) WITHOUT ROWID;
                 CREATE TABLE main.flat (asker TEXT, name TEXT, value TEXT NOT NULL, PRIMARY KEY (asker, name)) WITHOUT ROWID;",
            )?;
            conn.execute(
                "ATTACH ?1 AS appvol",
                [backend.volatile_uri(roots.app.as_deref())],
            )?;
            // 別の核がアプリの層の揮発へ書いている間は錠に当たるので、期限まで取り直す。
            let start = backend.now();
            loop {
                match conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS appvol.kv (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID",
                ) {
                    Err(rusqlite::Error::SqliteFailure(f, _))
                        if f.code == rusqlite::ErrorCode::DatabaseBusy
                            && backend.now().saturating_sub(start) <= OP_WAIT =>
                    {
                        backend.pause(RETRY_STEP);
                    }
                    result => break result.map(|()| conn),
                }
            }
        };
        let conn = setup().map_err(|e| {
            tracing::error!(target: LOG_TARGET, error = %e, "persist store open failed");
            e.to_string()
        })?;
        Ok(Store {
            conn,
            roots,
            backend,
            reported: BTreeSet::new(),
        })
    }
}
