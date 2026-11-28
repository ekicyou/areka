//! 番人のアクター: ゴーストごとに 1 本、住み場所の核を 1 つ持ち、電文を届いた順に片づける受信の輪。
//!
//! 覚える値の書き・消し・反映待ちは 1 本の列に並ぶ。列に書きが在る間は受信を 10 ミリ秒で区切り、
//! 区切りごとに列の頭を待たずに試み直す。読み・揮発の書き・平らな名前・層の付け替えは列を待たない。

use crate::actor::{Effect, SylphyaMsg, SylphyaPublisher, apply, plan_write};
use crate::asker::AskerId;
use crate::fast::FastRegistry;
use crate::key::parse_dotted;
use crate::keytable::{Home, KEY_TABLE, KeyRow};
use crate::kvs::{WriteFailed, WriteOutcome, no_home, outcome_of, split_write};
use crate::persist::{
    PersistFailure, PersistFailureCause, PersistOutcome, PersistScope, ScopeRoots,
};
use crate::reader::SylphyaReader;
use crate::store::{FileWrite, OP_WAIT, RETRY_STEP, Store, StoreBackend, WAIT_LIMIT, WriteAttempt};
use areka_actor::ReplySender;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

const LOG_TARGET: &str = "areka_sylphya::actor";
const PERSIST_LOG_TARGET: &str = "areka_sylphya::persist";

const CLOSE_BUSY: &str = "終了のとき、ほかの書き手が記憶のファイルを使っていて書けませんでした";

/// [`spawn_sylphya`] の起動の値（[`SylphyaInit::new`] で作る）。
pub struct SylphyaInit {
    roots: ScopeRoots,
    backend: Arc<dyn StoreBackend>,
    table: &'static [KeyRow],
}

impl SylphyaInit {
    /// 層の置き場と置き場の口から作る（鍵の表は本番の表）。
    pub fn new(roots: ScopeRoots, backend: Arc<dyn StoreBackend>) -> Self {
        SylphyaInit {
            roots,
            backend,
            table: KEY_TABLE,
        }
    }

    /// 鍵の表を試験の表に差し替える。
    #[cfg(test)]
    pub(crate) fn with_table(mut self, table: &'static [KeyRow]) -> Self {
        self.table = table;
        self
    }
}

/// [`spawn_sylphya`] の返却物（読み口・送信端・join ハンドルの三点セット）。
pub struct SylphyaParts {
    /// 読み口（複数消費エンジンへ clone 可）。
    pub reader: SylphyaReader,
    /// 変異投函の送信端（clone 可・複数供給者が共有）。
    pub publisher: SylphyaPublisher,
    /// アクタースレッドの join ハンドル（shutdown 時に join して panic を観測）。
    pub handle: areka_actor::ActorHandle,
}

/// 番人のアクターを起こす。
pub fn spawn_sylphya(init: SylphyaInit) -> SylphyaParts {
    spawn_sylphya_with_failures(init, None)
}

/// [`spawn_sylphya`] の派生。`failures` が `Some` なら、覚える値の書き・消しが `Degraded` になるたびに
/// 中身を 1 件送る（成功では送らない）。同じ書き手の失敗は投函の順に届く。
pub fn spawn_sylphya_with_failures(
    init: SylphyaInit,
    failures: Option<Sender<PersistFailure>>,
) -> SylphyaParts {
    let fast = FastRegistry::default();
    let table = init.table;
    let shared = fast.clone();
    let (tx, handle) = areka_actor::spawn_actor::<SylphyaMsg, _>("sylphya", move |rx| {
        Guardian::open(init, shared, failures).run(rx);
    });
    SylphyaParts {
        reader: SylphyaReader::new(tx.clone(), table, fast),
        publisher: SylphyaPublisher::new(tx),
        handle,
    }
}

/// 列の 1 件（覚える値の書き・消し、または反映待ち）。
enum Queued {
    Write {
        write: Box<FileWrite>,
        reply: Option<ReplySender<WriteOutcome>>,
        /// 同じ頼みのうち、列に入らなかった鍵の最初の失敗。
        earlier: Option<WriteFailed>,
    },
    Barrier(ReplySender<()>),
}

