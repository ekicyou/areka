//! 起動の依頼の持ち物（`--install` と exe の隣の `initial_install.nar`・spec: startup-install-option）。
//!
//! このファイルの純粋な判断（消す書庫・`callghost` を頼むか・頼み方）は World にもファイルにも触らない。

use std::path::{Path, PathBuf};

use areka_ghost::same_folder_name;
use areka_kanade::ChangeOrigin;
use bevy_ecs::world::World;

use super::procedure::ArchiveEnd;
use super::{InstallOrder, InstallOrigin, SubmitVerdict, submit};
use crate::cli::InstallArgs;
use crate::emo2_boot::ghost_switch::{GhostSpec, SwitchAcceptCount, SwitchRequest};

/// exe の隣に置くと最初の定常到達で 1 回だけ入れる書庫の名前（大小を問わない＝要件 4.1）。
pub(crate) const INITIAL_INSTALL_FILE: &str = "initial_install.nar";

/// 起動の依頼の持ち物（`main` が起動の解決の前に作り、[`install`] で World の資源へ移す）。
#[derive(bevy_ecs::prelude::Resource, Debug, Default)]
pub(crate) struct StartupInstall {
    /// 最初の定常到達で取り出す（取り出したら None＝1 回だけ）。
    pending: Option<Pending>,
    /// 起動の引数の依頼が積まれた後の控え（終わりを受けたら None）。
    #[allow(dead_code)]
    // 読み手は同じ spec の on_order_done・discard_for_exit の控えの記録（タスク 3.3）
    args_followup: Option<ArgsFollowup>,
}

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

/// 起動の引数の依頼の終わりの控え（`submit` が `Queued` を返したときだけ置く）。
#[allow(dead_code)] // 読み手は同じ spec の on_order_done（タスク 3.3）
#[derive(Debug)]
struct ArgsFollowup {
    call_ghost: bool,
    delete_source: bool,
    /// 依頼を積んだときの切り替えの受付の数（要件 5.3 の比べの片方）。
    switches_at_submit: u64,
}

impl StartupInstall {
    /// `main` から 1 度だけ（`--help`・`--offline-dump` の早い戻りの後・起動の解決の前）。
    #[allow(dead_code)] // 読み手は同じ spec の main の結線（タスク 4.1）
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
            args_followup: None,
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
#[allow(dead_code)] // 読み手は同じ spec の main の結線（タスク 4.1）
pub(crate) fn install(world: &mut World, startup: StartupInstall) {
    world.insert_resource(startup);
}

fn take_pending(world: &mut World) -> Option<Pending> {
    world
        .get_resource_mut::<StartupInstall>()
        .and_then(|mut startup| startup.pending.take())
}

/// どの定常到達でも呼ばれる（最初の 1 回だけ依頼を出す＝要件 2.1・4.1）。積むだけで終わりを待たない（要件 2.6）。
#[allow(dead_code)] // 読み手は同じ spec の install::desk::on_steady（タスク 4.1）
pub(crate) fn on_steady(world: &mut World) {
    let Some(pending) = take_pending(world) else {
        return;
    };
    // initial_install.nar が先・別の依頼（要件 4.7）。
    match pending.initial_install() {
        Some(path) => {
            submit(
                world,
                InstallOrder {
                    archives: vec![path],
                    origin: InstallOrigin::InitialInstall,
                },
            );
        }
        None => tracing::debug!(
            event = "initial_install_absent",
            dir = ?pending.exe_dir,
            "[install] exe の隣に initial_install.nar はありません"
        ),
    }
    let Pending {
        archives,
        call_ghost,
        delete_source,
        ..
    } = pending;
    if archives.is_empty() {
        return;
    }
    let switches_at_submit = world.get_resource::<SwitchAcceptCount>().map_or(0, |c| c.0);
    let verdict = submit(
        world,
        InstallOrder {
            archives,
            origin: InstallOrigin::StartupArgs,
        },
    );
    // 断られたら控えない＝消さず頼まない（要件 6.3・6.4・7.4）。
    if verdict == SubmitVerdict::Queued
        && let Some(mut startup) = world.get_resource_mut::<StartupInstall>()
    {
        startup.args_followup = Some(ArgsFollowup {
            call_ghost,
            delete_source,
            switches_at_submit,
        });
    }
}

/// 終了の後始末（定常の前に終わるなら出さずに warn 1 件＝要件 2.7）。
#[allow(dead_code)] // 読み手は同じ spec の install::desk::discard_for_exit（タスク 4.1）
pub(crate) fn discard_for_exit(world: &mut World) {
    if let Some(pending) = take_pending(world) {
        pending.record_not_submitted("exit_before_steady");
    }
}

/// 純粋: 消す書庫（`Installed` のパスだけ・重ねは最初の 1 つ・書いた順＝要件 3.1・3.2・4.3・4.4）。
///
/// `ends` が短い（途中でやめた）とき、終わり方の無い書庫は「まだ扱っていない」＝消さない。
#[allow(dead_code)] // 読み手は同じ spec の on_order_done（タスク 3.3）
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

/// `callghost` の判断に要る事実（集めるのは on_order_done）。
#[allow(dead_code)] // 作り手は同じ spec の on_order_done（タスク 3.3）
pub(crate) struct CallGhostFacts<'a> {
    pub archives: usize,
    pub ends: &'a [ArchiveEnd],
    /// 窓口の門が閉じている（終了が始まっている）。
    pub closing: bool,
    /// 切替の予約 `SwitchInFlight` が在る。
    pub switching: bool,
    /// 依頼を積んだとき・終わりを受けたときの切り替えの受付の数（`SwitchAcceptCount`）。
    pub switches_at_submit: u64,
    pub switches_now: u64,
    /// 今のゴーストのフォルダ名（`AlreadyRunning` の比べ）。
    pub ghost_now: Option<&'a str>,
}

