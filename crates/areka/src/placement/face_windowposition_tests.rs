//! 面の `windowposition` の置き直し（[`apply_face_windowposition`]・balloon-scroll-arrows 要件 1.7・
//! 11.1）の決定論テスト。
//!
//! 窓の一式は起動の準備（`prepare_ghost_windows_with_work_area`）が解いた配置を本番と同じ
//! `spawn_ghost_windows` で組み、材料は準備が運んだ [`DefaultPlacementBasis`] を置く
//! （`reset_balloon_pos_tests.rs` と同じやり方）。

use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{DPI, Point, WindowHandle, WindowPos};

use super::apply_face_windowposition;
use crate::placement::follow::{BalloonFollow, MonitorSnapshot, OffsetBase};
use crate::placement::prepare_ghost_windows_with_work_area;
use crate::placement::reseed::resolve_standard_scopes;
use crate::placement::reset_pos::DefaultPlacementBasis;
use crate::placement::resolver::PointPx;
use crate::placement::shared_test_support::{WA, balloon_root, emo2_root, with_com_initialized};
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};
use crate::placement::test_support::{LogEvent, capture_logs};

/// ドラッグでずらしたバルーンのずらし（標準のずらしと必ず違う値）。
const DRAGGED: PointPx = PointPx { x: 777, y: -333 };

/// 面 0 と違う面の値（emo2 の既定のバルーンの値と違う数）。
fn other_face() -> (WindowPosition, WindowPositionRaw) {
    (
        WindowPosition::new(Some(41), Some(-23)),
        WindowPositionRaw::new(Some("41".to_string()), None),
    )
}

/// emo2 の起動の準備（作業領域 `WA`・DPI 96）で窓を組み、準備が運んだ材料を置いた World。
fn booted() -> (World, GhostWindows) {
    let prepared =
        prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
            .expect("起動の配置の準備は成功する");
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![WA],
    });
    let windows = spawn_ghost_windows(&mut world, &prepared.placements, &prepared.titles);
    let mut raw = 0x100;
    for scope in windows.scopes() {
        for e in [windows.char_window(scope), windows.balloon_window(scope)] {
            raw += 1;
            world.entity_mut(e.unwrap()).insert(WindowHandle {
                hwnd: HWND(raw as *mut _),
                instance: HINSTANCE::default(),
            });
        }
    }
    // `WindowHandle` の付与の hook が偽 HWND の DPI を遅延で書くので、流してから置く。
    world.flush();
    for scope in windows.scopes() {
        for e in [windows.char_window(scope), windows.balloon_window(scope)] {
            world.entity_mut(e.unwrap()).insert(DPI::from_dpi(96, 96));
        }
    }
    world.insert_resource(prepared.default_basis);
    (world, windows)
}

fn balloon_pos(world: &World, windows: &GhostWindows, scope: usize) -> Point {
    world
        .get::<WindowPos>(windows.balloon_window(scope).unwrap())
        .unwrap()
        .position
        .unwrap()
}

fn follow(world: &World, windows: &GhostWindows, scope: usize) -> BalloonFollow {
    *world
        .get::<BalloonFollow>(windows.char_window(scope).unwrap())
        .unwrap()
}

fn basis(world: &World) -> DefaultPlacementBasis {
    world.resource::<DefaultPlacementBasis>().clone()
}

fn face_logs(events: &[LogEvent]) -> Vec<LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("balloon_face_windowposition"))
        .cloned()
        .collect()
}

