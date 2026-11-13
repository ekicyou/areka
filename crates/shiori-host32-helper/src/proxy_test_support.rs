//! helper のテストの共通の道具（audit-shiori-host32 U1・指摘 3・21）。
//!
//! - 成果物の探し方: [`resolve_testdll`]・[`i686_target_base`]。env の指定を先に見て、無ければ
//!   ワークスペースの `target\i686-pc-windows-msvc\{debug,release}` を探す。見つからなければ明確な
//!   panic で落とす（黙って戻って緑を装わない）。
//! - テストの根: [`TestRoot`]。`<ワークスペース>\target\test-roots\shiori-host32-<名>-<pid>-<連番>` に作り、
//!   Drop で消す。DLL を読む値（`ShioriByteProxy` など）より**先に**作る（後に消える＝DLL を放した後に消す）。
//!   host のテストの同じ道具は `crates/shiori-host32-host/tests/common/mod.rs`
//!   （クレートをまたいで共有するには新しいクレートが要るので、2 か所に置いて互いを注釈で指す）。
//! - 写し: [`copy_testdll_into_unique_target_dir`]。新しい根へ偽の DLL を `shiori.dll` として写す。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// ワークスペースのルート（`CARGO_MANIFEST_DIR` = `crates/shiori-host32-helper` の 2 つ上）。
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // ワークスペースルート
        .map(Path::to_path_buf)
        .unwrap_or(manifest_dir)
}

/// ワークスペースの `target\i686-pc-windows-msvc` ベース。
pub(crate) fn i686_target_base() -> PathBuf {
    workspace_root().join("target").join("i686-pc-windows-msvc")
}

/// testdll の `shiori.dll` を解決する。
/// 優先順: env `HOST32_TESTDLL_DLL` → ワークスペースの i686 の debug／release 成果物。
/// **silent skip 禁止**: 見つからなければ明確に panic（設計判断 (d)）。
pub(crate) fn resolve_testdll() -> PathBuf {
    if let Ok(p) = std::env::var("HOST32_TESTDLL_DLL") {
        let path = PathBuf::from(p);
        assert!(
            path.is_file(),
            "HOST32_TESTDLL_DLL={} が実ファイルでない",
            path.display()
        );
        return path;
    }
    let target_root = i686_target_base();
    for profile in ["debug", "release"] {
        let candidate = target_root.join(profile).join("shiori.dll");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "testdll shiori.dll not found. まず PowerShell で \
         `cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc` を実行するか、\
         env HOST32_TESTDLL_DLL に絶対パスを設定すること（silent skip 禁止・設計判断 (d)）。\
         探索した base: {}",
        target_root.display()
    );
}

/// テストの根。`<ワークスペース>\target\test-roots\shiori-host32-<名>-<pid>-<連番>` に作り、Drop で消す。
///
/// DLL を読む値より先に作ること。後に作った値が先に落ちるので、DLL を放してから根が消える。
pub(crate) struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    pub(crate) fn new(name: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let path = workspace_root()
            .join("target")
            .join("test-roots")
            .join(format!("shiori-host32-{name}-{}-{seq}", std::process::id()));
        // 前の走行の残り（同じ pid の再利用）を消してから作る。
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|e| panic!("テストの根 {} を作れない: {e}", path.display()));
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.path) {
            eprintln!("テストの根 {} を消せない: {e}", self.path.display());
        }
    }
}

/// 新しいテストの根を作り、偽の DLL を `shiori.dll` として写す。
/// 返り値は（根、写した `shiori.dll` の絶対パス）。根は DLL を読む値より先に作られている。
pub(crate) fn copy_testdll_into_unique_target_dir(name: &str) -> (TestRoot, PathBuf) {
    let root = TestRoot::new(name);
    let dll = root.path().join("shiori.dll");
    std::fs::copy(resolve_testdll(), &dll).expect("copy shiori.dll into load_dir");
    (root, dll)
}
