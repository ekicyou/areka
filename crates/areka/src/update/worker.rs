//! 更新の手続きを走らせる背景のスレッドと、kanade へ直接送る本物の口（design「背景スレッドと本物の口
//! （`update/worker.rs`）」）。
//!
//! スレッド `update` は仕事 [`UpdateJob`] を 1 件ずつ受けて [`run_order`] を走らせ、終わったら口を落として
//! 通信中の数を戻してから窓口へ [`DeskAsk::OrderDone`] を頼む（数は標準の手続きの間だけ立つ）。
//! 本物の口 [`KanadePorts`] はイベントと照会を仕事の送出端へ直接送って返事を待ち（UI を塞がない＝要件 1.11）、エンジンの一周を終了の門 [`WorkGate`] の出入りで囲む
//! （要件 7.1・7.3）。終了が始まった後は何も送らない（要件 7.4）。
//!
//! 窓口への頼みの型はこのモジュールが定義し、窓口（6.3）が読む。本番の取得口の型を綴る本体の
//! 本番ファイルは、このファイルと `install/fetch_url.rs` の 2 つだけ（`worker_tests` の字面の検査）。

use std::cell::Cell;
use std::convert::Infallible;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use areka_actor::{ActorHandle, reply_channel, run_inbox, spawn_actor};
use areka_kanade::online::{OnlineCounter, OnlineGuard};
use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{KanadeMsg, RaiseOutcome, ShioriMethod};
use areka_update::{Fetch, FetchError, Progress, UpdateRequest, WinHttpFetch};

use super::UpdateOrder;
use super::procedure::{EngineRun, GhostResources, Raised, UpdatePorts, run_order};
use crate::exit_wait::WorkGate;

/// 照会する SHIORI リソース（1 回の照会で 2 つ）。
const HOMEURL: &str = "homeurl";
const USEORIGIN1: &str = "useorigin1";

/// 窓口から背景スレッドへ渡す仕事（依頼と、依頼を受けた時点の kanade の送出端の写し）。
pub(crate) struct UpdateJob {
    pub order: UpdateOrder,
    pub kanade: Sender<KanadeMsg>,
}

/// 背景スレッドから窓口への頼み（返事は待たない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeskAsk {
    /// 標準の手続きが始まった（`OnUpdateProcessExec` に応えが無かった・照会の直前）。
    Started,
    /// 同じゴーストを読み直す（全対象の後・`changed` が 1 つ以上）。`tail` は後送りの列（先頭が
    /// 読み直した後の起動の知らせ・決めたこと 21）。
    Reload {
        ghost_dir: PathBuf,
        tail: Vec<(&'static str, Vec<String>)>,
    },
    /// 依頼 1 件が終わった。
    OrderDone,
}

/// 取得口の作り方（本番は [`winhttp_fetch`]・テストは偽の取得口）。対象ごとにスレッドの中で呼ぶ。
pub(crate) type NewFetch = Arc<dyn Fn() -> Result<Box<dyn Fetch>, FetchError> + Send + Sync>;

/// 本番の取得口の作り方（`WinHttpFetch::new`）。
pub(crate) fn winhttp_fetch() -> NewFetch {
    Arc::new(|| Ok(Box::new(WinHttpFetch::new()?) as Box<dyn Fetch>))
}

/// 背景スレッド `update` を起こし、仕事の送出端と取っ手を返す（窓口が最初の依頼で 1 度だけ呼ぶ）。
/// 仕事の送出端が全部落ちたらスレッドは終わる。`online` は標準の手続きの間だけ立てる通信中の数
/// （本番はプロセスの数）。
pub(crate) fn spawn_worker(
    desk: Sender<DeskAsk>,
    gate: Arc<WorkGate>,
    new_fetch: NewFetch,
    online: &'static OnlineCounter,
) -> (Sender<UpdateJob>, ActorHandle) {
    spawn_actor("update", move |jobs| {
        run_inbox(jobs, |job: UpdateJob| {
            let ports = KanadePorts {
                kanade: job.kanade,
                gate: gate.clone(),
                desk: desk.clone(),
                new_fetch: new_fetch.clone(),
                online,
                online_guard: Cell::new(None),
            };
            let end = run_order(&job.order, &ports);
            // 口を落として通信中の数を戻してから「終わった」を頼む（窓口が `Idle` へ移る時点で戻っている）。
            drop(ports);
            tracing::debug!(event = "update_order_done", ends = ?end.ends, "[update] 依頼を終えました");
            ask_desk(&desk, DeskAsk::OrderDone);
            Ok::<_, Infallible>(ControlFlow::Continue(()))
        });
    })
}

/// 本物の口: kanade へ直接送り、門に出入りし、エンジンを回す。
pub(crate) struct KanadePorts {
    kanade: Sender<KanadeMsg>,
    gate: Arc<WorkGate>,
    desk: Sender<DeskAsk>,
    new_fetch: NewFetch,
    /// 通信中の数（本番はプロセスの数）。
    online: &'static OnlineCounter,
    /// 標準の手続きの間の守り手（[`UpdatePorts::standard_started`] で立て、口を落とすと戻る）。
    online_guard: Cell<Option<OnlineGuard>>,
}

