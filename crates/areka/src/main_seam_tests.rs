use super::*;
use crate::placement::PlacementError;
use areka_parsers::package::MountError;
use std::path::PathBuf;

/// `PlacementError::Mount(StartPointMissing)`（fixture 不在という想定内の事象）は
/// 良性（`warn!` どまり）と分類される（design「main.rs seam」・DD14）。
#[test]
fn placement_start_point_missing_is_benign() {
    let err = PlacementError::Mount(MountError::StartPointMissing {
        expected: PathBuf::from("ghost/master/descript.txt"),
    });
    assert!(is_benign_placement_error(&err));
}

/// それ以外の `PlacementError`（読取不能・shell 不在・descript I/O・採寸・モニタ 0 台）は
/// 真に予期しない失敗として良性ではない（`error!`）と分類される。
#[test]
fn placement_other_errors_are_not_benign() {
    let unreadable = PlacementError::Mount(MountError::StartPointUnreadable {
        path: PathBuf::from("ghost/master/descript.txt"),
        kind: std::io::ErrorKind::PermissionDenied,
    });
    assert!(!is_benign_placement_error(&unreadable));

    let shell_missing = PlacementError::Mount(MountError::ShellDirMissing {
        expected: PathBuf::from("ghost/master/shell/master"),
    });
    assert!(!is_benign_placement_error(&shell_missing));

    let descript = PlacementError::DescriptRead {
        path: PathBuf::from("shell/master/descript.txt"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "boom"),
    };
    assert!(!is_benign_placement_error(&descript));

    let measure = PlacementError::Measure {
        scope: 0,
        reason: "合成失敗".to_string(),
    };
    assert!(!is_benign_placement_error(&measure));

    let monitor = PlacementError::Monitor {
        reason: "0 台".to_string(),
    };
    assert!(!is_benign_placement_error(&monitor));
}
