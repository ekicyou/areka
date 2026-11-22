//! 集めた層のテストのうち、本体の部品も使うものの置き場。
//!
//! 集めた層は本体を使えないので、本体の根の関数・`cli`・menu・install・update・mcp・`shiori_host` の本物を
//! 見るテストはここに置く。各ファイルは元の親のモジュールの `use` を自分で持つ。本番のコードは置かない。

use crate::session::alert::AlertScene;
use crate::session::boot_config::{BootResolved, resolve_boot_with};
use crate::session::roots::{ResolvedRoots, RootError};

/// テストの口: 引数の列を読み方（`cli::parse`）に通し、起動の求めで [`resolve_boot_with`] を呼ぶ。
pub(crate) fn resolve_boot_from(
    resolved: Result<ResolvedRoots, RootError>,
    args: &[String],
    app_profile_dir: &std::path::Path,
    pick: fn(usize) -> usize,
    prepare_ghost: fn(&areka_ghost::BasewareRoots, &std::path::Path) -> std::path::PathBuf,
) -> Result<BootResolved, AlertScene> {
    resolve_boot_with(
        resolved,
        &crate::cli::parse::parse(args).options.boot,
        None,
        app_profile_dir,
        pick,
        prepare_ghost,
    )
}

mod boot_config_balloon_tests;
mod boot_config_callghost_tests;
mod boot_config_copy_tests;
mod boot_config_input_tests;
mod boot_config_module_policy_tests;
mod boot_config_option_tests;
mod boot_config_roots_tests;
mod boot_resolve_mark_text_tests;
mod ghost_session_reload_tests;
mod ghost_session_restart_tests;
mod ghost_session_scope_rect_boot_tests;
mod ghost_session_scope_scale_boot_tests;
mod ghost_session_strict_tests;
mod ghost_session_switch_tests;
mod readonly_run_tests;
mod readonly_sites_tests;
mod reload_tests;
mod session_end_tests;
mod shell_balloon_switch_session_tests;
mod shell_balloon_switch_session_update_tests;
mod shell_balloon_switch_tests;
#[path = "write_failure_sites_tests.rs"]
mod sites_tests;
