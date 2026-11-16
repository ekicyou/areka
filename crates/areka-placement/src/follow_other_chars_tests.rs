//! 判定に入れるキャラ窓の矩形の列（balloon-flip-side-rules・task 3.1・要件 2.6／2.8／2.9／6.1）。
//!
//! World は本番の `spawn_ghost_windows` で組み、描画面の子（`Visual` を持つ `ChildOf` の子）は
//! テストが手で付ける（本番では `areka-emo-present` の装着が付ける）。

use bevy_ecs::prelude::*;
use wintf::ecs::{Visual, WindowPos};

use super::shown_char_rects;
use super::test_support::{base_placement, odd_edge_snapshot, rect};
use crate::resolver::{PointPx, ScopePlacement};
use crate::source::GhostTitles;
use crate::spawn::{GhostWindows, spawn_ghost_windows};
use log_capture_kit::capture;

/// scope ごとにキャラ窓の位置だけをずらした配置（寸は 434×600）。
fn placement_at(scope: usize, x: i32) -> ScopePlacement {
    let mut p = base_placement(true);
    p.scope = scope;
    p.char_pos = PointPx { x, y: 411 };
    p.balloon_pos = PointPx { x: x + 300, y: 811 };
    p
}

/// `scopes` の各スコープを `x` に置いて実 spawn し、キャラ窓の entity を昇順で返す。
fn spawn_scopes(scopes: &[(usize, i32)]) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot());
    let placements: Vec<_> = scopes.iter().map(|&(s, x)| placement_at(s, x)).collect();
    let titles = GhostTitles::from_scope_titles(scopes.iter().map(|&(s, _)| (s, format!("c{s}"))));
    let gw = spawn_ghost_windows(&mut world, &placements, &titles);
    let chars = scopes
        .iter()
        .map(|&(s, _)| gw.char_window(s).expect("キャラ窓が spawn される"))
        .collect();
    (world, chars)
}

/// キャラ窓に描画面の子を 1 つ付ける。
fn attach_surface(world: &mut World, char_window: Entity, is_visible: bool) {
    world.spawn((
        Visual {
            is_visible,
            ..Default::default()
        },
        ChildOf(char_window),
    ));
}

fn char_rect_at(x: i32) -> crate::resolver::RectPx {
    rect(x, 411, x + 434, 411 + 600)
}

#[test]
fn visible_window_is_included() {
    let (mut world, chars) = spawn_scopes(&[(0, 1301)]);
    attach_surface(&mut world, chars[0], true);
    assert_eq!(shown_char_rects(&world), vec![(0, char_rect_at(1301))]);
}

#[test]
fn window_with_any_visible_child_is_included() {
    let (mut world, chars) = spawn_scopes(&[(0, 1301)]);
    attach_surface(&mut world, chars[0], false);
    attach_surface(&mut world, chars[0], true);
    assert_eq!(shown_char_rects(&world), vec![(0, char_rect_at(1301))]);
}

#[test]
fn window_whose_children_are_all_invisible_is_excluded() {
    let (mut world, chars) = spawn_scopes(&[(0, 1301), (1, 500)]);
    attach_surface(&mut world, chars[0], true);
    attach_surface(&mut world, chars[1], false);
    attach_surface(&mut world, chars[1], false);
    assert_eq!(shown_char_rects(&world), vec![(0, char_rect_at(1301))]);
}

#[test]
fn window_without_surface_children_is_excluded() {
    let (mut world, chars) = spawn_scopes(&[(0, 1301), (1, 500)]);
    attach_surface(&mut world, chars[1], true);
    assert_eq!(shown_char_rects(&world), vec![(1, char_rect_at(500))]);
}

#[test]
fn window_with_undetermined_size_is_excluded() {
    let (mut world, chars) = spawn_scopes(&[(0, 1301), (1, 500)]);
    attach_surface(&mut world, chars[0], true);
    attach_surface(&mut world, chars[1], true);
    // 寸の決まらない窓（CW_USEDEFAULT の位置・寸なし）。
    world.entity_mut(chars[1]).insert(WindowPos::default());
    assert_eq!(shown_char_rects(&world), vec![(0, char_rect_at(1301))]);
}

#[test]
fn three_scopes_come_in_ascending_order() {
    // spawn に渡す順を崩しても、列はスコープの昇順（要件 2.8）。
    let (mut world, chars) = spawn_scopes(&[(2, 100), (0, 1301), (1, 700)]);
    for &c in &chars {
        attach_surface(&mut world, c, true);
    }
    assert_eq!(
        shown_char_rects(&world),
        vec![
            (0, char_rect_at(1301)),
            (1, char_rect_at(700)),
            (2, char_rect_at(100)),
        ]
    );
}

#[test]
fn no_ghost_windows_gives_empty_list_without_logging() {
    let (mut world, chars) = spawn_scopes(&[(0, 1301)]);
    attach_surface(&mut world, chars[0], true);
    let _ = world.remove_resource::<GhostWindows>();
    let (rects, events) = capture(|| shown_char_rects(&world));
    assert!(rects.is_empty());
    assert!(events.is_empty(), "記録は出さない: {events:?}");
}
