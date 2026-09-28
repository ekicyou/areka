//! 手続きを走らせる背景のスレッドと、窓口へ頼んで返事を待つ本物の口（design「areka / install / worker」）。
//!
//! スレッド `install` は依頼を 1 件ずつ受けて [`run_order`] を走らせ、終わったら窓口へ知らせる。
//! 本物の口 [`DeskPorts`] は、World に触る仕事を窓口（`desk`）への頼み [`DeskAsk`] と返信端で
//! 頼んで返事を待つ。イベントの結果は kanade から直接受ける。利用条件の画面と、起動中の
//! ゴースト以外の宛先への展開は、このスレッドで行う（UI を塞がない＝要件 1.11）。
//!
//! 頼みの型はこのモジュールが定義し、窓口が読む（design「Allowed Dependencies」）。

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use areka_actor::{ActorHandle, ReplySender, reply_channel, run_inbox, spawn_actor};
use areka_kanade::RaiseOutcome;
use areka_nar::{InstallOutcome, InstallRequest, NarArchive, NarError};

use super::InstallOrder;
use super::judge::GhostFacts;
use super::procedure::{InstallPorts, InstalledRecord, Overwritten, Raised, run_order};
use super::terms::TermsNotice;
use crate::alert::{self, YesNo};
use crate::exit_wait::WorkGate;

/// 背景のスレッドから窓口への頼み。どれも返信端を 1 つ持ち、窓口が答えるか落とす。
// 読み手（窓口 `desk`）は 6.2 で結ぶ。結んだら外す。
#[allow(dead_code)]
pub(crate) enum DeskAsk {
    /// イベントを今のゴーストへ GET で送る。`reply` は kanade へそのまま渡す（kanade が直接答える）。
    Raise {
        id: &'static str,
        references: Vec<String>,
        /// 前に送った頼みが「定常でない」で返ったので送り直す（窓口は定常到達の後に送る）。
        resend: bool,
        reply: ReplySender<RaiseOutcome>,
    },
    /// 今のゴーストの素性を尋ねる（ゴーストが居なければ None を答える）。
    Facts {
        reply: ReplySender<Option<GhostFacts>>,
    },
    /// 起動中のゴーストを降ろしてから入れ、起こし直す（8.1）。
    Overwrite {
        archive: NarArchive,
        target_ghost: Option<String>,
        reply: ReplySender<Overwritten>,
    },
    /// 入れた後の記録。反映が済んでから答える。
    Record {
        record: InstalledRecord,
        reply: ReplySender<()>,
    },
    /// 依頼 1 件が終わった（次の依頼を受け取れる）。
    OrderDone,
}

/// 背景のスレッド `install` を起こし、依頼の送出端と join の取っ手を返す。
/// 窓口が最初の依頼で 1 度だけ呼ぶ。依頼の送出端が全部落ちたらスレッドは終わる。
// 呼び手（窓口 `desk`）は 6.2 で結ぶ。結んだら外す。
#[allow(dead_code)]
pub(crate) fn spawn_worker(
    desk: Sender<DeskAsk>,
    gate: Arc<WorkGate>,
) -> (Sender<InstallOrder>, ActorHandle) {
    spawn_actor("install", move |orders| {
        let mut ports = DeskPorts::new(desk.clone(), gate.clone());
        run_inbox(orders, |order: InstallOrder| {
            let ends = run_order(&order, &mut ports);
            // 断った・拒否・失敗・途中でやめた書庫の名前を門に残さない。
            gate.end();
            tracing::debug!(event = "install_order_done", ends = ?ends, "[install] 依頼を終えました");
            if desk.send(DeskAsk::OrderDone).is_err() {
                tracing::debug!(
                    event = "install_desk_gone",
                    "[install] 窓口が居ないので、依頼の終わりを知らせられません"
                );
            }
            Ok::<_, Infallible>(ControlFlow::Continue(()))
        });
    })
}

/// 本物の口: 窓口へ頼んで返事を待つ。
pub(crate) struct DeskPorts {
    desk: Sender<DeskAsk>,
    gate: Arc<WorkGate>,
    /// ベースウェアの根（素性を最後に尋ねたときの値）。
    root: Option<PathBuf>,
    /// 今扱っている書庫のパス（`begin_archive` で覚える）。
    archive: PathBuf,
}

impl DeskPorts {
    pub(crate) fn new(desk: Sender<DeskAsk>, gate: Arc<WorkGate>) -> Self {
        DeskPorts {
            desk,
            gate,
            root: None,
            archive: PathBuf::new(),
        }
    }

    /// 窓口へ頼んで返事を待つ。窓口が居ない・返信端が落ちたら None（閉じた）。
    fn ask<T: Send>(&self, make: impl FnOnce(ReplySender<T>) -> DeskAsk) -> Option<T> {
        let (reply, answer) = reply_channel();
        if self.desk.send(make(reply)).is_err() {
            tracing::debug!(
                event = "install_desk_gone",
                "[install] 窓口が居ないので、頼みを閉じたとして扱います"
            );
            return None;
        }
        answer.recv().ok()
    }
}

