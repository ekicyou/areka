//! 起動の依頼の持ち物（`--install` と exe の隣の `initial_install.nar`・spec: startup-install-option）。
//!
//! このファイルの純粋な判断（消す書庫・`callghost` の相手）は World にもファイルにも触らない。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoots;
use bevy_ecs::world::World;

use super::before_boot_ports::BootPorts;
use super::procedure::{ArchiveEnd, run_order};
use super::{InstallOrder, InstallOrigin};
use crate::cli::{BootRequest, InstallArgs};

/// exe の隣に置くと起動のたびに 1 回だけ入れる書庫の名前（大小を問わない＝要件 4.1）。
pub(crate) const INITIAL_INSTALL_FILE: &str = "initial_install.nar";

/// 起動の依頼の持ち物（`main` が起動の解決の前に作って持ち、[`install`] で控えを World へ移して落とす）。
#[derive(Debug, Default)]
pub(crate) struct StartupInstall {
    /// 入れる物（根が決まったときに取り出す＝1 回だけ）。
    pending: Option<Pending>,
    /// 起動の前に入れた物の控え（入れた順・起動の後に World へ移す）。
    carried: Vec<CarriedInstall>,
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

/// 起動の前に入れた物 1 本の控え（目録の綴りに直した後の値・起動の後に World へ移す）。
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
            carried: Vec::new(),
        }
    }

    /// 根が決まった後に 1 回だけ（2 回目からは何もせず None）。`initial_install.nar` の依頼を先に、
    /// `--install` の並びを後に、別々の依頼として起動の前の口で手続きへ渡し、入った書庫を消し
    /// （`initial_install.nar` はいつも・`--install` は `deletesource` のときだけ）、控えを self に残す。
    /// 読み取りだけなら出どころごとに断り、何も入れず何も消さない。答えは `callghost` で起こす
    /// ゴーストのフォルダ名（使うときだけ・要件 1.1〜1.3・1.5・4.9・5.1〜5.7・6.1・6.2・7.1〜7.4）。
    pub(crate) fn run_before_boot(
        &mut self,
        roots: &BasewareRoots,
        request: &BootRequest,
        before: BeforeBootEnv,
    ) -> Option<String> {
        let pending = self.pending.take()?;
        let orders: Vec<InstallOrder> = [
            pending.initial_install().map(|path| InstallOrder {
                archives: vec![path],
                origin: InstallOrigin::InitialInstall,
            }),
            (!pending.archives.is_empty()).then(|| InstallOrder {
                archives: pending.archives.clone(),
                origin: InstallOrigin::StartupArgs,
            }),
        ]
        .into_iter()
        .flatten()
        .collect();
        if orders.is_empty() {
            tracing::debug!(
                event = "startup_install_nothing",
                "[install] 起動の前に入れる物はありません"
            );
            return None;
        }
        let mut args_ends = Vec::new();
        if before.readonly {
            for order in &orders {
                super::record_readonly_refused(order.origin, order.archives.len());
            }
        } else {
            args_ends = self.install_orders(roots, &orders, pending.delete_source, before);
        }
        let facts = CallGhostFacts {
            requested: pending.call_ghost,
            readonly: before.readonly,
            boot_target: BootTarget::of(request),
            ends: &args_ends,
        };
        match decide_call_ghost(&facts) {
            CallGhost::Choose { folder } => Some(folder),
            CallGhost::Skip(_) | CallGhost::NotRequested => None,
        }
    }

    /// 依頼を順に手続きへ渡し、記録して入った書庫を消す。答えは `--install` の依頼の終わり方
    /// （`initial_install.nar` の分は `callghost` の判断へ渡さない＝要件 4.9）。
    fn install_orders(
        &mut self,
        roots: &BasewareRoots,
        orders: &[InstallOrder],
        delete_source: bool,
        before: BeforeBootEnv,
    ) -> Vec<ArchiveEnd> {
        let mut args_ends = Vec::new();
        let mut ports = BootPorts::new(roots.clone(), before.alert_suppressed);
        for order in orders {
            tracing::info!(
                event = "startup_install_started",
                stage = "before_boot",
                origin = ?order.origin,
                count = order.archives.len(),
                "[install] 起動の前に起動の依頼を扱い始めます"
            );
            let ends = run_order(order, &mut ports);
            let results: Vec<(&Path, &str)> = order
                .archives
                .iter()
                .enumerate()
                .map(|(index, archive)| (archive.as_path(), end_word(ends.get(index))))
                .collect();
            tracing::info!(
                event = "startup_install_done",
                stage = "before_boot",
                origin = ?order.origin,
                results = ?results,
                "[install] 起動の依頼が終わりました"
            );
            let is_args = order.origin == InstallOrigin::StartupArgs;
            if !is_args || delete_source {
                delete_sources(order.origin, &sources_to_delete(&order.archives, &ends));
            }
            if is_args {
                args_ends = ends;
            }
        }
        self.carried.extend(ports.take_carried());
        args_ends
    }
}

