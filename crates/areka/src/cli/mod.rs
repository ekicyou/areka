//! 起動オプションの読み方（areka-P0-startup-options-core）。
//!
//! 引数の列を読む口と、読んだ中身の型。読んだ値は `main` が起動の解決・MCP の待受・
//! SHIORI の結線へ渡す。

pub(crate) mod help;
pub(crate) mod listen;
pub(crate) mod parse;

use std::path::PathBuf;

pub(crate) use listen::ListenPlan;

/// 起動オプションの中身。
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct StartupOptions {
    pub boot: BootRequest,
    pub listen: ListenPlan,
    pub modules: ModulePolicy,
    /// `--help` が渡された（値は使わない）。
    pub help: bool,
}

/// 起動の求め（起動の解決へ渡す）。
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct BootRequest {
    /// 位置の引数の 1 つ目（`--ghost` が渡されたら None＝2.4）。
    pub positional_ghost: Option<PathBuf>,
    /// 位置の引数の 2 つ目（2.5）。
    pub positional_balloon: Option<PathBuf>,
    /// `--ghost` の最初の値（値なしは None・残りは記録だけ＝4.5・4.7）。
    pub ghost: Option<String>,
    pub random: bool,
    pub no_boot_check: bool,
}

/// SHIORI の読み方の指定（`--option dontloadmodule` で `Skip`）。既定は読む。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ModulePolicy {
    #[default]
    Load,
    Skip,
}

/// 本物の口: OS の引数を読み（1.14）、読み方を通し、記録を出して中身を返す。
/// 記録の初期化の後に呼ぶ。
pub(crate) fn read() -> StartupOptions {
    read_from(&std::env::args_os().collect::<Vec<_>>())
}

/// `read` の中身（テストは OS の引数の代わりに列を渡す）。まとめは最後に出る。
fn read_from(args: &[std::ffi::OsString]) -> StartupOptions {
    let (args, os_notes) = parse::from_os(args);
    let parsed = parse::parse(&args);
    for note in os_notes.iter().chain(&parsed.notes) {
        note.log();
    }
    parsed.options
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
