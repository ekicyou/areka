//! 重ね読み: 層の組と住み場所ごとに SQL を組み、層のファイルは読みの間だけ付ける。

use super::schema::{self, Opened};
use super::{LOG_TARGET, OP_WAIT, RETRY_STEP, Store};
use crate::asker::AskerId;
use crate::key::parse_dotted;
use crate::keytable::Home;
use crate::persist::PersistScope;
use rusqlite::{ErrorCode, OptionalExtension};

/// 重ね読みの順（バルーン → シェル → ゴースト → アプリ）。
const OVERLAY: [PersistScope; 4] = [
    PersistScope::Balloon,
    PersistScope::Shell,
    PersistScope::Ghost,
    PersistScope::App,
];

/// 重ね読みの順の番号（主の DB の `vol.layer` もこの番号）。
pub(crate) fn layer_order(layer: PersistScope) -> i64 {
    match layer {
        PersistScope::Balloon => 0,
        PersistScope::Shell => 1,
        PersistScope::Ghost => 2,
        PersistScope::App => 3,
    }
}

/// 層のファイルを付ける DB の名前。
fn alias(layer: PersistScope) -> &'static str {
    match layer {
        PersistScope::Balloon => "balloon",
        PersistScope::Shell => "shell",
        PersistScope::Ghost => "ghost",
        PersistScope::App => "app",
    }
}

/// 錠に当たった（巻き戻して外した）。
struct Busy;

fn sql_error(e: rusqlite::Error, what: &str) -> Result<(), Busy> {
    if let rusqlite::Error::SqliteFailure(failure, _) = &e
        && matches!(
            failure.code,
            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
        )
    {
        return Err(Busy);
    }
    tracing::error!(target: LOG_TARGET, what, error = %e, "persist read failed; reading no value");
    Ok(())
}

