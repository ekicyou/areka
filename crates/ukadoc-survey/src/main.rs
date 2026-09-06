//! 実行ファイルの入口。
//!
//! 引数を読んで [`cli::run`] へ渡し、結果を終了コードへ写すだけの層である
//! （0 = 成功・1 = 失敗・2 = 使い方の誤り。設計「Error Handling / 見張り」）。
//! 判断は一切持たない。

use std::process::ExitCode;

use ukadoc_survey::cli;

fn main() -> ExitCode {
    // 実行ファイル名（先頭）は副手続きの振り分けに使わないので落とす。
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::run(&args) {
        Ok(cli::Outcome::Done) => ExitCode::from(0),
        Ok(cli::Outcome::Usage) => {
            eprintln!("{}", cli::usage());
            ExitCode::from(2)
        }
        Err(err) => {
            // 失敗の本文は必ず標準エラーへ出す。黙って終了コードだけを返さない。
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
