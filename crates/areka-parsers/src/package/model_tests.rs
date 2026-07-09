//! マウントモデル型（`model`）の単体テスト。
//!
//! 本モジュールは `model` とは別モジュールであり、公開パス
//! `crate::package::{MountModel, GhostNames, ShioriMount, ShellMount, MountError}`
//! 経由で型へアクセスする。これにより「下流 consumer（`ghost-setup` /
//! `host-32` / `shell-parse`）が公開面のみでマウントモデルを構築・比較でき、
//! 名前/ファイル名の欠落が `Option` の `None` で保持される（推測しない・
//! Req 1.4/2.3）」という I/O 契約（done 基準）を、別モジュール視点で固定する。

#![cfg(test)]

use crate::package::{
    BindGroupDefaults, GhostNames, MountError, MountModel, ShellMount, ShioriMount,
};
use std::io::ErrorKind;
use std::path::PathBuf;

#[test]
fn construct_mount_model_and_access_fields() {
    // 全サブ値を持つ MountModel を構築し、各フィールドへアクセスできる（Req 4.1）。
    let model = MountModel {
        names: GhostNames {
            name: Some("さくら".to_string()),
            sakura_name: Some("さくら".to_string()),
            kero_name: Some("うにゅう".to_string()),
        },
        shiori: ShioriMount {
            dir: PathBuf::from("ghost/master"),
            file: Some("shiori.dll".to_string()),
        },
        shell: ShellMount {
            dir: PathBuf::from("shell/master"),
        },
        bindgroups: BindGroupDefaults::default(),
    };

    // フィールドアクセス（正本の I/O 契約）。
    assert_eq!(model.names.name.as_deref(), Some("さくら"));
    assert_eq!(model.names.kero_name.as_deref(), Some("うにゅう"));
    assert_eq!(model.shiori.dir, PathBuf::from("ghost/master"));
    assert_eq!(model.shiori.file.as_deref(), Some("shiori.dll"));
    assert_eq!(model.shell.dir, PathBuf::from("shell/master"));
}

#[test]
fn ghost_names_default_is_all_none() {
    // 未指定＝欠落は None（推測しない・Req 1.4/2.3）。
    let names = GhostNames::default();
    assert_eq!(names.name, None);
    assert_eq!(names.sakura_name, None);
    assert_eq!(names.kero_name, None);
}

#[test]
fn ghost_names_partial_keeps_missing_as_none() {
    // 一部だけ指定した場合、残りは None のまま（欠落を保持・推測しない）。
    let names = GhostNames {
        name: Some("さくら".to_string()),
        ..GhostNames::default()
    };
    assert_eq!(names.name.as_deref(), Some("さくら"));
    assert_eq!(names.sakura_name, None);
    assert_eq!(names.kero_name, None);
}

#[test]
fn shiori_mount_missing_file_is_none() {
    // SHIORI ファイル名が未指定なら None（既定 shiori.dll へ推測しない・Req 2.3）。
    let shiori = ShioriMount {
        dir: PathBuf::from("ghost/master"),
        file: None,
    };
    assert_eq!(shiori.dir, PathBuf::from("ghost/master"));
    assert_eq!(shiori.file, None);
}

#[test]
fn mount_error_start_point_missing() {
    // 起点不在（Req 1.6/5.1）。
    let e = MountError::StartPointMissing {
        expected: PathBuf::from("ghost/master/descript.txt"),
    };
    assert_eq!(
        e,
        MountError::StartPointMissing {
            expected: PathBuf::from("ghost/master/descript.txt"),
        }
    );
    // 別 expected とは非等価。
    assert_ne!(
        e,
        MountError::StartPointMissing {
            expected: PathBuf::from("other/descript.txt"),
        }
    );
    // Debug が variant 名を含む。
    assert!(format!("{e:?}").contains("StartPointMissing"));
}

#[test]
fn mount_error_start_point_unreadable() {
    // 起点読取不能（I/O エラー・Req 1.1/5.1）。ErrorKind を保持する。
    let e = MountError::StartPointUnreadable {
        path: PathBuf::from("ghost/master/descript.txt"),
        kind: ErrorKind::PermissionDenied,
    };
    assert_eq!(
        e,
        MountError::StartPointUnreadable {
            path: PathBuf::from("ghost/master/descript.txt"),
            kind: ErrorKind::PermissionDenied,
        }
    );
    // kind が異なれば非等価（保持している証拠）。
    assert_ne!(
        e,
        MountError::StartPointUnreadable {
            path: PathBuf::from("ghost/master/descript.txt"),
            kind: ErrorKind::NotFound,
        }
    );
    assert!(format!("{e:?}").contains("StartPointUnreadable"));
}

#[test]
fn mount_error_shell_dir_missing() {
    // shell ディレクトリ不在（Req 3.3/5.1）。
    let e = MountError::ShellDirMissing {
        expected: PathBuf::from("shell/master"),
    };
    assert_eq!(
        e,
        MountError::ShellDirMissing {
            expected: PathBuf::from("shell/master"),
        }
    );
    assert!(format!("{e:?}").contains("ShellDirMissing"));
}

#[test]
fn mount_error_variants_are_distinct() {
    // 3 variant が相互に非等価（別失敗の取り違えを防ぐ）。
    let missing = MountError::StartPointMissing {
        expected: PathBuf::from("p"),
    };
    let unreadable = MountError::StartPointUnreadable {
        path: PathBuf::from("p"),
        kind: ErrorKind::Other,
    };
    let shell = MountError::ShellDirMissing {
        expected: PathBuf::from("p"),
    };
    assert_ne!(missing, unreadable);
    assert_ne!(unreadable, shell);
    assert_ne!(missing, shell);
}

#[test]
fn mount_model_derives_clone_and_eq() {
    // Clone / PartialEq / Eq が MountModel 全体で機能する（派生確認）。
    let model = MountModel {
        names: GhostNames::default(),
        shiori: ShioriMount {
            dir: PathBuf::from("ghost/master"),
            file: None,
        },
        shell: ShellMount {
            dir: PathBuf::from("shell/master"),
        },
        bindgroups: BindGroupDefaults::default(),
    };
    let cloned = model.clone();
    assert_eq!(model, cloned);
    // Debug が構造を出力する。
    assert!(format!("{model:?}").contains("MountModel"));
}
