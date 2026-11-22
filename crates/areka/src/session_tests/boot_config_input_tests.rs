// ---------------------------------------------------------------------------
// 起動前の解決の判断
// ---------------------------------------------------------------------------
// `resolve_boot_from` へ根と argv を注入して、argv の有無で分かれる判断だけを踏む
// （ゴーストの 9 通り・バルーンの 8 分岐は `boot_resolve_tests.rs`・`boot_resolve_balloon_tests.rs`、実プロセスの配線は smoke）。

mod boot {
    use crate::session_tests::{no_pick, resolve_boot_from, save_scope};
    use areka_session::alert::AlertScene;
    use areka_session::boot_resolve::{BalloonRoute, GhostRoute};
    use log_capture_kit::capture;
    use std::path::Path;
    use temp_path_kit::TempPath;

    fn args(rest: &[&Path]) -> Vec<String> {
        std::iter::once("areka.exe".to_owned())
            .chain(rest.iter().map(|p| p.display().to_string()))
            .collect()
    }

    /// argv のフォルダに `ghost/master/descript.txt` が無ければ、argv 付きの「ゴーストなし」。
    #[test]
    fn argv_ghost_without_master_descript_is_ghost_missing_with_argv() {
        let root = TempPath::new("boot-argv-not-ghost");
        let given = root.child("not-a-ghost");
        std::fs::create_dir_all(&given).expect("フォルダを組む");
        let got = resolve_boot_from(
            areka_session::roots::single_env(root.path().to_path_buf()),
            &args(&[&given]),
            root.path(),
            no_pick,
            |_, dir| dir.to_path_buf(),
        );
        assert_eq!(
            got.map(|_| ()),
            Err(AlertScene::GhostMissing {
                ghost_stores: vec![root.path().join("ghost")],
                argv: Some(given),
            })
        );
    }

    /// argv が両方あれば argv のとおりに決まり、列挙も記憶も読まない。
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
        save_scope(root.path(), "gone");
        let (got, events) = capture(|| {
            resolve_boot_from(
                areka_session::roots::single_env(root.path().to_path_buf()),
                &args(&[&ghost, &balloon]),
                root.path(),
                no_pick,
                |_, dir| dir.to_path_buf(),
            )
        });
        let (cfg, g, b, _halted, _) = got.expect("argv で決まる");
        assert_eq!((g.route, b.route), (GhostRoute::Argv, BalloonRoute::Argv));
        assert_eq!((cfg.ghost_root, cfg.balloon_root), (ghost, balloon));
        let warned: Vec<_> = events
            .iter()
            .filter(|e| e.level == tracing::Level::WARN)
            .collect();
        assert!(warned.is_empty(), "列挙・記憶を読んだ痕跡: {warned:?}");
    }

    /// argv が無く根にゴーストが 0 体なら、argv 無しの「ゴーストなし」。
    #[test]
    fn no_argv_and_empty_root_is_ghost_missing_without_argv() {
        let root = TempPath::new("boot-empty-root");
        let got = resolve_boot_from(
            areka_session::roots::single_env(root.path().to_path_buf()),
            &args(&[]),
            root.path(),
            no_pick,
            |_, dir| dir.to_path_buf(),
        );
        assert_eq!(
            got.map(|_| ()),
            Err(AlertScene::GhostMissing {
                ghost_stores: vec![root.path().join("ghost")],
                argv: None,
            })
        );
    }

    /// 起動の記録: 並びの 1 根ごとの `root_listed` は argv の有無によらず残し、隠れの件数
    /// （目録を読む）は argv でゴーストを渡した起動では残さない。
    #[test]
    fn boot_lists_roots_and_counts_hidden_only_without_argv() {
        let has = |events: &[log_capture_kit::CapturedEvent], name: &str| {
            events.iter().any(|e| e.field_str("event") == Some(name))
        };
        let root = TempPath::under_target("boot-records");
        let (_, events) = capture(|| {
            resolve_boot_from(
                areka_session::roots::single_env(root.path().to_path_buf()),
                &args(&[]),
                root.path(),
                no_pick,
                |_, dir| dir.to_path_buf(),
            )
        });
        assert!(has(&events, "root_listed"), "{events:?}");
        assert!(has(&events, "roots_hidden_count"), "{events:?}");

        let given = root.child("not-a-ghost");
        std::fs::create_dir_all(&given).expect("フォルダを組む");
        let (_, events) = capture(|| {
            resolve_boot_from(
                areka_session::roots::single_env(root.path().to_path_buf()),
                &args(&[&given]),
                root.path(),
                no_pick,
                |_, dir| dir.to_path_buf(),
            )
        });
        assert!(has(&events, "root_listed"), "{events:?}");
        assert!(!has(&events, "roots_hidden_count"), "{events:?}");
    }
}

