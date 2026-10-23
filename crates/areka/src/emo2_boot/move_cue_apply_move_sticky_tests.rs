//! `\![move]` での組の連れ動きと `ignore-sticky-window` の檻（areka-P0-window-link-directives
//! task 4・要件 5.1〜5.5・5.7・9.3・9.4・設計「sticky_follow」の `\![move]`）。
//!
//! 窓は本番と同じ `spawn_ghost_windows` で組み（兄弟の `move_cue_apply_move_tests.rs` の
//! `move_world`）、帳簿は 2.1 の `seed_sticky_descript`／`apply_set_sticky` で据える。
//! 動かすのはスコープ 1（基準はスコープ 0 の窓）、組の相手はスコープ 0。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_emo_compose::ScaleRatio;
use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya};
use bevy_ecs::prelude::{Entity, World};
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{Point, WindowHandle};

use super::apply_move_tests::{move_world, pos_of};
use super::*;
use crate::placement::follow::{Anchored, BalloonFollow};
use crate::placement::persist::PersistWiring;
use crate::placement::resolver::Anchor;
use crate::placement::spawn::GhostWindows;
use crate::placement::sticky_group_ledger::{
    StickyGroupLedger, apply_set_sticky, seed_sticky_descript,
};
use crate::placement::test_support::{LogEvent, capture_logs};

/// 帳簿を据え、台本の組 `members` を作る（2.1 の関数）。
fn group(world: &mut World, members: &[&str]) {
    seed_sticky_descript(world, None);
    let tokens: Vec<String> = members.iter().map(|m| m.to_string()).collect();
    assert!(apply_set_sticky(world, &tokens));
}

/// スコープ 1 を動かす `\![move]`（x・y の両方が動く。基準はスコープ 0 の窓）。
fn directive(tokens: &[&str]) -> MoveDirective {
    let tokens: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
    parse_move_directive(1, &tokens).directive
}

const MOVE_XY: [&str; 6] = ["-353", "40", "", "0", "base", "base"];

fn set_anchor(world: &mut World, gw: &GhostWindows, scope: usize, anchor: Anchor) {
    let e = gw.char_window(scope).unwrap();
    world.entity_mut(e).insert(Anchored(anchor));
}

fn diff(after: Point, before: Point) -> Point {
    Point {
        x: after.x - before.x,
        y: after.y - before.y,
    }
}

fn events_of<'a>(events: &'a [LogEvent], event: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

fn finish(world: World) {
    drop(world);
    let _residue = drain_window_pos_commands();
}

struct Pair {
    target: Entity,
    partner: Entity,
    partner_balloon: Entity,
}

fn pair(gw: &GhostWindows) -> Pair {
    Pair {
        target: gw.char_window(1).unwrap(),
        partner: gw.char_window(0).unwrap(),
        partner_balloon: gw.balloon_window(0).unwrap(),
    }
}

/// 5.1・5.4・9.3: 相手は動かした窓が実際に動いた量だけ動き、バルーンも付いて動く。
/// `sticky_follow`（move）を 1 行記録する。
#[test]
fn partner_follows_by_the_amount_the_target_actually_moved() {
    let (mut world, gw) = move_world();
    set_anchor(&mut world, &gw, 0, Anchor::Free);
    set_anchor(&mut world, &gw, 1, Anchor::Free);
    group(&mut world, &["0", "1"]);
    let p = pair(&gw);
    let (target_before, partner_before) = (pos_of(&world, p.target), pos_of(&world, p.partner));
    let offset = world.get::<BalloonFollow>(p.partner).unwrap().offset();

    let (applied, events) =
        capture_logs(|| apply_move_directive(&mut world, &directive(&MOVE_XY), ScaleRatio::ONE));
    assert!(applied);

    let moved = diff(pos_of(&world, p.target), target_before);
    assert!(moved.x != 0 && moved.y != 0, "前提: 両軸が動く: {moved:?}");
    let partner_now = pos_of(&world, p.partner);
    assert_eq!(diff(partner_now, partner_before), moved);
    let balloon = pos_of(&world, p.partner_balloon);
    assert_eq!(
        (balloon.x - partner_now.x, balloon.y - partner_now.y),
        (offset.x, offset.y),
        "相手のバルーンが今のずらしで付いて動く"
    );

    let logged = events_of(&events, "sticky_follow");
    assert_eq!(logged.len(), 1, "{events:?}");
    let e = logged[0];
    assert_eq!(e.field_str("trigger"), Some("move"));
    assert_eq!(e.field("scope"), Some("1"));
    assert_eq!(e.field("partners"), Some("[0]"));
    assert_eq!(e.field("dx"), Some(moved.x.to_string().as_str()));
    assert_eq!(e.field("dy"), Some(moved.y.to_string().as_str()));
    finish(world);
}

