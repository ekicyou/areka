//! 置き場の追加・外す・先頭へ動かすの判断と適用。
//!
//! 判断は今の並び・出どころ・足すフォルダ（または外す根の絶対パス）で決める。足す判断だけは足すフォルダの直下を見る。
//! 先頭へ動かす判断だけは実行ファイルの隣（`exe_dir`）も見る。
//! フォルダそのものは消さない・動かさない。触るのは並びの値だけ。
//! 適用（起動の文脈・記憶）は UI スレッドの入口 [`add_root`]・[`remove_root`]・[`move_to_front`]。
//! 変更のときの並びと隠れの記録は 1 本のスレッド `roots-log` が変更の順に残す（UI スレッドは根を見に行かない）。
//! 並べ替えは「先頭へ動かす」だけ。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, PoisonError};

use areka_ghost::{BasewareRoot, BasewareRoots, RootMark, same_root};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistOutcome, PersistScope, ScopeRoots, save_scope};
use bevy_ecs::world::World;

use areka_session::boot_config::BootContext;
use areka_session::ghost_session::GhostSlot;
use areka_session::roots::report::{ReportOccasion, log_hidden, log_roots};
use areka_session::roots::{ResolvedRoots, RootStatus, RootsOrigin, encode_memory};

/// 追加の判断。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AddVerdict {
    /// 末尾に足した。`empty` は直下に ghost も balloon も無い（warn）。
    Added { roots: BasewareRoots, empty: bool },
    /// すでにある（warn・変えない）。
    AlreadyListed { existing: PathBuf },
    /// `AREKA_ROOT` で渡している（変えない）。
    EnvLocked,
}

/// 外すの判断。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RemoveVerdict {
    Removed {
        roots: BasewareRoots,
        removed: PathBuf,
    },
    /// 覚える置き場が 1 つしか無い（変えない）。
    LastOne,
    EnvLocked,
    /// 並びに無い（メニューを出した後に並びが変わった・warn・変えない）。
    Stale,
    /// 一時の置き場だけに一致した（warn・変えない）。
    Temporary,
    /// 書ける根の印の項目（変えない）。
    UserRoot,
}

/// 先頭へ動かすの判断。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FrontVerdict {
    /// 先頭へ動かした（ほかの順は保つ）。
    Moved { roots: BasewareRoots },
    /// すでに先頭（変えない・warn）。
    AlreadyFront,
    /// `AREKA_ROOT` で渡している（変えない・warn）。
    EnvLocked,
    /// 並びに無い（メニューを出した後に並びが変わった・warn）。
    Stale,
    /// 一時の置き場だけに一致した（warn・変えない）。
    Temporary,
    /// 書ける根の印があるときの実行ファイルの隣（変えない・warn）。
    ExeSide,
}

/// 覚える置き場（一時でない項目）のうち `dir` と同じ根の位置（並び全体での位置）。
fn remembered_position(roots: &BasewareRoots, dir: &Path) -> Option<usize> {
    roots
        .roots()
        .iter()
        .position(|r| !r.is_temporary() && same_root(r.dir(), dir))
}

/// 覚える置き場には無く、一時の置き場だけに一致するか。
fn only_temporary(roots: &BasewareRoots, dir: &Path) -> bool {
    roots
        .roots()
        .iter()
        .any(|r| r.is_temporary() && same_root(r.dir(), dir))
}

/// 追加: 末尾に足す。一時の置き場は重なりに数えない＝同じフォルダは普通の置き場として足す。
/// 実行ファイルのフォルダも絶対パスのまま入る（記憶へは `encode_memory` が `<exe>` で書く）。
pub(crate) fn decide_add(roots: &BasewareRoots, origin: RootsOrigin, picked: &Path) -> AddVerdict {
    if origin == RootsOrigin::EnvVar {
        return AddVerdict::EnvLocked;
    }
    if let Some(index) = remembered_position(roots, picked) {
        return AddVerdict::AlreadyListed {
            existing: roots.roots()[index].dir().to_path_buf(),
        };
    }
    let added = BasewareRoot::new(picked.to_path_buf());
    let empty = !added.ghost_store().is_dir() && !added.balloon_store().is_dir();
    let mut list = roots.roots().to_vec();
    list.push(added);
    AddVerdict::Added {
        roots: BasewareRoots::new(list),
        empty,
    }
}

