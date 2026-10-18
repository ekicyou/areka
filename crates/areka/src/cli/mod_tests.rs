//! 起動オプションの中身の既定値（要件 1.13・7.1）。

use super::*;

#[test]
fn defaults_load_modules_listen_not_given_no_help() {
    let opts = StartupOptions::default();
    assert_eq!(opts.modules, ModulePolicy::Load);
    assert_eq!(opts.listen, ListenPlan::NotGiven);
    assert!(!opts.help);
    assert_eq!(opts.boot, BootRequest::default());
    assert_eq!(opts.boot.positional_ghost, None);
    assert_eq!(opts.boot.ghost, None);
    assert!(!opts.boot.random && !opts.boot.no_boot_check);
}
