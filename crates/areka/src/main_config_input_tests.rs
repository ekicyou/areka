use super::{ConfigInputs, default_balloon_root, default_ghost_root, resolve_config_inputs};
use std::path::PathBuf;

/// argv[1]/argv[2] が両方あるとき、両ルートを引数値でそのまま採用する（R3.3）。
#[test]
fn both_args_present_adopts_both() {
    let args = vec![
        "areka.exe".to_string(),
        "C:/custom/ghost".to_string(),
        "C:/custom/balloon".to_string(),
    ];
    let cfg = resolve_config_inputs(&args);
    assert_eq!(cfg.ghost_root, PathBuf::from("C:/custom/ghost"));
    assert_eq!(cfg.balloon_root, PathBuf::from("C:/custom/balloon"));
}

/// 引数なし（argv[0] のみ）のとき、両ルートとも既定へフォールバックする（R3.4）。
#[test]
fn no_args_uses_both_defaults() {
    let args = vec!["areka.exe".to_string()];
    let cfg = resolve_config_inputs(&args);
    assert_eq!(cfg.ghost_root, default_ghost_root());
    assert_eq!(cfg.balloon_root, default_balloon_root());
}

/// ghost のみ引数ありのとき、ghost は採用・balloon は既定にフォールバックする（R3.3/3.4）。
#[test]
fn ghost_only_arg_adopts_ghost_defaults_balloon() {
    let args = vec!["areka.exe".to_string(), "C:/custom/ghost".to_string()];
    let cfg = resolve_config_inputs(&args);
    assert_eq!(cfg.ghost_root, PathBuf::from("C:/custom/ghost"));
    assert_eq!(cfg.balloon_root, default_balloon_root());
}

/// 既定パスが `CARGO_MANIFEST_DIR` 相対で決定的に生成される（R3.4・DD1）。
#[test]
fn defaults_are_cargo_manifest_dir_relative_and_deterministic() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // 既定は CARGO_MANIFEST_DIR 配下にある（相対アンカー）。
    assert!(
        default_ghost_root().starts_with(&manifest),
        "ghost default must be under CARGO_MANIFEST_DIR: {:?}",
        default_ghost_root()
    );
    assert!(
        default_balloon_root().starts_with(&manifest),
        "balloon default must be under CARGO_MANIFEST_DIR: {:?}",
        default_balloon_root()
    );
    // 決定的: 呼び出しごとに同一値を返す。
    assert_eq!(default_ghost_root(), default_ghost_root());
    assert_eq!(default_balloon_root(), default_balloon_root());
}

/// `ConfigInputs` は解決済みルートパスを保持する（型の存在確認）。
#[test]
fn config_inputs_holds_resolved_roots() {
    let cfg = ConfigInputs {
        ghost_root: PathBuf::from("g"),
        balloon_root: PathBuf::from("b"),
    };
    assert_eq!(cfg.ghost_root, PathBuf::from("g"));
    assert_eq!(cfg.balloon_root, PathBuf::from("b"));
}
