//! ベースウェアの根の並び。
//!
//! 決め方の判断は [`resolve`]（環境変数と記憶は値で受け、根が在るかと相対の解決だけはその場で見る）。プロセスの環境変数と
//! 起動前の記憶を読むのは、この下の薄い口だけ。

pub mod report;
mod resolve;
pub mod user_root;

pub use resolve::*;

use std::path::Path;

use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, load_scope};

/// 起動前の記憶の直読み（App スコープの `[baseware] roots`）。鍵が無ければ `None`、空文字は空文字のまま。
pub fn read_memory(app_profile_dir: &Path) -> Option<String> {
    let roots = ScopeRoots {
        app: Some(app_profile_dir.to_path_buf()),
        ..ScopeRoots::default()
    };
    load_scope(PersistScope::App, &roots, &FsPersistIo)
        .into_iter()
        .find_map(|(k, v)| (k == PersistKey::BasewareRoots).then_some(v))
}

/// 薄い読み口: `AREKA_ROOT`・記憶・実行ファイルのフォルダ・並びに入れる書ける根を読んで [`resolve_roots_from`] へ渡す。
/// プロセスの環境変数を読むのはここだけ。
pub fn resolve_roots(app_profile_dir: &Path) -> Result<ResolvedRoots, RootError> {
    resolve_roots_from(
        std::env::var_os(ROOTS_ENV),
        read_memory(app_profile_dir),
        user_root::exe_dir(),
        user_root::listed_user_root(),
    )
}

/// テストの組み立て: 根 1 つ（`AREKA_ROOT` から・見つかった）で決まった並び。
#[cfg(any(test, feature = "test-support"))]
pub fn single_env(dir: std::path::PathBuf) -> Result<ResolvedRoots, RootError> {
    Ok(ResolvedRoots {
        roots: areka_ghost::BasewareRoots::single(dir.clone()),
        origin: RootsOrigin::EnvVar,
        status: vec![RootStatus { dir, found: true }],
        skipped: Vec::new(),
        memory_fell_back: false,
    })
}
