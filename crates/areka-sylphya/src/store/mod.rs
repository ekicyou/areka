//! 記憶の sqlite の住み場所（置き場の口・層を開いたときの検査・重ね読み）。

mod backend;
mod import;
mod locate;
mod read;
mod schema;
mod write;

pub(crate) use backend::forget_held;
pub use backend::{
    Access, FsBackend, Located, MemBackend, StoreBackend, UNUSABLE_APPLICATION_ID, WriterHold,
    fs_backend,
};
pub(crate) use write::{FileWrite, WriteAttempt};

use crate::persist::ScopeRoots;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const LOG_TARGET: &str = "areka_sylphya::store";

/// 1 回の操作の待ちの期限。
pub(crate) const OP_WAIT: Duration = Duration::from_secs(1);
/// 錠に当たったときの取り直しの間。
pub(crate) const RETRY_STEP: Duration = Duration::from_millis(10);
/// 錠を待ちきれずに書くのをやめたときの告知の文。
pub(crate) const WAIT_LIMIT: &str = "記憶のファイル sylphya.sqlite の錠を 1 秒待っても取れないので、書くのをやめました（ほかの areka が同じ記憶を書いている）";

/// 接続を 1 本持ち、層のファイルを操作の間だけ付けて読み書きする同期の核。
pub(crate) struct Store {
    conn: Connection,
    roots: ScopeRoots,
    backend: Arc<dyn StoreBackend>,
    /// 読みを飛ばす記録を出した置き場（置き場ごとに 1 回）。
    reported: BTreeSet<PathBuf>,
    /// 取り込みを見終えた置き場。
    imports: BTreeMap<PathBuf, import::Import>,
    /// 取り込みが錠に当たった置き場の、読んだ（鍵, 値）（重ねの値）。
    pending: BTreeMap<PathBuf, BTreeMap<String, String>>,
    /// 控えの DB を付けたか（控える範囲の中の層を初めて扱った時点で付け、以後は付けたまま）。
    held: bool,
}

impl Store {
    pub(crate) fn open(roots: ScopeRoots, backend: Arc<dyn StoreBackend>) -> Result<Store, String> {
        let setup = || -> rusqlite::Result<Connection> {
            let conn = schema::open_connection()?;
            conn.execute_batch(
                "CREATE TABLE main.vol (layer INTEGER, asker TEXT, key TEXT, value TEXT NOT NULL, PRIMARY KEY (layer, asker, key)) WITHOUT ROWID;
                 CREATE TABLE main.flat (asker TEXT, name TEXT, value TEXT NOT NULL, PRIMARY KEY (asker, name)) WITHOUT ROWID;",
            )?;
            // 別の核がアプリの層の揮発へ書いている間は、付けるのも表を作るのも錠に当たるので、期限まで取り直す。
            let start = backend.now();
            let mut attached = false;
            loop {
                let mut step = || -> rusqlite::Result<()> {
                    if !attached {
                        conn.execute(
                            "ATTACH ?1 AS appvol",
                            [backend.volatile_uri(roots.app.as_deref())],
                        )?;
                        attached = true;
                    }
                    conn.execute_batch(
                        "CREATE TABLE IF NOT EXISTS appvol.kv (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID",
                    )
                };
                match step() {
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
            imports: BTreeMap::new(),
            pending: BTreeMap::new(),
            held: false,
        })
    }

    pub(crate) fn roots(&self) -> &ScopeRoots {
        &self.roots
    }

    /// 控えの DB を付ける（付けていれば何もしない）。
    fn ensure_held(&mut self) -> rusqlite::Result<()> {
        if !self.held {
            self.conn
                .execute("ATTACH ?1 AS held", [self.backend.held_uri()])?;
            self.held = true;
        }
        Ok(())
    }
}

/// 控えの表で置き場を指す文字。
fn held_root(root: &std::path::Path) -> String {
    root.to_string_lossy().into_owned()
}

#[cfg(test)]
#[path = "withhold_tests.rs"]
mod withhold_tests;