// ---------------------------------------------------------------------------
// 1 ゴースト分のバルーンの解決と、起動中の印
// ---------------------------------------------------------------------------
// `resolve_balloon_for_ghost` は切替先のバルーンを argv 無しの分岐で解く。起動前の解決は
// それを呼び、戻りの 4 つ目に「起動中の印の値（前回落ちたゴーストの名前・読むだけで消さない）」を載せる。

mod switch_balloon {
    use crate::session_tests::{no_pick, put_ghost, resolve_boot_from, save_scope};
    use areka_session::boot_config::resolve_balloon_for_ghost;
    use areka_session::boot_resolve::{
        BalloonRoute, DEFAULT_BALLOON_FOLDER, DEFAULT_GHOST_FOLDER, GhostRoute, read_session_mark,
        write_session_mark,
    };
    use log_capture_kit::capture;
    use std::path::{Path, PathBuf};
    use temp_path_kit::TempPath;

    /// 根にゴースト `g1` とバルーン 3 つ（既定・b1・b2）を組み、ゴーストのフォルダを返す。
    fn fixture(tmp: &TempPath) -> PathBuf {
        let ghost = tmp.child("ghost").join("g1");
        let master = ghost.join("ghost").join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
        for folder in [DEFAULT_BALLOON_FOLDER, "b1", "b2"] {
            let dir = tmp.child("balloon").join(folder);
            std::fs::create_dir_all(&dir).expect("フォルダを組む");
            std::fs::write(dir.join("descript.txt"), "charset,UTF-8\n").expect("descript");
        }
        ghost
    }

    fn put_companion(ghost: &Path, folder: &str) {
        std::fs::write(
            ghost.join("install.txt"),
            format!("charset,UTF-8\nballoon.directory,{folder}\n"),
        )
        .expect("install.txt");
    }

    fn resolve(tmp: &TempPath, ghost: &Path) -> (BalloonRoute, Option<String>) {
        let root = areka_ghost::BasewareRoots::single(tmp.path().to_path_buf());
        let got = resolve_balloon_for_ghost(&root, ghost, no_pick).expect("決まる");
        assert_eq!(
            got.dir,
            tmp.child("balloon").join(got.folder.as_deref().unwrap())
        );
        (got.route, got.folder)
    }

    /// 初回起動の argv の第 2 引数は、1 ゴースト分の解決には効かない:
    /// 同じ根・同じゴーストでも起動前の解決は argv のバルーン、括り出した関数は既定になる。
    #[test]
    fn argv_second_argument_has_no_effect() {
        let tmp = TempPath::new("switch-balloon-argv");
        let ghost = fixture(&tmp);
        let argv_balloon = tmp.child("balloon").join("b1");
        let args = vec![
            "areka.exe".to_owned(),
            ghost.display().to_string(),
            argv_balloon.display().to_string(),
        ];
        let (_, _, b, _, _) = resolve_boot_from(
            areka_session::roots::single_env(tmp.path().to_path_buf()),
            &args,
            tmp.path(),
            no_pick,
            |_, dir| dir.to_path_buf(),
        )
        .expect("argv で決まる");
        assert_eq!((b.route, b.dir), (BalloonRoute::Argv, argv_balloon));
        assert_eq!(
            resolve(&tmp, &ghost),
            (
                BalloonRoute::Default,
                Some(DEFAULT_BALLOON_FOLDER.to_owned())
            )
        );
    }

