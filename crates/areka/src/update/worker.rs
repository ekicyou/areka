//! 更新の手続きを走らせる背景のスレッドと、kanade へ直接送る本物の口。
//!
//! スレッド `update` は仕事 [`UpdateJob`] を 1 件ずつ受けて [`run_order`] を走らせ、終わったら口を落として
//! 通信中の数を戻してから窓口へ [`DeskAsk::OrderDone`] を頼む（数は標準の手続きの間だけ立つ）。
//! 本物の口 [`KanadePorts`] はイベントと照会を仕事の送出端へ直接送って返事を待ち（UI を塞がない）、
//! エンジンの一周を終了の門 [`WorkGate`] の出入りで囲む。
//! 終了が始まった後は何も送らない。
//!
//! 窓口への頼みの型はこのモジュールが定義し、窓口が読む。本番の取得口の型を綴る本体の
//! 本番ファイルは、このファイルと `install/fetch_url.rs` の 2 つだけ（`worker_tests` の字面の検査）。

use std::cell::Cell;
use std::convert::Infallible;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use areka_actor::{ActorHandle, reply_channel, run_inbox, spawn_actor};
use areka_ghost::{BasewareRoots, StoreKind};
use areka_kanade::online::{OnlineCounter, OnlineGuard};
use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{KanadeMsg, RaiseOutcome, ShioriMethod};
use areka_update::{Fetch, FetchError, Progress, UpdateRequest, WinHttpFetch};

use super::procedure::{EngineRun, GhostResources, Raised, UpdatePorts, run_order};
use super::{TargetKind, TargetSpec, UpdateOrder, UpdateTail, trimmed_homeurl};
use areka_session::exit_wait::WorkGate;

/// 照会する SHIORI リソース（1 回の照会で 2 つ）。窓口の `homeurl` の照会も同じ名前を使う。
pub(super) const HOMEURL: &str = "homeurl";
pub(super) const USEORIGIN1: &str = "useorigin1";

/// 窓口から背景スレッドへ渡す仕事（依頼と、依頼を受けた時点の kanade の送出端・並びの写し）。
pub(crate) struct UpdateJob {
    pub order: UpdateOrder,
    pub kanade: Sender<KanadeMsg>,
    /// 並び（バルーンの対象を写す判断に使う）。
    pub roots: BasewareRoots,
    /// 実行ファイルのフォルダ（本番は `areka_session::roots::user_root::exe_dir()`・テストは差し替える）。
    pub exe_dir: Option<PathBuf>,
}

/// 背景スレッドから窓口への頼み（返事は待たない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeskAsk {
    /// 標準の手続きが始まった（メニューからは `OnUpdateProcessExec` に応えが無かったとき・台本からは送らずに・照会の直前）。
    Started,
    /// 同じゴーストを読み直す（全対象の後・`changed` が 1 つ以上）。`tail` は後送りの列（先頭が
    /// 読み直した後の起動の知らせ）。
    Reload {
        ghost_dir: PathBuf,
        tail: UpdateTail,
    },
    /// 依頼 1 件が終わった。
    OrderDone,
    /// 依頼の最中にスレッドが倒れた（次の依頼でスレッドを起こし直す）。
    WorkerPanicked,
}

/// 依頼 1 件の守り手。倒れて落ちるときだけ、門を手放して窓口へ倒れを知らせる。
struct OrderGuard {
    gate: Arc<WorkGate>,
    desk: Sender<DeskAsk>,
}

impl Drop for OrderGuard {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        tracing::error!(
            event = "update_worker_panicked",
            "[update] 背景スレッドが依頼の最中に倒れました（次の依頼で起こし直します）"
        );
        self.gate.end();
        ask_desk(&self.desk, DeskAsk::WorkerPanicked);
    }
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
            // 口より先に作る（倒れて落ちるときは口が先に落ち、通信中の数が戻ってから知らせる）。
            let _guard = OrderGuard {
                gate: gate.clone(),
                desk: desk.clone(),
            };
            let ports = KanadePorts {
                kanade: job.kanade,
                gate: gate.clone(),
                desk: desk.clone(),
                new_fetch: new_fetch.clone(),
                online,
                online_guard: Cell::new(None),
                roots: job.roots,
                exe_dir: job.exe_dir,
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
    /// 並び（バルーンの対象を写す判断に使う）。
    roots: BasewareRoots,
    /// 実行ファイルのフォルダ（本番は `areka_session::roots::user_root::exe_dir()`・テストは差し替える）。
    exe_dir: Option<PathBuf>,
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
                found.homeurl = value.as_deref().and_then(trimmed_homeurl);
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
        // 作れないときの `error!` は手続きが 1 件だけ残す。
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

    fn request_reload(&self, ghost_dir: &Path, tail: UpdateTail) {
        ask_desk(
            &self.desk,
            DeskAsk::Reload {
                ghost_dir: ghost_dir.to_path_buf(),
                tail,
            },
        );
    }

    /// バルーンの対象を写す口へ通して答えのフォルダへ替える。
    /// ゴースト・シェルは起こしたフォルダ（写し）なので通さない。バルーンの更新先は descript の homeurl
    /// だけなので、それの無いもの（表示だけ・`run_target` が飛ばす）も写さない。
    fn prepare_targets(&self, targets: &mut [TargetSpec]) {
        for spec in targets
            .iter_mut()
            .filter(|spec| spec.kind == TargetKind::Balloon && spec.descript_homeurl.is_some())
        {
            spec.dir = areka_session::roots::user_root::copy_for_write(
                &self.roots,
                self.exe_dir.as_deref(),
                StoreKind::Balloon,
                &spec.dir,
            );
        }
    }

    fn standard_started(&self) {
        // 承諾待ち（`OnUpdateProcessExec` の答え待ち）は含めない。口を落とすと戻る。
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

#[cfg(test)]
pub(super) fn ports_for_test(kanade: Sender<KanadeMsg>) -> KanadePorts {
    KanadePorts {
        kanade,
        gate: Arc::default(),
        desk: std::sync::mpsc::channel().0,
        new_fetch: winhttp_fetch(),
        online: Box::leak(Box::default()),
        online_guard: Cell::new(None),
        roots: BasewareRoots::new(Vec::new()),
        exe_dir: None,
    }
}
