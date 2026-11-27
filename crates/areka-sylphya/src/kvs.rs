//! KVS の口: 鍵の名前と層の名前だけで読む・書く・消す口と、書き・消しの結果。

use crate::actor::{Effect, SylphyaMsg, Writer, plan_write};
use crate::asker::AskerId;
use crate::guardian::{WAIT_LIMIT, lock_failed, store_not_open};
use crate::key::parse_dotted;
use crate::keytable::{Home, KEY_TABLE, KeyRow, rule_of};
use crate::persist::{PersistFailure, PersistKey, PersistOutcome, PersistScope, ScopeRoots};
use crate::reader::SylphyaReader;
use crate::store::{FileWrite, OP_WAIT, RETRY_STEP, Store, StoreBackend, WriteAttempt};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const LOG_TARGET: &str = "areka_sylphya::kvs";

/// 書き・消しの結果（4 つの形）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteOutcome {
    /// 書けた。
    Written,
    /// 名前では書けない鍵だったので断った。
    Refused,
    /// 書けなかった。
    Failed(WriteFailed),
    /// 読み取りだけの走りなので控えた。
    Withheld,
}

/// 書けなかった理由。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteFailed {
    /// 覚える値を書けなかった（告知の道へ渡る中身と同じ物）。
    Persist(PersistFailure),
    /// 揮発の値・速い値を書けなかった（告知の道へは渡らない）。
    Volatile { key: String, reason: String },
    /// 番人が止まっていて、返事が来なかった（告知の道へは渡らない）。
    Stopped,
}

/// 番人を通る口（[`SylphyaReader::kvs`] で得る・複製できる）。
#[derive(Clone)]
pub struct Kvs {
    reader: SylphyaReader,
    asker: AskerId,
}

impl Kvs {
    pub(crate) fn new(reader: SylphyaReader, asker: AskerId) -> Self {
        Kvs { reader, asker }
    }

    /// 層を重ねて読む。
    pub fn get(&self, key: &str) -> Option<String> {
        self.reader.get(&self.asker, key, None)
    }

    /// その層だけを読む。
    pub fn get_in(&self, layer: PersistScope, key: &str) -> Option<String> {
        self.reader.get(&self.asker, key, Some(layer))
    }

    /// 外から来た名前で書く（鍵の表が断りうる）。
    pub fn set(&self, layer: PersistScope, key: &str, value: &str) -> WriteOutcome {
        self.write(Writer::ByName, layer, &[(key, value)], &[])
    }

    /// 外から来た名前で消す。
    pub fn unset(&self, layer: PersistScope, key: &str) -> WriteOutcome {
        self.write(Writer::ByName, layer, &[], &[key])
    }

    /// 決まった書き手の書き。
    pub fn put(&self, layer: PersistScope, entries: &[(&str, &str)]) -> WriteOutcome {
        self.write(Writer::Fixed, layer, entries, &[])
    }

    /// 決まった書き手の消し。
    pub fn remove(&self, layer: PersistScope, keys: &[&str]) -> WriteOutcome {
        self.write(Writer::Fixed, layer, &[], keys)
    }

    fn write(
        &self,
        writer: Writer,
        layer: PersistScope,
        puts: &[(&str, &str)],
        removes: &[&str],
    ) -> WriteOutcome {
        let (reply, rx) = areka_actor::reply_channel();
        let msg = SylphyaMsg::Write {
            writer,
            asker: self.asker.clone(),
            layer,
            puts: owned_puts(puts),
            removes: removes.iter().map(|k| k.to_string()).collect(),
            reply,
        };
        self.reader
            .ask(msg, rx)
            .unwrap_or(WriteOutcome::Failed(WriteFailed::Stopped))
    }
}

/// 同期の口（番人が居ない時点・呼んだスレッドの上で行う）。
pub struct KvsDirect {
    store: Mutex<Result<Store, String>>,
    backend: Arc<dyn StoreBackend>,
    table: &'static [KeyRow],
}

impl KvsDirect {
    /// 層の置き場と置き場の口から開く（開けなければ、読みは値なし・書きは失敗で縮退する）。
    pub fn open(roots: ScopeRoots, backend: Arc<dyn StoreBackend>) -> KvsDirect {
        KvsDirect {
            store: Mutex::new(Store::open(roots, backend.clone())),
            backend,
            table: KEY_TABLE,
        }
    }

    /// 層を重ねて読む。
    pub fn get(&self, key: &str) -> Option<String> {
        self.read(key, None)
    }

    /// その層だけを読む。
    pub fn get_in(&self, layer: PersistScope, key: &str) -> Option<String> {
        self.read(key, Some(layer))
    }

    /// 外から来た名前で書く（鍵の表が断りうる）。
    pub fn set(&self, layer: PersistScope, key: &str, value: &str) -> WriteOutcome {
        self.write(Writer::ByName, layer, &[(key, value)], &[])
    }

    /// 外から来た名前で消す。
    pub fn unset(&self, layer: PersistScope, key: &str) -> WriteOutcome {
        self.write(Writer::ByName, layer, &[], &[key])
    }

    /// 決まった書き手の書き。
    pub fn put(&self, layer: PersistScope, entries: &[(&str, &str)]) -> WriteOutcome {
        self.write(Writer::Fixed, layer, entries, &[])
    }

