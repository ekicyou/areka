//! 実行ファイルの入口。
//!
//! 引数と環境変数の値を [`cli::run`] へ渡し、結果を終了コードへ写すだけの層で、判断は一切
//! 持たない（0 = できた・1 = 失敗・2 = 使い方の誤り・3 = 当てはまらなかった。写像は
//! [`cli::exit_code`] の 1 か所。設計「Error Handling / 終了コード」）。
//!
//! モジュールの読み込みの向きは `error` → `state` → `plan` → `home`／`presence`／`status`
//! → `store` → `wait` → `cli` → `main` の一方向だけ（逆向きは誤り）。

use std::process::ExitCode;

mod cli;
mod error;
mod home;
mod plan;
mod presence;
mod state;
mod status;
mod store;
mod wait;

fn main() -> ExitCode {
    // 実行ファイル名（先頭）は落とす。Unicode に直せない引数は置き換えの字にして渡す
    // （`std::env::args` はそこで落ちる。置き換えの字は形の決まりが断る）。
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    ExitCode::from(cli::exit_code(&cli::run(&args, home::env_value())))
}

#[cfg(test)]
#[path = "main_layering_tests.rs"]
mod layering_tests;
