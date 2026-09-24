//! 起動解決の純粋な判断の決定論テスト（要件 4.9・5.10・8.3）。
//!
//! 確かめること: ゴースト 6 分岐（argv／記憶／唯一／既定／無作為／0 体）・バルーン 7 分岐
//! （argv／記憶／同梱／唯一／既定／無作為／0）と、記憶の指す先が無い 2 通り・同梱の指す先が
//! 無い 1 通り。無作為は固定の添字（`|n| n - 1`）を注入し、選ばれたものが列挙に含まれることと
//! 経路が `Random` であることを見る。既定の分岐は `listed` に `"emo2"`／`"StayseeBalloon"` を
//! 並べるだけ（検体の登記に依存しない）。プロセス・fs・乱数には触れない。

use std::path::{Path, PathBuf};

use log_capture_kit::{CapturedEvent, capture};

use super::*;

// ---------------------------------------------------------------- 道具立て

/// 実在を問わない根（判断は fs を見ないので実在は要らない）。
fn root() -> BasewareRoot {
    BasewareRoot::new(PathBuf::from(r"C:\areka-boot-resolve-tests"))
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

/// 呼ばれてはならない添字（無作為の段に届かない分岐で使う）。
fn no_pick(n: usize) -> usize {
    panic!("無作為の段に届いてはならない（候補数 {n}）")
}

/// 最後の候補を選ぶ固定の添字（先頭を選ぶ実装と区別するため末尾）。
fn last(n: usize) -> usize {
    n - 1
}

/// 記録の中から `event` が一致するものを数える。
fn count_event(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// `event` の記録がちょうど 1 件あり、そのレベルが `level` であることを確かめる。
fn assert_one_event(events: &[CapturedEvent], event: &str, level: tracing::Level) {
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect();
    assert_eq!(hits.len(), 1, "{event} はちょうど 1 件: {events:?}");
    assert_eq!(hits[0].level, level, "{event} のレベル");
}

fn ghost(
    root: &BasewareRoot,
    argv: Option<&Path>,
    memory: Option<&str>,
    listed: &[String],
    pick: impl FnOnce(usize) -> usize,
) -> (Result<GhostDecision, NoGhost>, Vec<CapturedEvent>) {
    let inputs = GhostInputs {
        root,
        argv,
        memory,
        listed,
    };
    capture(|| resolve_ghost(&inputs, pick))
}

fn balloon(
    root: &BasewareRoot,
    argv: Option<&Path>,
    memory: Option<&str>,
    companion: Option<&str>,
    listed: &[String],
    pick: impl FnOnce(usize) -> usize,
) -> (Result<BalloonDecision, NoBalloon>, Vec<CapturedEvent>) {
    let inputs = BalloonInputs {
        root,
        argv,
        memory,
        companion,
        listed,
    };
    capture(|| resolve_balloon(&inputs, pick))
}

fn ghost_at(root: &BasewareRoot, route: GhostRoute, folder: &str) -> GhostDecision {
    GhostDecision {
        route,
        dir: root.ghost_dir(folder),
        folder: Some(folder.to_owned()),
    }
}

fn balloon_at(root: &BasewareRoot, route: BalloonRoute, folder: &str) -> BalloonDecision {
    BalloonDecision {
        route,
        dir: root.balloon_dir(folder),
        folder: Some(folder.to_owned()),
    }
}

// ---------------------------------------------------------------- ゴースト 6 分岐（要件 4.1〜4.7）

/// argv: 渡されたパスそのもの・名前は持たない。記憶の指す先が無くても列挙が 0 でも見ない
/// （`warn!` も出ない）（要件 4.1）。
#[test]
fn ghost_argv_wins_without_looking_at_memory_or_listing() {
    let root = root();
    let argv = PathBuf::from(r"C:\somewhere\outside\my_ghost");
    for listed in [names(&[]), names(&["a", "b"])] {
        let (got, events) = ghost(&root, Some(&argv), Some("gone"), &listed, no_pick);
        assert_eq!(
            got,
            Ok(GhostDecision {
                route: GhostRoute::Argv,
                dir: argv.clone(),
                folder: None,
            })
        );
        assert!(events.is_empty(), "argv では記憶も列挙も見ない: {events:?}");
    }
}

/// 記憶: 列挙に在れば記憶のゴースト（唯一・既定より先）（要件 4.2）。
#[test]
fn ghost_memory_listed_wins_over_default() {
    let root = root();
    let listed = names(&["a", "emo2", "z"]);
    let (got, events) = ghost(&root, None, Some("z"), &listed, no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Memory, "z")));
    assert!(events.is_empty(), "{events:?}");
}

/// 唯一: 記憶が無く列挙が 1 体（要件 4.3）。
#[test]
fn ghost_only_one_listed() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&["solo"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Only, "solo")));
}

/// 唯一は既定より先: 列挙が `emo2` 1 体だけなら経路は `Only`（要件 4.3・4.4 は 2 体以上）。
#[test]
fn ghost_only_emo2_is_only_not_default() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&["emo2"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Only, "emo2")));
}

