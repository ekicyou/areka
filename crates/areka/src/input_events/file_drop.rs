//! 窓への投げ込み（areka-P0-file-drop）。
//!
//! ゴースト窓に落とされた物を「インストール対象でないファイル」「フォルダ」「インストール対象」の
//! 3 つに分ける。振り分け [`sort_drops`] は純粋で、fs にも記録にも触れない——フォルダかどうかと
//! 書庫の目次に `install.txt` が在るかは閉包で注入し、問い合わせの失敗は [`ProbeNote`] として
//! 戻り値に載せる（記録は呼び手が出す）。

use std::path::{Path, PathBuf};

/// 振り分けの結果（1 回の投げ込みにつき 1 つ）。
///
/// # 不変条件
///
/// `files.len() + dirs.len() + installs.len()` は入力の数と等しく、3 つの `Vec` はどれも入力の順。
/// `notes` に載った物も `files` に入る（問い合わせに失敗しても一覧から落ちない）。
#[derive(Debug, Default)]
pub(crate) struct DropSort {
    /// インストール対象でないファイル。
    pub files: Vec<PathBuf>,
    /// フォルダ。
    pub dirs: Vec<PathBuf>,
    /// インストール対象（拡張子が `.nar`／`.zip` で目次に `install.txt` が在る書庫）。
    pub installs: Vec<PathBuf>,
    /// 問い合わせの失敗（記録は呼び手が出す）。
    pub notes: Vec<ProbeNote>,
}

/// 問い合わせの失敗。どちらも当該の物は「インストール対象でないファイル」側へ倒れる。
#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum ProbeNote {
    /// フォルダかどうかを問えなかった（フォルダでないものとして拡張子で決めた・要件 2.5）。
    IsDirFailed {
        path: PathBuf,
        error: std::io::Error,
    },
    /// 書庫の目次が読めなかった（`files` へ倒した・要件 2.4）。
    ArchiveUnreadable {
        path: PathBuf,
        error: areka_nar::NarError,
    },
}

/// 純粋な振り分け（要件 2.1〜2.6）。1 つの物につき上から順に最初に当たった種類に決める。
///
/// 1. `is_dir` が `Ok(true)` → `dirs`（`install.txt` を持つフォルダもフォルダ・要件 2.3）。
/// 2. `is_dir` が `Err` → [`ProbeNote::IsDirFailed`] を積み、フォルダでないものとして 3 へ。
/// 3. 拡張子が `.nar`／`.zip`（ASCII 大小無視）でなければ → `files`（目次は問わない）。
/// 4. `has_install_txt` が `Ok(true)` → `installs`／`Ok(false)` → `files`／
///    `Err` → [`ProbeNote::ArchiveUnreadable`] を積んで `files`。
///
/// 2 つの閉包と拡張子のほかは見ない（中身・大きさ・他のエントリを見ない・要件 2.2）。3 つの `Vec` は
/// 入力の順に押す（要件 2.6）。記録は出さず、問い合わせの失敗は `notes` に載せる。
///
/// 本番の注入は `fs::metadata(p).map(|m| m.is_dir())` と [`areka_nar::peek_install_txt`]。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn sort_drops(
    paths: Vec<PathBuf>,
    mut is_dir: impl FnMut(&Path) -> std::io::Result<bool>,
    mut has_install_txt: impl FnMut(&Path) -> Result<bool, areka_nar::NarError>,
) -> DropSort {
    let mut sort = DropSort::default();
    for path in paths {
        match is_dir(&path) {
            Ok(true) => {
                sort.dirs.push(path);
                continue;
            }
            Ok(false) => {}
            Err(error) => sort.notes.push(ProbeNote::IsDirFailed {
                path: path.clone(),
                error,
            }),
        }
        if !is_archive_ext(&path) {
            sort.files.push(path);
            continue;
        }
        match has_install_txt(&path) {
            Ok(true) => sort.installs.push(path),
            Ok(false) => sort.files.push(path),
            Err(error) => {
                sort.notes.push(ProbeNote::ArchiveUnreadable {
                    path: path.clone(),
                    error,
                });
                sort.files.push(path);
            }
        }
    }
    sort
}

/// 拡張子が `nar`／`zip`（ASCII 大小無視）か。
fn is_archive_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("nar") || e.eq_ignore_ascii_case("zip"))
}

#[cfg(test)]
#[path = "file_drop_tests.rs"]
mod tests;
