//! `BalloonSyncScaleState` の檻（shell-scale-linkage 要件 6.5・7.4）: 基準を入れ直すと前の指定が消え、
//! 入れ直さない巡は読まない。

use std::collections::BTreeMap;
use std::path::PathBuf;

use wintf::ecs::Update;

use super::*;
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::source::{DescriptSource, GhostTitles};

fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn basis(ghost_kv: &[(&str, &str)], shell_kv: &[(&str, &str)]) -> DefaultPlacementBasis {
    DefaultPlacementBasis {
        source: DescriptSource {
            ghost_kv: kv(ghost_kv),
            shell_kv: kv(shell_kv),
            shell_dir: PathBuf::from("shell/master"),
            titles: GhostTitles::from_scope_titles([(0, "s".to_string())]),
        },
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: BTreeMap::new(),
        },
    }
}

fn world() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register(&mut world);
    world
}

fn synced(world: &World, scope: u32) -> bool {
    world.resource::<BalloonSyncScaleState>().0.is_synced(scope)
}

/// 基準を入れ直すと、前の基準から読んだ指定は残らず新しい基準の指定に置き換わる。
#[test]
fn reinserting_the_basis_replaces_the_whole_state() {
    let mut world = world();
    world.insert_resource(basis(&[("balloon.syncscale", "true")], &[]));
    world.run_schedule(Update);
    assert!(synced(&world, 0) && synced(&world, 1));

    world.insert_resource(basis(&[], &[("sakura.balloon.syncscale", "true")]));
    world.run_schedule(Update);
    assert!(synced(&world, 0));
    assert!(!synced(&world, 1), "前のゴーストの段の指定が残っている");
}

/// 基準を入れ直さない巡は読まない（資源を手で書き換えても上書きされない）。
#[test]
fn ticks_without_reinsertion_do_not_read() {
    let mut world = world();
    world.insert_resource(basis(&[("balloon.syncscale", "true")], &[]));
    world.run_schedule(Update);
    assert!(synced(&world, 0));

    world.insert_resource(BalloonSyncScaleState::default());
    world.run_schedule(Update);
    world.run_schedule(Update);
    assert!(!synced(&world, 0), "入れ直していない巡で読み直した");
}

/// 基準が無い間は資源を作らない。
#[test]
fn no_basis_no_state() {
    let mut world = world();
    world.run_schedule(Update);
    assert!(!world.contains_resource::<BalloonSyncScaleState>());
}
