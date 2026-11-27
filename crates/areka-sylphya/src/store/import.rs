//! 層の置き場の `sylphya.toml` を、その層を初めて使うときに一度だけ取り込む。

use super::read::alias;
use super::schema::{self, Opened};
use super::write::{Stop, create_tables};
use super::{LOG_TARGET, Store};
use crate::persist::{PersistScope, doc_to_entries, format};
use rusqlite::OptionalExtension;
use std::collections::BTreeMap;
use std::path::Path;

/// 層のファイルの `meta` に置く、取り込み済みの印の名前。
pub(crate) const IMPORTED_MARK: &str = "imported-sylphya-toml";

/// 置き場ごとの取り込みの覚え（`Store` の中だけ・`set_root` で戻る）。
#[derive(Clone, Debug)]
pub(crate) enum Import {
    /// 取り込んだ、取り込み済みだった、または古いファイルが無かった。
    Done,
    /// 古いファイルが使えない（理由）。
    Failed(String),
}

impl Store {
    /// 層を付け、まだ見ていない置き場なら隣の `sylphya.toml` を取り込む。
    ///
    /// `write` は書きの直前の呼び（取り込みが錠に当たれば `Busy`、ほかの失敗は `Io`）。読みの
    /// 呼びでは、取り込めなかった値を重ねの値として持ち、付けた層をそのまま返す。
    /// 付けたままで返すのは `Attached` と `LegacyUnusable { attached: true }` だけ。
    pub(super) fn open_imported(
        &mut self,
        layer: PersistScope,
        root: &Path,
        write: bool,
    ) -> Opened {
        let name = alias(layer);
        let opened = self.import(layer, root, name, write);
        if !matches!(
            opened,
            Opened::Attached { .. } | Opened::LegacyUnusable { attached: true, .. }
        ) {
            schema::detach(&self.conn, name);
        }
        opened
    }

    fn import(&mut self, layer: PersistScope, root: &Path, name: &str, write: bool) -> Opened {
        let backend = self.backend.clone();
        let opened =
            schema::open_layer(&self.conn, backend.as_ref(), root, name, &mut self.reported);
        if !matches!(opened, Opened::Missing | Opened::Attached { .. }) {
            return opened;
        }
        match self.imports.get(root) {
            Some(Import::Done) => return opened,
            Some(Import::Failed(reason)) => return legacy_unusable(opened, reason.clone()),
            None => {}
        }
        let content = match backend.read_legacy(root) {
            Ok(Some(content)) => content,
            Ok(None) => {
                self.finish(root);
                return opened;
            }
            Err(e) => {
                return self.fail(
                    root,
                    opened,
                    format!("記憶のファイル sylphya.toml を読めませんでした: {e}"),
                );
            }
        };
        let (doc, table) = match format::parse_toml_str(&content) {
            Ok(parsed) => parsed,
            Err(reason) => return self.fail(root, opened, reason),
        };
        let entries: Vec<(String, String)> = doc_to_entries(layer, &doc)
            .into_iter()
            .map(|(key, value)| (key.to_canonical_key(), value))
            .collect();
        if backend.withholds(root) {
            if opened == (Opened::Attached { fresh: false }) {
                match self.marked(name) {
                    Ok(false) => {}
                    Ok(true) => {
                        self.finish(root);
                        return opened;
                    }
                    Err(e) => {
                        return self.postpone(root, &entries, Stop::from(e), write, opened);
                    }
                }
            }
            self.pending
                .insert(root.to_path_buf(), first_spellings(&entries));
            self.imports.insert(root.to_path_buf(), Import::Done);
            return opened;
        }
        if opened == (Opened::Attached { fresh: false }) {
            match self.marked(name) {
                Ok(false) => {}
                Ok(true) => {
                    self.finish(root);
                    self.retire_again(root);
                    return opened;
                }
                Err(e) => return self.postpone(root, &entries, Stop::from(e), write, opened),
            }
        }
        let opened = if opened == Opened::Missing {
            match backend.ensure_dir(root) {
                Ok(()) => schema::create_layer(
                    &self.conn,
                    backend.as_ref(),
                    root,
                    name,
                    &mut self.reported,
                ),
                Err(e) => Opened::Io(e.to_string()),
            }
        } else {
            opened
        };
        let Opened::Attached { fresh } = opened else {
            let stop = match opened {
                Opened::Busy => Stop::Busy,
                other => Stop::Failed(format!("{other:?}")),
            };
            return self.postpone(root, &entries, stop, write, Opened::Missing);
        };
        match self.import_transaction(name, root, fresh, &entries) {
            Ok(Some(inserted)) => {
                self.finish(root);
                tracing::info!(
                    target: LOG_TARGET,
                    event = "persist_imported",
                    root = %root.display(),
                    imported = inserted,
                    skipped = entries.len() - inserted,
                    unknown = count_unknown(&table),
                    "sylphya.toml imported into the layer file"
                );
                self.retire(root);
                Opened::Attached { fresh: false }
            }
            Ok(None) => {
                self.finish(root);
                self.retire_again(root);
                Opened::Attached { fresh: false }
            }
            Err(stop) => self.postpone(root, &entries, stop, write, opened),
        }
    }