/// 外す: メニューに出した根の絶対パスで受け、並びの値から外すだけ。
/// 一時の置き場は外さず、「最後の 1 つ」は覚える置き場で数える。
pub(crate) fn decide_remove(
    roots: &BasewareRoots,
    origin: RootsOrigin,
    dir: &Path,
) -> RemoveVerdict {
    if origin == RootsOrigin::EnvVar {
        return RemoveVerdict::EnvLocked;
    }
    let Some(index) = remembered_position(roots, dir) else {
        if only_temporary(roots, dir) {
            return RemoveVerdict::Temporary;
        }
        return RemoveVerdict::Stale;
    };
    if roots.roots().iter().filter(|r| !r.is_temporary()).count() == 1 {
        return RemoveVerdict::LastOne;
    }
    if matches!(roots.roots()[index].mark(), RootMark::UserRoot { .. }) {
        return RemoveVerdict::UserRoot;
    }
    let mut list = roots.roots().to_vec();
    let removed = list.remove(index).dir().to_path_buf();
    RemoveVerdict::Removed {
        roots: BasewareRoots::new(list),
        removed,
    }
}

/// 先頭へ動かす: 項目を複写して動かす（書ける根の印は残る）。動かす先は覚える置き場の先頭＝一時の項目の直後で、
/// 一時の置き場は動かさない。
/// 書ける根の印があるとき、実行ファイルの隣（`exe_dir`）は動かさない。
pub(crate) fn decide_front(
    roots: &BasewareRoots,
    origin: RootsOrigin,
    dir: &Path,
    exe_dir: Option<&Path>,
) -> FrontVerdict {
    if origin == RootsOrigin::EnvVar {
        return FrontVerdict::EnvLocked;
    }
    let front = roots
        .roots()
        .iter()
        .take_while(|r| r.is_temporary())
        .count();
    let marked = roots
        .roots()
        .iter()
        .any(|r| matches!(r.mark(), RootMark::UserRoot { .. }));
    let exe_side = marked && exe_dir.is_some_and(|exe| same_root(exe, dir));
    match remembered_position(roots, dir) {
        None if only_temporary(roots, dir) => FrontVerdict::Temporary,
        None => FrontVerdict::Stale,
        Some(index) if index == front => FrontVerdict::AlreadyFront,
        Some(_) if exe_side => FrontVerdict::ExeSide,
        Some(index) => {
            let mut list = roots.roots().to_vec();
            let picked = list.remove(index);
            list.insert(front, picked);
            FrontVerdict::Moved {
                roots: BasewareRoots::new(list),
            }
        }
    }
}

/// UI スレッドの入口: 置き場を並びの先頭へ動かす（今のゴースト・バルーンは切り替えない）。
/// 記憶へ書けなかった結果は `apply` の中で扱う。
pub(crate) fn move_to_front(world: &mut World, dir: PathBuf) {
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::error!(
            event = "roots_change_no_context",
            "[RootsEdit] 起動の文脈が無いので置き場を先頭へ動かせません"
        );
        return;
    };
    let exe_dir = areka_session::roots::user_root::exe_dir();
    let reason = match decide_front(&ctx.roots, ctx.roots_origin, &dir, exe_dir.as_deref()) {
        FrontVerdict::Moved { roots } => {
            tracing::info!(
                event = "roots_moved_to_front",
                path = %dir.display(),
                "[RootsEdit] 置き場を並びの先頭へ動かしました"
            );
            return apply(world, roots, exe_dir);
        }
        FrontVerdict::AlreadyFront => "already_front",
        FrontVerdict::EnvLocked => "env_var",
        FrontVerdict::Stale => "stale",
        FrontVerdict::Temporary => "temporary",
        FrontVerdict::ExeSide => "exe_side",
    };
    tracing::warn!(
        event = "roots_front_refused",
        path = %dir.display(),
        reason,
        "[RootsEdit] 置き場を先頭へ動かせないので変えません"
    );
}

