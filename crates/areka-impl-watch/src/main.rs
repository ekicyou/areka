//! 実行ファイルの入口。
//!
//! 引数を受けて終了コードを返すだけの層で、判断は一切持たない
//! （0 = できた・1 = 失敗・2 = 使い方の誤り・3 = 当てはまらなかった。設計「Error Handling / 終了コード」）。
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
    // 実行ファイル名（先頭）は落とす。
    let args: Vec<String> = std::env::args().skip(1).collect();
    // コマンドの表（cli）が載るまでは、どの引数も「知らないコマンド」＝使い方の誤り。
    // 黙って終了コードだけを返さない。引数の中身は ASCII とは限らないので文に混ぜない。
    eprintln!(
        "areka-impl-watch: unknown command ({} argument(s) given)",
        args.len()
    );
    ExitCode::from(2)
}