    /// 取り込めなかった値を重ねの値として持ち、層は「まだ取り込んでいない」のまま返す。
    fn postpone(
        &mut self,
        root: &Path,
        entries: &[(String, String)],
        stop: Stop,
        write: bool,
        for_read: Opened,
    ) -> Opened {
        self.pending
            .insert(root.to_path_buf(), first_spellings(entries));
        match stop {
            Stop::Busy => {
                tracing::debug!(
                    target: LOG_TARGET,
                    root = %root.display(),
                    "sylphya.toml import met a lock; reading its values over the layer until it lands"
                );
                if write { Opened::Busy } else { for_read }
            }
            Stop::Failed(reason) => {
                tracing::error!(
                    target: LOG_TARGET,
                    event = "persist_import_failed",
                    root = %root.display(),
                    reason = %reason,
                    "sylphya.toml import could not be written; trying again on the next use"
                );
                if write { Opened::Io(reason) } else { for_read }
            }
        }
    }

    /// 値の `INSERT OR IGNORE` と印を 1 つのトランザクションで書く。印が既に在れば `None`。
    fn import_transaction(
        &self,
        name: &str,
        root: &Path,
        fresh: bool,
        entries: &[(String, String)],
    ) -> Result<Option<usize>, Stop> {
        let run = || -> Result<Option<usize>, Stop> {
            self.conn.execute_batch("BEGIN")?;
            if fresh {
                create_tables(&self.conn, name)?;
            }
            let marked = self.conn.execute(
                &format!("INSERT OR IGNORE INTO {name}.meta (name, value) VALUES (?1, '1')"),
                [IMPORTED_MARK],
            )?;
            if marked == 0 {
                self.conn.execute_batch("COMMIT")?;
                return Ok(None);
            }
            let mut insert = self.conn.prepare_cached(&format!(
                "INSERT OR IGNORE INTO {name}.kv (key, value) VALUES (?1, ?2)"
            ))?;
            let mut inserted = 0;
            for (key, value) in entries {
                inserted += insert.execute([key, value])?;
            }
            self.backend
                .before_write(root)
                .map_err(|e| Stop::Failed(e.to_string()))?;
            self.conn.execute_batch("COMMIT")?;
            Ok(Some(inserted))
        };
        let result = run();
        if result.is_err()
            && !self.conn.is_autocommit()
            && let Err(e) = self.conn.execute_batch("ROLLBACK")
        {
            tracing::error!(target: LOG_TARGET, error = %e, "persist import rollback failed");
        }
        result
    }