/// UI スレッドの入口: 置き場を足す（判断・適用・記録）。
pub(crate) fn add_root(world: &mut World, picked: PathBuf) {
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::error!(
            event = "roots_change_no_context",
            "[RootsEdit] 起動の文脈が無いので置き場を足せません"
        );
        return;
    };
    match decide_add(&ctx.roots, ctx.roots_origin, &picked) {
        AddVerdict::Added { roots, empty } => {
            if empty {
                tracing::warn!(
                    event = "roots_add_empty_root",
                    path = %picked.display(),
                    "[RootsEdit] 足した置き場の直下に ghost も balloon もありません"
                );
            }
            apply(world, roots, areka_session::roots::user_root::exe_dir());
        }
        AddVerdict::AlreadyListed { existing } => tracing::warn!(
            event = "roots_add_already_listed",
            path = %picked.display(),
            existing = %existing.display(),
            "[RootsEdit] すでに並びにある置き場なので変えません"
        ),
        AddVerdict::EnvLocked => tracing::warn!(
            event = "roots_add_refused",
            path = %picked.display(),
            reason = "env_var",
            "[RootsEdit] AREKA_ROOT で並びを渡しているので変えません"
        ),
    }
}

/// UI スレッドの入口: 置き場を外す（並びの値だけ・フォルダは消さず動かさない）。
pub(crate) fn remove_root(world: &mut World, dir: PathBuf) {
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::error!(
            event = "roots_change_no_context",
            "[RootsEdit] 起動の文脈が無いので置き場を外せません"
        );
        return;
    };
    let reason = match decide_remove(&ctx.roots, ctx.roots_origin, &dir) {
        RemoveVerdict::Removed { roots, removed } => {
            tracing::info!(
                event = "roots_removed",
                path = %removed.display(),
                "[RootsEdit] 置き場を並びから外しました"
            );
            return apply(world, roots, areka_session::roots::user_root::exe_dir());
        }
        RemoveVerdict::LastOne => "last_one",
        RemoveVerdict::EnvLocked => "env_var",
        RemoveVerdict::Stale => "stale",
        RemoveVerdict::Temporary => "temporary",
        RemoveVerdict::UserRoot => "user_root",
    };
    tracing::warn!(
        event = "roots_remove_refused",
        path = %dir.display(),
        reason,
        "[RootsEdit] 置き場を外せないので変えません"
    );
}

/// 適用: ①起動の文脈の並びと出どころ（記憶）を書き換える（その回に効かせる）
/// ②記憶へ書く③変更のときの並びと隠れの記録をスレッド `roots-log` へ渡す。
/// 今のゴースト・バルーン（`BootContext.current`）には触らない。
fn apply(world: &mut World, roots: BasewareRoots, exe_dir: Option<PathBuf>) {
    let value = encode_memory(&roots, exe_dir.as_deref());
    let Some(mut ctx) = world.get_resource_mut::<BootContext>() else {
        tracing::error!(
            event = "roots_change_no_context",
            "[RootsEdit] 起動の文脈が無いので置き場の変更を適用できません"
        );
        return;
    };
    ctx.roots = roots.clone();
    ctx.roots_origin = RootsOrigin::Memory;
    let app_profile_dir = ctx.app_profile_dir.clone();

    let entries = vec![(PersistKey::BasewareRoots, value)];
    let runtime = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime());
    match runtime {
        // 動いているゴーストの記憶の書き手を通す（同じファイルを 2 か所から書かない）。
        // 書けなければ書き手が致命の机へ送る（下の `None` の腕と同じく終わる）。
        Some(runtime) => runtime
            .sylphya_publisher()
            .persist_put(PersistScope::App, entries),
        None => {
            let scope_roots = ScopeRoots {
                app: Some(app_profile_dir.clone()),
                ..ScopeRoots::default()
            };
            // 書けなければ記録を残して致命へ渡す（書き換えた起動の文脈の並びは戻さない）。
            if let PersistOutcome::Degraded(failure) =
                save_scope(PersistScope::App, &scope_roots, &FsPersistIo, entries)
            {
                tracing::error!(
                    event = "roots_memory_write_failed",
                    dir = %app_profile_dir.display(),
                    reason = ?failure.cause,
                    "[RootsEdit] 置き場の並びを記憶へ書けません（終了します）"
                );
                areka_session::write_failure::fatal(world, failure);
            }
        }
    }

    queue_change_log(&CHANGE_LOG, roots, log_change);
}

