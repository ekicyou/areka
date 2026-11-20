//! ワークツリーの `target\` の下に最小のゴーストを植える口 [`MinimalGhost`]（OS の一時フォルダは使わない）。
//! 記憶の送り口と窓の一式は `areka_runtime::frame::persist_test_support` にある。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

/// `target\test-roots\` の下に植えた最小のゴースト（マウントの解決が通る形）。破棄で中身ごと消える。
pub(super) struct MinimalGhost(TempPath);

impl MinimalGhost {
    /// `ghost\master\descript.txt` と `shell\master\` だけを植える。記憶のファイルは植えない。
    pub(super) fn plant(label: &str) -> Self {
        let ghost = MinimalGhost(TempPath::new(&format!("cpsoe-{label}")));
        std::fs::create_dir_all(ghost.ghost_master()).expect("ghost/master を作れる");
        std::fs::write(
            ghost.ghost_master().join("descript.txt"),
            "charset,UTF-8\nname,テスト\nsakura.name,さくら\nkero.name,うにゅう\n".as_bytes(),
        )
        .expect("descript.txt を書ける");
        std::fs::create_dir_all(ghost.0.child("shell").join("master"))
            .expect("shell/master を作れる");
        ghost
    }

    /// ゴーストの根（起動の準備の関数へ渡す）。
    pub(super) fn root(&self) -> &Path {
        self.0.path()
    }

    pub(super) fn ghost_master(&self) -> PathBuf {
        self.0.child("ghost").join("master")
    }
}