    /// 決まった書き手の消し。
    pub fn remove(&self, layer: PersistScope, keys: &[&str]) -> WriteOutcome {
        self.write(Writer::Fixed, layer, &[], keys)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Result<Store, String>> {
        // 核は 1 回の操作の中で整合に戻ってから返るので、毒は無視してよい。
        self.store.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn read(&self, key: &str, layer: Option<PersistScope>) -> Option<String> {
        if let Err(e) = parse_dotted(key) {
            tracing::warn!(target: LOG_TARGET, key, error = %e, "read of an unresolvable name; reading no value");
            return None;
        }
        let row = rule_of(self.table, key);
        let found = match row.home {
            Some(home @ (Home::File | Home::Memory)) => {
                let asker = AskerId::new("");
                self.lock()
                    .as_mut()
                    .ok()
                    .and_then(|store| store.read(&asker, key, layer, home))
            }
            Some(Home::Fast) | None => None,
        };
        match layer {
            Some(_) => found,
            None => found.or_else(|| row.default.map(str::to_owned)),
        }
    }

    fn write(
        &self,
        writer: Writer,
        layer: PersistScope,
        puts: &[(&str, &str)],
        removes: &[&str],
    ) -> WriteOutcome {
        let asker = AskerId::new("");
        let removes: Vec<String> = removes.iter().map(|k| k.to_string()).collect();
        let effect = plan_write(
            self.table,
            writer,
            &asker,
            layer,
            &owned_puts(puts),
            &removes,
        );
        let Some(Effect::Write { puts, removes, .. }) = effect.into_iter().next() else {
            return WriteOutcome::Refused;
        };
        let mut guard = self.lock();
        let root = guard
            .as_ref()
            .ok()
            .and_then(|store| store.roots().root_of(layer).cloned());
        let (file, earlier) = split_write(
            self.table,
            layer,
            root,
            puts,
            removes,
            |home, key, value| match (home, guard.as_mut()) {
                (Some(Home::Memory), Ok(store)) => store
                    .write_volatile(layer, &asker, &key, value.as_deref())
                    .err()
                    .map(|reason| WriteFailed::Volatile { key, reason }),
                (Some(Home::Memory), Err(reason)) => Some(WriteFailed::Volatile {
                    key,
                    reason: reason.clone(),
                }),
                _ => Some(no_home(layer, key)),
            },
        );
        let Some(file) = file else {
            return earlier.map_or(WriteOutcome::Written, WriteOutcome::Failed);
        };
        let outcome = match guard.as_mut() {
            Ok(store) => write_within_wait(store, self.backend.as_ref(), &file),
            Err(reason) => store_not_open(&file, reason),
        };
        outcome_of(earlier, outcome)
    }
}

/// 錠に当たる間は置き場の口の時計で期限まで取り直して書く（呼んだスレッドの上）。
pub(crate) fn write_within_wait(
    store: &mut Store,
    backend: &dyn StoreBackend,
    file: &FileWrite,
) -> PersistOutcome {
    let start = backend.now();
    loop {
        match store.try_write(file) {
            WriteAttempt::Done(outcome) => return outcome,
            WriteAttempt::Busy if backend.now().saturating_sub(start) <= OP_WAIT => {
                backend.pause(RETRY_STEP);
            }
            WriteAttempt::Busy => return lock_failed(file, WAIT_LIMIT),
        }
    }
}

fn owned_puts(puts: &[(&str, &str)]) -> Vec<(String, String)> {
    puts.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// 住み場所ごとに分ける: ファイルの鍵は 1 回の書きにまとめ、ほかの鍵は `other` で書く。
/// 返すのは（ファイルの書き〔鍵が在れば〕, ほかの鍵の最初の失敗）。
pub(crate) fn split_write(
    table: &'static [KeyRow],
    layer: PersistScope,
    root: Option<PathBuf>,
    puts: Vec<(String, String)>,
    removes: Vec<String>,
    mut other: impl FnMut(Option<Home>, String, Option<String>) -> Option<WriteFailed>,
) -> (Option<FileWrite>, Option<WriteFailed>) {
    let mut file = FileWrite {
        layer,
        root,
        puts: Vec::new(),
        removes: Vec::new(),
        typed: Vec::new(),
    };
    let mut earlier = None;
    let entries = puts
        .into_iter()
        .map(|(key, value)| (key, Some(value)))
        .chain(removes.into_iter().map(|key| (key, None)));
    for (key, value) in entries {
        match rule_of(table, &key).home {
            Some(Home::File) => match value {
                Some(value) => file.puts.push((key, value)),
                None => file.removes.push(key),
            },
            home => {
                let failed = other(home, key, value);
                if earlier.is_none() {
                    earlier = failed;
                }
            }
        }
    }
    file.typed = file
        .puts
        .iter()
        .map(|(key, _)| key)
        .chain(&file.removes)
        .filter_map(|key| PersistKey::from_canonical_key(key))
        .collect();
    let file = (!file.puts.is_empty() || !file.removes.is_empty()).then_some(file);
    (file, earlier)
}

/// 書く住み場所の無い名前（error を残す）。
pub(crate) fn no_home(layer: PersistScope, key: String) -> WriteFailed {
    tracing::error!(
        target: LOG_TARGET,
        scope = ?layer,
        key = %key,
        "name has no home to write in this layer; not written"
    );
    WriteFailed::Volatile {
        key,
        reason: "この名前の住み場所がこの層に無いので書けません".to_owned(),
    }
}

/// ファイルの書きの結果を口の結果へ（ほかの鍵の失敗が先）。
pub(crate) fn outcome_of(earlier: Option<WriteFailed>, outcome: PersistOutcome) -> WriteOutcome {
    match (earlier, outcome) {
        (Some(failed), _) => WriteOutcome::Failed(failed),
        (None, PersistOutcome::Saved) => WriteOutcome::Written,
        (None, PersistOutcome::Withheld) => WriteOutcome::Withheld,
        (None, PersistOutcome::Degraded(failure)) => {
            WriteOutcome::Failed(WriteFailed::Persist(failure))
        }
    }
}

#[cfg(test)]
#[path = "kvs_tests.rs"]
mod tests;
