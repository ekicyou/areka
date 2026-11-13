//! host の結合テストの共通の道具（audit-shiori-host32 U1・指摘 3・21・48）。
//!
//! - 成果物の探し方: [`resolve_helper_exe`]・[`resolve_testdll`]・[`workspace_i686_target_base`]。
//!   env の指定を先に見て、無ければワークスペースの `target\i686-pc-windows-msvc\{debug,release}` を探す。
//!   見つからなければ明確な panic で落とす（黙って戻って緑を装わない）。
//! - テストの根: [`TestRoot`]。`<ワークスペース>\target\test-roots\shiori-host32-<名>-<pid>-<連番>` に作り、
//!   Drop で消す。helper を持つ値より**先に**作る（後に消える＝helper が DLL を放した後に消す）。
//!   helper のテストの同じ道具は `crates/shiori-host32-helper/src/proxy_test_support.rs`
//!   （クレートをまたいで共有するには新しいクレートが要るので、2 か所に置いて互いを注釈で指す）。
//! - 握手の上限: [`HANDSHAKE_TIMEOUT`]（30 秒）。的は「上限の中で HELLO が届く」ことで、
//!   時間そのものではない。負荷の高い机で i686 helper の起動が遅れても赤にしない上限にする（指摘 48）。
// 結合テストのファイルごとに使う関数が違い、使わないファイルで dead_code の警告が出るため許す。
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// ハンドシェイク（HELLO 受領）の上限時間。
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// ワークスペースのルート（`CARGO_MANIFEST_DIR` = `crates/shiori-host32-host` の 2 つ上）。
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // ワークスペースルート
        .map(Path::to_path_buf)
        .unwrap_or(manifest_dir)
}

/// ワークスペースの `target/i686-pc-windows-msvc` ベース。
pub fn workspace_i686_target_base() -> PathBuf {
    workspace_root().join("target").join("i686-pc-windows-msvc")
}

/// env `env_key` の明示指定 → `target/i686-pc-windows-msvc/{debug,release}/<file_name>` の順に探す。
fn resolve_i686_artifact(env_key: &str, file_name: &str, package: &str) -> PathBuf {
    if let Ok(explicit) = std::env::var(env_key) {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return p;
        }
        panic!(
            "{env_key}={explicit:?} が指すファイルが存在しません。\
             先にビルドし正しいパスを指してください:\n  \
             cargo build -p {package} --target i686-pc-windows-msvc"
        );
    }

    let target_base = workspace_i686_target_base();
    for profile in ["debug", "release"] {
        let candidate = target_base.join(profile).join(file_name);
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "{file_name} が見つかりません（探索先: {}\\{{debug,release}}\\{file_name}）。\
         PowerShell で先にビルドしてください（Git Bash 不可）:\n  \
         cargo build -p {package} --target i686-pc-windows-msvc\n\
         あるいは env {env_key} でパスを明示してください。",
        target_base.display()
    );
}

/// 事前ビルドした i686 helper exe（env `HOST32_HELPER_EXE` が先）。
pub fn resolve_helper_exe() -> PathBuf {
    resolve_i686_artifact(
        "HOST32_HELPER_EXE",
        "shiori-host32-helper.exe",
        "shiori-host32-helper",
    )
}

/// 事前ビルドした i686 testdll `shiori.dll`（env `HOST32_TESTDLL_DLL` が先）。
pub fn resolve_testdll() -> PathBuf {
    resolve_i686_artifact("HOST32_TESTDLL_DLL", "shiori.dll", "shiori-host32-testdll")
}

/// テストの根。`<ワークスペース>\target\test-roots\shiori-host32-<名>-<pid>-<連番>` に作り、Drop で消す。
///
/// helper を持つ値（`HelperLifecycle` など）より先に作ること。後に作った値が先に落ちるので、
/// helper が終わって DLL を放してから根が消える。
pub struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    pub fn new(name: &str) -> Self {
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

    pub fn path(&self) -> &Path {
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
