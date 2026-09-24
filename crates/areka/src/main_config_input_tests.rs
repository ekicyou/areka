use super::ConfigInputs;
use std::path::PathBuf;

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

// ---------------------------------------------------------------------------
// 起動前の解決の判断（task 5.1・要件 4.1・4.7・4.8・5.1）
// ---------------------------------------------------------------------------
// `resolve_boot_from` へ根と argv を注入して、argv の有無で分かれる判断だけを踏む
// （ゴースト・バルーンの 13 分岐は `boot_resolve_tests.rs`、実プロセスの配線は smoke）。

mod boot {
    use crate::alert::AlertScene;
    use crate::boot_config::RootSource;
    use crate::boot_config::resolve_boot_from;
    use crate::boot_resolve::{BalloonRoute, GhostRoute};
    use log_capture_kit::capture;
    use std::path::Path;
    use temp_path_kit::TempPath;

    /// 呼ばれない添字（この檻の入力では無作為の段へ届かない）。
    fn no_pick(n: usize) -> usize {
        panic!("無作為の段へ届いてはならない（候補 {n}）")
    }

    fn args(rest: &[&Path]) -> Vec<String> {
        std::iter::once("areka.exe".to_owned())
            .chain(rest.iter().map(|p| p.display().to_string()))
            .collect()
    }

    /// argv のフォルダに `ghost/master/descript.txt` が無ければ、argv 付きの「ゴーストなし」（要件 4.8）。
    #[test]
    fn argv_ghost_without_master_descript_is_ghost_missing_with_argv() {
        let root = TempPath::new("boot-argv-not-ghost");
        let given = root.child("not-a-ghost");
        std::fs::create_dir_all(&given).expect("フォルダを組む");
        let got = resolve_boot_from(
            Ok((root.path().to_path_buf(), RootSource::EnvVar)),
            &args(&[&given]),
            root.path(),
            no_pick,
        );
        assert_eq!(
            got.map(|_| ()),
            Err(AlertScene::GhostMissing {
                ghost_store: root.path().join("ghost"),
                argv: Some(given),
            })
        );
    }

    /// argv が両方あれば argv のとおりに決まり、列挙も記憶も読まない（要件 4.1・5.1・7.1）。
    /// 根には読めば warn になる罠（指す先の無い記憶・`type,plugin` のバルーン）を置く。
    #[test]
    fn argv_both_resolves_without_listing_or_reading_memory() {
        let root = TempPath::new("boot-argv-both");
        let ghost = root.child("elsewhere-ghost");
        let master = ghost.join("ghost").join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
        let balloon = root.child("elsewhere-balloon");
        let plugin = root.child("balloon").join("plugin");
        std::fs::create_dir_all(&plugin).expect("フォルダを組む");
        std::fs::write(plugin.join("descript.txt"), "charset,UTF-8\ntype,plugin\n")
            .expect("descript");
        std::fs::write(
            root.child("sylphya.toml"),
            "format-version = 1\n[last]\nghost = \"gone\"\n",
        )
        .expect("記憶");
        let (got, events) = capture(|| {
            resolve_boot_from(
                Ok((root.path().to_path_buf(), RootSource::EnvVar)),
                &args(&[&ghost, &balloon]),
                root.path(),
                no_pick,
            )
        });
        let (cfg, g, b) = got.expect("argv で決まる");
        assert_eq!((g.route, b.route), (GhostRoute::Argv, BalloonRoute::Argv));
        assert_eq!((cfg.ghost_root, cfg.balloon_root), (ghost, balloon));
        let warned: Vec<_> = events
            .iter()
            .filter(|e| e.level == tracing::Level::WARN)
            .collect();
        assert!(warned.is_empty(), "列挙・記憶を読んだ痕跡: {warned:?}");
    }

    /// argv が無く根にゴーストが 0 体なら、argv 無しの「ゴーストなし」（要件 4.7）。
    #[test]
    fn no_argv_and_empty_root_is_ghost_missing_without_argv() {
        let root = TempPath::new("boot-empty-root");
        let got = resolve_boot_from(
            Ok((root.path().to_path_buf(), RootSource::EnvVar)),
            &args(&[]),
            root.path(),
            no_pick,
        );
        assert_eq!(
            got.map(|_| ()),
            Err(AlertScene::GhostMissing {
                ghost_store: root.path().join("ghost"),
                argv: None,
            })
        );
    }
}