/// 5.2: 下に吸着した相手は横にだけ動く（吸着の軸の量は乗らない）。
#[test]
fn bottom_partner_takes_only_the_horizontal_amount() {
    let (mut world, gw) = move_world();
    set_anchor(&mut world, &gw, 1, Anchor::Free);
    group(&mut world, &["0", "1"]);
    let p = pair(&gw);
    let (target_before, partner_before) = (pos_of(&world, p.target), pos_of(&world, p.partner));

    assert!(apply_move_directive(
        &mut world,
        &directive(&MOVE_XY),
        ScaleRatio::ONE
    ));

    let moved = diff(pos_of(&world, p.target), target_before);
    assert!(moved.y != 0, "前提: 縦にも動く: {moved:?}");
    assert_eq!(
        pos_of(&world, p.partner),
        Point {
            x: partner_before.x + moved.x,
            y: partner_before.y
        }
    );
    finish(world);
}

/// 5.3・9.4: `ignore-sticky-window` では相手が動かず組は残り、記録が出る。その後の
/// `\![move]` では新しい位置の関係のまま組で動く。
#[test]
fn ignore_option_moves_only_the_target_and_the_group_moves_on_with_the_new_relation() {
    let (mut world, gw) = move_world();
    set_anchor(&mut world, &gw, 0, Anchor::Free);
    set_anchor(&mut world, &gw, 1, Anchor::Free);
    group(&mut world, &["0", "1"]);
    let p = pair(&gw);
    let partner_before = pos_of(&world, p.partner);

    let ignore = directive(&[
        "--x=-353",
        "--y=40",
        "--base=0",
        "--base-offset=base",
        "--move-offset=base",
        "--option=ignore-sticky-window",
    ]);
    assert!(ignore.ignore_sticky_window, "前提: 名前付きの形で読める");
    let (applied, events) =
        capture_logs(|| apply_move_directive(&mut world, &ignore, ScaleRatio::ONE));
    assert!(applied);
    assert_eq!(pos_of(&world, p.partner), partner_before, "相手は動かない");
    let ignored = events_of(&events, "sticky_follow_ignored");
    assert_eq!(ignored.len(), 1, "{events:?}");
    assert_eq!(ignored[0].field("scope"), Some("1"));
    assert_eq!(ignored[0].field("partners"), Some("[0]"));
    assert!(events_of(&events, "sticky_follow").is_empty(), "{events:?}");
    assert_eq!(
        world.resource::<StickyGroupLedger>().partners_of(1),
        vec![0],
        "組は解かない"
    );

    // 新しい関係のまま組で動く。
    let relation = diff(pos_of(&world, p.target), pos_of(&world, p.partner));
    assert!(apply_move_directive(
        &mut world,
        &directive(&["-100", "-60", "", "0", "base", "base"]),
        ScaleRatio::ONE
    ));
    assert_ne!(
        pos_of(&world, p.partner),
        partner_before,
        "2 回目は相手も動く"
    );
    assert_eq!(
        diff(pos_of(&world, p.target), pos_of(&world, p.partner)),
        relation
    );
    finish(world);
}

struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// 5.4: 連れ動いた相手の位置を記憶に書かない。
#[test]
fn partner_position_is_not_persisted() {
    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo(shared.clone())),
        runtime_sink: None,
    });
    let (mut world, gw) = move_world();
    world.insert_non_send(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    set_anchor(&mut world, &gw, 0, Anchor::Free);
    group(&mut world, &["0", "1"]);
    let p = pair(&gw);
    let partner_before = pos_of(&world, p.partner);

    assert!(apply_move_directive(
        &mut world,
        &directive(&MOVE_XY),
        ScaleRatio::ONE
    ));
    assert_ne!(
        pos_of(&world, p.partner),
        partner_before,
        "前提: 相手が動いた"
    );

    parts.publisher.barrier().expect("barrier");
    let loaded: HashMap<PersistKey, String> =
        load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()))
            .into_iter()
            .collect();
    assert!(
        !loaded
            .keys()
            .any(|k| matches!(k, PersistKey::WindowPos { .. })),
        "窓の位置を記憶に書かない: {loaded:?}"
    );
    parts.publisher.close();
    let _ = parts.handle.join();
    finish(world);
}

