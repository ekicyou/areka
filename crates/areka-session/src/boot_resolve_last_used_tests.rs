//! 最後に使った物の記憶の書き手・直読み・起動中の印・起動のシェルの決定論テスト。
//!
//! 確かめること: argv で決まった側は書かない・フォルダ名が無い側とシェルの名が取れない記録は
//! 書かずに残す・1 つだけ書く書き手は他の鍵に触れない・起動中の印の往復と書けないときの警告・
//! 記憶のシェルは `shell/<名>/descript.txt` が在るときだけ使う。書き手は fake の置き場か、
//! ワークツリーの `target\` の下の一時の置き場へ書く。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_ghost::BasewareRoot;
use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_placement::test_support::SharedFakeIo;
use areka_runtime::spine::join_bounded;
use areka_sylphya::persist::{FakePersistIo, FsPersistIo};
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya};
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::boot_resolve_tests::{assert_one_event, balloon_at, count_event, ghost_at, root};
use super::*;

fn balloon_argv() -> BalloonDecision {
    BalloonDecision {
        route: BalloonRoute::Argv,
        dir: PathBuf::from("/dev/balloon"),
        folder: None,
    }
}

fn ghost_argv() -> GhostDecision {
    GhostDecision {
        route: GhostRoute::Argv,
        dir: PathBuf::from("/dev/ghost"),
        folder: None,
    }
}

/// 読み戻した (App, Ghost, 捕捉したログ)。
type Loaded = (
    Vec<(PersistKey, String)>,
    Vec<(PersistKey, String)>,
    Vec<CapturedEvent>,
);

/// fake の置き場へ `seed` を Ghost スコープに置いてから `write` を実 publisher へ投函し、
/// barrier 後に App・Ghost の 2 スコープを読み戻す。2 つ目は書き手（アクター）が commit した回数。
fn write_and_load(
    seed: Vec<(PersistKey, String)>,
    write: impl FnOnce(&SylphyaPublisher),
) -> (Loaded, usize) {
    let io = SharedFakeIo::new(Arc::new(FakePersistIo::new()));
    let roots = ScopeRoots {
        app: Some(PathBuf::from("/app")),
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    if !seed.is_empty() {
        save_scope(PersistScope::Ghost, &roots, &io, seed);
    }
    let seeded = io.commit_count();
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(io.clone()),
        runtime_sink: None,
    });
    let ((), events) = capture(|| write(&parts.publisher));
    parts
        .publisher
        .barrier()
        .expect("アクターが生きている間は barrier が返る");
    let app = load_scope(PersistScope::App, &roots, &io);
    let ghost_scope = load_scope(PersistScope::Ghost, &roots, &io);
    parts.publisher.close();
    join_bounded("sylphya", parts.handle).expect("記憶の書き手が panic せずに終わる");
    ((app, ghost_scope, events), io.commit_count() - seeded)
}

/// `record` を投函して読み戻す（前から在る記憶なし）。
fn record_and_load(ghost: &GhostDecision, balloon: &BalloonDecision, shell: &str) -> Loaded {
    write_and_load(vec![], |p| {
        LastUsed {
            ghost,
            balloon,
            shell_folder: shell,
        }
        .record(p)
    })
    .0
}

fn entry(key: PersistKey, value: &str) -> (PersistKey, String) {
    (key, value.to_owned())
}

/// argv 以外: App に ghost、Ghost に balloon と shell。
#[test]
fn record_non_argv_writes_all_three() {
    let root = root();
    let g = ghost_at(&root, GhostRoute::Memory, "emo2");
    let b = balloon_at(&root, BalloonRoute::Companion, "StayseeBalloon");
    let (app, ghost_scope, events) = record_and_load(&g, &b, "master");
    assert_eq!(app, vec![entry(PersistKey::LastGhost, "emo2")]);
    assert_eq!(
        ghost_scope,
        vec![
            entry(PersistKey::LastBalloon, "StayseeBalloon"),
            entry(PersistKey::LastShell, "master"),
        ]
    );
    assert_one_event(&events, "last_used_recorded", tracing::Level::INFO);
    assert_eq!(count_event(&events, "last_used_skipped_argv"), 0);
}