/// `callghost` の判断の答え。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallGhost {
    /// 切り替え先のフォルダ名（最後に入ったゴーストの書庫の本体の `place` の末尾）。
    Request {
        folder: String,
    },
    Skip(CallGhostSkip),
}

/// 頼まない理由（記録 `startup_callghost_skipped` の `reason`）。
#[allow(dead_code)] // 理由の語へ下ろす読み手は同じ spec の on_order_done（タスク 3.3）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallGhostSkip {
    Abandoned,
    Closing,
    NoGhostInstalled,
    Switching,
    SwitchedMeanwhile,
    AlreadyRunning,
}

/// 純粋: `callghost` を頼むか（design「`callghost` の判断」の順＝要件 5.1〜5.4）。
#[allow(dead_code)] // 読み手は同じ spec の on_order_done（タスク 3.3）
pub(crate) fn decide_call_ghost(facts: &CallGhostFacts<'_>) -> CallGhost {
    let ends = facts.ends;
    if ends.len() < facts.archives || ends.contains(&ArchiveEnd::Abandoned) {
        return CallGhost::Skip(CallGhostSkip::Abandoned);
    }
    if facts.closing {
        return CallGhost::Skip(CallGhostSkip::Closing);
    }
    let folder = ends.iter().rev().find_map(|end| match end {
        ArchiveEnd::Installed(items) if items.first().is_some_and(|i| i.kind == "ghost") => {
            Path::new(&items[0].place)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        }
        _ => None,
    });
    let Some(folder) = folder else {
        return CallGhost::Skip(CallGhostSkip::NoGhostInstalled);
    };
    if facts.switching {
        return CallGhost::Skip(CallGhostSkip::Switching);
    }
    if facts.switches_now != facts.switches_at_submit {
        return CallGhost::Skip(CallGhostSkip::SwitchedMeanwhile);
    }
    if facts
        .ghost_now
        .is_some_and(|now| same_folder_name(now, &folder))
    {
        return CallGhost::Skip(CallGhostSkip::AlreadyRunning);
    }
    CallGhost::Request { folder }
}

/// 純粋: メニューの「ゴースト」と同じ形の頼み（Folder・raise_event 真・Manual・boot_event なし＝要件 5.1・10.1）。
#[allow(dead_code)] // 読み手は同じ spec の on_order_done（タスク 3.3）
pub(crate) fn callghost_request(folder: String) -> SwitchRequest {
    SwitchRequest {
        ghost: GhostSpec::Folder(folder),
        raise_event: true,
        origin: ChangeOrigin::Manual,
        boot_event: None,
    }
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
