//! ドラッグの控えと、組の相手を同じ量だけ動かす手続きの決定論の檻（task 3.1・要件 2.4／3.6／
//! 4.1〜4.5／9.3・設計「sticky_follow」・設計の決め 4）。
//!
//! 窓は偽の部品（偽 HWND・`WindowPos`・`Anchored`・`CharWindowMarker`・`DragConfig`・
//! `BalloonFollow`）だけで組み、帳簿は 2.1 の `apply_set_sticky` で据える。関数は受け手を通さず
//! 直接呼ぶ（受け手へのつなぎは task 3.2）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
};
use bevy_ecs::prelude::*;
use wintf::ecs::Point;
use wintf::ecs::drag::DragConfig;
use wintf::ecs::window::drain_window_pos_commands;

use super::super::persist::PersistWiring;
use super::super::sticky_group_ledger::{apply_set_sticky, seed_sticky_descript};
use super::super::test_support::{LogEvent, capture_logs};
use super::sticky_follow::{
    CharDragSession, begin_char_drag, drag_anchor, drag_wintf_moves, finish_char_drag,
    follow_partners_on_drag,
};
use super::test_support::{fake_handle, position_of, single_monitor_snapshot, window_pos_sized};
use super::{Anchored, BalloonFollow, CharWindowMarker, OffsetBase};
use crate::placement::resolver::{Anchor, PointPx};

/// キャラクターの窓の大きさ（物理 px・96 の倍数を避ける）。
const W: i32 = 434;
const H: i32 = 687;
/// 作業領域の下端（`single_monitor_snapshot` の wa.bottom）。
const WA_BOTTOM: i32 = 1043;
/// 相手のバルーンのずらし。
const OFFSET: PointPx = PointPx { x: -412, y: -25 };

struct Win {
    char: Entity,
    balloon: Entity,
}

fn spawn_char(world: &mut World, scope: usize, anchor: Anchor, pos: (i32, i32)) -> Win {
    let raw = 0x1000 + scope * 0x10;
    let balloon = world
        .spawn((fake_handle(raw + 1), window_pos_sized(0, 0, 200, 100)))
        .id();
    let char = world
        .spawn((
            fake_handle(raw),
            window_pos_sized(pos.0, pos.1, W, H),
            CharWindowMarker { scope },
            Anchored(anchor),
            DragConfig {
                move_window: anchor.is_free(),
                ..Default::default()
            },
            BalloonFollow::new(balloon, OffsetBase::unpinned(OFFSET)),
        ))
        .id();
    Win { char, balloon }
}

/// 帳簿を据え、台本の組 `members` を作る（2.1 の関数）。
fn group(world: &mut World, members: &[&str]) {
    seed_sticky_descript(world, None);
    let tokens: Vec<String> = members.iter().map(|m| m.to_string()).collect();
    assert!(apply_set_sticky(world, &tokens));
}

fn world_with_snapshot() -> World {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    world
}