impl UpdatePorts for KanadePorts {
    fn raise(&self, id: &'static str, references: Vec<String>) -> Raised {
        if self.gate.is_closing() {
            return Raised::Closed;
        }
        let (reply, answer) = reply_channel();
        let sent = self.kanade.send(KanadeMsg::RaiseEvent {
            id: id.to_owned(),
            references,
            method: ShioriMethod::Get,
            reply: Some(reply),
        });
        let Some(outcome) = sent.ok().and_then(|()| answer.recv().ok()) else {
            tracing::debug!(
                event = "update_kanade_gone",
                id,
                "[update] kanade が居ないので、閉じたとして扱います"
            );
            return Raised::Closed;
        };
        match outcome {
            RaiseOutcome::Script => Raised::Script,
            RaiseOutcome::NoReply => Raised::NoReply,
            RaiseOutcome::NotSteady => {
                tracing::warn!(
                    event = "update_not_steady",
                    id,
                    "[update] ゴーストが定常でないので送られませんでした（閉じたとして扱います）"
                );
                Raised::Closed
            }
            RaiseOutcome::NotAllowed => {
                tracing::error!(
                    event = "update_event_not_allowed",
                    id,
                    "[update] 許可表に無いイベントなので送られませんでした（返事なしとして続けます）"
                );
                Raised::NoReply
            }
            RaiseOutcome::Failed => {
                tracing::error!(
                    event = "update_event_failed",
                    id,
                    "[update] ゴーストとの往復が失敗しました（以後はイベントを送りません）"
                );
                Raised::Closed
            }
        }
    }

    fn resources(&self) -> Option<GhostResources> {
        if self.gate.is_closing() {
            return None;
        }
        let (reply, answer) = reply_channel();
        let sent = self.kanade.send(KanadeMsg::ResourceQuery {
            ids: vec![HOMEURL, USEORIGIN1],
            reply,
        });
        let Some(pairs) = sent.ok().and_then(|()| answer.recv().ok()) else {
            tracing::debug!(
                event = "update_kanade_gone",
                "[update] kanade が居ないので、リソースを照会できません"
            );
            return None;
        };
        let mut found = GhostResources::default();
        for (id, outcome) in pairs {
            let value = match outcome {
                ResourceOutcome::Value(value) if !value.is_empty() => Some(value),
                ResourceOutcome::Value(_) | ResourceOutcome::NoContent => None,
                ResourceOutcome::Failed(reason) => {
                    tracing::warn!(
                        event = "update_resource_failed",
                        id,
                        reason = %reason,
                        "[update] リソースを照会できなかったので、無いとして扱います"
                    );
                    None
                }
            };
            if id == HOMEURL {
                found.homeurl = value;
            } else {
                found.useorigin1 = value;
            }
        }
        Some(found)
    }

    fn run_engine(
        &self,
        homeurl: &str,
        target: &Path,
        observe: &mut dyn FnMut(&Progress),
    ) -> EngineRun {
        if !self.gate.begin(format!("{homeurl} → {}", target.display())) {
            tracing::debug!(
                event = "update_gate_closed",
                homeurl,
                target = %target.display(),
                "[update] 終了が始まっているので、エンジンを回しません"
            );
            return EngineRun::Closed;
        }
        // 作れないときの `error!` は手続きが 1 件だけ残す（design の手続きの節）。
        let fetch = match (self.new_fetch)() {
            Ok(fetch) => fetch,
            Err(error) => {
                self.gate.end();
                return EngineRun::Unavailable(error);
            }
        };
        if !self.gate.enter_write() {
            self.gate.end();
            tracing::debug!(
                event = "update_gate_closed",
                homeurl,
                target = %target.display(),
                "[update] 終了が始まっているので、エンジンを回しません"
            );
            return EngineRun::Closed;
        }
        let result = areka_update::run(&UpdateRequest { homeurl, target }, fetch.as_ref(), observe);
        // 書き終えた直後に手放す（間で終了が始まると「書く前」と記録されるので続けて呼ぶ）。
        self.gate.leave_write();
        self.gate.end();
        EngineRun::Done(result)
    }

    fn request_reload(&self, ghost_dir: &Path, tail: Vec<(&'static str, Vec<String>)>) {
        ask_desk(
            &self.desk,
            DeskAsk::Reload {
                ghost_dir: ghost_dir.to_path_buf(),
                tail,
            },
        );
    }

    fn standard_started(&self) {
        // 承諾待ち（`OnUpdateProcessExec` の答え待ち）は含めない（要件 2.1）。口を落とすと戻る（要件 2.3）。
        self.online_guard.set(Some(self.online.begin("update")));
        ask_desk(&self.desk, DeskAsk::Started);
    }
}

/// 窓口へ頼む（返事は待たない）。窓口が居なければ `debug!` で落とす。
fn ask_desk(desk: &Sender<DeskAsk>, ask: DeskAsk) {
    if let Err(err) = desk.send(ask) {
        tracing::debug!(
            event = "update_desk_gone",
            ask = ?err.0,
            "[update] 窓口が居ないので、頼みを落とします"
        );
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod worker_tests;