type ChangeLogJob = (areka_actor::CarriedLog, BasewareRoots);
type ChangeLogSlot = Mutex<Option<Sender<ChangeLogJob>>>;

static CHANGE_LOG: ChangeLogSlot = Mutex::new(None);

/// 変更のときの並びと隠れを `roots-log` で記録する。根が遅い置き場（落ちた NAS）でも UI スレッドを止めない。
/// 記録は 1 本のスレッドが渡した順に残す。スレッドが居なければ起こす（`work` を使うのは起こすときだけ）。
fn queue_change_log(
    slot: &ChangeLogSlot,
    roots: BasewareRoots,
    work: impl Fn(BasewareRoots) + Send + 'static,
) {
    let mut job = (areka_actor::CarriedLog::here(), roots);
    let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(tx) = slot.as_ref() {
        match tx.send(job) {
            Ok(()) => return,
            Err(mpsc::SendError(back)) => job = back,
        }
    }
    let (tx, rx) = mpsc::channel::<ChangeLogJob>();
    let spawned = std::thread::Builder::new()
        .name("roots-log".to_owned())
        .spawn(move || {
            for (log, roots) in rx {
                let _log = log.enter();
                if let Err(payload) =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(roots)))
                {
                    let detail = payload
                        .downcast_ref::<&str>()
                        .map(|s| (*s).to_owned())
                        .or_else(|| payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "the roots log thread panicked".to_owned());
                    tracing::error!(
                        event = "roots_log_spawn_failed",
                        reason = "panicked",
                        detail = %detail,
                        "[RootsEdit] 置き場の変更を記録できません"
                    );
                }
            }
        });
    match spawned {
        Ok(_) => {
            if tx.send(job).is_err() {
                tracing::error!(
                    event = "roots_log_spawn_failed",
                    reason = "gone",
                    detail = "the roots log thread ended before taking the record",
                    "[RootsEdit] 置き場の変更を記録できません"
                );
            }
            *slot = Some(tx);
        }
        Err(e) => tracing::error!(
            event = "roots_log_spawn_failed",
            reason = "spawn",
            detail = %e,
            "[RootsEdit] 置き場の変更を記録できません"
        ),
    }
}

fn log_change(roots: BasewareRoots) {
    let status = roots
        .roots()
        .iter()
        .map(|r| RootStatus {
            dir: r.dir().to_path_buf(),
            found: r.dir().is_dir(),
        })
        .collect();
    let hidden = areka_ghost::catalog_roots::survey(&roots).hidden;
    log_roots(
        &ResolvedRoots {
            roots,
            origin: RootsOrigin::Memory,
            status,
            skipped: Vec::new(),
            memory_fell_back: false,
        },
        ReportOccasion::Change,
    );
    log_hidden(&hidden, ReportOccasion::Change);
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "edit_temporary_tests.rs"]
mod temporary_tests;

#[cfg(test)]
#[path = "edit_write_failure_tests.rs"]
mod write_failure_tests;

#[cfg(test)]
#[path = "edit_log_order_tests.rs"]
mod log_order_tests;