/// 既定: 2 体以上で `emo2` が列挙される（要件 4.4）。
#[test]
fn ghost_default_emo2_when_listed_among_many() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&["a", "emo2", "z"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Default, "emo2")));
}

/// 無作為: 2 体以上で `emo2` が無い。注入した添字の候補が選ばれ、info を 1 件残す（要件 4.5）。
#[test]
fn ghost_random_picks_from_listing_and_logs_info() {
    let root = root();
    let listed = names(&["a", "b", "c"]);
    let (got, events) = ghost(&root, None, None, &listed, last);
    let got = got.expect("無作為で決まる");
    assert_eq!(got, ghost_at(&root, GhostRoute::Random, "c"));
    assert!(
        listed.contains(got.folder.as_ref().unwrap()),
        "列挙に含まれる"
    );
    assert_one_event(&events, "ghost_picked_randomly", tracing::Level::INFO);
}

/// 0 体: ゴーストの格納フォルダを載せて返す（要件 4.7）。
#[test]
fn ghost_none_listed_returns_store() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&[]), no_pick);
    assert_eq!(
        got,
        Err(NoGhost {
            ghost_store: root.ghost_store(),
        })
    );
}

/// 記憶の指す先が無い: `warn!` を 1 件残して次の段（ここでは唯一）へ（要件 4.6）。
#[test]
fn ghost_memory_not_listed_warns_then_falls_through() {
    let root = root();
    let (got, events) = ghost(&root, None, Some("gone"), &names(&["solo"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Only, "solo")));
    assert_one_event(&events, "last_ghost_not_found", tracing::Level::WARN);
}

// ---------------------------------------------------------------- バルーン 7 分岐（要件 5.1〜5.8）

/// argv: 渡されたパスそのもの。記憶・同梱の指す先が無くても列挙が 0 でも見ない（要件 5.1）。
#[test]
fn balloon_argv_wins_without_looking_at_anything_else() {
    let root = root();
    let argv = PathBuf::from(r"C:\somewhere\outside\my_balloon");
    for listed in [names(&[]), names(&["a", "b"])] {
        let (got, events) = balloon(
            &root,
            Some(&argv),
            Some("gone"),
            Some("also_gone"),
            &listed,
            no_pick,
        );
        assert_eq!(
            got,
            Ok(BalloonDecision {
                route: BalloonRoute::Argv,
                dir: argv.clone(),
                folder: None,
            })
        );
        assert!(events.is_empty(), "argv では他を見ない: {events:?}");
    }
}

/// 記憶: 列挙に在れば同梱より先（裁定 4・要件 5.2）。
#[test]
fn balloon_memory_wins_over_companion() {
    let root = root();
    let listed = names(&["mine", "bundled", "StayseeBalloon"]);
    let (got, events) = balloon(&root, None, Some("mine"), Some("bundled"), &listed, no_pick);
    assert_eq!(got, Ok(balloon_at(&root, BalloonRoute::Memory, "mine")));
    assert!(events.is_empty(), "{events:?}");
}

/// 同梱: 記憶が無く、`install.txt` の名が列挙に在る（唯一・既定より先）（要件 5.3）。
#[test]
fn balloon_companion_wins_over_default() {
    let root = root();
    let listed = names(&["bundled", "StayseeBalloon"]);
    let (got, _) = balloon(&root, None, None, Some("bundled"), &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Companion, "bundled"))
    );
}

/// 唯一: 記憶・同梱が無く列挙が 1 つ（要件 5.4）。
#[test]
fn balloon_only_one_listed() {
    let root = root();
    let (got, _) = balloon(&root, None, None, None, &names(&["solo"]), no_pick);
    assert_eq!(got, Ok(balloon_at(&root, BalloonRoute::Only, "solo")));
}

/// 唯一は既定より先: 列挙が `StayseeBalloon` 1 つだけなら経路は `Only`（要件 5.4・5.5）。
#[test]
fn balloon_only_staysee_is_only_not_default() {
    let root = root();
    let (got, _) = balloon(
        &root,
        None,
        None,
        None,
        &names(&["StayseeBalloon"]),
        no_pick,
    );
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Only, "StayseeBalloon"))
    );
}

/// 既定: 2 つ以上で `StayseeBalloon` が列挙される（要件 5.5）。
#[test]
fn balloon_default_staysee_when_listed_among_many() {
    let root = root();
    let listed = names(&["a", "StayseeBalloon", "z"]);
    let (got, _) = balloon(&root, None, None, None, &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Default, "StayseeBalloon"))
    );
}