fn point(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
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

/// 途中の量は「始めからの量」で、回を重ねても積もらない。相手のバルーンも付いて動く（4.1・4.3・4.4）。
#[test]
fn partner_follows_delta_from_start_without_accumulating() {
    let mut world = world_with_snapshot();
    let dragged = spawn_char(&mut world, 0, Anchor::Free, (500, 200));
    let partner = spawn_char(&mut world, 1, Anchor::Free, (900, 150));
    group(&mut world, &["0", "1"]);

    begin_char_drag(&mut world, dragged.char);
    follow_partners_on_drag(&mut world, dragged.char, point(510, 205));
    follow_partners_on_drag(&mut world, dragged.char, point(530, 215));

    // 1 回ごとの差（10,5）＋（20,10）を積むのではなく、始めからの量（30,15）。
    assert_eq!(position_of(&world, partner.char), Point { x: 930, y: 165 });
    assert_eq!(
        position_of(&world, partner.balloon),
        Point {
            x: 930 + OFFSET.x,
            y: 165 + OFFSET.y
        }
    );
    // ドラッグした窓自身は書かない（受け手の領分）。
    assert_eq!(position_of(&world, dragged.char), Point { x: 500, y: 200 });
    finish(world);
}

/// 相手が下に吸着していれば、量は横にだけ乗り、下端に付いたまま（4.2）。途中は記録しない（9.3）。
#[test]
fn bottom_partner_takes_only_horizontal_delta() {
    let mut world = world_with_snapshot();
    let dragged = spawn_char(&mut world, 0, Anchor::Free, (500, 200));
    let partner = spawn_char(&mut world, 1, Anchor::Bottom, (900, WA_BOTTOM - H));
    group(&mut world, &["0", "1"]);

    begin_char_drag(&mut world, dragged.char);
    let ((), events) = capture_logs(|| {
        follow_partners_on_drag(&mut world, dragged.char, point(470, 120));
    });

    assert_eq!(
        position_of(&world, partner.char),
        Point {
            x: 870,
            y: WA_BOTTOM - H
        }
    );
    assert!(
        events_of(&events, "sticky_follow").is_empty(),
        "途中の 1 回ごとは記録しない: {events:?}"
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

/// 終わりは最終の量で相手を書き、相手の位置を記憶の形（下吸着なら下端中央）で書き、
/// `sticky_follow`（drag）を 1 行記録して控えを外す（4.5・9.3）。
#[test]
fn finish_writes_partner_persists_logs_and_clears_session() {
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
    let mut world = world_with_snapshot();
    world.insert_non_send(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    let dragged = spawn_char(&mut world, 0, Anchor::Free, (500, 200));
    let partner = spawn_char(&mut world, 1, Anchor::Bottom, (900, WA_BOTTOM - H));
    group(&mut world, &["0", "1"]);

    begin_char_drag(&mut world, dragged.char);
    follow_partners_on_drag(&mut world, dragged.char, point(520, 210));
    let ((), events) = capture_logs(|| {
        finish_char_drag(&mut world, dragged.char, point(537, 180));
    });

    let final_x = 900 + 37;
    assert_eq!(
        position_of(&world, partner.char),
        Point {
            x: final_x,
            y: WA_BOTTOM - H
        }
    );
    assert!(world.get_resource::<CharDragSession>().is_none());

    let logged = events_of(&events, "sticky_follow");
    assert_eq!(logged.len(), 1, "{events:?}");
    let e = logged[0];
    assert_eq!(e.field_str("trigger"), Some("drag"));
    assert_eq!(e.field("scope"), Some("0"));
    assert_eq!(e.field("partners"), Some("[1]"));
    assert_eq!(e.field("dx"), Some("37"));
    assert_eq!(e.field("dy"), Some("-20"));

    parts.publisher.barrier().expect("barrier");
    let loaded: HashMap<PersistKey, String> =
        load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()))
            .into_iter()
            .collect();
    let key = |axis| PersistKey::WindowPos { scope: 1, axis };
    assert_eq!(
        loaded.get(&key(Axis::X)),
        Some(&(final_x + W / 2).to_string())
    );
    assert_eq!(
        loaded.get(&key(Axis::Y)),
        Some(&(WA_BOTTOM - H).to_string())
    );
    // ドラッグした窓自身の記憶は受け手が書く（ここでは書かない）。
    assert!(!loaded.contains_key(&PersistKey::WindowPos {
        scope: 0,
        axis: Axis::X
    }));

    parts.publisher.close();
    let _ = parts.handle.join();
    finish(world);
}

/// 控えが無い・別の窓の控えなら相手を動かさず、組の相手がいるときだけ記録する（4.1）。
#[test]
fn without_session_partner_stays_and_skip_is_logged() {
    let mut world = world_with_snapshot();
    let a = spawn_char(&mut world, 0, Anchor::Free, (500, 200));
    let b = spawn_char(&mut world, 1, Anchor::Free, (900, 150));
    let lone = spawn_char(&mut world, 2, Anchor::Free, (100, 100));
    group(&mut world, &["0", "1"]);

    let ((), events) = capture_logs(|| {
        follow_partners_on_drag(&mut world, a.char, point(600, 300));
        finish_char_drag(&mut world, a.char, point(600, 300));
    });
    assert_eq!(position_of(&world, b.char), Point { x: 900, y: 150 });
    let skipped = events_of(&events, "sticky_follow_skipped");
    assert_eq!(skipped.len(), 2, "{events:?}");
    for e in skipped {
        assert_eq!(e.field_str("reason"), Some("no_session"));
        assert_eq!(e.field("scope"), Some("0"));
    }

    // 別の窓（組の相手 b）の控えで a を動かしても、b の始めの位置は変わらない。
    begin_char_drag(&mut world, b.char);
    let ((), events) = capture_logs(|| {
        follow_partners_on_drag(&mut world, a.char, point(600, 300));
    });
    assert_eq!(position_of(&world, b.char), Point { x: 900, y: 150 });
    let skipped = events_of(&events, "sticky_follow_skipped");
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].field_str("reason"), Some("other_window"));
    assert!(world.get_resource::<CharDragSession>().is_some());

    // 組に無い窓は記録しない。
    let ((), events) = capture_logs(|| {
        follow_partners_on_drag(&mut world, lone.char, point(0, 0));
    });
    assert!(events_of(&events, "sticky_follow_skipped").is_empty());
    finish(world);
}