impl InstallPorts for DeskPorts {
    fn begin_archive(&mut self, path: &Path) {
        self.archive = path.to_path_buf();
        // 書庫を扱い始めた（書く前の段で終了が始まれば、途中でやめた書庫として記録される）。
        if !self.gate.begin(path.display().to_string()) {
            // 終了が始まっている。以後の頼みは窓口が落とすので、手続きは閉じた扱いで止まる。
            tracing::debug!(
                event = "install_gate_closed",
                archive = %path.display(),
                "[install] 終了が始まっているので、書庫を扱い始めません"
            );
        }
    }

    fn raise(&mut self, id: &'static str, references: Vec<String>) -> Raised {
        let mut resend = false;
        loop {
            let references = references.clone();
            let Some(outcome) = self.ask(|reply| DeskAsk::Raise {
                id,
                references,
                resend,
                reply,
            }) else {
                return Raised::Closed;
            };
            match outcome {
                RaiseOutcome::Script => return Raised::Script,
                RaiseOutcome::NoReply => return Raised::NoReply,
                RaiseOutcome::NotAllowed => {
                    tracing::error!(
                        event = "install_event_not_allowed",
                        id,
                        "[install] 許可表に無いイベントなので送られませんでした（返事なしとして続けます）"
                    );
                    return Raised::NoReply;
                }
                RaiseOutcome::Failed => {
                    tracing::error!(
                        event = "install_event_failed",
                        id,
                        "[install] ゴーストとの往復が失敗しました（以後はイベントを送りません）"
                    );
                    return Raised::Closed;
                }
                // 定常でない: 窓口が次の定常到達の後に送り直す。
                RaiseOutcome::NotSteady => {
                    tracing::debug!(
                        event = "install_event_not_steady",
                        id,
                        "[install] 定常でないので、定常到達の後に送り直します"
                    );
                    resend = true;
                }
            }
        }
    }

    fn ghost_facts(&mut self) -> Option<GhostFacts> {
        let facts = self.ask(|reply| DeskAsk::Facts { reply }).flatten()?;
        self.root = Some(facts.root.clone());
        Some(facts)
    }

    fn ask_terms(&mut self, title: &str, notice: &TermsNotice) -> YesNo {
        alert::ask_yes_no(title, &notice.body, alert::suppressed())
    }

    fn install_elsewhere(
        &mut self,
        archive: &NarArchive,
        target_ghost: Option<&str>,
    ) -> Option<Result<InstallOutcome, NarError>> {
        let root = match self.root.clone() {
            Some(root) => root,
            None => self.ghost_facts()?.root,
        };
        // 門: 名前を宛先つきへ差し替える（要件 8.3）。終了が始まっていれば書く段へ入らない（要件 8.4）。
        if !self
            .gate
            .begin(gate_label(&self.archive, &root, archive, target_ghost))
        {
            return None;
        }
        if !self.gate.enter_write() {
            self.gate.end();
            return None;
        }
        let result = archive.install(&InstallRequest {
            root: &root,
            target_ghost,
        });
        // 書き終えた直後に手放す（間で終了が始まると「書く前」と記録されるので続けて呼ぶ）。
        self.gate.leave_write();
        self.gate.end();
        Some(result)
    }

    fn overwrite_running(
        &mut self,
        archive: NarArchive,
        target_ghost: Option<String>,
    ) -> Overwritten {
        let overwritten = self
            .ask(|reply| DeskAsk::Overwrite {
                archive,
                target_ghost,
                reply,
            })
            .unwrap_or(Overwritten::Closed);
        // 書き終えた書庫を、完了の知らせを待つ間に「書く前にやめた」と記録させない。
        if matches!(overwritten, Overwritten::Ran(_)) {
            self.gate.end();
        }
        overwritten
    }

    fn record(&mut self, record: InstalledRecord) {
        // 窓口が居なければ記録は反映されない。続く `raise` が閉じた扱いで手続きを止める。
        let _ = self.ask(|reply| DeskAsk::Record { record, reply });
    }
}

/// 門に置く記録用の名前（書庫のパスと宛先＝要件 8.3）。
fn gate_label(
    path: &Path,
    root: &Path,
    archive: &NarArchive,
    target_ghost: Option<&str>,
) -> String {
    let manifest = archive.manifest();
    let target = target_ghost
        .map(|ghost| format!("・宛先のゴースト {ghost}"))
        .unwrap_or_default();
    format!(
        "{} → 根 {} の {:?} {}{target}",
        path.display(),
        root.display(),
        manifest.kind,
        manifest.directory
    )
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod worker_tests;
