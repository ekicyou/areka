use super::*;
use crate::boot_request::ModulePolicy;
use areka_ghost::{GhostBootError, ShioriWiring, TickerMode};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::MountError;
use std::path::PathBuf;

/// `default_helper_exe_path` はファイル名 `shiori-host32-helper.exe` で終わるパスを返す
/// （実際の親ディレクトリは実行環境依存のため、構造のみを確認する）。
#[test]
fn default_helper_exe_path_ends_with_expected_filename() {
    let path = default_helper_exe_path();
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some("shiori-host32-helper.exe"),
        "helper exe path should end with the expected filename: {path:?}"
    );
}

/// `ghost_boot_options` は渡された `ghost_root`／`helper_exe`／置き場をそのまま
/// `GhostBootOptions` へ写し、`ShioriWiring::Helper`・`DefaultEncoding::Ansi`・
/// `TickerMode::Real` を選ぶ。
#[test]
fn ghost_boot_options_wires_expected_fields() {
    let ghost_root = PathBuf::from("C:/custom/ghost");
    let helper_exe = PathBuf::from("C:/custom/exe-dir/shiori-host32-helper.exe");

    // 置き場は書かない固定のパスを渡す（本物の環境で置き場を決めない）。
    let app_profile_dir = PathBuf::from("C:/custom/profile/areka");

    let options = ghost_boot_options(
        ghost_root.clone(),
        helper_exe.clone(),
        Some(app_profile_dir.clone()),
        crate::boot_request::ModulePolicy::Load,
    );

    assert_eq!(options.ghost_root, ghost_root);
    assert_eq!(options.app_profile_dir, Some(app_profile_dir));
    assert_eq!(options.default_encoding, DefaultEncoding::Ansi);
    // 窓の無いゴーストにも strict の判定を入れる（無いと strict の記録が無言で 0 件）。
    assert!(options.strict_judges.is_some(), "strict の判定が入っている");
    match options.shiori {
        ShioriWiring::Helper {
            helper_exe: actual, ..
        } => assert_eq!(actual, helper_exe),
        ShioriWiring::Custom(_) => panic!("expected ShioriWiring::Helper, got Custom"),
        // `InProc` は本番 main では選ばれない（本番結線は emo2＝Helper 経路のまま）。網羅性のためのみ列挙する。
        ShioriWiring::InProc => panic!("expected ShioriWiring::Helper, got InProc"),
    }
    match options.ticker {
        TickerMode::Real(cfg) => {
            assert_eq!(cfg.base_interval, std::time::Duration::from_millis(50));
            assert_eq!(cfg.kanade_interval, std::time::Duration::from_millis(1000));
        }
        TickerMode::Disabled => panic!("expected TickerMode::Real, got Disabled"),
    }
}

/// `MountError::StartPointMissing`（プレースホルダ ghost_root の不在という想定内の
/// 事象）は良性と分類される。
#[test]
fn start_point_missing_is_classified_as_benign() {
    let err = GhostBootError::Mount(MountError::StartPointMissing {
        expected: PathBuf::from("ghost/master/descript.txt"),
    });
    assert!(is_benign_boot_error(&err));
}

/// `MountError::StartPointUnreadable`／`ShellDirMissing`（真に予期しない I/O 問題）は
/// 良性ではないと分類される。
#[test]
fn other_mount_errors_are_not_classified_as_benign() {
    let unreadable = GhostBootError::Mount(MountError::StartPointUnreadable {
        path: PathBuf::from("ghost/master/descript.txt"),
        kind: std::io::ErrorKind::PermissionDenied,
    });
    assert!(!is_benign_boot_error(&unreadable));

    let shell_missing = GhostBootError::Mount(MountError::ShellDirMissing {
        expected: PathBuf::from("ghost/master/shell/master"),
    });
    assert!(!is_benign_boot_error(&shell_missing));
}

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
        assert_wiring(policy, shiori_wiring_for(policy, helper()));
    }
}

#[test]
fn fallback_boot_options_follow_module_policy() {
    for policy in [ModulePolicy::Load, ModulePolicy::Skip] {
        let options = ghost_boot_options(PathBuf::from("C:/custom/ghost"), helper(), None, policy);
        assert_wiring(policy, options.shiori);
    }
}
