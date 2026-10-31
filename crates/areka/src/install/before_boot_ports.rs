//! 起動の前の手続きの口（design「BootPorts」・spec: install-before-boot）。
//!
//! ゴーストも窓口も居ない起動の前に、手続き [`run_order`](super::procedure::run_order) の口として
//! 答える。イベントはどれも送らない（送る相手を持たない）・今のゴーストの素性は「在る・中身は空」・
//! 入れた物は目録の綴りに直して控え、起動の後に引き継ぐ。置き場の決まりと画面は、窓へ落としたときと
//! 同じもの（[`install_into`]・[`alert`]）を直に呼ぶ。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoots;
use areka_nar::{InstallKind, InstallOutcome, NarArchive, NarError};

use super::desk::catalog_spelling;
use super::destination::install_into;
use super::judge::GhostFacts;
use super::procedure::{InstallPorts, InstalledRecord, ON_INSTALL_REFUSE, Overwritten, Raised};
use super::startup::CarriedInstall;
use super::terms::TermsNotice;
use crate::alert::{self, YesNo};

/// 起動の前の口。根の並びと告知の抑止と、入れた物の控えを持つ。
pub(crate) struct BootPorts {
    roots: BasewareRoots,
    /// 真なら利用条件と警告の画面を出さない（`AREKA_NO_ALERT` か `AREKA_APP_SMOKE_EXIT_MS`）。
    alert_suppressed: bool,
    /// 今扱っている書庫のパス（`begin_archive` で覚える・記録に載せる）。
    archive: PathBuf,
    /// 入れた物の控え（入れた順）。
    carried: Vec<CarriedInstall>,
}

impl BootPorts {
    pub(crate) fn new(roots: BasewareRoots, alert_suppressed: bool) -> BootPorts {
        BootPorts {
            roots,
            alert_suppressed,
            archive: PathBuf::new(),
            carried: Vec::new(),
        }
    }

    /// 控えを取り出す（入れた順）。
    pub(crate) fn take_carried(&mut self) -> Vec<CarriedInstall> {
        std::mem::take(&mut self.carried)
    }

    /// 画面を出す前に、プロセスの DPI の扱いを立てる（UI を作る前に呼ぶ決まり・抑止なら出さない）。
    fn before_dialog(&self) {
        if !self.alert_suppressed {
            wintf::ensure_process_dpi_awareness();
        }
    }
}

impl InstallPorts for BootPorts {
    fn begin_archive(&mut self, path: &Path) {
        self.archive = path.to_path_buf();
    }

    fn raise(&mut self, id: &'static str, _references: Vec<String>) -> Raised {
        // 起動の前の断りは、どれも受け取るゴーストが居ないため（要件 2.5）。
        if id == ON_INSTALL_REFUSE {
            tracing::info!(
                event = "install_refused_no_ghost",
                archive = %self.archive.display(),
                "[install] 起動の前なので受け取るゴーストが居らず、書庫を入れませんでした"
            );
        }
        Raised::Unsent
    }

    fn ghost_facts(&mut self) -> Option<GhostFacts> {
        Some(GhostFacts {
            roots: self.roots.clone(),
            folder: None,
            running_dir: None,
            name: String::new(),
            sakura_name: None,
            install_accept: Vec::new(),
        })
    }

    fn ask_terms(&mut self, title: &str, notice: &TermsNotice) -> YesNo {
        self.before_dialog();
        alert::ask_yes_no(title, &notice.body, self.alert_suppressed)
    }

    fn install_elsewhere(
        &mut self,
        archive: &NarArchive,
        target_ghost: Option<&str>,
    ) -> Option<Result<InstallOutcome, NarError>> {
        // 起動中のゴーストは居ない・終了の門も無い。
        Some(install_into(&self.roots, archive, target_ghost, None))
    }

    fn overwrite_running(
        &mut self,
        archive: NarArchive,
        _target_ghost: Option<String>,
    ) -> Overwritten {
        // 素性に起動中のゴーストが無いので来ない。来ても落とさず、ほかの所へ入れる道へ倒す。
        tracing::debug!(
            event = "install_overwrite_without_running",
            archive = %self.archive.display(),
            "[install] 起動の前なので起動中のゴーストは無く、ほかの所へ入れます"
        );
        Overwritten::NotRunning(archive)
    }

    fn record(&mut self, record: InstalledRecord) {
        let (folder, ghost_name) = catalog_spelling(Some(&self.roots), &record);
        if record.kind == InstallKind::Balloon {
            tracing::info!(
                event = "install_balloon_memory_skipped",
                folder = folder.as_str(),
                "[install] 起動の前なので、入れたバルーンをどのゴーストの記憶にも書きません"
            );
        }
        self.carried.push(CarriedInstall {
            kind: record.kind,
            folder,
            ghost_folder: record.ghost_folder,
            object_name: record.object_name,
            ghost_name,
        });
    }

    fn warn_skipped(&mut self, title: &str, body: &str) {
        self.before_dialog();
        alert::warn_ok(title, body, self.alert_suppressed);
    }
}

#[cfg(test)]
#[path = "before_boot_ports_tests.rs"]
mod before_boot_ports_tests;
