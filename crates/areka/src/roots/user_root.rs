//! プロセスに 1 つの形と、写す判断（spec: areka-P0-user-data-root 要件 3.4・4.3〜4.9・7.1）。
//!
//! 形は [`layout`] が 1 回だけ決めて覚える（`exe_location` と同じ型）。写す判断 [`plan_copy`] は
//! 純粋で、並びの印（`RootMark::UserRoot`）・実行ファイルのフォルダ・種類・フォルダだけを読む。
//! 写すのは [`copy_for_write`] だけ（すべてのフックが通る 1 つの口）。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use areka_ghost::{BasewareRoot, BasewareRoots, RootMark, StoreKind, same_root};
use areka_user_data::{CopyDone, Layout, UserRoot};

use super::ROOTS_ENV;

/// プロセスに 1 つの形（初回に `establish` を 1 回だけ呼んで覚える）。
pub(crate) fn layout() -> &'static Layout {
    static LAYOUT: OnceLock<Layout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        let profile_env = std::env::var_os(areka_user_data::PROFILE_DIR_ENV);
        areka_user_data::establish(exe_dir().as_deref(), profile_env.as_deref())
    })
}

/// 並びに入れる書ける根（`AREKA_ROOT` が在るとき・今の形のときは `None`）。読み手は `resolve_roots` だけ。
pub(crate) fn listed_user_root() -> Option<&'static UserRoot> {
    listed_from(std::env::var_os(ROOTS_ENV).as_deref(), layout())
}

/// [`listed_user_root`] の値の判断: `AREKA_ROOT` が在れば（空でも）足さない（要件 3.4）。
pub(crate) fn listed_from<'a>(
    roots_env: Option<&OsStr>,
    layout: &'a Layout,
) -> Option<&'a UserRoot> {
    match roots_env {
        Some(_) => None,
        None => layout.user_root.as_ref(),
    }
}

/// 写す判断の答え。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CopyPlan {
    /// 写さない（印の付いた項目が無い・実行ファイルの隣の根の直下の物でない・書ける根が並びで下）。
    AsIs,
    /// 写す（`to` は `<書ける根>\<ghost|balloon>\<フォルダ名>`・書ける根の綴りは並びの `dir()` のまま）。
    Copy { from: PathBuf, to: PathBuf },
}

fn store_of(root: &BasewareRoot, kind: StoreKind) -> PathBuf {
    match kind {
        StoreKind::Ghost => root.ghost_store(),
        StoreKind::Balloon => root.balloon_store(),
    }
}

/// 純粋な判断（要件 4.3・4.4・3.4・7.1）。`Copy` になるのは、並びに印の付いた項目が在り、
/// `dir` の親が `<exe_dir>\<kind の置き場>` で、並びで書ける根が実行ファイルの隣の根より上に在るとき。
pub(crate) fn plan_copy(
    roots: &BasewareRoots,
    exe_dir: Option<&Path>,
    kind: StoreKind,
    dir: &Path,
) -> CopyPlan {
    let (Some(exe_dir), Some(folder), Some(parent)) = (exe_dir, dir.file_name(), dir.parent())
    else {
        return CopyPlan::AsIs;
    };
    let items = roots.roots();
    let user_at = items
        .iter()
        .position(|r| matches!(r.mark(), RootMark::UserRoot { .. }));
    let exe_at = items.iter().position(|r| same_root(r.dir(), exe_dir));
    match (user_at, exe_at) {
        (Some(u), Some(e)) if u < e && same_root(parent, &store_of(&items[e], kind)) => {
            CopyPlan::Copy {
                from: dir.to_path_buf(),
                to: store_of(&items[u], kind).join(folder),
            }
        }
        _ => CopyPlan::AsIs,
    }
}

fn kind_name(kind: StoreKind) -> &'static str {
    match kind {
        StoreKind::Ghost => "ghost",
        StoreKind::Balloon => "balloon",
    }
}

/// 判断して写し、使うフォルダを返す（すべてのフックが通る 1 つの口）。
/// 行き先が在れば写さずにそれ（要件 4.8・4.9）・写せた／先を越されたら行き先・失敗なら `dir` のまま（要件 4.7）。
pub(crate) fn copy_for_write(
    roots: &BasewareRoots,
    exe_dir: Option<&Path>,
    kind: StoreKind,
    dir: &Path,
) -> PathBuf {
    let CopyPlan::Copy { from, to } = plan_copy(roots, exe_dir, kind, dir) else {
        return dir.to_path_buf();
    };
    if to.exists() {
        return to;
    }
    // 作業の棚は並びの書ける根の `dir()` の下（`<書ける根>\.copy-work\`）。
    let Some(work_root) = roots.user_root().map(|r| r.dir().to_path_buf()) else {
        return dir.to_path_buf();
    };
    let kind = kind_name(kind);
    match areka_user_data::copy_tree_atomic(&from, &to, &work_root) {
        Ok(CopyDone::Copied(stats)) => {
            tracing::info!(
                event = "user_data_copied",
                kind,
                from = %from.display(),
                to = %to.display(),
                files = stats.files,
                bytes = stats.bytes,
                ms = stats.millis,
                "copied a bundled folder into the user root"
            );
            to
        }
        Ok(CopyDone::Raced) => {
            tracing::info!(
                event = "user_data_copy_raced",
                kind,
                from = %from.display(),
                to = %to.display(),
                "another process copied it first; using that copy"
            );
            to
        }
        Err(error) => {
            tracing::error!(
                event = "user_data_copy_failed",
                kind,
                from = %from.display(),
                to = %to.display(),
                error = %error,
                "could not copy into the user root; using the original folder"
            );
            from
        }
    }
}

/// 実行ファイルのフォルダ（`exe_location` の親・環境変数を読まず物も作らない）。
fn exe_dir() -> Option<PathBuf> {
    crate::boot_config::exe_location()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

/// 本番の口: [`copy_for_write`] に実行ファイルのフォルダを渡す。
#[allow(dead_code)] // 呼び手は切替・インストール・更新のフック（task 4.2〜4.4）
pub(crate) fn writable_copy(roots: &BasewareRoots, kind: StoreKind, dir: &Path) -> PathBuf {
    copy_for_write(roots, exe_dir().as_deref(), kind, dir)
}

/// `resolve_boot_from` へ渡す口（ゴースト用）。
#[allow(dead_code)] // 呼び手は resolve_boot（task 4.1）
pub(crate) fn writable_ghost(roots: &BasewareRoots, dir: &Path) -> PathBuf {
    writable_copy(roots, StoreKind::Ghost, dir)
}

#[cfg(test)]
#[path = "user_root_tests.rs"]
mod tests;