    fn marked(&self, name: &str) -> rusqlite::Result<bool> {
        self.conn
            .query_row(
                &format!("SELECT 1 FROM {name}.meta WHERE name = ?1"),
                [IMPORTED_MARK],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
    }

    fn finish(&mut self, root: &Path) {
        self.imports.insert(root.to_path_buf(), Import::Done);
        self.pending.remove(root);
    }

    fn fail(&mut self, root: &Path, opened: Opened, reason: String) -> Opened {
        tracing::error!(
            target: LOG_TARGET,
            event = "persist_import_failed",
            root = %root.display(),
            reason = %reason,
            "sylphya.toml unusable; not importing, layer writes refused until it is fixed"
        );
        self.pending.remove(root);
        self.imports
            .insert(root.to_path_buf(), Import::Failed(reason.clone()));
        legacy_unusable(opened, reason)
    }

    /// 取り込み済みの層にまた在る `sylphya.toml` の名前だけを替える。
    fn retire_again(&self, root: &Path) {
        tracing::info!(
            target: LOG_TARGET,
            event = "persist_import_skipped",
            root = %root.display(),
            "layer already imported; sylphya.toml only renamed, its values not imported"
        );
        self.retire(root);
    }

    fn retire(&self, root: &Path) {
        if let Err(e) = self.backend.retire_legacy(root) {
            tracing::warn!(
                target: LOG_TARGET,
                event = "persist_import_rename_failed",
                root = %root.display(),
                error = %e,
                "sylphya.toml could not be renamed after the import; continuing with the imported values"
            );
        }
    }

    /// 取り込めていない層の重ねの値（ファイルに無い鍵だけを補う）。
    pub(super) fn pending_value(&self, layer: PersistScope, key: &str) -> Option<&String> {
        let root = self.roots.root_of(layer)?;
        self.pending.get(root)?.get(key)
    }

    pub(super) fn pending_layer(&self, layer: PersistScope) -> Option<&BTreeMap<String, String>> {
        self.pending.get(self.roots.root_of(layer)?)
    }
}

fn legacy_unusable(opened: Opened, reason: String) -> Opened {
    Opened::LegacyUnusable {
        attached: opened == (Opened::Attached { fresh: false }),
        reason,
    }
}

/// 同じ鍵に読める綴りが 2 つ在れば、先の綴りの値を採る。
fn first_spellings(entries: &[(String, String)]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (key, value) in entries {
        out.entry(key.clone()).or_insert_with(|| value.clone());
    }
    out
}

/// この版が取り込まない値の数（知らない表・知らない欄・番号に読めない綴り・文字列でない値）。
fn count_unknown(table: &toml::Table) -> usize {
    table
        .iter()
        .map(|(name, value)| match (name.as_str(), value.as_table()) {
            ("format-version", _) => 0,
            ("last", Some(t)) => unknown_fields(t, &["ghost", "balloon", "shell", "running"]),
            ("baseware", Some(t)) => unknown_fields(t, &["roots"]),
            ("boot" | "vanish", Some(t)) => unknown_fields(t, &["count"]),
            ("window" | "balloon-offset", Some(t)) => t
                .iter()
                .map(|(id, entry)| match entry.as_table() {
                    Some(axes) if id.parse::<u32>().is_ok() => unknown_fields(axes, &["x", "y"]),
                    _ => leaves(entry),
                })
                .sum(),
            ("window-adjacent", Some(t)) => t
                .iter()
                .filter(|(id, v)| id.parse::<u32>().is_err() || !v.is_str())
                .map(|(_, v)| leaves(v))
                .sum(),
            _ => leaves(value),
        })
        .sum()
}

fn unknown_fields(table: &toml::Table, known: &[&str]) -> usize {
    table
        .iter()
        .filter(|(field, v)| !(known.contains(&field.as_str()) && v.is_str()))
        .map(|(_, v)| leaves(v))
        .sum()
}

fn leaves(value: &toml::Value) -> usize {
    value.as_table().map_or(1, |t| t.values().map(leaves).sum())
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
