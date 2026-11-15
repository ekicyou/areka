//! 面の `windowposition` の置き直し（[`apply_face_windowposition`]・balloon-scroll-arrows 要件 1.7・
//! 11.1）の決定論テスト。
//!
//! 窓の一式は起動の準備（`prepare_ghost_windows_with_work_area`）が解いた配置を本番と同じ
//! `spawn_ghost_windows` で組み、材料は準備が運んだ [`DefaultPlacementBasis`] を置く
//! （`reset_balloon_pos_tests.rs` と同じやり方）。

use std::time::Instant;

use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};
use areka_sylphya::PersistKey;
use bevy_ecs::prelude::*;
use wintf::ecs::drag::DragEvent;
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{DPI, Point, SizeI, WindowPos};

use super::apply_face_windowposition;
use crate::follow::{
    BalloonFollow, MonitorSnapshot, OffsetRescale, on_balloon_drag, rescale_follow_offset,
};
use crate::persist::{apply_restored_placements, balloon_offset_entries};
use crate::reseed::{StandardScope, resolve_standard_scopes};
use crate::reset_balloon_pos::{BalloonResetTrigger, reset_balloon_positions};
use crate::reset_pos::DefaultPlacementBasis;
use crate::reset_pos::reset_pos_tests::give_fake_hwnds;
use crate::sample_test_support::{balloon_root, emo2_root};
use crate::spawn::{GhostWindows, spawn_ghost_windows};
use crate::test_support::{WA, with_com_initialized};
use crate::{PreparedPlacement, prepare_ghost_windows_with_work_area};
use log_capture_kit::{CapturedEvent, capture};

/// 面 0 と違う面の値（emo2 の既定のバルーンの値と違う数）。
fn other_face() -> (WindowPosition, WindowPositionRaw) {
    (
        WindowPosition::new(Some(41), Some(-23)),
        WindowPositionRaw::new(Some("41".to_string()), None),
    )
}

/// [`other_face`] とも emo2 の既定とも違う、3 つ目の面の値。
fn third_face() -> (WindowPosition, WindowPositionRaw) {
    (
        WindowPosition::new(Some(-17), Some(9)),
        WindowPositionRaw::new(Some("-17".to_string()), None),
    )
}

/// emo2 の起動の準備（作業領域 `WA`・表示 DPI `dpi`）。
fn prepared(dpi: u32) -> PreparedPlacement {
    prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(dpi))
        .expect("起動の配置の準備は成功する")
}

/// emo2 の起動の準備（作業領域 `WA`・DPI 96）で窓を組み、準備が運んだ材料を置いた World。
fn booted() -> (World, GhostWindows) {
    booted_with(&[])
}

