//! 手続きを走らせる背景のスレッドと、窓口へ頼んで返事を待つ本物の口。
//!
//! スレッド `install` は依頼を 1 件ずつ受けて [`run_order`] を走らせ、台本の取得で落とした書庫を消し、
//! 終わったら窓口へ知らせる。
//! 本物の口 [`DeskPorts`] は、World に触る仕事を窓口（`desk`）への頼み [`DeskAsk`] と返信端で
//! 頼んで返事を待つ。イベントの結果は kanade から直接受ける。利用条件の画面と書庫の展開（起動中の
//! ゴーストへは、窓口が降ろしたと答えた後）は、このスレッドで行う（UI を塞がない）。
//!
//! 頼みの型はこのモジュールが定義し、窓口が読む。

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;

use areka_actor::{ActorHandle, ReplySender, reply_channel, run_inbox, spawn_actor};
use areka_ghost::BasewareRoots;
use areka_ghost::catalog::master_install_keys;
use areka_ghost::catalog_roots::{balloons_for, find_balloon, find_ghost, same_folder_name};
use areka_kanade::RaiseOutcome;
use areka_nar::{InstallKind, InstallOutcome, NarArchive, NarError};

use super::destination::install_into;
use super::fetch_url::download_dir;
use super::judge::GhostFacts;
use super::procedure::{InstallPorts, InstalledRecord, Overwritten, Raised, run_order};
use super::startup::CarriedInstall;
use super::terms::TermsNotice;
use super::{InstallOrder, InstallOrigin};
use areka_session::alert::{self, YesNo};
use areka_session::exit_wait::WorkGate;

/// 背景のスレッド（`install`・`install-fetch`）から窓口への頼みと知らせ。返信端を持つ頼みは、窓口が
/// 答えるか落とす。
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
    /// 起動中のゴーストを降ろしてから入れ、起こし直す。
    Overwrite {
        archive: NarArchive,
        target_ghost: Option<String>,
        reply: ReplySender<OverwriteAnswer>,
    },
    /// 降ろした起動中のゴーストへ展開し終えた（窓口は切替を続け、定常到達で答える）。
    Extracted { reply: ReplySender<()> },
    /// 目録の綴りに直した入れた後の記録。反映が済んでから答える。
    Record {
        installed: CarriedInstall,
        reply: ReplySender<()>,
    },
    /// 依頼 1 件が終わった（次の依頼を受け取れる）。
    OrderDone,
    /// 依頼の最中にスレッドが倒れた（次の依頼でスレッドを起こし直す）。
    WorkerPanicked,
    /// 取得のスレッドが URL を 1 件終えた（`panicked` ならスレッドは抜けたので、次の URL で起こし直す）。
    FetchDone { panicked: bool },
}

/// 上書きの頼みへの窓口の答え。
pub(crate) enum OverwriteAnswer {
    /// 降ろして全窓を閉じた（起こすのは展開し終えた答えまで待つ）。展開の宛先の並びと起動中のゴーストの場所。
    Down {
        archive: NarArchive,
        roots: BasewareRoots,
        running: Option<PathBuf>,
    },
    /// 宛先はもう起動中のゴーストではない（書庫を返す）。
    NotRunning(NarArchive),
}

/// 依頼 1 件の守り手。倒れて落ちるときだけ、門を手放して窓口へ倒れを知らせる。
pub(super) struct OrderGuard {
    pub(super) gate: Arc<WorkGate>,
    pub(super) desk: Sender<DeskAsk>,
}

impl Drop for OrderGuard {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        tracing::error!(
            event = "install_worker_panicked",
            "[install] 背景のスレッドが依頼の最中に倒れました（次の依頼で起こし直します）"
        );
        self.gate.end();
        if self.desk.send(DeskAsk::WorkerPanicked).is_err() {
            tracing::debug!(
                event = "install_desk_gone",
                "[install] 窓口が居ないので、倒れを知らせられません"
            );
        }
    }
}

