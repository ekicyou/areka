//! `cli` のテストの支え: 引数の組み立て・差し替えた書き手での実行・一時の置き場所・居る印・
//! 状態ファイルとフォルダの中身の読み取り。

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{Outcome, exit_code, run_with};
use crate::error::WatchError;
use crate::home::Home;
use crate::presence::{Held, hold};
use crate::state::{State, WaitKind};

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

/// 一時の置き場所。道筋に ASCII の外の字が入る。
pub(super) fn home_in(root: &TempPath) -> PathBuf {
    root.child("置き場")
}

/// 走らせて、終了コード・標準出力・標準エラーを返す。端末へ出る文は ASCII だけ。
pub(super) fn call(home: &Path, words: &[&str]) -> (u8, String, String) {
    let (result, out, err) = run_captured(&args(words), Some(home.into()));
    assert!(
        out.is_ascii(),
        "{words:?}: 標準出力に ASCII の外の字: {out:?}"
    );
    assert!(
        err.is_ascii(),
        "{words:?}: 標準エラーに ASCII の外の字: {err:?}"
    );
    (exit_code(&result), out, err)
}

/// できた: 終了コード 0・標準出力にその 1 行・標準エラーは空。
pub(super) fn done(line: &str) -> (u8, String, String) {
    (0, format!("{line}\n"), String::new())
}

/// その識別の、その種類の居る印（ロックファイル）を握る。返した値を持っている間だけ「居る」。
pub(super) fn hold_sign(home: &Path, id: &str, kind: WaitKind) -> Held {
    let home = Home {
        dir: home.to_path_buf(),
    };
    hold(&home.alive_path(id, kind))
        .expect("ロックファイルを開ける")
        .expect("まだ誰も握っていない")
}

/// その識別たちの見張りの居る印を握る。
pub(super) fn hold_watches(home: &Path, ids: &[&str]) -> Vec<Held> {
    let watch = |id: &&str| hold_sign(home, id, WaitKind::Watch);
    ids.iter().map(watch).collect()
}

/// フォルダの直下に在るものの一覧（並べ替え済み）。
pub(super) fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| entry.expect("項目を読める").path())
        .collect();
    found.sort();
    found
}

/// 置き場所の直下に在るものの名前（並べ替え済み）。
pub(super) fn names(home: &Path) -> Vec<String> {
    let name = |path: PathBuf| {
        path.file_name()
            .expect("名前が在る")
            .to_string_lossy()
            .into_owned()
    };
    entries(home).into_iter().map(name).collect()
}

pub(super) fn state_bytes(home: &Path) -> Vec<u8> {
    fs::read(home.join("state.json")).expect("状態ファイルが在る")
}

pub(super) fn read_state(home: &Path) -> State {
    serde_json::from_slice(&state_bytes(home)).expect("状態ファイルが状態として読める")
}

/// 状態ファイルを直に置く（コマンドの列では作れない状態・壊れたファイル）。
pub(super) fn put_state(home: &Path, text: &str) {
    fs::create_dir_all(home).expect("置き場所を作れる");
    fs::write(home.join("state.json"), text).expect("書ける");
}
