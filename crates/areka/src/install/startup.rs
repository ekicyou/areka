//! 起動の依頼の持ち物（`--install` と exe の隣の `initial_install.nar`・spec: startup-install-option）。
//!
//! このファイルの純粋な判断（消す書庫・`callghost` の相手）は World にもファイルにも触らない。

use std::path::{Path, PathBuf};

use bevy_ecs::world::World;

use super::InstallOrigin;
use super::procedure::ArchiveEnd;
use crate::cli::{BootRequest, InstallArgs};

/// exe の隣に置くと起動のたびに 1 回だけ入れる書庫の名前（大小を問わない＝要件 4.1）。
pub(crate) const INITIAL_INSTALL_FILE: &str = "initial_install.nar";

/// 起動の依頼の持ち物（`main` が起動の解決の前に作り、[`install`] で World の資源へ移す）。
#[derive(bevy_ecs::prelude::Resource, Debug, Default)]
pub(crate) struct StartupInstall {
    /// 入れる物（取り出したら None＝1 回だけ）。
    pending: Option<Pending>,
}

// 起動の前の道（タスク 4.x）が呼ぶまでは使い手が無い（定常の後の道はタスク 4.1 で外した）。
#[allow(dead_code)]
#[derive(Debug)]
struct Pending {
    /// 絶対に直した `--install` の値（書いた順）。
    archives: Vec<PathBuf>,
    call_ghost: bool,
    delete_source: bool,
    /// exe の本当の場所のフォルダ（解けなければ None＝`initial_install.nar` を見ない）。
    exe_dir: Option<PathBuf>,
}

impl Pending {
    /// exe の隣の `initial_install.nar`（在るときだけ）。
    fn initial_install(&self) -> Option<PathBuf> {
        self.exe_dir
            .as_ref()
            .map(|dir| dir.join(INITIAL_INSTALL_FILE))
            .filter(|path| path.is_file())
    }

    /// 出さずに終わる記録（入れる物が在るときだけ warn 1 件＝要件 2.7・7.1・7.2）。
    fn record_not_submitted(&self, reason: &'static str) {
        let initial_install = self.initial_install();
        if self.archives.is_empty() && initial_install.is_none() {
            return;
        }
        tracing::warn!(
            event = "startup_install_not_submitted",
            reason,
            archives = ?self.archives,
            initial_install = ?initial_install,
            "[install] 起動の依頼を出さずに終わります"
        );
    }
}

/// 起動の前に入れた物 1 本の控え（目録の綴りに直した後の値・起動の後に World へ移す）。
// 起動の前の道（タスク 4.x）が持ち物に載せるまでは、起動の前の口とテストだけが使う。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CarriedInstall {
    pub kind: areka_nar::InstallKind,
    /// 入れた先のフォルダ名（目録の綴り）。
    pub folder: String,
    /// 関わったゴーストのフォルダ名（シェル・追加ファイル）。
    pub ghost_folder: Option<String>,
    /// %lastobjectname の値（install.txt の name）。
    pub object_name: String,
    /// %lastghostname の値（バルーンは None＝前の値のまま）。
    pub ghost_name: Option<String>,
}

impl StartupInstall {
    /// `main` から 1 度だけ（`--help`・`--offline-dump` の早い戻りの後・起動の解決の前）。
    pub(crate) fn new(args: InstallArgs) -> StartupInstall {
        StartupInstall::new_with(args, crate::roots::user_root::exe_dir())
    }

    /// `new` の中身（テストは exe の隣の代わりに `target\` の下のフォルダを渡す）。areka はどこでも
    /// 今のフォルダを変えないので、ここで直すのは「起動したときの今のフォルダ」から読むのと同じ（要件 1.2）。
    pub(crate) fn new_with(args: InstallArgs, exe_dir: Option<PathBuf>) -> StartupInstall {
        let archives = args
            .archives
            .into_iter()
            .map(|archive| match std::path::absolute(&archive) {
                Ok(path) => path,
                Err(error) => {
                    tracing::warn!(
                        event = "install_path_not_absolute",
                        archive = %archive.display(),
                        %error,
                        "[cli] --install のパスを絶対に直せないので、書いた綴りのまま渡します"
                    );
                    archive
                }
            })
            .collect();
        StartupInstall {
            pending: Some(Pending {
                archives,
                call_ghost: args.call_ghost,
                delete_source: args.delete_source,
                exe_dir,
            }),
        }
    }
}