/// argv のゴースト: App に書かれない・Ghost の balloon と shell は書かれる。
#[test]
fn record_argv_ghost_skips_app_keeps_shell() {
    let root = root();
    let b = balloon_at(&root, BalloonRoute::Only, "b1");
    let (app, ghost_scope, events) = record_and_load(&ghost_argv(), &b, "alt");
    assert_eq!(app, vec![], "argv のゴーストは App の記憶を書き換えない");
    assert_eq!(
        ghost_scope,
        vec![
            entry(PersistKey::LastBalloon, "b1"),
            entry(PersistKey::LastShell, "alt"),
        ]
    );
    assert_one_event(&events, "last_used_skipped_argv", tracing::Level::INFO);
    assert_one_event(&events, "last_used_recorded", tracing::Level::INFO);
}

/// argv のバルーン: Ghost の balloon だけ書かれない（ghost と shell は書く）。
#[test]
fn record_argv_balloon_skips_balloon_only() {
    let root = root();
    let g = ghost_at(&root, GhostRoute::Random, "g2");
    let (app, ghost_scope, events) = record_and_load(&g, &balloon_argv(), "master");
    assert_eq!(app, vec![entry(PersistKey::LastGhost, "g2")]);
    assert_eq!(ghost_scope, vec![entry(PersistKey::LastShell, "master")]);
    assert_one_event(&events, "last_used_skipped_argv", tracing::Level::INFO);
}

/// 両方 argv: shell だけ書かれる。
#[test]
fn record_both_argv_writes_shell_only() {
    let (app, ghost_scope, events) = record_and_load(&ghost_argv(), &balloon_argv(), "master");
    assert_eq!(app, vec![]);
    assert_eq!(ghost_scope, vec![entry(PersistKey::LastShell, "master")]);
    assert_eq!(count_event(&events, "last_used_skipped_argv"), 2);
}

/// argv でないのにフォルダ名の無いゴースト: `warn!` 1 件で App には何も書かない（Ghost の 2 つは書く）。
#[test]
fn record_without_ghost_folder_warns_and_skips_app() {
    let root = root();
    let g = GhostDecision {
        route: GhostRoute::Memory,
        dir: root.ghost_dir("lost"),
        folder: None,
    };
    let b = balloon_at(&root, BalloonRoute::Memory, "b1");
    let (app, ghost_scope, events) = record_and_load(&g, &b, "master");
    assert_eq!(app, vec![], "フォルダ名の無いゴーストを App に書いた");
    assert_eq!(
        ghost_scope,
        vec![
            entry(PersistKey::LastBalloon, "b1"),
            entry(PersistKey::LastShell, "master"),
        ]
    );
    assert_one_event(&events, "last_used_folder_missing", tracing::Level::WARN);
    assert_eq!(count_event(&events, "last_used_skipped_argv"), 0);
}

