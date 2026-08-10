use super::*;
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

/// `ghost_boot_options` は渡された `ghost_root`／`helper_exe` をそのまま
/// `GhostBootOptions` へ写し、`ShioriWiring::Helper`・`DefaultEncoding::Ansi`・
/// `TickerMode::Real` を選ぶ（design.md「main の ghost boot／shutdown 結線」）。
#[test]
fn ghost_boot_options_wires_expected_fields() {
    let ghost_root = PathBuf::from("C:/custom/ghost");
    let helper_exe = PathBuf::from("C:/custom/exe-dir/shiori-host32-helper.exe");

    let options = ghost_boot_options(ghost_root.clone(), helper_exe.clone());

    assert_eq!(options.ghost_root, ghost_root);
    assert_eq!(options.default_encoding, DefaultEncoding::Ansi);
    match options.shiori {
        ShioriWiring::Helper {
            helper_exe: actual, ..
        } => assert_eq!(actual, helper_exe),
        ShioriWiring::Custom(_) => panic!("expected ShioriWiring::Helper, got Custom"),
        // `InProc`（areka-P0-shiori4-test-ghost の第 3 結線）は本番 main では選ばれない
        // （要件 7.2: 本番結線は emo2＝Helper 経路のまま）。網羅性のためのみ列挙する。
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
/// 事象）は良性と分類される（要件 8.2）。
#[test]
fn start_point_missing_is_classified_as_benign() {
    let err = GhostBootError::Mount(MountError::StartPointMissing {
        expected: PathBuf::from("ghost/master/descript.txt"),
    });
    assert!(is_benign_boot_error(&err));
}

/// `MountError::StartPointUnreadable`／`ShellDirMissing`（真に予期しない I/O 問題）は
/// 良性ではないと分類される（要件 8.2）。
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
