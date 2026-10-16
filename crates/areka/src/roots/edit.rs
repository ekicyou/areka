//! 置き場の追加と外すの判断（純粋・design.md「RootsEdit」）。
//!
//! 値だけで判断する: 今の並び・出どころ・足すフォルダ（または外す根の絶対パス）。
//! フォルダそのものは消さない・動かさない（要件 7.2）。触るのは並びの値だけ。
//! 適用（起動の文脈・記憶・記録）は tasks.md 5.3。並べ替えの判断は作らない（要件 7.4）。
#![allow(dead_code)] // 使い手は並びの変更の適用（tasks.md 5.3）

use std::path::{Path, PathBuf};

use areka_ghost::{BasewareRoot, BasewareRoots, same_root};

use super::RootsOrigin;

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

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