/// [`booted`] と同じだが、起動と同じく記憶の項目 `restored` を重ねた配置で窓を組む。
fn booted_with(restored: &[(PersistKey, String)]) -> (World, GhostWindows) {
    let prepared = prepared(96);
    let mut world = World::new();
    let snapshot = MonitorSnapshot {
        work_areas: vec![WA],
    };
    let placements = apply_restored_placements(prepared.placements, restored, &snapshot);
    world.insert_resource(snapshot);
    let windows = spawn_ghost_windows(&mut world, &placements, &prepared.titles);
    give_fake_hwnds(&mut world, &windows);
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

/// 今の材料で解いたスコープ 0 の標準の位置。
fn standard_of_scope0(world: &World, windows: &GhostWindows) -> StandardScope {
    let b = basis(world);
    resolve_standard_scopes(world, windows, &b.source, &b.balloon)
        .into_iter()
        .find(|s| s.running.input.scope == 0)
        .expect("スコープ 0 の標準の位置を解ける")
}

/// 本番のドラッグの口（[`on_balloon_drag`]）で、スコープのバルーンを横へ `dx` だけずらす
/// （wndproc が窓を動かした後の位置を置いてから呼ぶ）。
fn drag_balloon(world: &mut World, windows: &GhostWindows, scope: usize, dx: i32) {
    let b = windows.balloon_window(scope).unwrap();
    let p = balloon_pos(world, windows, scope);
    world.get_mut::<WindowPos>(b).unwrap().position = Some(Point::new(p.x + dx, p.y));
    let ev = Phase::Bubble(DragEvent {
        target: b,
        start_position: Point::new(0, 0),
        position: Point::new(dx, 0),
        timestamp: Instant::now(),
    });
    assert!(!on_balloon_drag(world, b, b, &ev));
}

fn face_logs(events: &[CapturedEvent]) -> Vec<CapturedEvent> {
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

        let (moved, events) = capture(|| apply_face_windowposition(&mut world, 0, other_face()));
        assert!(moved, "標準の位置のバルーンは置き直す");

        assert_eq!(
            basis(&world).balloon.windowpositions.get(&0),
            Some(&other_face())
        );
        let want = standard_of_scope0(&world, &windows);
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

/// 1px だけドラッグしたバルーンも位置とずらしを保ち、材料だけが新しい面の値になる
/// （動かした量でなく、動かしたことで決める）。
#[test]
fn dragged_balloon_keeps_its_position() {
    with_com_initialized(|| {
        let (mut world, windows) = booted();
        drag_balloon(&mut world, &windows, 0, 1);
        let before = (
            balloon_pos(&world, &windows, 0),
            follow(&world, &windows, 0),
        );

        let (moved, events) = capture(|| apply_face_windowposition(&mut world, 0, other_face()));
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

        let (moved, events) = capture(|| apply_face_windowposition(&mut world, 0, same));
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

/// 材料（`DefaultPlacementBasis`）・窓の一式（`GhostWindows`）が無ければ `warn!` を 1 行出して
/// 何もしない。
#[test]
fn missing_material_only_warns() {
    with_com_initialized(|| {
        for reason in ["no_default_basis", "no_ghost_windows"] {
            let (mut world, _windows) = booted();
            if reason == "no_default_basis" {
                world.remove_resource::<DefaultPlacementBasis>();
            } else {
                world.remove_resource::<GhostWindows>();
            }
            let (moved, events) =
                capture(|| apply_face_windowposition(&mut world, 0, other_face()));
            assert!(!moved, "{reason}: 何もしない");
            let skips: Vec<_> = events
                .iter()
                .filter(|e| e.field_str("event") == Some("balloon_face_windowposition_skipped"))
                .collect();
            assert_eq!(skips.len(), 1, "{reason}: warn が 1 行: {events:?}");
            assert_eq!(skips[0].level, tracing::Level::WARN);
            assert_eq!(skips[0].field_str("reason"), Some(reason));
            assert!(face_logs(&events).is_empty(), "{reason}: {events:?}");
            let _ = drain_window_pos_commands();
        }
    });
}

/// 96 で面を変えて標準の位置へ置き、144 へ移ってずらしを追随させただけのバルーンは、また面を
/// 変えると新しい面の標準の位置へ置き直される（追随の値と 144 で解き直した値は丸めで 1px
/// 食い違う）。
#[test]
fn balloon_followed_from_96_to_144_moves_to_the_new_face_position() {
    with_com_initialized(|| {
        let (mut world, windows) = booted();
        assert!(
            apply_face_windowposition(&mut world, 0, other_face()),
            "前提: 96 で面の標準の位置へ置く"
        );
        let at_144 = prepared(144);
        // 144 の表示の窓寸と DPI を置き、本体のずらしの追随と同じ換算でずらしを引き直す。
        for p in &at_144.placements {
            let c = windows.char_window(p.scope).unwrap();
            let b = windows.balloon_window(p.scope).unwrap();
            world.get_mut::<WindowPos>(c).unwrap().size =
                Some(SizeI::new(p.char_size.w, p.char_size.h));
            world.get_mut::<WindowPos>(b).unwrap().size =
                Some(SizeI::new(p.balloon_size.w, p.balloon_size.h));
            for e in [c, b] {
                world.entity_mut(e).insert(DPI::from_dpi(144, 144));
            }
            let mut f = world.get_mut::<BalloonFollow>(c).unwrap();
            match rescale_follow_offset(f.base(), DPI::from_dpi(144, 144)) {
                OffsetRescale::Rescaled { offset, .. } => f.apply_rescaled(offset),
                other => panic!("96 の基準は 144 へ追随する: {other:?}"),
            }
        }
        let standard_144 = standard_of_scope0(&world, &windows)
            .placement
            .balloon_offset_base
            .offset;
        assert_ne!(
            follow(&world, &windows, 0).offset(),
            standard_144,
            "前提: 追随の値と 144 で解き直した標準の値は丸めで食い違う"
        );

        assert!(
            apply_face_windowposition(&mut world, 0, third_face()),
            "利用者が動かしていないバルーンは置き直す"
        );
        assert_eq!(
            follow(&world, &windows, 0).base(),
            standard_of_scope0(&world, &windows)
                .placement
                .balloon_offset_base
        );
        let _ = drain_window_pos_commands();
    });
}

/// 記憶から戻したバルーンは、記憶の値が標準の値と同じでも置き直さない
/// （値の違いでなく、記憶から戻したことで決める）。
#[test]
fn balloon_restored_from_memory_keeps_its_position() {
    with_com_initialized(|| {
        let standard = prepared(96).placements[0].balloon_offset;
        let (mut world, windows) = booted_with(&balloon_offset_entries(0, standard));
        assert_eq!(
            follow(&world, &windows, 0).offset(),
            standard,
            "前提: 記憶の値は標準の値と同じ"
        );
        let before = (
            balloon_pos(&world, &windows, 0),
            follow(&world, &windows, 0),
        );

        assert!(
            !apply_face_windowposition(&mut world, 0, other_face()),
            "記憶から戻したバルーンは置き直さない"
        );
        assert_eq!(
            (
                balloon_pos(&world, &windows, 0),
                follow(&world, &windows, 0)
            ),
            before
        );
        let _ = drain_window_pos_commands();
    });
}

/// ドラッグしたバルーンも、記憶から戻したバルーンも、標準の位置へ据え直した（バルーン位置
/// 初期化）後は、面を変えると再び置き直される。
#[test]
fn balloon_reset_to_standard_moves_again_on_face_change() {
    with_com_initialized(|| {
        let standard = prepared(96).placements[0].balloon_offset;
        for (name, restored, dx) in [
            ("ドラッグ", Vec::new(), 1),
            ("記憶", balloon_offset_entries(0, standard), 0),
        ] {
            let (mut world, windows) = booted_with(&restored);
            if dx != 0 {
                drag_balloon(&mut world, &windows, 0, dx);
            }
            assert!(
                reset_balloon_positions(&mut world, BalloonResetTrigger::Menu),
                "{name}: 前提: 標準の位置へ据え直す"
            );
            assert!(
                apply_face_windowposition(&mut world, 0, other_face()),
                "{name}: 据え直した後のバルーンは置き直す"
            );
            let _ = drain_window_pos_commands();
        }
    });
}
