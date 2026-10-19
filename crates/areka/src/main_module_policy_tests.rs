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

/// 据え付けた作り口が指定どおりの結線を組む: 初回の起動（`Plain`）と切替の先（別の根）の両方で、
/// 「読まない」なら差し替え口、「読む」なら helper の結線（要件 7.1・7.6）。
#[test]
fn installed_source_follows_module_policy_for_first_boot_and_switch() {
    use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
    let base = PathBuf::from("C:/custom/root");
    let cfg = |name: &str| ConfigInputs {
        ghost_root: base.join("ghost").join(name),
        balloon_root: base.join("balloon").join("b"),
    };
    for policy in [ModulePolicy::Load, ModulePolicy::Skip] {
        let ctx = boot_config::BootContext {
            roots: areka_ghost::BasewareRoots::single(base.clone()),
            roots_origin: crate::roots::RootsOrigin::EnvVar,
            app_profile_dir: base.join("app-profile"),
            helper_exe: helper(),
            argv_session: false,
            current: boot_config::CurrentGhost {
                cfg: cfg("A"),
                ghost: GhostDecision {
                    route: GhostRoute::Memory,
                    dir: base.join("ghost").join("A"),
                    folder: Some("A".to_owned()),
                },
                balloon: BalloonDecision {
                    route: BalloonRoute::Default,
                    dir: base.join("balloon").join("b"),
                    folder: Some("b".to_owned()),
                },
            },
        };
        let mut world = World::new();
        let (tx, _rx) = std::sync::mpsc::channel();
        install_boot_context(&mut world, ctx, tx, policy);
        let source = world.non_send::<ghost_session::GhostBootInputsSource>();
        for (name, origin) in [
            ("A", areka_kanade::BootOrigin::Plain),
            ("B", areka_kanade::BootOrigin::Plain),
        ] {
            let inputs = (source.0)(&cfg(name), origin);
            assert_eq!(inputs.modules, policy);
            assert_wiring(policy, inputs.wiring.shiori);
        }
    }
}
