use super::*;
use areka_placement::persist::{PersistWiring, char_pos_entries, persist_entries};
use areka_placement::resolver::PointPx;
use areka_placement::test_support::SharedFakeIo;
use areka_runtime::spine::join_bounded;
use areka_sylphya::persist::FakePersistIo;
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
};
use std::path::Path;
use std::sync::Arc;

/// 挿入シーム檻: `insert_persist_wiring` で headless World へ実 publisher を
/// 挿入すると、(a) NonSend `PersistWiring` が存在し、(b) その World 越しの `persist_entries` 投函が
/// Ghost スコープへ write-through され、barrier 後に別ハンドルの `load_scope` で読み戻せる。
#[test]
fn insert_persist_wiring_establishes_world_conduit_reaching_the_store() {
    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(std::path::PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo::new(shared.clone())),
        runtime_sink: None,
    });

    let mut world = World::new();
    insert_persist_wiring(&mut world, parts.publisher.clone());

    assert!(
        world.get_non_send::<PersistWiring>().is_some(),
        "insert_persist_wiring 後、World に PersistWiring が挿入されているべき"
    );

    let entries = char_pos_entries(0, PointPx { x: 1486, y: 353 });
    persist_entries(&world, entries);

    // barrier 復帰＝上記 put の write-through 保存（save_scope）まで完了（同一送信端 FIFO）。
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // アクターと同一ストアを別ハンドルの load_scope で観測（実 IO 通過＝投函の証明）。
    let loaded = load_scope(
        PersistScope::Ghost,
        &roots,
        &SharedFakeIo::new(shared.clone()),
    );
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X
            },
            "1486".to_string()
        )),
        "World 導管越しの persist_entries が Ghost へ write-through していない（WindowPos.x）: {loaded:?}"
    );
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::Y
            },
            "353".to_string()
        )),
        "World 導管越しの persist_entries が Ghost へ write-through していない（WindowPos.y）: {loaded:?}"
    );

    parts.publisher.close();
    join_bounded("sylphya", parts.handle).expect("記憶の書き手が panic せずに終わる");
}

/// 起動の呼び出しが返った時点の記憶: 経路が切替なら書かず
/// `debug!(last_used_deferred)` 1 件（定常到達で書く）、それ以外の経路（初回の起動の記憶・既定への
/// 戻し）は最後のゴースト（App）とシェル（Ghost）を書く。
#[test]
fn last_used_is_deferred_only_for_the_switch_route() {
    use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
    let run = |route: GhostRoute| {
        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            app: Some(std::path::PathBuf::from("/a")),
            ghost: Some(std::path::PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo::new(shared.clone())),
            runtime_sink: None,
        });
        let ghost = GhostDecision {
            route,
            dir: std::path::PathBuf::from("ghost/B"),
            folder: Some("B".to_owned()),
        };
        let balloon = BalloonDecision {
            route: BalloonRoute::Default,
            dir: std::path::PathBuf::from("balloon/S"),
            folder: Some("S".to_owned()),
        };
        let ((), events) = log_capture_kit::capture(|| {
            record_last_used_at_boot(
                &parts.publisher,
                Path::new("ghost/B/shell/master"),
                &ghost,
                &balloon,
            )
        });
        parts.publisher.barrier().expect("barrier");
        let app = load_scope(
            PersistScope::App,
            &roots,
            &SharedFakeIo::new(shared.clone()),
        );
        let ghost_scope = load_scope(
            PersistScope::Ghost,
            &roots,
            &SharedFakeIo::new(shared.clone()),
        );
        parts.publisher.close();
        join_bounded("sylphya", parts.handle).expect("記憶の書き手が panic せずに終わる");
        let deferred = crate::session_end::test_support::levels_of(&events, "last_used_deferred");
        (
            app.contains(&(PersistKey::LastGhost, "B".to_owned())),
            ghost_scope.contains(&(PersistKey::LastShell, "master".to_owned())),
            deferred,
        )
    };
    assert_eq!(
        [
            run(GhostRoute::Switched),
            run(GhostRoute::Memory),
            run(GhostRoute::Default)
        ],
        [
            (false, false, vec![tracing::Level::DEBUG]),
            (true, true, vec![]),
            (true, true, vec![]),
        ],
        "(最後のゴースト・シェル・先送りの記録) を 切替・記憶・既定 の順に"
    );
}