/// 背景のスレッド `install` を起こし、依頼の送出端と join の取っ手を返す。
/// 窓口がスレッドの居ないとき（最初の依頼と、倒れた後）に呼ぶ。依頼の送出端が全部落ちたらスレッドは終わる。
pub(crate) fn spawn_worker(
    desk: Sender<DeskAsk>,
    gate: Arc<WorkGate>,
) -> (Sender<InstallOrder>, ActorHandle) {
    spawn_actor("install", move |orders| {
        let mut ports = DeskPorts::new(desk.clone(), gate.clone());
        run_inbox(orders, |order: InstallOrder| {
            let _guard = OrderGuard {
                gate: gate.clone(),
                desk: desk.clone(),
            };
            let ends = run_order(&order, &mut ports);
            remove_downloaded(&order, &download_dir());
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

/// 出どころが台本で `dir` の直下にある書庫を消す（消せなければ取得の 7 日の掃除に任せる）。
pub(super) fn remove_downloaded(order: &InstallOrder, dir: &Path) {
    if order.origin != InstallOrigin::Script {
        return;
    }
    for archive in &order.archives {
        if archive.file_name().is_none() || archive.parent() != Some(dir) {
            continue;
        }
        if let Err(err) = std::fs::remove_file(archive) {
            tracing::debug!(
                event = "install_fetch_sweep_skipped",
                path = %archive.display(),
                error = %err,
                "[install] 落とした書庫を消せないので、7 日の掃除に任せます"
            );
        }
    }
}

/// 本物の口: 窓口へ頼んで返事を待つ。
pub(crate) struct DeskPorts {
    desk: Sender<DeskAsk>,
    gate: Arc<WorkGate>,
    /// ベースウェアの根の並びの写し（素性を最後に尋ねたときの値）。
    roots: Option<BasewareRoots>,
    /// 起動中のゴーストの場所（素性を最後に尋ねたときの値）。
    running: Option<PathBuf>,
    /// 今のゴーストの場所（素性を最後に尋ねたときの値・隠れの判定に使う）。
    ghost_dir: Option<PathBuf>,
    /// 今扱っている書庫のパス（`begin_archive` で覚える）。
    archive: PathBuf,
}

impl DeskPorts {
    pub(crate) fn new(desk: Sender<DeskAsk>, gate: Arc<WorkGate>) -> Self {
        DeskPorts {
            desk,
            gate,
            roots: None,
            running: None,
            ghost_dir: None,
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
        // 書く前の段で終了が始まれば、途中でやめた書庫として記録される。
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
        let keys = master_install_keys(&facts.dir);
        self.roots = Some(facts.roots.clone());
        self.running = facts.running_dir.clone();
        self.ghost_dir = Some(facts.dir.clone());
        Some(GhostFacts {
            sakura_name: keys.sakura_name,
            install_accept: keys.install_accept,
            ..facts
        })
    }

    fn ask_terms(&mut self, title: &str, notice: &TermsNotice) -> YesNo {
        alert::ask_yes_no(title, &notice.body, alert::suppressed())
    }

    fn install_elsewhere(
        &mut self,
        archive: &NarArchive,
        target_ghost: Option<&str>,
    ) -> Option<Result<InstallOutcome, NarError>> {
        let roots = match self.roots.clone() {
            Some(roots) => roots,
            None => self.ghost_facts()?.roots,
        };
        // 門: 名前を宛先つきへ差し替える。終了が始まっていれば書く段へ入らない。
        if !self
            .gate
            .begin(gate_label(&self.archive, &roots, archive, target_ghost))
        {
            return None;
        }
        if !self.gate.enter_write() {
            self.gate.end();
            return None;
        }
        let result = install_into(&roots, archive, target_ghost, self.running.as_deref());
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
        let folder = target_ghost
            .clone()
            .unwrap_or_else(|| archive.manifest().directory.clone());
        let answer = self.ask(|reply| DeskAsk::Overwrite {
            archive,
            target_ghost: target_ghost.clone(),
            reply,
        });
        let (archive, roots, running) = match answer {
            None => return Overwritten::Closed,
            Some(OverwriteAnswer::NotRunning(archive)) => return Overwritten::NotRunning(archive),
            Some(OverwriteAnswer::Down {
                archive,
                roots,
                running,
            }) => (archive, roots, running),
        };
        let label = gate_label(&self.archive, &roots, &archive, target_ghost.as_deref());
        if !self.gate.begin(label) || !self.gate.enter_write() {
            self.gate.end();
            return Overwritten::Closed;
        }
        let started = Instant::now();
        let result = install_into(
            &roots,
            &archive,
            target_ghost.as_deref(),
            running.as_deref(),
        );
        // 書き終えた書庫を、完了の知らせを待つ間に「書く前にやめた」と記録させない。
        self.gate.leave_write();
        self.gate.end();
        tracing::info!(
            event = "install_overwrite_done",
            ms = started.elapsed().as_millis() as u64,
            ok = result.is_ok(),
            folder = %folder,
            "[install] 降ろした起動中のゴーストのフォルダへ展開しました"
        );
        // 書き終えているので、返事が閉じても（終了が始まった）結果を手続きへ返す。
        let _ = self.ask(|reply| DeskAsk::Extracted { reply });
        Overwritten::Ran(result)
    }

    fn warn_skipped(&mut self, title: &str, body: &str) {
        // インストールの背景のスレッドの上で出す（UI スレッドとゴーストの会話は止まらない）。
        alert::warn_ok(title, body, alert::suppressed());
    }

    fn record(&mut self, record: InstalledRecord) {
        let installed = carry_record(self.roots.as_ref(), self.ghost_dir.as_deref(), record);
        // 窓口が居なければ記録は反映されない。続く `raise` が閉じた扱いで手続きを止める。
        let _ = self.ask(|reply| DeskAsk::Record { installed, reply });
    }
}

/// 入れた後の記録を目録の綴りに直し、バルーンだけなら隠れを判定する（背景のスレッドで呼ぶ）。
/// `ghost_dir` は今のゴーストの場所（無ければ隠れを見ない）。
pub(super) fn carry_record(
    roots: Option<&BasewareRoots>,
    ghost_dir: Option<&Path>,
    record: InstalledRecord,
) -> CarriedInstall {
    let (folder, ghost_name) = catalog_spelling(roots, &record);
    if record.kind == InstallKind::Balloon
        && let (Some(roots), Some(dir)) = (roots, ghost_dir)
    {
        note_hidden_by_ghost(roots, dir, &record.folder);
    }
    CarriedInstall {
        kind: record.kind,
        folder,
        ghost_folder: record.ghost_folder,
        object_name: record.object_name,
        ghost_name,
    }
}

/// 目録の綴りに直したフォルダ名と、%lastghostname の値（窓口と起動の前の口の共通の決まり）。
/// フォルダ名はゴースト・バルーンなら根の目録の綴り（目録に無ければ書庫の綴りのまま）、他は書庫の
/// 綴りのまま。%lastghostname は関わったゴーストの目録の `name`（無ければゴーストは `install.txt` の
/// `name`・他はそのフォルダ名）で、関わったゴーストが無ければ None。
pub(super) fn catalog_spelling(
    roots: Option<&BasewareRoots>,
    record: &InstalledRecord,
) -> (String, Option<String>) {
    // ゴーストの書庫の `ghost_folder` は `folder` と同じなので、目録の引き当ては 1 回で足りる。
    let ghost_entry = record
        .ghost_folder
        .as_deref()
        .and_then(|folder| roots.and_then(|roots| find_ghost(roots, folder)));
    let ghost_name = record.ghost_folder.as_deref().map(|folder| {
        match ghost_entry
            .as_ref()
            .and_then(|entry| entry.identity.name.clone())
        {
            Some(name) => name,
            None if record.kind == InstallKind::Ghost => record.object_name.clone(),
            None => folder.to_owned(),
        }
    });
    let folder = match record.kind {
        InstallKind::Ghost => ghost_entry.map(|e| e.identity.folder),
        InstallKind::Balloon => {
            roots.and_then(|roots| find_balloon(roots, &record.folder).map(|e| e.identity.folder))
        }
        InstallKind::Shell | InstallKind::Supplement => None,
    }
    .unwrap_or_else(|| record.folder.clone());
    (folder, ghost_name)
}

/// バルーンだけの書庫を根へ入れた後、今のゴーストの中の同じフォルダ名に隠れるなら info
/// `installed_balloon_hidden_by_ghost` を 1 件残す。
fn note_hidden_by_ghost(roots: &BasewareRoots, ghost_dir: &Path, folder: &str) {
    let hidden = balloons_for(roots, ghost_dir).hidden;
    if let Some(h) = hidden.iter().find(|h| same_folder_name(&h.folder, folder)) {
        tracing::info!(
            event = "installed_balloon_hidden_by_ghost",
            folder = h.folder.as_str(),
            installed = %h.hidden.display(),
            by = %h.by.display(),
            "[install] 入れたバルーンは今のゴーストの中の同じフォルダ名に隠れます"
        );
    }
}

/// 門に置く記録用の名前（書庫のパスと宛先）。宛先の根は並びの全部を `; ` でつなぐ。
fn gate_label(
    path: &Path,
    roots: &BasewareRoots,
    archive: &NarArchive,
    target_ghost: Option<&str>,
) -> String {
    let manifest = archive.manifest();
    let target = target_ghost
        .map(|ghost| format!("・宛先のゴースト {ghost}"))
        .unwrap_or_default();
    let roots = roots
        .roots()
        .iter()
        .map(|root| root.dir().display().to_string())
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "{} → 根 {roots} の {:?} {}{target}",
        path.display(),
        manifest.kind,
        manifest.directory
    )
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod worker_tests;