    /// argv 無しの起動前の解決は後半を括り出した関数へ任せる（初回起動の結果は不変）。
    #[test]
    fn resolve_boot_uses_the_same_balloon_resolution() {
        let tmp = TempPath::new("switch-balloon-boot");
        let ghost = fixture(&tmp);
        put_companion(&ghost, "b1");
        let args = vec!["areka.exe".to_owned()];
        let (cfg, g, b, mark, _) = resolve_boot_from(
            areka_session::roots::single_env(tmp.path().to_path_buf()),
            &args,
            tmp.path(),
            no_pick,
            |_, dir| dir.to_path_buf(),
        )
        .expect("決まる");
        assert_eq!(g.dir, ghost);
        assert_eq!(
            (b.route, b.folder.as_deref()),
            (BalloonRoute::Companion, Some("b1"))
        );
        assert_eq!(cfg.balloon_root, tmp.child("balloon").join("b1"));
        assert_eq!(mark, None, "印が無ければ無し");
    }

    /// 根にゴースト `g1` と既定ゴーストを置き、最後のゴーストの記憶を `g1` にする。
    fn two_ghosts_remembering_g1(tmp: &TempPath, app_dir: &Path) {
        fixture(tmp);
        put_ghost(&tmp.child("ghost").join(DEFAULT_GHOST_FOLDER));
        save_scope(app_dir, "g1");
    }

    /// 印が在れば: `info!(session_mark_found)` の上で最後のゴーストの記憶を読まずに
    /// 既定の段で決まり、印の値が 4 つ目に載る。読むだけで消さない（2 度目も同じ）。
    /// 印が無ければ記憶の経路。
    #[test]
    fn session_mark_is_returned_and_last_ghost_is_not_read() {
        let tmp = TempPath::new("switch-balloon-mark");
        let app_dir = tmp.child("app-profile");
        two_ghosts_remembering_g1(&tmp, &app_dir);
        let args = vec!["areka.exe".to_owned()];
        let boot = || {
            let (_, g, _, mark, _) = resolve_boot_from(
                areka_session::roots::single_env(tmp.path().to_path_buf()),
                &args,
                &app_dir,
                no_pick,
                |_, dir| dir.to_path_buf(),
            )
            .expect("決まる");
            (g.route, g.folder, mark)
        };
        let without_mark = boot();
        write_session_mark(&app_dir, "落ちた子").expect("印を書く");
        let (first, events) = capture(boot);
        let second = boot();
        let found = events
            .iter()
            .filter(|e| {
                e.level == tracing::Level::INFO
                    && e.field_str("event") == Some("session_mark_found")
            })
            .count();
        let with_mark = (
            GhostRoute::Default,
            Some(DEFAULT_GHOST_FOLDER.to_owned()),
            Some("落ちた子".to_owned()),
        );
        assert_eq!(
            (
                without_mark,
                first,
                second,
                found,
                read_session_mark(&app_dir)
            ),
            (
                (GhostRoute::Memory, Some("g1".to_owned()), None),
                with_mark.clone(),
                with_mark,
                1,
                Some("落ちた子".to_owned())
            ),
            "印の読みが崩れた（印なし・印あり 1 度目・2 度目・session_mark_found の件数・印は残る）"
        );
    }

    /// argv でゴーストを指定した起動（開発者の上書き）は印を読まず消さない。
    #[test]
    fn argv_ghost_neither_reads_nor_clears_session_mark() {
        let tmp = TempPath::new("switch-balloon-mark-argv");
        let ghost = fixture(&tmp);
        let app_dir = tmp.child("app-profile");
        write_session_mark(&app_dir, "落ちた子").expect("印を書く");
        let args = vec!["areka.exe".to_owned(), ghost.display().to_string()];
        let (got, events) = capture(|| {
            resolve_boot_from(
                areka_session::roots::single_env(tmp.path().to_path_buf()),
                &args,
                &app_dir,
                no_pick,
                |_, dir| dir.to_path_buf(),
            )
            .expect("argv で決まる")
            .3
        });
        let found = events
            .iter()
            .filter(|e| e.field_str("event") == Some("session_mark_found"))
            .count();
        assert_eq!(
            (got, found, read_session_mark(&app_dir)),
            (None, 0, Some("落ちた子".to_owned())),
            "argv の起動が印に触れた（戻り・session_mark_found の件数・印は残る）"
        );
    }
}
