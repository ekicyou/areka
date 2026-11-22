//! 起動の求め（`BootRequest`）と SHIORI の読み方の指定（`ModulePolicy`）。

use std::path::PathBuf;

/// 起動の求め（起動の解決へ渡す）。
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct BootRequest {
    /// 位置の引数の 1 つ目（`--ghost` が渡されたら None）。
    pub positional_ghost: Option<PathBuf>,
    /// 位置の引数の 2 つ目。
    pub positional_balloon: Option<PathBuf>,
    /// `--ghost` の最初の値（値なしは None・残りは記録だけ）。
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