/// 標準の位置にあるバルーンは新しい面の標準の位置へ動き、材料が新しい面の値になり、
/// 他のスコープは動かない。記録は info が 1 行（moved=true）。
#[test]
fn standard_balloon_moves_to_the_new_face_position() {
    with_com_initialized(|| {
        let (mut world, windows) = booted();
        let other_before = (
            balloon_pos(&world, &windows, 1),
            follow(&world, &windows, 1),
        );
        let old_offset = follow(&world, &windows, 0).offset();

        let (moved, events) =
            capture_logs(|| apply_face_windowposition(&mut world, 0, other_face()));
        assert!(moved, "標準の位置のバルーンは置き直す");

        let b = basis(&world);
        assert_eq!(b.balloon.windowpositions.get(&0), Some(&other_face()));
        let want = resolve_standard_scopes(&world, &windows, &b.source, &b.balloon)
            .into_iter()
            .find(|s| s.running.input.scope == 0)
            .expect("スコープ 0 の標準の位置を解ける");
        let base = want.placement.balloon_offset_base;
        assert_ne!(
            base.offset, old_offset,
            "前提: 面の値で標準のずらしが変わる"
        );
        assert_eq!(
            follow(&world, &windows, 0).base(),
            base,
            "新しい標準のずらし"
        );
        let char_pos = want.running.char_pos;
        assert_eq!(
            balloon_pos(&world, &windows, 0),
            Point::new(char_pos.x + base.offset.x, char_pos.y + base.offset.y),
            "バルーンが新しい面の標準の位置へ動く"
        );
        assert_eq!(
            (
                balloon_pos(&world, &windows, 1),
                follow(&world, &windows, 1)
            ),
            other_before,
            "他のスコープは動かない"
        );
        let logs = face_logs(&events);
        assert_eq!(logs.len(), 1, "変化 1 回につき 1 行: {events:?}");
        assert_eq!(logs[0].level, tracing::Level::INFO);
        assert_eq!(logs[0].field("scope"), Some("0"));
        assert_eq!(logs[0].field("moved"), Some("true"));
        let _ = drain_window_pos_commands();
    });
}

/// ドラッグで動かしたバルーンは位置とずらしを保ち、材料だけが新しい面の値になる。
#[test]
fn dragged_balloon_keeps_its_position() {
    with_com_initialized(|| {
        let (mut world, windows) = booted();
        let c = windows.char_window(0).unwrap();
        let b = windows.balloon_window(0).unwrap();
        world
            .entity_mut(c)
            .insert(BalloonFollow::new(b, OffsetBase::unpinned(DRAGGED)));
        let before = (
            balloon_pos(&world, &windows, 0),
            follow(&world, &windows, 0),
        );

        let (moved, events) =
            capture_logs(|| apply_face_windowposition(&mut world, 0, other_face()));
        assert!(!moved, "動かしてあるバルーンは置き直さない");
        assert_eq!(
            (
                balloon_pos(&world, &windows, 0),
                follow(&world, &windows, 0)
            ),
            before
        );
        assert_eq!(
            basis(&world).balloon.windowpositions.get(&0),
            Some(&other_face()),
            "材料は新しい面の値"
        );
        let logs = face_logs(&events);
        assert_eq!(logs.len(), 1, "{events:?}");
        assert_eq!(logs[0].field("moved"), Some("false"));
        let _ = drain_window_pos_commands();
    });
}

/// 値が前と同じなら何もしない（材料も位置も記録も変わらない）。
#[test]
fn same_value_does_nothing() {
    with_com_initialized(|| {
        let (mut world, windows) = booted();
        let before_basis = basis(&world);
        let same = before_basis
            .balloon
            .windowpositions
            .get(&0)
            .cloned()
            .expect("前提: 面 0 の windowposition が材料に在る");
        let before: Vec<_> = windows
            .scopes()
            .map(|s| {
                (
                    balloon_pos(&world, &windows, s),
                    follow(&world, &windows, s),
                )
            })
            .collect();

        let (moved, events) = capture_logs(|| apply_face_windowposition(&mut world, 0, same));
        assert!(!moved);
        assert_eq!(basis(&world), before_basis);
        let after: Vec<_> = windows
            .scopes()
            .map(|s| {
                (
                    balloon_pos(&world, &windows, s),
                    follow(&world, &windows, s),
                )
            })
            .collect();
        assert_eq!(after, before);
        assert!(face_logs(&events).is_empty(), "{events:?}");
        let _ = drain_window_pos_commands();
    });
}