/// 組の中で窓の無い相手は、始めに `sticky_member_absent`（drag）で外す（3.6）。
#[test]
fn absent_member_is_logged_at_begin_and_left_out() {
    let mut world = world_with_snapshot();
    let dragged = spawn_char(&mut world, 0, Anchor::Free, (500, 200));
    let partner = spawn_char(&mut world, 1, Anchor::Free, (900, 150));
    group(&mut world, &["0", "1", "2"]);

    let ((), events) = capture_logs(|| begin_char_drag(&mut world, dragged.char));
    let absent = events_of(&events, "sticky_member_absent");
    assert_eq!(absent.len(), 1, "{events:?}");
    assert_eq!(absent[0].field("scope"), Some("2"));
    assert_eq!(absent[0].field_str("when"), Some("drag"));

    let ((), events) = capture_logs(|| finish_char_drag(&mut world, dragged.char, point(510, 200)));
    assert_eq!(position_of(&world, partner.char), Point { x: 910, y: 150 });
    assert_eq!(
        events_of(&events, "sticky_follow")[0].field("partners"),
        Some("[1]")
    );
    finish(world);
}

/// ドラッグ中に向きと書き手が変わっても、そのドラッグの終わりまで始めの値を返し（2.4）、
/// 終わりに今の向きで当て直す。
#[test]
fn anchor_change_during_drag_keeps_start_until_finish_then_reanchors() {
    let mut world = world_with_snapshot();
    let dragged = spawn_char(&mut world, 0, Anchor::Free, (500, 200));
    begin_char_drag(&mut world, dragged.char);

    // 台本がドラッグ中に下へ吸着させた（2.2 の適用と 1.3 の書き手の付け直しを模す）。
    world.get_mut::<Anchored>(dragged.char).unwrap().0 = Anchor::Bottom;
    world
        .get_mut::<DragConfig>(dragged.char)
        .unwrap()
        .move_window = false;
    assert_eq!(drag_anchor(&world, dragged.char), Some(Anchor::Free));
    assert!(drag_wintf_moves(&world, dragged.char));

    finish_char_drag(&mut world, dragged.char, point(500, 200));
    assert_eq!(
        position_of(&world, dragged.char),
        Point {
            x: 500,
            y: WA_BOTTOM - H
        }
    );
    // 控えを外した後は今の値を返す。
    assert_eq!(drag_anchor(&world, dragged.char), Some(Anchor::Bottom));
    assert!(!drag_wintf_moves(&world, dragged.char));
    finish(world);
}

/// 始めに向きか位置が読めなければ控えを作らず、同じ窓の前のドラッグの控えも外す（使い回さない）。
#[test]
fn unreadable_begin_clears_a_stale_session() {
    let mut world = world_with_snapshot();
    let dragged = spawn_char(&mut world, 0, Anchor::Bottom, (500, WA_BOTTOM - H));
    let partner = spawn_char(&mut world, 1, Anchor::Free, (900, 150));
    group(&mut world, &["0", "1"]);

    begin_char_drag(&mut world, dragged.char);
    assert!(
        world.contains_resource::<CharDragSession>(),
        "前提: 前の控えがある"
    );
    world.entity_mut(dragged.char).remove::<Anchored>();
    begin_char_drag(&mut world, dragged.char);

    assert!(!world.contains_resource::<CharDragSession>());
    // 控えが無いので、ドラッグの途中でも相手は動かない。
    follow_partners_on_drag(&mut world, dragged.char, point(600, WA_BOTTOM - H));
    assert_eq!(position_of(&world, partner.char), Point { x: 900, y: 150 });
    finish(world);
}
