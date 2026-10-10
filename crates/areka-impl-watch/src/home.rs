//! 置き場所 `AREKA_IMPL_WATCH_HOME` の解決と、置き場所の下のパスの組み立て。
//!
//! 置き場所は環境変数の値だけで決まり、既定の場所へは倒れない（要件 1.1）。決めるのは値を
//! 引数で受ける [`resolve`] で、環境を読むのは [`env_value`] の 1 行だけ。

// 使い手（presence・store・cli）が載るまで、本番のビルドではここが未使用になる。
// 「満たされない expect」の警告が出たら外す。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "使い手のモジュールは後のタスクで載る")
)]

use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::WatchError;
use crate::state::WaitKind;

/// 解決の済んだ置き場所（フォルダは在る）。
#[derive(Debug)]
pub struct Home {
    pub dir: PathBuf,
}

/// 環境変数の今の値。`main` が読んで `cli::run` へ渡す（テストは値を直に渡す）。
#[cfg_attr(
    test,
    expect(dead_code, reason = "テストはプロセスの環境変数を読まない")
)]
pub fn env_value() -> Option<OsString> {
    std::env::var_os("AREKA_IMPL_WATCH_HOME")
}

/// 置き場所を決め、フォルダが無ければ作る（途中のフォルダも）。
///
/// 無い・空は何も作らずに `HomeUnset`、作れなければ `HomeNotCreatable`。作るのはフォルダだけ。
/// 空の値を通すと `create_dir_all("")` が成功し、置き場所がカレントに化けるので先に断る。
/// ASCII の空白だけの値も空に数える（フォルダの名前にならない。「設定してください」の文で返す）。
pub fn resolve(value: Option<OsString>) -> Result<Home, WatchError> {
    let dir = value
        .filter(|value| !value.as_encoded_bytes().iter().all(u8::is_ascii_whitespace))
        .map(PathBuf::from)
        .ok_or(WatchError::HomeUnset)?;
    std::fs::create_dir_all(&dir).map_err(|err| WatchError::home_not_creatable(&dir, &err))?;
    Ok(Home { dir })
}

impl Home {
    pub fn state_path(&self) -> PathBuf {
        self.dir.join("state.json")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.dir.join("state.lock")
    }

    pub fn status_path(&self) -> PathBuf {
        self.dir.join("status.md")
    }

    pub fn log_path(&self) -> PathBuf {
        self.dir.join("impl-watch.log")
    }

    /// 居る印のフォルダ。ここでは作らない。
    pub fn alive_dir(&self) -> PathBuf {
        self.dir.join("alive")
    }

    /// 居る印のロックファイル `alive/<id>.<kind>.lock`。`<kind>` は状態ファイルと同じ綴り。
    pub fn alive_path(&self, id: &str, kind: WaitKind) -> PathBuf {
        let kind = match kind {
            WaitKind::Watch => "watch",
            WaitKind::Merge => "merge",
            WaitKind::Load => "load",
            WaitKind::Resume => "resume",
        };
        self.alive_dir().join(format!("{id}.{kind}.lock"))
    }
}

#[cfg(test)]
#[path = "home_tests.rs"]
mod tests;
