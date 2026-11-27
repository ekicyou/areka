//! 重ね読み: 層の組と住み場所ごとに SQL を組み、層のファイルは読みの間だけ付ける。

use super::schema::{self, Opened};
use super::{LOG_TARGET, OP_WAIT, RETRY_STEP, Store, held_root};
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
pub(super) fn alias(layer: PersistScope) -> &'static str {
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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the typed wrappers move onto the store later")
    )]
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
            match self.open_imported(layer, &root, false) {
                Opened::Attached { fresh: false }
                | Opened::LegacyUnusable { attached: true, .. } => {
                    attached.push(layer);
                    with_table.push(layer);
                }
                Opened::Attached { .. } => attached.push(layer),
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

    /// 控える範囲の中の層の置き場（控えの DB を付けた後）。付けられなければ記録して `None`。
    fn held_root_of(&mut self, layer: PersistScope) -> Option<String> {
        let root = self.roots.root_of(layer)?.clone();
        if !self.backend.withholds(&root) {
            return None;
        }
        match self.ensure_held() {
            Ok(()) => Some(held_root(&root)),
            Err(e) => {
                tracing::error!(target: LOG_TARGET, error = %e, "held db could not be attached; reading the files only");
                None
            }
        }
    }

    fn read_file(&mut self, key: &str, layers: &[PersistScope]) -> Result<Option<String>, Busy> {
        let (with_table, attached) = self.attach(layers)?;
        let mut params = vec![key.to_owned()];
        let mut parts = Vec::new();
        let mut held_roots = Vec::new();
        for &l in layers {
            let (o, a) = (layer_order(l), alias(l));
            let file = with_table.contains(&l);
            let held = self.held_root_of(l);
            if let Some(root) = &held {
                params.push(root.clone());
                let n = params.len();
                parts.push(format!(
                    "SELECT {o} AS o, value FROM held.held WHERE root = ?{n} AND key = ?1 AND value IS NOT NULL"
                ));
                if file {
                    parts.push(format!(
                        "SELECT {o} AS o, value FROM {a}.kv WHERE key = ?1 AND NOT EXISTS (SELECT 1 FROM held.held WHERE root = ?{n} AND key = ?1)"
                    ));
                }
            } else if file {
                parts.push(format!("SELECT {o} AS o, value FROM {a}.kv WHERE key = ?1"));
            }
            held_roots.push(held);
        }
        let found = if parts.is_empty() {
            Ok(None)
        } else {
            let sql = format!(
                "SELECT o, value FROM ({}) ORDER BY o LIMIT 1",
                parts.join(" UNION ALL ")
            );
            match self.conn.prepare_cached(&sql).and_then(|mut stmt| {
                stmt.query_row(rusqlite::params_from_iter(&params), |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .optional()
            }) {
                Ok(found) => Ok(found),
                Err(e) => sql_error(e, key).map(|()| None),
            }
        };
        self.detach(&attached);
        let found: Option<(i64, String)> = found?;
        for (&l, held) in layers.iter().zip(&held_roots) {
            if found.as_ref().is_some_and(|(o, _)| layer_order(l) >= *o) {
                break;
            }
            if let Some(root) = held
                && self.has_held_row(root, key)?
            {
                continue;
            }
            if let Some(value) = self.pending_value(l, key) {
                return Ok(Some(value.clone()));
            }
        }
        Ok(found.map(|(_, value)| value))
    }

    /// 控えの DB にこの鍵の行（消した印も含む）が在るか。
    fn has_held_row(&self, root: &str, key: &str) -> Result<bool, Busy> {
        match self
            .conn
            .prepare_cached("SELECT 1 FROM held.held WHERE root = ?1 AND key = ?2")
            .and_then(|mut stmt| stmt.exists([root, key]))
        {
            Ok(found) => Ok(found),
            Err(e) => sql_error(e, key).map(|()| false),
        }
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

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the typed wrappers move onto the store later")
    )]
    fn read_whole(&mut self, layer: PersistScope) -> Result<Vec<(String, String)>, Busy> {
        let (with_table, attached) = self.attach(std::slice::from_ref(&layer))?;
        let found = if with_table.is_empty() {
            Ok(Vec::new())
        } else {
            let sql = format!("SELECT key, value FROM {}.kv", alias(layer));
            match self.conn.prepare_cached(&sql).and_then(|mut stmt| {
                stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()
            }) {
                Ok(pairs) => Ok(pairs),
                Err(e) => sql_error(e, alias(layer)).map(|()| Vec::new()),
            }
        };
        self.detach(&attached);
        let mut whole: std::collections::BTreeMap<String, String> = found?.into_iter().collect();
        let mut held_keys = std::collections::BTreeSet::new();
        if let Some(root) = self.held_root_of(layer) {
            let held = self
                .conn
                .prepare_cached("SELECT key, value FROM held.held WHERE root = ?1 AND key <> ''")
                .and_then(|mut stmt| {
                    stmt.query_map([&root], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .collect::<rusqlite::Result<Vec<(String, Option<String>)>>>()
                });
            let held = match held {
                Ok(rows) => rows,
                Err(e) => {
                    sql_error(e, alias(layer))?;
                    Vec::new()
                }
            };
            for (key, value) in held {
                held_keys.insert(key.clone());
                match value {
                    Some(value) => whole.insert(key, value),
                    None => whole.remove(&key),
                };
            }
        }
        for (key, value) in self.pending_layer(layer).into_iter().flatten() {
            if held_keys.contains(key) {
                continue;
            }
            whole.entry(key.clone()).or_insert_with(|| value.clone());
        }
        Ok(whole.into_iter().collect())
    }
}

#[cfg(test)]
#[path = "layered_read_tests.rs"]
mod tests;