/// 5.5: 時間つきの `\![move]` でも相手は最終の位置へすぐ置かれる。
#[test]
fn timed_move_places_the_partner_at_once() {
    let (mut world, gw) = move_world();
    set_anchor(&mut world, &gw, 0, Anchor::Free);
    set_anchor(&mut world, &gw, 1, Anchor::Free);
    group(&mut world, &["0", "1"]);
    let p = pair(&gw);
    let (target_before, partner_before) = (pos_of(&world, p.target), pos_of(&world, p.partner));

    let timed = directive(&["-353", "40", "500", "0", "base", "base"]);
    assert!(apply_move_directive(&mut world, &timed, ScaleRatio::ONE));

    let moved = diff(pos_of(&world, p.target), target_before);
    assert_ne!(moved, Point { x: 0, y: 0 });
    assert_eq!(diff(pos_of(&world, p.partner), partner_before), moved);
    finish(world);
}

/// 5.7: 組に入っていない窓への `\![move]` は今のまま（ほかの窓は動かず、連れ動きの記録も無い）。
/// 帳簿が無いときも同じ。
#[test]
fn ungrouped_window_moves_alone_as_before() {
    for seed in [false, true] {
        let (mut world, gw) = move_world();
        if seed {
            seed_sticky_descript(&mut world, None);
        }
        let p = pair(&gw);
        let partner_before = pos_of(&world, p.partner);

        let (applied, events) = capture_logs(|| {
            apply_move_directive(&mut world, &directive(&MOVE_XY), ScaleRatio::ONE)
        });
        assert!(applied);
        assert_eq!(pos_of(&world, p.partner), partner_before, "seed={seed}");
        for name in [
            "sticky_follow",
            "sticky_follow_ignored",
            "sticky_member_absent",
        ] {
            assert!(
                events_of(&events, name).is_empty(),
                "seed={seed}: {events:?}"
            );
        }
        finish(world);
    }
}

/// 3.6: 窓の無い相手は `sticky_member_absent`（when=move）で外し、窓のある相手は動かす。
#[test]
fn absent_member_is_logged_and_left_out() {
    let (mut world, gw) = move_world();
    set_anchor(&mut world, &gw, 0, Anchor::Free);
    group(&mut world, &["0", "1", "5"]);
    let p = pair(&gw);
    let partner_before = pos_of(&world, p.partner);

    let (applied, events) =
        capture_logs(|| apply_move_directive(&mut world, &directive(&MOVE_XY), ScaleRatio::ONE));
    assert!(applied);
    assert_ne!(pos_of(&world, p.partner), partner_before);
    let absent = events_of(&events, "sticky_member_absent");
    assert_eq!(absent.len(), 1, "{events:?}");
    assert_eq!(absent[0].field("scope"), Some("5"));
    assert_eq!(absent[0].field_str("when"), Some("move"));
    let logged = events_of(&events, "sticky_follow");
    assert_eq!(logged.len(), 1, "{events:?}");
    assert_eq!(logged[0].field("partners"), Some("[0]"));
    finish(world);
}

/// 動かす窓を書けなかったときは相手を動かさない。量が 0 のときも何もしない。
#[test]
fn refused_write_or_zero_amount_moves_no_partner() {
    let (mut world, gw) = move_world();
    set_anchor(&mut world, &gw, 0, Anchor::Free);
    set_anchor(&mut world, &gw, 1, Anchor::Free);
    group(&mut world, &["0", "1"]);
    let p = pair(&gw);

    // 量が 0: x・y を省いた（今の位置のまま）指令。
    let partner_before = pos_of(&world, p.partner);
    let (applied, events) = capture_logs(|| {
        apply_move_directive(&mut world, &directive(&["", "", "", "0"]), ScaleRatio::ONE)
    });
    assert!(applied);
    assert_eq!(pos_of(&world, p.partner), partner_before);
    assert!(events_of(&events, "sticky_follow").is_empty(), "{events:?}");

    // 書けない: 動かす窓の窓ハンドルを外す。
    world.entity_mut(p.target).remove::<WindowHandle>();
    let (applied, events) = capture_logs(|| {
        apply_move_directive(
            &mut world,
            &directive(&["-100", "-60", "", "0", "base", "base"]),
            ScaleRatio::ONE,
        )
    });
    assert!(!applied);
    assert_eq!(pos_of(&world, p.partner), partner_before);
    assert!(events_of(&events, "sticky_follow").is_empty(), "{events:?}");
    finish(world);
}
