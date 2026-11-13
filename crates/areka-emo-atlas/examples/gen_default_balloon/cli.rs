//! 道具の引数の読み・動きの種類・最初の違い・置き換えの書き込み・入出力の文（`main.rs` から移した）。
//!
//! ずれの見張り（`tests/default_balloon_drift_test.rs`）が `#[path]` で読むので、ここのテストはそのターゲットで走る。

use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub(crate) enum Mode {
    Write,
    Check,
}

pub(crate) fn parse_args(args: &[String]) -> Option<Mode> {
    match args {
        [] => Some(Mode::Write),
        [a] if a == "--check" => Some(Mode::Check),
        _ => None,
    }
}

pub(crate) fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(at) => Some(at),
        None if a.len() != b.len() => Some(a.len().min(b.len())),
        None => None,
    }
}

/// 一時名へ書いてから名前替えする。書き出し先のフォルダが無ければ作らずに失敗し、
/// どこで失敗しても一時名を消す（今の書庫はそのまま残る）。
pub(crate) fn write_replacing(dest: &Path, tmp: &Path, bytes: &[u8]) -> Result<(), String> {
    if !dest.parent().is_some_and(Path::is_dir) {
        return Err("destination folder does not exist".to_string());
    }
    let tmp_dir: PathBuf = tmp.parent().map(Path::to_path_buf).unwrap_or_default();
    let result = std::fs::create_dir_all(&tmp_dir)
        .and_then(|()| std::fs::write(tmp, bytes))
        .and_then(|()| std::fs::rename(tmp, dest));
    result.map_err(|e| {
        let _ = std::fs::remove_file(tmp);
        io_text(&e)
    })
}

/// `io::Error` の文（Windows では日本語）を出さず、種類と OS のエラー番号だけにする。
pub(crate) fn io_text(e: &io::Error) -> String {
    match e.raw_os_error() {
        Some(code) => format!("{:?} (os error {code})", e.kind()),
        None => format!("{:?}", e.kind()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn args_are_none_or_check_only() {
        assert_eq!(parse_args(&args(&[])), Some(Mode::Write));
        assert_eq!(parse_args(&args(&["--check"])), Some(Mode::Check));
        assert_eq!(parse_args(&args(&["--write"])), None);
        assert_eq!(parse_args(&args(&["--check", "--check"])), None);
    }

    #[test]
    fn first_difference_finds_byte_and_length_changes() {
        assert_eq!(first_difference(b"abc", b"abc"), None);
        assert_eq!(first_difference(b"abc", b"abd"), Some(2));
        assert_eq!(first_difference(b"abc", b"ab"), Some(2));
    }

    #[test]
    fn missing_destination_folder_writes_nothing() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("gen-default-balloon-test");
        let tmp = base.join("x.nar.tmp");
        let dest = base.join("no-such-folder").join("x.nar");
        let err = write_replacing(&dest, &tmp, b"data").unwrap_err();
        assert!(err.is_ascii(), "{err}");
        assert!(!tmp.exists() && !dest.exists());
    }

    #[test]
    fn io_text_is_ascii_kind_and_number() {
        let e = io::Error::from_raw_os_error(5);
        let t = io_text(&e);
        assert!(t.is_ascii() && t.contains("os error 5"), "{t}");
    }
}
