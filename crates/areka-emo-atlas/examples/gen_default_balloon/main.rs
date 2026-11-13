//! 既定バルーン `areka-balloon` の書庫を作り直す道具（spec: default-balloon-selfmade 要件 8・10.1・13.8）。
//!
//! 呼び方: `cargo run -p areka-emo-atlas --example gen_default_balloon`（書く）／`-- --check`（書かずに比べる）。
//! 終了コード: 0＝書いた／同じ・1＝失敗（今の書庫は上書きしない）・2＝引数の誤り。端末へ出す文は ASCII だけ。

mod build;
mod cli;
mod draw;
mod texts;
mod webp;

use cli::{Mode, first_difference, io_text, parse_args, write_replacing};
use sample_ghost_kit::NarBuilder;
use std::path::Path;
use std::process::ExitCode;

const NAME: &str = "areka-balloon";

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