/// 起動の前の道が読むプロセスの旗（本番は `readonly::is_on` と、告知の抑止または有界の自動終了）。
#[derive(Clone, Copy)]
pub(crate) struct BeforeBootEnv {
    pub readonly: bool,
    pub alert_suppressed: bool,
}

/// 起動の前の告知の抑止（純粋）: `AREKA_NO_ALERT` の抑止か、`AREKA_APP_SMOKE_EXIT_MS` が立つ
/// 有界の自動終了の走行（人が見ていない・自動終了の時計は窓のアプリの後に始まる）なら画面を出さない。
pub(crate) fn before_boot_alert_suppressed(no_alert: bool, smoke_exit_ms: Option<u64>) -> bool {
    no_alert || smoke_exit_ms.is_some()
}

/// 本物の口（`main` から 1 回）。env・記憶・exe の場所・読み取りだけの旗・告知の抑止を読み、
/// [`resolve_boot_with_env`] を本番の根の解決と選び方で呼ぶ。
// main の呼ぶ所（タスク 5.1）が呼ぶまでは使われない。
#[allow(dead_code, reason = "5.1 が呼ぶ")]
pub(crate) fn resolve_boot(
    request: &BootRequest,
    startup: &mut StartupInstall,
) -> Result<crate::boot_config::BootResolved, crate::alert::AlertScene> {
    let app_profile_dir = crate::boot_config::default_app_profile_dir();
    let before = BeforeBootEnv {
        readonly: crate::readonly::is_on(),
        alert_suppressed: before_boot_alert_suppressed(
            crate::alert::suppressed(),
            crate::smoke_exit::smoke_exit_ms(),
        ),
    };
    resolve_boot_with_env(
        crate::roots::resolve_roots(&app_profile_dir),
        request,
        startup,
        before,
        &app_profile_dir,
        crate::boot_resolve::pick_index,
        crate::roots::user_root::writable_ghost,
    )
}

/// 中身（テストは根の結果・旗・選び方を差し込む）。根が決まれば起動の前の道を通し、決まらなければ
/// 入れる物を取り出して warn `root_missing` を 1 件残す。どちらの後も、`callghost` の相手を添えて
/// 起動の解決へ渡す（根の記録と告知の場面は `resolve_boot_with` が出す＝要件 1.1・1.7・4.1・6.3）。
pub(crate) fn resolve_boot_with_env(
    resolved: Result<crate::roots::ResolvedRoots, crate::roots::RootError>,
    request: &BootRequest,
    startup: &mut StartupInstall,
    before: BeforeBootEnv,
    app_profile_dir: &Path,
    pick: fn(usize) -> usize,
    prepare_ghost: fn(&BasewareRoots, &Path) -> PathBuf,
) -> Result<crate::boot_config::BootResolved, crate::alert::AlertScene> {
    let call_ghost = match &resolved {
        Ok(found) => startup.run_before_boot(&found.roots, request, before),
        Err(_) => {
            if let Some(pending) = startup.pending.take() {
                pending.record_not_submitted("root_missing");
            }
            None
        }
    };
    crate::boot_config::resolve_boot_with(
        resolved,
        request,
        call_ghost.as_deref(),
        app_profile_dir,
        pick,
        prepare_ghost,
    )
}

impl Drop for StartupInstall {
    fn drop(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.record_not_submitted("boot_stopped");
        }
    }
}

/// `main` から 1 度だけ（系の登録の後）。控えを入れた順に、窓口の記録と同じ受け皿と置換語へ置き
/// （最後の 1 本が「最後に入れた物」）、持ち物を落とす。記憶へは書かず、ゴーストへイベントは送らない
/// （要件 3.2・3.4）。
pub(crate) fn install(world: &mut World, mut startup: StartupInstall) {
    let carried = std::mem::take(&mut startup.carried);
    if !carried.is_empty() {
        tracing::info!(
            event = "startup_install_carried",
            count = carried.len(),
            "[install] 起動の前に入れた物の控えを起動の後へ引き継ぎます"
        );
    }
    for installed in carried {
        super::desk::place_last_installed(world, installed);
    }
}

/// 終わり方の語（記録 `startup_install_done` の `results`・並びの残りは `not_started`）。
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

#[cfg(test)]
#[path = "startup_flow_tests.rs"]
mod flow_tests;
