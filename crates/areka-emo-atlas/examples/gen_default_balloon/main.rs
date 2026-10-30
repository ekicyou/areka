//! 既定バルーン `areka-balloon` の書庫を作り直す道具（spec: default-balloon-selfmade 要件 8・10.1・13.8）。
//!
//! 呼び方: `cargo run -p areka-emo-atlas --example gen_default_balloon`（書く）／`-- --check`（書かずに比べる）。
//! 終了コード: 0＝書いた／同じ・1＝失敗（今の書庫は上書きしない）・2＝引数の誤り。端末へ出す文は ASCII だけ。

mod build;
mod draw;
mod texts;
mod webp;

use sample_ghost_kit::NarBuilder;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const NAME: &str = "areka-balloon";

#[derive(Debug, PartialEq)]
enum Mode {
    Write,
    Check,
}

fn parse_args(args: &[String]) -> Option<Mode> {
    match args {
        [] => Some(Mode::Write),
        [a] if a == "--check" => Some(Mode::Check),
        _ => None,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = parse_args(&args) else {
        println!("error: unknown argument (usage: gen_default_balloon [--check])");
        return ExitCode::from(2);
    };
    match run(mode) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(mode: Mode) -> Result<String, String> {
    let files = build::build()?;
    let nar = fold(&files);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let file_name = format!("{NAME}.nar");
    let dest = root.join("vendors").join("sample_ghost").join(&file_name);
    match mode {
        Mode::Check => {
            let stored = std::fs::read(&dest)
                .map_err(|e| format!("cannot read {file_name}: {}", io_text(&e)))?;
            match first_difference(&stored, &nar) {
                None => Ok(format!("check: {file_name} is identical")),
                Some(at) => Err(format!(
                    "check: {file_name} differs at byte {at} (stored {} bytes, built {} bytes)",
                    stored.len(),
                    nar.len()
                )),
            }
        }
        Mode::Write => {
            let tmp = root
                .join("target")
                .join("gen-default-balloon")
                .join(format!("{file_name}.tmp"));
            write_replacing(&dest, &tmp, &nar)
                .map_err(|e| format!("cannot write {file_name}: {e}"))?;
            Ok(format!(
                "wrote {file_name}: {} files, {} bytes",
                files.len(),
                nar.len()
            ))
        }
    }
}

/// 名前順の 23 ファイルを足した順のまま書庫に畳む（時刻などの揺れる値を持たない）。
fn fold(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    files
        .iter()
        .fold(NarBuilder::new(), |b, (name, bytes)| {
            b.file(name.as_bytes(), bytes).done()
        })
        .bytes()
}

fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(at) => Some(at),
        None if a.len() != b.len() => Some(a.len().min(b.len())),
        None => None,
    }
}

/// 一時名へ書いてから名前替えする。書き出し先のフォルダが無ければ作らずに失敗し、
/// どこで失敗しても一時名を消す（今の書庫はそのまま残る）。
fn write_replacing(dest: &Path, tmp: &Path, bytes: &[u8]) -> Result<(), String> {
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
fn io_text(e: &io::Error) -> String {
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
