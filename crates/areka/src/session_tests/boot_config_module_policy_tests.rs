//! 据え付けた作り口が SHIORI の読み方の指定（`areka_session::boot_request::ModulePolicy`）どおりの結線を組むこと。
//!
//! 指定は値として渡すだけなので、テストどうしは互いに影響しない。口そのものの表は `areka-session` の `boot_config_ghost_wiring_tests.rs`。

use crate::install_boot_context;
use areka_ghost::ShioriWiring;
use areka_session::boot_config;
use areka_session::boot_config::*;
use areka_session::boot_request::ModulePolicy;
use areka_session::ghost_session;
use bevy_ecs::prelude::World;
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

/// 据え付けた作り口が指定どおりの結線を組む: 由来が `Plain` の 2 回（別の根の「A」「B」）の両方で、
/// 「読まない」なら差し替え口、「読む」なら helper の結線。
#[test]
fn installed_source_follows_module_policy_for_first_boot_and_switch() {
    use areka_session::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
    let base = PathBuf::from("C:/custom/root");
    let cfg = |name: &str| ConfigInputs {
        ghost_root: base.join("ghost").join(name),
        balloon_root: base.join("balloon").join("b"),
    };
    for policy in [ModulePolicy::Load, ModulePolicy::Skip] {
        let ctx = boot_config::BootContext {
            roots: areka_ghost::BasewareRoots::single(base.clone()),
            roots_origin: areka_session::roots::RootsOrigin::EnvVar,
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