/// シェルの置き場の末尾が取れない（`C:\`）: `error!` 1 件で 3 つとも書かず、書き手は 1 度も commit しない。
#[test]
fn record_last_used_without_shell_folder_writes_nothing() {
    let root = root();
    let g = ghost_at(&root, GhostRoute::Memory, "emo2");
    let b = balloon_at(&root, BalloonRoute::Memory, "b1");
    let ((app, ghost_scope, events), commits) = write_and_load(vec![], |p| {
        crate::boot_seam::record_last_used(p, Path::new(r"C:\"), &g, &b)
    });
    assert_eq!(
        (app, ghost_scope, commits),
        (vec![], vec![], 0),
        "シェルの名が取れないのに記憶を書いた（App・Ghost・commit の回数）"
    );
    assert_one_event(
        &events,
        "last_used_shell_folder_missing",
        tracing::Level::ERROR,
    );
    assert_eq!(count_event(&events, "last_used_recorded"), 0);
}

/// 直読みは boot と同じ場所を見る: 実 fs へ record したものを `read_last_*` で拾える。
/// 書かれていなければ無し。
#[test]
fn read_last_sees_what_record_wrote_on_real_fs() {
    let tmp = TempPath::new("boot-resolve-last-used");
    let app_dir = tmp.child("app-profile");
    let ghost_dir = tmp.child("ghost-a");
    assert_eq!(read_last_ghost(&app_dir), None);
    assert_eq!(read_last_balloon(&ghost_dir), None);

    let parts = spawn_sylphya(SylphyaInit {
        roots: ScopeRoots {
            app: Some(app_dir.clone()),
            ghost: Some(profile_areka_root(&ghost_dir.join("ghost").join("master"))),
            ..ScopeRoots::default()
        },
        io: Box::new(FsPersistIo),
        runtime_sink: None,
    });
    let root = root();
    LastUsed {
        ghost: &ghost_at(&root, GhostRoute::Only, "a"),
        balloon: &balloon_at(&root, BalloonRoute::Default, "StayseeBalloon"),
        shell_folder: "master",
    }
    .record(&parts.publisher);
    parts.publisher.barrier().expect("barrier");
    parts.publisher.close();
    join_bounded("sylphya", parts.handle).expect("記憶の書き手が panic せずに終わる");

    assert_eq!(read_last_ghost(&app_dir).as_deref(), Some("a"));
    assert_eq!(
        read_last_balloon(&ghost_dir).as_deref(),
        Some("StayseeBalloon")
    );
}

/// 切替で決まったゴーストは記憶を書く経路: App に書かれ argv の skip は出ない。
#[test]
fn record_switched_writes_last_ghost() {
    let root = root();
    let g = ghost_at(&root, GhostRoute::Switched, "g2");
    let b = balloon_at(&root, BalloonRoute::Memory, "b1");
    let (app, _, events) = record_and_load(&g, &b, "master");
    assert_eq!(app, vec![entry(PersistKey::LastGhost, "g2")]);
    assert_eq!(count_event(&events, "last_used_skipped_argv"), 0);
}

/// 起動中の印の往復: 書く → 読む → 消す（空文字＝無し）。最後のゴーストには触れない。
#[test]
fn session_mark_round_trip_and_clear() {
    let tmp = TempPath::new("boot-resolve-session-mark");
    let app_dir = tmp.child("app-profile");
    let before = read_session_mark(&app_dir);

    let (_, written) = capture(|| write_session_mark(&app_dir, "動いている子"));
    let after_write = read_session_mark(&app_dir);
    let (_, cleared) = capture(|| clear_session_mark(&app_dir));

    assert_eq!(
        (
            before,
            after_write,
            read_session_mark(&app_dir),
            read_last_ghost(&app_dir)
        ),
        (None, Some("動いている子".to_owned()), None, None),
        "印の往復が崩れた（書く前・書いた後・消した後・最後のゴースト）"
    );
    assert_one_event(&written, "session_mark_written", tracing::Level::INFO);
    assert_one_event(&cleared, "session_mark_cleared", tracing::Level::INFO);
}

/// 切替の降ろした直後: 1 回の書き込みで最後のゴースト＝既定・印＝切替先。
/// 印を渡さない（argv で始まったプロセス）なら最後のゴーストだけで、前から在った印は残る。
#[test]
fn switch_drop_writes_default_and_mark_at_once() {
    let tmp = TempPath::new("boot-resolve-switch-drop");
    let app_dir = tmp.child("app-profile");
    let argv_dir = tmp.child("app-profile-argv");
    let _ = write_session_mark(&argv_dir, "前からの印");

    let (_, events) = capture(|| write_switch_drop(&app_dir, Some("切替先")));
    write_switch_drop(&argv_dir, None).expect("argv の降ろした直後を書く");

    assert_eq!(
        (
            read_last_ghost(&app_dir),
            read_session_mark(&app_dir),
            read_last_ghost(&argv_dir),
            read_session_mark(&argv_dir),
        ),
        (
            Some(DEFAULT_GHOST_FOLDER.to_owned()),
            Some("切替先".to_owned()),
            Some(DEFAULT_GHOST_FOLDER.to_owned()),
            Some("前からの印".to_owned()),
        ),
        "降ろした直後の記憶が崩れた（最後のゴースト・印／argv の最後のゴースト・印）"
    );
    assert_one_event(&events, "switch_drop_recorded", tracing::Level::INFO);
}

/// 書けないとき（App の置き場が普通のファイル）: 3 つの書き手（切替の降ろした直後は印あり・argv の
/// 印なしの 2 通り）はそれぞれ `warn!` 1 件で戻り、
/// 成功の info は出ない。
#[test]
fn session_mark_writers_warn_once_when_they_cannot_write() {
    let tmp = TempPath::new("boot-resolve-session-mark-degraded");
    let app_dir = tmp.child("app-profile");
    std::fs::create_dir_all(app_dir.parent().unwrap()).unwrap();
    std::fs::write(&app_dir, "not a dir").unwrap();
    let roots = ScopeRoots {
        app: Some(app_dir.clone()),
        ..ScopeRoots::default()
    };
    // 前提の確認: この置き場への保存は本当に縮退する。
    assert!(matches!(
        save_scope(PersistScope::App, &roots, &FsPersistIo, vec![]),
        PersistOutcome::Degraded(_)
    ));
    let cases: [(&str, &str, Box<dyn Fn()>); 4] = [
        (
            "session_mark_write_degraded",
            "session_mark_written",
            Box::new(|| drop(write_session_mark(&app_dir, "動いている子"))),
        ),
        (
            "switch_drop_record_degraded",
            "switch_drop_recorded",
            Box::new(|| drop(write_switch_drop(&app_dir, Some("切替先")))),
        ),
        (
            // argv で始まったプロセス（印に触れない）でも最後のゴーストが書けなければ警告する。
            "switch_drop_record_degraded",
            "switch_drop_recorded",
            Box::new(|| drop(write_switch_drop(&app_dir, None))),
        ),
        (
            "session_mark_clear_degraded",
            "session_mark_cleared",
            Box::new(|| drop(clear_session_mark(&app_dir))),
        ),
    ];
    for (warn, info, write) in cases {
        let ((), events) = capture(|| write());
        assert_one_event(&events, warn, tracing::Level::WARN);
        assert_eq!(count_event(&events, info), 0, "{info} が出た");
    }
}

/// 印に書く名前: 目録の同じフォルダの descript の `name`、無ければフォルダ名。
#[test]
fn running_name_prefers_descript_name_then_folder() {
    let tmp = TempPath::new("boot-resolve-running-name");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    for (folder, descript) in [
        ("named", "charset,UTF-8\nname,名のある子\n"),
        ("nameless", "charset,UTF-8\n"),
    ] {
        let master = root.ghost_dir(folder).join("ghost").join("master");
        std::fs::create_dir_all(&master).unwrap();
        std::fs::write(master.join("descript.txt"), descript).unwrap();
    }
    assert_eq!(
        [
            running_name(&ghost_at(&root, GhostRoute::Memory, "named")),
            running_name(&ghost_at(&root, GhostRoute::Default, "nameless")),
            running_name(&ghost_at(&root, GhostRoute::Only, "missing")),
        ],
        [
            "名のある子".to_owned(),
            "nameless".to_owned(),
            "missing".to_owned()
        ],
        "印の名前が descript の name → フォルダ名の順でない（在る・name 無し・目録に無い）"
    );
}

// ------------------------------------------------ 1 つだけ書く書き手と起動のシェル

/// 前から在った Ghost スコープの 2 つの鍵を置いてから `write` を投函して読み戻す。
fn write_one_and_load(write: impl FnOnce(&SylphyaPublisher)) -> Loaded {
    let seed = vec![
        entry(PersistKey::LastBalloon, "old-balloon"),
        entry(PersistKey::LastShell, "old-shell"),
    ];
    write_and_load(seed, write).0
}

/// シェルの差し替えは `LastShell` だけ、バルーンの差し替えは `LastBalloon` だけを書く。
/// 他方の鍵と App スコープには触れない。
#[test]
fn single_key_writers_touch_only_their_key() {
    let (app, ghost_scope, events) = write_one_and_load(|p| record_last_shell(p, "second"));
    assert_eq!(app, vec![], "シェルの書き手が App に触れた");
    assert_eq!(
        ghost_scope,
        vec![
            entry(PersistKey::LastBalloon, "old-balloon"),
            entry(PersistKey::LastShell, "second"),
        ],
        "シェルの書き手がバルーンの記憶に触れた"
    );
    assert_one_event(&events, "last_shell_recorded", tracing::Level::INFO);
    assert_eq!(count_event(&events, "last_used_recorded"), 0);

    let (app, ghost_scope, events) = write_one_and_load(|p| record_last_balloon(p, "b2"));
    assert_eq!(app, vec![], "バルーンの書き手が App に触れた");
    assert_eq!(
        ghost_scope,
        vec![
            entry(PersistKey::LastBalloon, "b2"),
            entry(PersistKey::LastShell, "old-shell"),
        ],
        "バルーンの書き手がシェルの記憶に触れた"
    );
    assert_one_event(&events, "last_balloon_recorded", tracing::Level::INFO);
    assert_eq!(count_event(&events, "last_used_recorded"), 0);
}

/// 実 fs の、起動と同じ置き場へ `record_last_shell` で書く（`read_last_shell` が拾う場所）。
fn remember_shell(ghost_dir: &Path, name: &str) {
    let parts = spawn_sylphya(SylphyaInit {
        roots: ScopeRoots {
            ghost: Some(profile_areka_root(&ghost_dir.join("ghost").join("master"))),
            ..ScopeRoots::default()
        },
        io: Box::new(FsPersistIo),
        runtime_sink: None,
    });
    record_last_shell(&parts.publisher, name);
    parts.publisher.barrier().expect("barrier");
    parts.publisher.close();
    join_bounded("sylphya", parts.handle).expect("記憶の書き手が panic せずに終わる");
}

fn put_descript(dir: &Path, body: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("descript.txt"), body).unwrap();
}

/// 起動のシェル: 記憶なし → 既定（`None`・警告なし）・記憶の名の
/// `shell/<名>/descript.txt` が在る（隠しでも）→ その名・先が無い／descript が無い →
/// `warn!` 1 件で既定。
#[test]
fn decide_boot_shell_follows_memory_only_when_descript_exists() {
    let tmp = TempPath::new("boot-resolve-boot-shell");
    let ghost_dir = tmp.child("ghost-a");
    put_descript(&ghost_dir.join("shell").join("master"), "charset,UTF-8\n");
    put_descript(
        &ghost_dir.join("shell").join("second"),
        "charset,UTF-8\nname,隠れたシェル\nmenu,hidden\n",
    );
    std::fs::create_dir_all(ghost_dir.join("shell").join("nodesc")).unwrap();

    let (none, events) = capture(|| decide_boot_shell(&ghost_dir));
    assert_eq!((read_last_shell(&ghost_dir), none), (None, None));
    assert_eq!(count_event(&events, "boot_shell_missing"), 0);

    remember_shell(&ghost_dir, "second");
    let (found, events) = capture(|| decide_boot_shell(&ghost_dir));
    assert_eq!(read_last_shell(&ghost_dir).as_deref(), Some("second"));
    assert_eq!(found.as_deref(), Some("second"));
    assert_eq!(count_event(&events, "boot_shell_missing"), 0);

    for gone in ["gone", "nodesc"] {
        remember_shell(&ghost_dir, gone);
        let (missing, events) = capture(|| decide_boot_shell(&ghost_dir));
        assert_eq!(missing, None, "{gone} で既定に戻らない");
        assert_one_event(&events, "boot_shell_missing", tracing::Level::WARN);
    }
}

/// 壊れた記憶が `shell/` の外を指しても使わない（`..`・区切り・絶対パス・`.`）。
/// 外の先に descript.txt が実在していても `warn!` 1 件で既定。
#[test]
fn decide_boot_shell_rejects_names_outside_shell_dir() {
    let tmp = TempPath::new("boot-resolve-boot-shell-escape");
    let ghost_dir = tmp.child("ghost-a");
    let outside = tmp.child("outside");
    put_descript(&outside, "charset,UTF-8\n");
    put_descript(&ghost_dir.join("escape"), "charset,UTF-8\n");
    put_descript(
        &ghost_dir.join("shell").join("a").join("b"),
        "charset,UTF-8\n",
    );
    put_descript(&ghost_dir.join("shell"), "charset,UTF-8\n");
    let absolute = outside.to_string_lossy().into_owned();
    for bad in [
        "../escape",
        r"..\escape",
        "a/b",
        r"a\b",
        absolute.as_str(),
        ".",
    ] {
        remember_shell(&ghost_dir, bad);
        let (decided, events) = capture(|| decide_boot_shell(&ghost_dir));
        assert_eq!(decided, None, "{bad:?} を受け入れた");
        assert_one_event(&events, "boot_shell_missing", tracing::Level::WARN);
    }
}