/// 無作為: 2 つ以上で既定が無い。注入した添字の候補が選ばれ、info を 1 件残す（要件 5.6）。
#[test]
fn balloon_random_picks_from_listing_and_logs_info() {
    let root = root();
    let listed = names(&["a", "b", "c"]);
    let (got, events) = balloon(&root, None, None, None, &listed, last);
    let got = got.expect("無作為で決まる");
    assert_eq!(got, balloon_at(&root, BalloonRoute::Random, "c"));
    assert!(
        listed.contains(got.folder.as_ref().unwrap()),
        "列挙に含まれる"
    );
    assert_one_event(&events, "balloon_picked_randomly", tracing::Level::INFO);
}

/// 0: バルーンの格納フォルダを載せて返す（要件 5.8）。
#[test]
fn balloon_none_listed_returns_store() {
    let root = root();
    let (got, _) = balloon(&root, None, None, None, &names(&[]), no_pick);
    assert_eq!(
        got,
        Err(NoBalloon {
            balloon_store: root.balloon_store(),
        })
    );
}

/// 記憶の指す先が無い: `warn!` を 1 件残して次の段（同梱）へ（要件 5.7）。
#[test]
fn balloon_memory_not_listed_warns_then_companion() {
    let root = root();
    let listed = names(&["bundled", "other"]);
    let (got, events) = balloon(&root, None, Some("gone"), Some("bundled"), &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Companion, "bundled"))
    );
    assert_one_event(&events, "last_balloon_not_found", tracing::Level::WARN);
    assert_eq!(count_event(&events, "companion_balloon_not_found"), 0);
}

/// 同梱の指す先が無い: `warn!` を 1 件残して次の段（既定）へ（要件 5.7）。
#[test]
fn balloon_companion_not_listed_warns_then_default() {
    let root = root();
    let listed = names(&["a", "StayseeBalloon"]);
    let (got, events) = balloon(&root, None, None, Some("gone"), &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Default, "StayseeBalloon"))
    );
    assert_one_event(&events, "companion_balloon_not_found", tracing::Level::WARN);
    assert_eq!(count_event(&events, "last_balloon_not_found"), 0);
}

// ---------------------------------------------------------------- 本番の添字

/// `pick_index` は候補の範囲 `0..n` に収まる（質は問わない＝design の Risks）。
#[test]
fn pick_index_stays_in_range() {
    for n in 2..=7 {
        for _ in 0..32 {
            assert!(pick_index(n) < n);
        }
    }
}

// ---------------------------------------------------------------- 記憶の書き込みと直読み

mod last_used {
    use std::sync::Arc;

    use areka_ghost::sylphya_wiring::profile_areka_root;
    use areka_sylphya::persist::{FakePersistIo, FsPersistIo, PersistIo};
    use areka_sylphya::{
        PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };
    use temp_path_kit::TempPath;

    use super::*;

    /// 共有 fake IO（アクターへ移送しつつ、同じストアを別ハンドルの `load_scope` で観測する。
    /// `main_persist_wiring_seam_tests.rs` と同じ形）。
    struct SharedFakeIo(Arc<FakePersistIo>);
    impl PersistIo for SharedFakeIo {
        fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

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

    /// `record` を実 publisher へ投函し、barrier 後に App・Ghost の 2 スコープを読み戻す。
    fn record_and_load(ghost: &GhostDecision, balloon: &BalloonDecision, shell: &str) -> Loaded {
        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            app: Some(PathBuf::from("/app")),
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });
        let ((), events) = capture(|| {
            LastUsed {
                ghost,
                balloon,
                shell_folder: shell,
            }
            .record(&parts.publisher)
        });
        parts
            .publisher
            .barrier()
            .expect("アクターが生きている間は barrier が返る");
        let app = load_scope(PersistScope::App, &roots, &SharedFakeIo(shared.clone()));
        let ghost_scope = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared));
        parts.publisher.close();
        let _ = parts.handle.join();
        (app, ghost_scope, events)
    }

    fn entry(key: PersistKey, value: &str) -> (PersistKey, String) {
        (key, value.to_owned())
    }

    /// argv 以外: App に ghost、Ghost に balloon と shell（要件 3.2・3.3・3.4）。
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

    /// argv のゴースト: App に書かれない・Ghost の balloon と shell は書かれる（要件 3.5・裁定 5）。
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

    /// 両方 argv: shell だけ書かれる（要件 3.4 は argv の有無によらない）。
    #[test]
    fn record_both_argv_writes_shell_only() {
        let (app, ghost_scope, events) = record_and_load(&ghost_argv(), &balloon_argv(), "master");
        assert_eq!(app, vec![]);
        assert_eq!(ghost_scope, vec![entry(PersistKey::LastShell, "master")]);
        assert_eq!(count_event(&events, "last_used_skipped_argv"), 2);
    }

    /// 直読みは boot と同じ場所を見る: 実 fs へ record したものを `read_last_*` で拾える。
    /// 書かれていなければ無し（要件 4.2・5.2）。
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
        let _ = parts.handle.join();

        assert_eq!(read_last_ghost(&app_dir).as_deref(), Some("a"));
        assert_eq!(
            read_last_balloon(&ghost_dir).as_deref(),
            Some("StayseeBalloon")
        );
    }
}
