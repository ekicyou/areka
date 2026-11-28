//! `SylphyaReader`: `resolve_flat` / `resolve_dotted` / `talk_snapshot` と KVS の口の読み。
//!
//! 読みは鍵の表で住み場所を決め、速い住み場所は登録された住み場所を番人を通らずに読み、
//! ほかは番人へ聞き、無ければ登録された読み手（[`PullSource`]）、無ければ表の既定値の順。

use crate::actor::SylphyaMsg;
use crate::asker::{AskerContext, AskerId};
use crate::fast::{FastHome, FastRegistry};
use crate::key::{PropPath, parse_dotted};
use crate::keytable::{Home, KeyRow, rule_of};
use crate::kvs::Kvs;
use crate::persist::PersistScope;
use crate::pull::{PullRegistry, PullShape, PullSource};
use crate::value::{DottedResolution, FlatResolution};
use crate::vocab::DegradePolicy;
use crate::vocab::flat::FLAT_VOCAB;
use areka_actor::ReplyReceiver;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::mpsc::Sender;

const LOG_TARGET: &str = "areka_sylphya::reader";

/// 番人への読み口（複製できる・複製どうしで登録の表を分け合う）。
#[derive(Clone)]
pub struct SylphyaReader {
    tx: Sender<SylphyaMsg>,
    table: &'static [KeyRow],
    pull: PullRegistry,
    fast: FastRegistry,
}

impl SylphyaReader {
    pub(crate) fn new(
        tx: Sender<SylphyaMsg>,
        table: &'static [KeyRow],
        fast: FastRegistry,
    ) -> Self {
        SylphyaReader {
            tx,
            table,
            pull: PullRegistry::default(),
            fast,
        }
    }

    /// 名前の形に読み手を登録する（同じ形は置き換え）。複製どうしで共有される。
    ///
    /// 住み場所に値の無い点付きの名前だけがこの読み手に届く。答えはどこにも書かない。
    pub fn register_pull(&self, shape: PullShape, source: Arc<dyn PullSource>) {
        self.pull.register(shape, source);
    }

    /// 名前の形に、その層の速い住み場所を登録する（同じ形は置き換え）。
    pub fn register_fast(&self, shape: PullShape, layer: PersistScope, home: Arc<dyn FastHome>) {
        self.fast.register(shape, layer, home);
    }

    /// 問い手の KVS の口。
    pub fn kvs(&self, asker: AskerId) -> Kvs {
        Kvs::new(self.clone(), asker)
    }

    /// 平らな名前（`%` 抜き）を読む。値が無ければ台帳（[`FLAT_VOCAB`]）の縮退の決まり。
    pub fn resolve_flat(&self, asker: &AskerContext, name: &str) -> FlatResolution {
        let (reply, rx) = areka_actor::reply_channel();
        let msg = SylphyaMsg::GetFlat {
            asker: asker.asker.clone(),
            name: name.to_owned(),
            reply,
        };
        if let Some(v) = self.ask(msg, rx).flatten() {
            return FlatResolution::Value(v);
        }
        let policy = flat_degrade_policy(name);
        tracing::debug!(
            target: LOG_TARGET,
            asker = asker.asker.as_str(),
            name = name,
            policy = ?policy,
            "flat resolution degraded"
        );
        FlatResolution::Degraded(policy)
    }

    /// 点付きの名前を層を重ねて読む。
    pub fn resolve_dotted(&self, asker: &AskerContext, path: &PropPath) -> DottedResolution {
        match self.lookup(&asker.asker, path, None) {
            Some(v) => DottedResolution::Value(v),
            None => {
                tracing::debug!(
                    target: LOG_TARGET,
                    asker = asker.asker.as_str(),
                    key = %path.to_canonical_string(),
                    "dotted resolution not found"
                );
                DottedResolution::NotFound
            }
        }
    }

    /// 文字列の鍵の便宜口（解けない鍵は warn を残して `NotFound`）。
    pub fn resolve_dotted_str(&self, asker: &AskerContext, key: &str) -> DottedResolution {
        match parse_dotted(key) {
            Ok(path) => self.resolve_dotted(asker, &path),
            Err(e) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    asker = asker.asker.as_str(),
                    key = key,
                    error = %e,
                    "dotted key parse failed; degrading to NotFound"
                );
                DottedResolution::NotFound
            }
        }
    }

    /// トークの写し: 問い手の値の在る平らな名前の全部（所有の写し・後の書きで変わらない）。
    pub fn talk_snapshot(&self, asker: &AskerContext) -> BTreeMap<String, String> {
        let (reply, rx) = areka_actor::reply_channel();
        let msg = SylphyaMsg::TalkSnapshot {
            asker: asker.asker.clone(),
            reply,
        };
        self.ask(msg, rx).unwrap_or_default()
    }

    /// KVS の口の読み（`layer` が `None` なら重ね読み）。
    pub(crate) fn get(
        &self,
        asker: &AskerId,
        key: &str,
        layer: Option<PersistScope>,
    ) -> Option<String> {
        match parse_dotted(key) {
            Ok(path) => self.lookup(asker, &path, layer),
            Err(e) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    asker = asker.as_str(),
                    key,
                    error = %e,
                    "read of an unresolvable name; reading no value"
                );
                None
            }
        }
    }

    /// 住み場所 → 読み手 → 既定値。層を名指しした読みはその層の住み場所だけを見る。
    fn lookup(
        &self,
        asker: &AskerId,
        path: &PropPath,
        layer: Option<PersistScope>,
    ) -> Option<String> {
        let key = path.to_canonical_string();
        let row = rule_of(self.table, &key);
        let found = match row.home {
            Some(Home::Fast) => self
                .fast
                .find(path)
                .filter(|(at, _)| layer.is_none_or(|l| l == *at))
                .and_then(|(_, home)| home.read_text(path)),
            Some(home) => {
                let (reply, rx) = areka_actor::reply_channel();
                let msg = SylphyaMsg::Get {
                    asker: asker.clone(),
                    key,
                    layer,
                    home,
                    reply,
                };
                self.ask(msg, rx).flatten()
            }
            None => None,
        };
        if layer.is_some() {
            return found;
        }
        // `find` は鍵を外してから返すので、読み手を呼ぶのは鍵の外。
        found
            .or_else(|| self.pull.find(path).and_then(|s| s.read(path)))
            .or_else(|| row.default.map(str::to_owned))
    }

    /// 番人へ送って返事を待つ。番人が止まっていれば warn を 1 件残して `None`。
    pub(crate) fn ask<T: Send>(&self, msg: SylphyaMsg, rx: ReplyReceiver<T>) -> Option<T> {
        if self.tx.send(msg).is_err() {
            tracing::warn!(target: LOG_TARGET, "sylphya guardian stopped; no answer");
            return None;
        }
        match rx.recv() {
            Ok(answer) => Some(answer),
            Err(e) => {
                tracing::warn!(target: LOG_TARGET, error = ?e, "sylphya guardian stopped before answering; no answer");
                None
            }
        }
    }
}

/// フラット名の縮退政策を台帳から引く。台帳外の名は `PassThroughRaw`。
fn flat_degrade_policy(name: &str) -> DegradePolicy {
    FLAT_VOCAB
        .iter()
        .find(|e| e.token == name)
        .map(|e| e.degrade)
        .unwrap_or(DegradePolicy::PassThroughRaw)
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