impl Drop for StartupInstall {
    fn drop(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.record_not_submitted("boot_stopped");
        }
    }
}

/// `main` から 1 度だけ（`readonly::install` の次）。持ち物を World の資源へ移す。
pub(crate) fn install(world: &mut World, startup: StartupInstall) {
    world.insert_resource(startup);
}

/// 終わり方の語（記録 `startup_install_done` の `results`・並びの残りは `not_started`）。
// 起動の前の道（タスク 4.x）が呼ぶまでは使い手が無い（定常の後の道はタスク 4.1 で外した）。
#[allow(dead_code)]
fn end_word(end: Option<&ArchiveEnd>) -> &'static str {
    match end {
        Some(ArchiveEnd::Installed(_)) => "installed",
        Some(ArchiveEnd::Refused) => "refused",
        Some(ArchiveEnd::Declined) => "declined",
        Some(ArchiveEnd::Failed(_)) => "failed",
        Some(ArchiveEnd::Abandoned) => "abandoned",
        None => "not_started",
    }
}

/// 1 本ずつ `remove_file`（フォルダは消えない・1 本の失敗でほかをやめない＝要件 3.3・3.4・3.6・4.5）。
// 起動の前の道（タスク 4.x）が呼ぶまでは使い手が無い（定常の後の道はタスク 4.1 で外した）。
#[allow(dead_code)]
fn delete_sources(origin: InstallOrigin, archives: &[PathBuf]) {
    for archive in archives {
        match std::fs::remove_file(archive) {
            Ok(()) => tracing::info!(
                event = "startup_source_deleted",
                origin = ?origin,
                archive = %archive.display(),
                "[install] 入った書庫を消しました"
            ),
            Err(error) => tracing::warn!(
                event = "startup_source_delete_failed",
                origin = ?origin,
                archive = %archive.display(),
                %error,
                "[install] 書庫を消せないので残して次へ進みます"
            ),
        }
    }
}

/// 純粋: 消す書庫（`Installed` のパスだけ・重ねは最初の 1 つ・書いた順＝要件 3.1・3.2・4.3・4.4）。
///
/// `ends` が短い（途中でやめた）とき、終わり方の無い書庫は「まだ扱っていない」＝消さない。
// 起動の前の道（タスク 4.x）が呼ぶまでは使い手が無い（定常の後の道はタスク 4.1 で外した）。
#[allow(dead_code)]
pub(crate) fn sources_to_delete(archives: &[PathBuf], ends: &[ArchiveEnd]) -> Vec<PathBuf> {
    let key = |path: &Path| path.to_string_lossy().to_lowercase();
    let mut out: Vec<PathBuf> = Vec::new();
    for (index, archive) in archives.iter().enumerate() {
        let lower = key(archive);
        if out.iter().any(|done| key(done) == lower) {
            continue;
        }
        // 同じパスのどれか 1 つでも入ったなら、最初の綴りで 1 回だけ。
        let installed = archives
            .iter()
            .zip(ends)
            .skip(index)
            .any(|(other, end)| key(other) == lower && matches!(end, ArchiveEnd::Installed(_)));
        if installed {
            out.push(archive.clone());
        }
    }
    out
}

/// `callghost` の判断に要る事実（design「CallGhostDecision」）。
// 起動の前の道（タスク 4.4）が呼ぶまでは使い手が無い。
#[allow(dead_code)]
pub(crate) struct CallGhostFacts<'a> {
    /// `--option callghost` が `--install` と一緒に渡された（cli が下ろした後の値）。
    pub requested: bool,
    pub readonly: bool,
    /// 起動するゴーストの指定（位置引数・`--ghost` の値・`--random` のどれか）。在れば使わない。
    pub boot_target: Option<BootTarget>,
    /// `--install` の依頼の終わり方（`initial_install.nar` の分は渡さない＝要件 4.9）。
    pub ends: &'a [ArchiveEnd],
}

