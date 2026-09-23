// `default_ghost_root`／`default_balloon_root` の消費者は本檻だけなので、crate 直下の
// 再輸出には載せず定義元 `boot_config` から直接引く（本番ビルドで unused にならない）。
use super::{ConfigInputs, resolve_config_inputs};
use crate::boot_config::{default_balloon_root, default_ghost_root};
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

// ---------------------------------------------------------------------------
// 根の決め方（task 2.3・要件 1.2〜1.4・8.2）
// ---------------------------------------------------------------------------
// `resolve_root_from` へ env の値と実行ファイルの場所を注入して踏む。プロセスの env は
// 書かない（要件 8.2）。一時フォルダは共通窓口 `temp-path-kit` から受け取る。

mod root {
    use crate::boot_config::{RootError, RootSource, resolve_root_from};
    use std::path::PathBuf;
    use temp_path_kit::TempPath;

    /// `AREKA_ROOT` が実在のフォルダを指すとき、その値を根とし exe は見ない（要件 1.3）。
    #[test]
    fn env_present_existing_dir_is_root_and_exe_is_ignored() {
        let root = TempPath::new("root-env-exists");
        // exe は実在しない場所を指す。env があれば見ないので結果に現れない。
        let exe = PathBuf::from(r"C:\areka-no-such-dir\areka.exe");
        let got = resolve_root_from(Some(root.path().to_path_buf()), Some(exe));
        assert_eq!(got, Ok((root.path().to_path_buf(), RootSource::EnvVar)));
    }

    /// `AREKA_ROOT` が実在しない場所を指すとき、exe の隣へ倒さず失敗する（要件 1.3・1.4）。
    #[test]
    fn env_present_missing_dir_fails_without_falling_back_to_exe() {
        let exe_dir = TempPath::new("root-env-missing-exe");
        let missing = exe_dir.child("no-such-root");
        let got = resolve_root_from(Some(missing.clone()), Some(exe_dir.child("areka.exe")));
        assert_eq!(
            got,
            Err(RootError::NotADirectory {
                dir: missing,
                source: RootSource::EnvVar,
            })
        );
    }

    /// `AREKA_ROOT` が無いとき、実行ファイルのあるフォルダを根とする（要件 1.2・9.1）。
    #[test]
    fn env_absent_exe_dir_exists_is_root() {
        let exe_dir = TempPath::new("root-exe-exists");
        let got = resolve_root_from(None, Some(exe_dir.child("areka.exe")));
        assert_eq!(got, Ok((exe_dir.path().to_path_buf(), RootSource::ExeDir)));
    }

    /// `AREKA_ROOT` が無く、exe の隣が実在しないとき失敗する（要件 1.4）。
    #[test]
    fn env_absent_exe_dir_missing_fails() {
        let base = TempPath::new("root-exe-missing");
        let missing = base.child("gone");
        let got = resolve_root_from(None, Some(missing.join("areka.exe")));
        assert_eq!(
            got,
            Err(RootError::NotADirectory {
                dir: missing,
                source: RootSource::ExeDir,
            })
        );
    }

    /// `AREKA_ROOT` も exe の場所も無いとき、`"."` へ倒さず失敗する（要件 1.4）。
    #[test]
    fn env_absent_exe_unavailable_fails_without_dot_fallback() {
        assert_eq!(
            resolve_root_from(None, None),
            Err(RootError::ExeLocationUnavailable)
        );
    }

    /// 空の `AREKA_ROOT` は「設定あり」: exe の隣へもカレントへも倒さず失敗する。
    #[test]
    fn empty_env_fails_without_fallback() {
        assert_eq!(
            resolve_root_from(
                Some(PathBuf::new()),
                Some(PathBuf::from("C:\\x\\areka.exe"))
            ),
            Err(RootError::NotADirectory {
                dir: PathBuf::new(),
                source: RootSource::EnvVar
            })
        );
    }

    /// 相対の `AREKA_ROOT` はカレント基準で絶対化してから検査し、絶対パスで返る
    /// （`canonicalize` ではなく `std::path::absolute`）。
    #[test]
    fn relative_env_is_returned_absolute() {
        // `cargo test` のカレントはパッケージの根で、`src` は必ず実在する。
        let got = resolve_root_from(Some(PathBuf::from("src")), None);
        let expected = std::env::current_dir().expect("カレント取得").join("src");
        assert_eq!(got, Ok((expected.clone(), RootSource::EnvVar)));
        assert!(expected.is_absolute());
    }
}