struct Guardian {
    /// 開けなかったときは理由（読みは値なし・書きは失敗で縮退する）。
    store: Result<Store, String>,
    backend: Arc<dyn StoreBackend>,
    table: &'static [KeyRow],
    fast: FastRegistry,
    failures: Option<Sender<PersistFailure>>,
    queue: VecDeque<Queued>,
    /// 列の頭を最初に試みた時刻（置き場の口の時計）。
    head_since: Option<Duration>,
}

impl Guardian {
    fn open(
        init: SylphyaInit,
        fast: FastRegistry,
        failures: Option<Sender<PersistFailure>>,
    ) -> Self {
        let SylphyaInit {
            roots,
            backend,
            table,
        } = init;
        Guardian {
            store: Store::open(roots, backend.clone()),
            backend,
            table,
            fast,
            failures,
            queue: VecDeque::new(),
            head_since: None,
        }
    }

    fn run(mut self, rx: Receiver<SylphyaMsg>) {
        loop {
            let next = if self.queue.is_empty() {
                rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
            } else {
                rx.recv_timeout(RETRY_STEP)
            };
            match next {
                Ok(msg) => {
                    if self.handle(msg) {
                        self.settle_rest();
                        return;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.settle_rest();
                    return;
                }
            }
            self.advance_queue();
        }
    }

    /// 1 通を片づける。返すのは、止まるか。
    fn handle(&mut self, msg: SylphyaMsg) -> bool {
        match msg {
            SylphyaMsg::Get {
                asker,
                key,
                layer,
                home,
                reply,
            } => {
                let value = self
                    .store
                    .as_mut()
                    .ok()
                    .and_then(|s| s.read(&asker, &key, layer, home));
                let _ = reply.send(value);
            }
            SylphyaMsg::GetFlat { asker, name, reply } => {
                let value = self
                    .store
                    .as_mut()
                    .ok()
                    .and_then(|s| s.read_flat(&asker, &name));
                let _ = reply.send(value);
            }
            SylphyaMsg::TalkSnapshot { asker, reply } => {
                let snapshot = self
                    .store
                    .as_mut()
                    .map(|s| s.flat_snapshot(&asker))
                    .unwrap_or_default();
                let _ = reply.send(snapshot);
            }
            SylphyaMsg::Barrier { reply } => {
                if self.queue.is_empty() {
                    let _ = reply.send(());
                } else {
                    self.queue.push_back(Queued::Barrier(reply));
                }
            }
            SylphyaMsg::Write {
                writer,
                asker,
                layer,
                puts,
                removes,
                reply,
            } => {
                let effects = plan_write(self.table, writer, &asker, layer, &puts, &removes);
                self.write(effects, reply);
            }
            SylphyaMsg::Close => return true,
            _ => self.run_effects(apply(&msg)),
        }
        false
    }

    fn run_effects(&mut self, effects: Vec<Effect>) {
        for effect in &effects {
            match effect {
                Effect::SetFlatPerAsker { asker, name, value } => {
                    self.flat(asker, name, Some(value));
                }
                Effect::RemoveFlatPerAsker { asker, name } => self.flat(asker, name, None),
                Effect::SetDottedGlobal { key, value } => {
                    let _ = self.volatile(PersistScope::App, &AskerId::new(""), key, Some(value));
                }
                Effect::RemoveDottedGlobal { key } => {
                    let _ = self.volatile(PersistScope::App, &AskerId::new(""), key, None);
                }
                Effect::SetDottedPerAsker { asker, key, value } => {
                    let _ = self.volatile(PersistScope::Ghost, asker, key, Some(value));
                }
                Effect::PersistSave { scope, entries } => {
                    let write = FileWrite {
                        layer: *scope,
                        root: self.root_of(*scope),
                        puts: entries
                            .iter()
                            .map(|(key, value)| (key.to_canonical_key(), value.clone()))
                            .collect(),
                        removes: Vec::new(),
                        typed: entries.iter().map(|(key, _)| *key).collect(),
                    };
                    self.enqueue(write, None, None);
                }
                Effect::PersistRemove { scope, keys } => {
                    let write = FileWrite {
                        layer: *scope,
                        root: self.root_of(*scope),
                        puts: Vec::new(),
                        removes: keys.iter().map(|key| key.to_canonical_key()).collect(),
                        typed: keys.clone(),
                    };
                    self.enqueue(write, None, None);
                }
                Effect::SetLayerRoot { layer, root } => {
                    if let Ok(store) = &mut self.store {
                        store.set_root(*layer, root.clone());
                    }
                    self.fast.clear_layer(*layer);
                }
                Effect::RecordAbsentFlat { .. }
                | Effect::RuntimeCommandReserved { .. }
                | Effect::NotSettable { .. }
                | Effect::Refused { .. }
                | Effect::Write { .. }
                | Effect::Barrier
                | Effect::Stop => {}
            }
        }
    }

    /// 層を名指しした書き・消し: 揮発の鍵はすぐ書き、ファイルの鍵は受けた時点の置き場で列へ。
    fn write(&mut self, effects: Vec<Effect>, reply: ReplySender<WriteOutcome>) {
        for effect in effects {
            match effect {
                Effect::Refused { .. } => {
                    let _ = reply.send(WriteOutcome::Refused);
                    return;
                }
                Effect::Write {
                    asker,
                    layer,
                    puts,
                    removes,
                } => {
                    let root = self.root_of(layer);
                    let (file, earlier) = split_write(
                        self.table,
                        layer,
                        root,
                        puts,
                        removes,
                        |home, key, value| match home {
                            Some(Home::Memory) => {
                                self.volatile(layer, &asker, &key, value.as_deref()).err()
                            }
                            Some(Home::Fast) => self.fast_write(layer, key, value.as_deref()),
                            _ => Some(no_home(layer, key)),
                        },
                    );
                    match file {
                        Some(file) => self.enqueue(file, Some(reply), earlier),
                        None => {
                            let _ = reply
                                .send(earlier.map_or(WriteOutcome::Written, WriteOutcome::Failed));
                        }
                    }
                    return;
                }
                _ => {}
            }
        }
    }

    fn enqueue(
        &mut self,
        write: FileWrite,
        reply: Option<ReplySender<WriteOutcome>>,
        earlier: Option<WriteFailed>,
    ) {
        self.queue.push_back(Queued::Write {
            write: Box::new(write),
            reply,
            earlier,
        });
    }

    /// 列の頭から、片づくものを片づける。頭が錠に当たって期限の内なら、そこで止める。
    fn advance_queue(&mut self) {
        while let Some(head) = self.queue.pop_front() {
            match head {
                Queued::Barrier(reply) => {
                    let _ = reply.send(());
                }
                Queued::Write {
                    write,
                    reply,
                    earlier,
                } => {
                    let since = *self.head_since.get_or_insert_with(|| self.backend.now());
                    let outcome = match self.attempt(&write) {
                        WriteAttempt::Done(outcome) => outcome,
                        WriteAttempt::Busy
                            if self.backend.now().saturating_sub(since) <= OP_WAIT =>
                        {
                            self.queue.push_front(Queued::Write {
                                write,
                                reply,
                                earlier,
                            });
                            return;
                        }
                        WriteAttempt::Busy => lock_failed(&write, WAIT_LIMIT),
                    };
                    self.head_since = None;
                    self.settle(outcome, reply, earlier);
                }
            }
        }
    }

    /// 止まる前に、列の残りを届いた順に 1 回ずつ、待たずに試みる。
    fn settle_rest(&mut self) {
        while let Some(head) = self.queue.pop_front() {
            match head {
                Queued::Barrier(reply) => {
                    let _ = reply.send(());
                }
                Queued::Write {
                    write,
                    reply,
                    earlier,
                } => {
                    let outcome = match self.attempt(&write) {
                        WriteAttempt::Done(outcome) => outcome,
                        WriteAttempt::Busy => lock_failed(&write, CLOSE_BUSY),
                    };
                    self.settle(outcome, reply, earlier);
                }
            }
        }
    }

    fn attempt(&mut self, write: &FileWrite) -> WriteAttempt {
        match &mut self.store {
            Ok(store) => store.try_write(write),
            Err(reason) => WriteAttempt::Done(store_not_open(write, reason)),
        }
    }

    /// 片づいた書きの失敗を受け口へ 1 件送り、返事を待つ呼び手へ結果を返す。
    fn settle(
        &self,
        outcome: PersistOutcome,
        reply: Option<ReplySender<WriteOutcome>>,
        earlier: Option<WriteFailed>,
    ) {
        if let (PersistOutcome::Degraded(failure), Some(failures)) = (&outcome, &self.failures)
            && failures.send(failure.clone()).is_err()
        {
            // 失敗そのものは書きの側が記録済み。受け手が落ちていても続ける。
            tracing::warn!(
                target: PERSIST_LOG_TARGET,
                scope = ?failure.scope,
                "persist failure receiver closed; failure not forwarded (continuing)"
            );
        }
        if let Some(reply) = reply {
            let _ = reply.send(outcome_of(earlier, outcome));
        }
    }

    fn root_of(&self, layer: PersistScope) -> Option<std::path::PathBuf> {
        self.store.as_ref().ok()?.roots().root_of(layer).cloned()
    }

    fn volatile(
        &mut self,
        layer: PersistScope,
        asker: &AskerId,
        key: &str,
        value: Option<&str>,
    ) -> Result<(), WriteFailed> {
        let result = match &mut self.store {
            Ok(store) => store.write_volatile(layer, asker, key, value),
            Err(reason) => {
                tracing::error!(
                    target: LOG_TARGET,
                    scope = ?layer,
                    key,
                    error = %reason,
                    "volatile value not written; store not open"
                );
                Err(reason.clone())
            }
        };
        result.map_err(|reason| WriteFailed::Volatile {
            key: key.to_owned(),
            reason,
        })
    }

    fn flat(&mut self, asker: &AskerId, name: &str, value: Option<&str>) {
        if let Ok(store) = &mut self.store {
            let _ = store.write_flat(asker, name, value);
        }
    }

    /// 速い住み場所へ名前で書く（登録された層だけ・消しは受けない）。
    fn fast_write(
        &self,
        layer: PersistScope,
        key: String,
        value: Option<&str>,
    ) -> Option<WriteFailed> {
        let found = parse_dotted(&key)
            .ok()
            .and_then(|path| Some((self.fast.find(&path)?, path)));
        let (home, path) = match found {
            Some(((at, home), path)) if at == layer => (home, path),
            _ => return Some(no_home(layer, key)),
        };
        let Some(value) = value else {
            return Some(no_home(layer, key));
        };
        home.write_text(&path, value).err().map(|reason| {
            tracing::error!(target: LOG_TARGET, scope = ?layer, key = %key, error = %reason, "fast value not written");
            WriteFailed::Volatile { key, reason }
        })
    }
}

fn key_names(write: &FileWrite) -> Vec<&str> {
    write
        .puts
        .iter()
        .map(|(key, _)| key.as_str())
        .chain(write.removes.iter().map(String::as_str))
        .collect()
}

fn degraded(write: &FileWrite, cause: PersistFailureCause) -> PersistOutcome {
    PersistOutcome::Degraded(PersistFailure {
        scope: write.layer,
        dir: write.root.clone(),
        keys: write.typed.clone(),
        cause,
    })
}

/// 核を開けなかったときの覚える値の書き。
pub(crate) fn store_not_open(write: &FileWrite, reason: &str) -> PersistOutcome {
    tracing::error!(
        target: PERSIST_LOG_TARGET,
        scope = ?write.layer,
        keys = ?key_names(write),
        error = %reason,
        "persist store not open; reporting Degraded"
    );
    degraded(write, PersistFailureCause::Io(reason.to_owned()))
}

/// 錠に当たったまま諦めた書き（巻き戻し済み・ファイルは変わっていない）。
pub(crate) fn lock_failed(write: &FileWrite, reason: &str) -> PersistOutcome {
    tracing::error!(
        target: PERSIST_LOG_TARGET,
        event = "persist_lock_failed",
        scope = ?write.layer,
        path = ?write.root,
        keys = ?key_names(write),
        error = reason,
        "persist write lock not acquired; reporting Degraded"
    );
    degraded(write, PersistFailureCause::Io(reason.to_owned()))
}

/// 錠と期限と列の順を、本物のファイルの上で、眠らずに確かめる。
#[cfg(test)]
#[path = "guardian_wait_tests.rs"]
pub(crate) mod wait_tests;

#[cfg(test)]
#[path = "guardian_tests.rs"]
mod tests;
