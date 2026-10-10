//! `cli` のテストの支え: 引数の組み立て・差し替えた書き手での実行・フォルダの中身の読み取り。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{Outcome, run_with};
use crate::error::WatchError;

pub(super) fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

/// 差し替えた書き手で走らせ、結果・標準出力・標準エラーを返す。
pub(super) fn run_captured(
    words: &[String],
    home: Option<OsString>,
) -> (Result<Outcome, WatchError>, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let result = run_with(words, home, &mut out, &mut err);
    let text = |bytes: Vec<u8>| String::from_utf8(bytes).expect("UTF-8 で出る");
    (result, text(out), text(err))
}

/// フォルダの直下に在るものの一覧（並べ替え済み）。
pub(super) fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| entry.expect("項目を読める").path())
        .collect();
    found.sort();
    found
}