impl Store {
    /// `layer` が `None` なら重ね読み。`home` は鍵の表が決めた住み場所（`File` か `Memory`）。
    pub(crate) fn read(
        &mut self,
        asker: &AskerId,
        key: &str,
        layer: Option<PersistScope>,
        home: Home,
    ) -> Option<String> {
        let canonical = match parse_dotted(key) {
            Ok(path) => path.to_canonical_string(),
            Err(e) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    key,
                    error = %e,
                    "persist read of an unresolvable name; reading no value"
                );
                return None;
            }
        };
        let layers: &[PersistScope] = match &layer {
            Some(one) => std::slice::from_ref(one),
            None => &OVERLAY,
        };
        match home {
            Home::Fast => None,
            Home::Memory => self
                .retry(&canonical, |s| s.read_memory(asker, &canonical, layers))
                .flatten(),
            Home::File => self
                .retry(&canonical, |s| s.read_file(&canonical, layers))
                .flatten(),
        }
    }

    /// 1 つの層のファイルの全部の（鍵, 値）（鍵の順）。
    pub(crate) fn read_layer(&mut self, layer: PersistScope) -> Vec<(String, String)> {
        self.retry(alias(layer), |s| s.read_whole(layer))
            .unwrap_or_default()
    }

    /// 錠に当たったら置き場の口の時計で取り直し、期限を越えたら error を残して `None`。
    fn retry<T>(
        &mut self,
        what: &str,
        mut attempt: impl FnMut(&mut Self) -> Result<T, Busy>,
    ) -> Option<T> {
        let start = self.backend.now();
        loop {
            if let Ok(found) = attempt(self) {
                return Some(found);
            }
            if self.backend.now().saturating_sub(start) > OP_WAIT {
                tracing::error!(
                    target: LOG_TARGET,
                    event = "persist_read_busy",
                    what,
                    "persist read kept meeting a lock past the wait limit; reading no value"
                );
                return None;
            }
            self.backend.pause(RETRY_STEP);
        }
    }

    /// `layers` のうち置き場とファイルの在る層を付ける。返すのは（表の在る層, 付けた全部）。
    fn attach(
        &mut self,
        layers: &[PersistScope],
    ) -> Result<(Vec<PersistScope>, Vec<PersistScope>), Busy> {
        let mut with_table = Vec::new();
        let mut attached = Vec::new();
        for &layer in layers {
            let Some(root) = self.roots.root_of(layer).cloned() else {
                continue;
            };
            match schema::open_layer(
                &self.conn,
                self.backend.as_ref(),
                &root,
                alias(layer),
                &mut self.reported,
            ) {
                Opened::Attached { fresh } => {
                    attached.push(layer);
                    if !fresh {
                        with_table.push(layer);
                    }
                }
                Opened::Busy => {
                    self.detach(&attached);
                    return Err(Busy);
                }
                _ => {}
            }
        }
        Ok((with_table, attached))
    }

    fn detach(&self, layers: &[PersistScope]) {
        for &layer in layers {
            schema::detach(&self.conn, alias(layer));
        }
    }

    fn read_file(&mut self, key: &str, layers: &[PersistScope]) -> Result<Option<String>, Busy> {
        let (with_table, attached) = self.attach(layers)?;
        let found = if with_table.is_empty() {
            Ok(None)
        } else {
            let sql = with_table
                .iter()
                .map(|&l| {
                    format!(
                        "SELECT {} AS o, value FROM {}.kv WHERE key = ?1",
                        layer_order(l),
                        alias(l)
                    )
                })
                .collect::<Vec<_>>()
                .join(" UNION ALL ");
            let sql = format!("SELECT value FROM ({sql}) ORDER BY o LIMIT 1");
            match self
                .conn
                .prepare_cached(&sql)
                .and_then(|mut stmt| stmt.query_row([key], |r| r.get(0)).optional())
            {
                Ok(found) => Ok(found),
                Err(e) => sql_error(e, key).map(|()| None),
            }
        };
        self.detach(&attached);
        found
    }

    fn read_memory(
        &mut self,
        asker: &AskerId,
        key: &str,
        layers: &[PersistScope],
    ) -> Result<Option<String>, Busy> {
        let parts: Vec<String> = layers
            .iter()
            .map(|&l| match l {
                PersistScope::App => "SELECT 3 AS o, value FROM appvol.kv WHERE key = ?1".to_owned(),
                _ => format!(
                    "SELECT layer AS o, value FROM main.vol WHERE layer = {} AND asker = ?2 AND key = ?1",
                    layer_order(l)
                ),
            })
            .collect();
        let sql = format!(
            "SELECT value FROM ({}) ORDER BY o LIMIT 1",
            parts.join(" UNION ALL ")
        );
        let uses_asker = layers.iter().any(|&l| l != PersistScope::App);
        let result = self.conn.prepare_cached(&sql).and_then(|mut stmt| {
            if uses_asker {
                stmt.query_row([key, asker.as_str()], |r| r.get(0))
            } else {
                stmt.query_row([key], |r| r.get(0))
            }
            .optional()
        });
        match result {
            Ok(found) => Ok(found),
            Err(e) => sql_error(e, key).map(|()| None),
        }
    }

    fn read_whole(&mut self, layer: PersistScope) -> Result<Vec<(String, String)>, Busy> {
        let (with_table, attached) = self.attach(std::slice::from_ref(&layer))?;
        let found = if with_table.is_empty() {
            Ok(Vec::new())
        } else {
            let sql = format!("SELECT key, value FROM {}.kv ORDER BY key", alias(layer));
            match self.conn.prepare_cached(&sql).and_then(|mut stmt| {
                stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()
            }) {
                Ok(pairs) => Ok(pairs),
                Err(e) => sql_error(e, alias(layer)).map(|()| Vec::new()),
            }
        };
        self.detach(&attached);
        found
    }
}

#[cfg(test)]
#[path = "layered_read_tests.rs"]
mod tests;
