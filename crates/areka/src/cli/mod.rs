//! 起動オプションの読み方（areka-P0-startup-options-core）。
//!
//! 引数の列を読む口と、読んだ中身の型。読んだ値は `main` が起動の解決・MCP の待受・
//! SHIORI の結線へ渡す。
// 6.1 で `main` へ結線するまで使われない型と口がある。6.1 で外す。
#![allow(dead_code)]

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

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