/// 起動するゴーストの指定の種類（記録 `startup_callghost_skipped` の `target`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BootTarget {
    Positional,
    GhostOption,
    Random,
}

// 起動の前の道（タスク 4.4）が呼ぶまでは使い手が無い。
#[allow(dead_code)]
impl BootTarget {
    /// 起動の求めに値が在るか（`--ghost` が置き場に当たらなくても指定あり＝要件 4.5）。
    pub(crate) fn of(request: &BootRequest) -> Option<BootTarget> {
        if request.positional_ghost.is_some() {
            Some(BootTarget::Positional)
        } else if request.ghost.is_some() {
            Some(BootTarget::GhostOption)
        } else if request.random {
            Some(BootTarget::Random)
        } else {
            None
        }
    }

    fn word(self) -> &'static str {
        match self {
            BootTarget::Positional => "positional",
            BootTarget::GhostOption => "ghost_option",
            BootTarget::Random => "random",
        }
    }
}

/// `callghost` の判断の答え。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallGhost {
    /// 起こすゴーストのフォルダ名（並びの最後に入ったゴーストの書庫の本体の `place` の末尾）。
    Choose {
        folder: String,
    },
    Skip(CallGhostSkip),
    /// `callghost` が渡されていない（記録を出さない）。
    NotRequested,
}

/// 使わない理由（記録 `startup_callghost_skipped` の `reason`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallGhostSkip {
    Readonly,
    BootTargetGiven(BootTarget),
    NoGhostInstalled,
}

impl CallGhostSkip {
    fn word(self) -> &'static str {
        match self {
            CallGhostSkip::Readonly => "readonly",
            CallGhostSkip::BootTargetGiven(_) => "boot_target_given",
            CallGhostSkip::NoGhostInstalled => "no_ghost_installed",
        }
    }
}

/// `callghost` を使うか（順＝渡されていない → 読み取りだけ → 指定あり → ゴーストが無い → 選ぶ・
/// 要件 4.3〜4.6・7.4）。判断は事実だけから決まり、答えを info で 1 件残す（`NotRequested` は残さない）。
// 起動の前の道（タスク 4.4）が呼ぶまでは使い手が無い。
#[allow(dead_code)]
pub(crate) fn decide_call_ghost(facts: &CallGhostFacts<'_>) -> CallGhost {
    let answer = if !facts.requested {
        CallGhost::NotRequested
    } else if facts.readonly {
        CallGhost::Skip(CallGhostSkip::Readonly)
    } else if let Some(target) = facts.boot_target {
        CallGhost::Skip(CallGhostSkip::BootTargetGiven(target))
    } else {
        let folder = facts.ends.iter().rev().find_map(|end| match end {
            ArchiveEnd::Installed(items) if items.first().is_some_and(|i| i.kind == "ghost") => {
                Path::new(&items[0].place)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            }
            _ => None,
        });
        match folder {
            Some(folder) => CallGhost::Choose { folder },
            None => CallGhost::Skip(CallGhostSkip::NoGhostInstalled),
        }
    };
    match &answer {
        CallGhost::Choose { folder } => tracing::info!(
            event = "startup_callghost_chosen",
            folder = %folder,
            origin = ?InstallOrigin::StartupArgs,
            "[install] 最後に入ったゴーストを起こすゴーストに選びました"
        ),
        CallGhost::Skip(reason) => {
            let target = match reason {
                CallGhostSkip::BootTargetGiven(target) => Some(target.word()),
                _ => None,
            };
            tracing::info!(
                event = "startup_callghost_skipped",
                reason = reason.word(),
                target,
                "[install] callghost を使いません"
            )
        }
        CallGhost::NotRequested => {}
    }
    answer
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
