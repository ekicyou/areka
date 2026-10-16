//! ベースウェアの根の並び（areka-P0-baseware-root-list）。
//!
//! 決め方の判断は [`resolve`]（純粋・値の注入だけでテストする）。プロセスの環境変数と
//! 起動前の記憶を読むのは、この下の薄い口だけ（要件 12.7）。

pub(crate) mod report;
mod resolve;

pub(crate) use resolve::*;

use std::path::Path;

use areka_ghost::{BasewareRoot, BasewareRoots};

use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, load_scope};

/// 起動前の記憶の直読み（App スコープの `[baseware] roots`）。鍵が無ければ `None`、空文字は空文字のまま。
pub(crate) fn read_memory(app_profile_dir: &Path) -> Option<String> {
    let roots = ScopeRoots {
        app: Some(app_profile_dir.to_path_buf()),
        ..ScopeRoots::default()
    };
    load_scope(PersistScope::App, &roots, &FsPersistIo)
        .into_iter()
        .find_map(|(k, v)| (k == PersistKey::BasewareRoots).then_some(v))
}

/// 薄い読み口: `AREKA_ROOT`・記憶・実行ファイルのフォルダを読んで [`resolve_roots_from`] へ渡す。
/// プロセスの環境変数を読むのはここだけ。
pub(crate) fn resolve_roots(app_profile_dir: &Path) -> Result<ResolvedRoots, RootError> {
    resolve_roots_from(
        std::env::var_os(ROOTS_ENV),
        read_memory(app_profile_dir),
        crate::boot_config::exe_location()
            .and_then(Path::parent)
            .map(Path::to_path_buf),
    )
}

/// 移り途中の仮の形（3.1）: まだ根 1 つを受ける呼び手へ渡す根。見つかるいちばん上の根、
/// 1 つも見つからなければ並びの先頭。呼び手が和の上へ移ったら（3.2・4.x）外す（7.1）。
pub(crate) fn head_root(roots: &BasewareRoots) -> BasewareRoot {
    roots
        .present()
        .next()
        .or_else(|| roots.roots().first())
        .cloned()
        .unwrap_or_else(|| BasewareRoot::new(std::path::PathBuf::new()))
}

/// テストの組み立て: 根 1 つ（`AREKA_ROOT` から・見つかった）で決まった並び。
#[cfg(test)]
pub(crate) fn single_env(dir: std::path::PathBuf) -> Result<ResolvedRoots, RootError> {
    Ok(ResolvedRoots {
        roots: BasewareRoots::single(dir.clone()),
        origin: RootsOrigin::EnvVar,
        status: vec![RootStatus { dir, found: true }],
        skipped: Vec::new(),
        memory_fell_back: false,
    })
}
