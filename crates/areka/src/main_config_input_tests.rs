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
// 起動前の解決の判断（task 5.1・要件 4.1・4.7・4.8・5.1）
// ---------------------------------------------------------------------------
// `resolve_boot_from` へ根と argv を注入して、argv の有無で分かれる判断だけを踏む
// （ゴースト・バルーンの 13 分岐は `boot_resolve_tests.rs`、実プロセスの配線は smoke）。

mod boot {
    use crate::alert::AlertScene;
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
            crate::roots::single_env(root.path().to_path_buf()),
            &args(&[&given]),
            root.path(),
            no_pick,
        );
        assert_eq!(
            got.map(|_| ()),
            Err(AlertScene::GhostMissing {
                ghost_stores: vec![root.path().join("ghost")],
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
                crate::roots::single_env(root.path().to_path_buf()),
                &args(&[&ghost, &balloon]),
                root.path(),
                no_pick,
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

    /// argv が無く根にゴーストが 0 体なら、argv 無しの「ゴーストなし」（要件 4.7）。
    #[test]
    fn no_argv_and_empty_root_is_ghost_missing_without_argv() {
        let root = TempPath::new("boot-empty-root");
        let got = resolve_boot_from(
            crate::roots::single_env(root.path().to_path_buf()),
            &args(&[]),
            root.path(),
            no_pick,
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
    /// （目録を読む）は argv でゴーストを渡した起動では残さない（要件 1.15・2.5・11.2）。
    #[test]
    fn boot_lists_roots_and_counts_hidden_only_without_argv() {
        let has = |events: &[log_capture_kit::CapturedEvent], name: &str| {
            events.iter().any(|e| e.field_str("event") == Some(name))
        };
        let root = TempPath::new("boot-records");
        let (_, events) = capture(|| {
            resolve_boot_from(
                crate::roots::single_env(root.path().to_path_buf()),
                &args(&[]),
                root.path(),
                no_pick,
            )
        });
        assert!(has(&events, "root_listed"), "{events:?}");
        assert!(has(&events, "roots_hidden_count"), "{events:?}");

        let given = root.child("not-a-ghost");
        std::fs::create_dir_all(&given).expect("フォルダを組む");
        let (_, events) = capture(|| {
            resolve_boot_from(
                crate::roots::single_env(root.path().to_path_buf()),
                &args(&[&given]),
                root.path(),
                no_pick,
            )
        });
        assert!(has(&events, "root_listed"), "{events:?}");
        assert!(!has(&events, "roots_hidden_count"), "{events:?}");
    }
}

// ---------------------------------------------------------------------------
// 1 ゴースト分のバルーンの解決と、起動中の印（task 5.3・11.1・要件 4.7・12.4・12.5）
// ---------------------------------------------------------------------------
// `resolve_balloon_for_ghost` は切替先のバルーンを argv 無しの分岐で解く。起動前の解決は
// それを呼び、戻りの 4 つ目に「起動中の印の値（前回落ちたゴーストの名前・読むだけで消さない）」を載せる。

mod switch_balloon {
    use crate::boot_config::{resolve_balloon_for_ghost, resolve_boot_from};
    use crate::boot_resolve::{
        BalloonRoute, DEFAULT_GHOST_FOLDER, GhostRoute, read_session_mark, write_session_mark,
    };
    use log_capture_kit::capture;
    use std::path::{Path, PathBuf};
    use temp_path_kit::TempPath;

    /// 呼ばれない添字（候補に既定のバルーンを置くので無作為の段へ届かない）。
    fn no_pick(n: usize) -> usize {
        panic!("無作為の段へ届いてはならない（候補 {n}）")
    }

    /// 根にゴースト `g1` とバルーン 3 つ（既定・b1・b2）を組み、ゴーストのフォルダを返す。
    fn fixture(tmp: &TempPath) -> PathBuf {
        let ghost = tmp.child("ghost").join("g1");
        let master = ghost.join("ghost").join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
        for folder in ["StayseeBalloon", "b1", "b2"] {
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

    /// そのゴーストの最後のバルーンの記憶（Ghost スコープ・boot が据える場所と同じ）。
    fn put_memory(ghost: &Path, folder: &str) {
        let dir = ghost
            .join("ghost")
            .join("master")
            .join("profile")
            .join("areka");
        std::fs::create_dir_all(&dir).expect("フォルダを組む");
        std::fs::write(
            dir.join("sylphya.toml"),
            format!("format-version = 1\n[last]\nballoon = \"{folder}\"\n"),
        )
        .expect("記憶");
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

    /// 同梱バルーンがあればそれ（記憶が無いとき）。
    #[test]
    fn companion_is_used_when_no_memory() {
        let tmp = TempPath::new("switch-balloon-companion");
        let ghost = fixture(&tmp);
        put_companion(&ghost, "b1");
        assert_eq!(
            resolve(&tmp, &ghost),
            (BalloonRoute::Companion, Some("b1".to_owned()))
        );
    }

    /// そのゴーストの最後のバルーンの記憶があれば、同梱より先にそれ（起動解決と同じ順）。
    #[test]
    fn memory_of_that_ghost_is_used() {
        let tmp = TempPath::new("switch-balloon-memory");
        let ghost = fixture(&tmp);
        put_companion(&ghost, "b1");
        put_memory(&ghost, "b2");
        assert_eq!(
            resolve(&tmp, &ghost),
            (BalloonRoute::Memory, Some("b2".to_owned()))
        );
    }

    /// 同梱も記憶も無ければ既定のバルーン。
    #[test]
    fn default_when_neither_companion_nor_memory() {
        let tmp = TempPath::new("switch-balloon-default");
        let ghost = fixture(&tmp);
        assert_eq!(
            resolve(&tmp, &ghost),
            (BalloonRoute::Default, Some("StayseeBalloon".to_owned()))
        );
    }

    /// 初回起動の argv の第 2 引数は、1 ゴースト分の解決には効かない（要件 4.7）:
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
            crate::roots::single_env(tmp.path().to_path_buf()),
            &args,
            tmp.path(),
            no_pick,
        )
        .expect("argv で決まる");
        assert_eq!((b.route, b.dir), (BalloonRoute::Argv, argv_balloon));
        assert_eq!(
            resolve(&tmp, &ghost),
            (BalloonRoute::Default, Some("StayseeBalloon".to_owned()))
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
            crate::roots::single_env(tmp.path().to_path_buf()),
            &args,
            tmp.path(),
            no_pick,
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
        use areka_sylphya::persist::FsPersistIo;
        use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, save_scope};
        fixture(tmp);
        let master = tmp
            .child("ghost")
            .join(DEFAULT_GHOST_FOLDER)
            .join("ghost")
            .join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
        save_scope(
            PersistScope::App,
            &ScopeRoots {
                app: Some(app_dir.to_path_buf()),
                ..ScopeRoots::default()
            },
            &FsPersistIo,
            vec![(PersistKey::LastGhost, "g1".to_owned())],
        );
    }

    /// 印が在れば（要件 12.4）: `info!(session_mark_found)` の上で最後のゴーストの記憶を読まずに
    /// 既定の段で決まり、印の値が 4 つ目に載る。読むだけで消さない（2 度目も同じ）。
    /// 印が無ければ今日どおり記憶の経路。
    #[test]
    fn session_mark_is_returned_and_last_ghost_is_not_read() {
        let tmp = TempPath::new("switch-balloon-mark");
        let app_dir = tmp.child("app-profile");
        two_ghosts_remembering_g1(&tmp, &app_dir);
        let args = vec!["areka.exe".to_owned()];
        let boot = || {
            let (_, g, _, mark, _) = resolve_boot_from(
                crate::roots::single_env(tmp.path().to_path_buf()),
                &args,
                &app_dir,
                no_pick,
            )
            .expect("決まる");
            (g.route, g.folder, mark)
        };
        let without_mark = boot();
        write_session_mark(&app_dir, "落ちた子");
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

    /// argv でゴーストを指定した起動（開発者の上書き・要件 12.5）は印を読まず消さない。
    #[test]
    fn argv_ghost_neither_reads_nor_clears_session_mark() {
        let tmp = TempPath::new("switch-balloon-mark-argv");
        let ghost = fixture(&tmp);
        let app_dir = tmp.child("app-profile");
        write_session_mark(&app_dir, "落ちた子");
        let args = vec!["areka.exe".to_owned(), ghost.display().to_string()];
        let (got, events) = capture(|| {
            resolve_boot_from(
                crate::roots::single_env(tmp.path().to_path_buf()),
                &args,
                &app_dir,
                no_pick,
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

    /// 起動前の解決の結果から起動の文脈を組み、World の資源として読める（据え付けは 8.5）。
    #[test]
    fn boot_context_holds_the_resolved_current_ghost() {
        use crate::boot_config::{BootContext, CurrentGhost};
        let tmp = TempPath::new("switch-balloon-context");
        let ghost = fixture(&tmp);
        let (cfg, g, b, _, _) = resolve_boot_from(
            crate::roots::single_env(tmp.path().to_path_buf()),
            &["areka.exe".to_owned()],
            tmp.path(),
            no_pick,
        )
        .expect("決まる");
        let mut world = bevy_ecs::world::World::new();
        world.insert_resource(BootContext {
            roots: areka_ghost::BasewareRoots::single(tmp.path().to_path_buf()),
            roots_origin: crate::roots::RootsOrigin::EnvVar,
            app_profile_dir: tmp.path().to_path_buf(),
            helper_exe: tmp.child("shiori-host32-helper.exe"),
            argv_session: false,
            current: CurrentGhost {
                cfg,
                ghost: g,
                balloon: b,
            },
        });
        let ctx = world.resource::<BootContext>();
        assert_eq!(ctx.current.cfg.ghost_root, ghost);
        assert_eq!(ctx.current.ghost.folder.as_deref(), Some("g1"));
        assert_eq!(
            ctx.current.balloon.folder.as_deref(),
            Some("StayseeBalloon")
        );
        assert_eq!(
            ctx.roots.roots()[0].balloon_dir("b1"),
            tmp.child("balloon").join("b1")
        );
        assert_eq!(ctx.app_profile_dir, tmp.path());
        assert_eq!(ctx.helper_exe, tmp.child("shiori-host32-helper.exe"));
    }
}
