//! 置き場の追加と外すの判断（純粋・design.md「RootsEdit」）。
//!
//! 値だけで判断する: 今の並び・出どころ・足すフォルダ（または外す根の絶対パス）。
//! フォルダそのものは消さない・動かさない（要件 7.2）。触るのは並びの値だけ。
//! 適用（起動の文脈・記憶・記録）は UI スレッドの入口 [`add_root`]・[`remove_root`]。
//! 並べ替えの判断は作らない（要件 7.4）。

use std::path::{Path, PathBuf};

use areka_ghost::{BasewareRoot, BasewareRoots, same_root};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistOutcome, PersistScope, ScopeRoots, save_scope};
use bevy_ecs::world::World;

use super::report::{ReportOccasion, log_hidden, log_roots};
use super::{ResolvedRoots, RootStatus, RootsOrigin, encode_memory};
use crate::boot_config::BootContext;
use crate::ghost_session::GhostSlot;

/// 追加の判断。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AddVerdict {
    /// 末尾に足した。`empty` は直下に ghost も balloon も無い（6.7 の warn）。
    Added { roots: BasewareRoots, empty: bool },
    /// すでにある（6.6 の warn・変えない）。
    AlreadyListed { existing: PathBuf },
    /// `AREKA_ROOT` で渡している（8.6・変えない）。
    EnvLocked,
}

/// 外すの判断。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RemoveVerdict {
    Removed {
        roots: BasewareRoots,
        removed: PathBuf,
    },
    /// 根が 1 つしか無い（7.3・変えない）。
    LastOne,
    EnvLocked,
    /// 並びに無い（メニューを出した後に並びが変わった・warn・変えない）。
    Stale,
}

/// 追加: 末尾に足す（要件 6.4）。実行ファイルのフォルダも絶対パスのまま入る（記憶へは
/// `encode_memory` が `<exe>` で書く＝要件 6.8）。
pub(crate) fn decide_add(roots: &BasewareRoots, origin: RootsOrigin, picked: &Path) -> AddVerdict {
    if origin == RootsOrigin::EnvVar {
        return AddVerdict::EnvLocked;
    }
    if let Some(existing) = roots.roots().iter().find(|r| same_root(r.dir(), picked)) {
        return AddVerdict::AlreadyListed {
            existing: existing.dir().to_path_buf(),
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

/// 外す: メニューに出した根の絶対パスで受け、並びの値から外すだけ（要件 7.2・7.3・8.6）。
pub(crate) fn decide_remove(
    roots: &BasewareRoots,
    origin: RootsOrigin,
    dir: &Path,
) -> RemoveVerdict {
    if origin == RootsOrigin::EnvVar {
        return RemoveVerdict::EnvLocked;
    }
    let Some(index) = roots.roots().iter().position(|r| same_root(r.dir(), dir)) else {
        return RemoveVerdict::Stale;
    };
    if roots.roots().len() == 1 {
        return RemoveVerdict::LastOne;
    }
    let mut list = roots.roots().to_vec();
    let removed = list.remove(index).dir().to_path_buf();
    RemoveVerdict::Removed {
        roots: BasewareRoots::new(list),
        removed,
    }
}

/// UI スレッドの入口: 置き場を足す（判断・適用・記録・design「RootsEdit」）。
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
            apply(world, roots, exe_dir());
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

/// UI スレッドの入口: 置き場を外す（並びの値だけ・フォルダは消さず動かさない・要件 7.2）。
#[allow(dead_code)] // 呼び手はメニューの置き場の項目（tasks.md 5.6）
pub(crate) fn remove_root(world: &mut World, dir: PathBuf) {
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::error!(
            event = "roots_change_no_context",
            "[RootsEdit] 起動の文脈が無いので置き場を外せません"
        );
        return;
    };
    let reason = match decide_remove(&ctx.roots, ctx.roots_origin, &dir) {
        RemoveVerdict::Removed { roots, .. } => return apply(world, roots, exe_dir()),
        RemoveVerdict::LastOne => "last_one",
        RemoveVerdict::EnvLocked => "env_var",
        RemoveVerdict::Stale => "stale",
    };
    tracing::warn!(
        event = "roots_remove_refused",
        path = %dir.display(),
        reason,
        "[RootsEdit] 置き場を外せないので変えません"
    );
}

/// 実行ファイルのフォルダ（記憶へ `<exe>` で書く相手）。
fn exe_dir() -> Option<PathBuf> {
    crate::boot_config::exe_location()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

/// 適用: ①起動の文脈の並びと出どころ（記憶）を書き換える（その回に効かせる・要件 8.2・8.5）
/// ②記憶へ書く（要件 8.1）③変更のときの並びと隠れを記録する（要件 8.3）。
/// 今のゴースト・バルーン（`BootContext.current`）には触らない（要件 8.4）。
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
        // 書けなければ `save_scope` が error を残す。
        Some(runtime) => runtime
            .sylphya_publisher()
            .persist_put(PersistScope::App, entries),
        None => {
            let scope_roots = ScopeRoots {
                app: Some(app_profile_dir.clone()),
                ..ScopeRoots::default()
            };
            if save_scope(PersistScope::App, &scope_roots, &FsPersistIo, entries)
                == PersistOutcome::Degraded
            {
                tracing::error!(
                    event = "roots_memory_write_failed",
                    dir = %app_profile_dir.display(),
                    "[RootsEdit] 置き場の並びを記憶へ書けません（この回の並びは効いています）"
                );
            }
        }
    }

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
