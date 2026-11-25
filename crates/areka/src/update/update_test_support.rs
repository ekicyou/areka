//! `update/` のテストが分け合う小道具（`#[test]` を持たない）。

use std::path::{Path, PathBuf};

use areka_runtime::spine::RecordedCall;

/// `dir` に `descript.txt`（`charset,UTF-8` の頭＋`body`＋末尾の改行・CRLF）を置く。
pub(crate) fn write_descript(dir: &Path, body: &str) {
    std::fs::create_dir_all(dir).expect("検体のフォルダを作る");
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{body}\r\n"),
    )
    .expect("descript.txt を書く");
}

/// ゴーストの `.update-work/` の下に、成功した走行の残り（印 `committed` と古い DLL の写し）を置く。
pub(crate) fn plant_committed(ghost_dir: &Path, run: &str) -> PathBuf {
    let dir = ghost_dir.join(".update-work").join(run);
    std::fs::create_dir_all(dir.join("old")).expect("走行フォルダを組む");
    std::fs::write(dir.join("old").join("yaya.dll"), b"old").expect("古い写しを置く");
    std::fs::write(dir.join("committed"), b"").expect("印を置く");
    dir
}

/// `id` の GET の Reference（最初の 1 件）。
pub(crate) fn refs_of(calls: &[RecordedCall], id: &str) -> Option<Vec<String>> {
    areka_session::ghost_session::shell_balloon_switch_session_lap_test_support::refs_of(calls, id)
        .into_iter()
        .next()
}
