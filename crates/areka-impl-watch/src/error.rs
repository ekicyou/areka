//! 失敗の型 [`WatchError`]。
//!
//! 文は全部 ASCII の 1 行で、`Display` をそのまま標準エラーへ出す。`io::Error` と
//! `serde_json::Error` の `Display` には OS や入力のローカル言語の文が混ざりうるので
//! そのまま包まず、OS の失敗は種類と OS の番号、JSON の失敗は行と列だけを綴る。
//!
//! 層の一番下に在り、同じクレートの他のモジュールを読み込まない（待ちの種類も ASCII の
//! 名前で受ける）。

use std::io;
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum WatchError {
    #[error(
        "AREKA_IMPL_WATCH_HOME is not set or empty. Set it to an absolute folder (for example %USERPROFILE%\\.areka-impl-watch), then run tools\\impl-watch-install.ps1 to build and place areka-impl-watch.exe there. See doc/impl-watch.md section 2"
    )]
    HomeUnset,
    /// `dir` は環境変数の値そのまま。ASCII の外の字は出すときに `\u{...}` へ逃がす。
    #[error(
        "AREKA_IMPL_WATCH_HOME must be an absolute path such as C:\\Users\\me\\.areka-impl-watch, but it is '{}'. See doc/impl-watch.md", escape_path(.dir)
    )]
    HomeNotAbsolute { dir: String },
    /// `dir` は置き場所の道筋。ASCII の外の字は出すときに `\u{...}` へ逃がす。
    #[error("AREKA_IMPL_WATCH_HOME cannot be created: {}: {kind} (os error {code})", escape_path(.dir))]
    HomeNotCreatable {
        dir: String,
        kind: String,
        code: i32,
    },
    #[error("state.lock is busy for 10 s; another areka-impl-watch may be stuck")]
    LockBusy,
    #[error(
        "state file version mismatch: file has {found}, this exe knows {known}. Do not mix old and new exes; see doc/impl-watch.md"
    )]
    VersionMismatch { found: u64, known: u32 },
    /// 読むだけ（`status`）のときに状態ファイルが壊れていた。訳はログにだけ載り、文には出ない。
    #[error("state file is broken; see impl-watch.log")]
    Broken(Damage),
    /// `id` は引数の形の決まり（ASCII）を通ったもの、`kind` は待ちの種類の ASCII の名前。
    #[error("a {kind} wait for {id} is already running")]
    AlreadyRunning { id: String, kind: &'static str },
    /// [`WatchError::io`] で作る。`op` は何をしていたかの ASCII の短い名前。
    #[error("io {op}: {kind} (os error {code})")]
    Io {
        op: &'static str,
        kind: String,
        code: i32,
    },
    /// `From<serde_json::Error>` で作る（中身は行と列だけ）。
    #[error("json: {0}")]
    Json(String),
}

/// 状態ファイルが壊れていた訳。ログにだけ載り、端末の文には出ない。中身は種類の名前と数だけ
/// （`serde_json::Error` の文は入力の値を含むので写さない）。ログでは `Debug` の綴りで載る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Damage {
    /// JSON として読めない（途中で切れた・BOM 付きを含む）。行と列は serde_json の示す位置。
    NotJson { line: usize, column: usize },
    /// 版は 1 だが欄の形が合わない。行と列は元のバイト列を読み直して得た位置。
    WrongShape { line: usize, column: usize },
    /// 版の数字が無い・数でない・一番外が表でない。
    NoVersion,
}

impl WatchError {
    /// OS の失敗を、種類と OS の番号だけにして写す。
    pub fn io(op: &'static str, err: &io::Error) -> Self {
        let (kind, code) = kind_and_code(err);
        WatchError::Io { op, kind, code }
    }

    /// 置き場所のフォルダを作れなかった失敗を、道筋・種類・OS の番号にして写す。
    pub fn home_not_creatable(dir: &Path, err: &io::Error) -> Self {
        let (kind, code) = kind_and_code(err);
        WatchError::HomeNotCreatable {
            dir: dir.to_string_lossy().into_owned(),
            kind,
            code,
        }
    }
}

impl From<serde_json::Error> for WatchError {
    fn from(err: serde_json::Error) -> Self {
        WatchError::Json(format!("line {} column {}", err.line(), err.column()))
    }
}

/// 種類は `ErrorKind` の名前（`NotFound` の形）。OS の番号が無い失敗は 0。
fn kind_and_code(err: &io::Error) -> (String, i32) {
    (format!("{:?}", err.kind()), err.raw_os_error().unwrap_or(0))
}

/// 置き場所の道筋を端末へ出せる形にする: 印字できる ASCII はそのまま、他は `\u{...}`。
pub(crate) fn escape_path(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' '..='~' => c.to_string(),
            _ => c.escape_unicode().to_string(),
        })
        .collect()
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
