//! SHIORI の読み方の指定（`cli::ModulePolicy`）から結線を決める口と倒れ先の表
//! （areka-P0-startup-options-core・要件 7.1・11.4）。
//!
//! 指定は値として渡すだけなので、テストどうしは互いに影響しない。

use super::*;
use crate::cli::ModulePolicy;
use areka_ghost::ShioriWiring;
use std::path::PathBuf;

fn helper() -> PathBuf {
    PathBuf::from("C:/custom/exe-dir/shiori-host32-helper.exe")
}

/// 「読む」なら補助のプロセスの結線（渡した helper のパスをそのまま持つ）、
/// 「読まない」なら差し替え口（`Custom`）で、その閉包はつながる（`empty_connect`）。
fn assert_wiring(policy: ModulePolicy, wiring: ShioriWiring) {
    match (policy, wiring) {
        (ModulePolicy::Load, ShioriWiring::Helper { helper_exe, .. }) => {
            assert_eq!(helper_exe, helper())
        }
        (ModulePolicy::Skip, ShioriWiring::Custom(connect)) => {
            assert!(
                connect().is_ok(),
                "Skip の差し替え口はつながる SHIORI を返す"
            )
        }
        (policy, other) => panic!(
            "{policy:?} に対する結線が違う: {}",
            match other {
                ShioriWiring::Helper { .. } => "Helper",
                ShioriWiring::Custom(_) => "Custom",
                ShioriWiring::InProc => "InProc",
            }
        ),
    }
}

#[test]
fn shiori_wiring_for_follows_module_policy() {
    for policy in [ModulePolicy::Load, ModulePolicy::Skip] {
        assert_wiring(policy, boot_config::shiori_wiring_for(policy, helper()));
    }
}

#[test]
fn fallback_boot_options_follow_module_policy() {
    for policy in [ModulePolicy::Load, ModulePolicy::Skip] {
        let options = ghost_boot_options(PathBuf::from("C:/custom/ghost"), helper(), None, policy);
        assert_wiring(policy, options.shiori);
    }
}
