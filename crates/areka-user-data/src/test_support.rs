//! テストの道具（置き場の決めと準備のテストが分け合う・各 1 つ）。根はすべて `target\` の下。

use crate::{Signals, Writability};
use log_capture_kit::CapturedEvent;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// `dir\profile\areka`。
pub(crate) fn profile(dir: &Path) -> PathBuf {
    dir.join("profile").join("areka")
}

/// 実行ファイルのフォルダ（隣に記憶 `profile\areka\sylphya.toml` と `mcp-bridge` 在り）と、まだ無い書ける根。
pub(crate) fn fixture(label: &str) -> (TempPath, PathBuf, PathBuf) {
    let tmp = TempPath::under_target(label);
    let exe = tmp.child("exe");
    let beside = profile(&exe);
    fs::create_dir_all(beside.join("mcp-bridge")).unwrap();
    fs::write(beside.join("sylphya.toml"), "old = 1").unwrap();
    let root = tmp.child("user");
    (tmp, exe, root)
}

/// 書ける根を環境変数で指した入力（ほかの入力は空）。
pub(crate) fn env_signals(root: &Path) -> Signals {
    Signals {
        user_root_env: Some(OsString::from(root)),
        local_app_data: None,
        winget_marker: None,
        writability: Writability::Writable,
    }
}

/// 木の写し（相対パスと、ファイルなら中身）。並びは名前順。
pub(crate) fn tree(dir: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap().flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(dir).unwrap().to_path_buf();
            if path.is_dir() {
                stack.push(path);
                out.push((rel, None));
            } else {
                out.push((rel, Some(fs::read(&path).unwrap())));
            }
        }
    }
    out.sort();
    out
}

/// `event` が `name` の記録。
pub(crate) fn logged<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}
