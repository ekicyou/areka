//! 判定に入れるキャラ窓の矩形の列（balloon-flip-side-rules・task 3.1・要件 2.6／2.8／2.9／6.1）。
//!
//! World は本番の `spawn_ghost_windows` で組み、描画面の子（`Visual` を持つ `ChildOf` の子）は
//! テストが手で付ける（本番では `areka-emo-present` の装着が付ける）。
//!
//! 後半は起動の並べ直しの後の判定し直しの系（`balloon-flip-side-rules` task 4.1・要件 2.7／2.10／4.4／5.1／5.2／6.4）。

use bevy_ecs::prelude::*;
use wintf::ecs::{Visual, WindowPos};

use super::test_support::{base_placement, odd_edge_snapshot, rect};
use super::{BALLOON_LIMIT_CLAMP_TAG, move_window_to, shown_char_rects};
use crate::chain_finalize::ChainFinalized;
use crate::resolver::{PointPx, ScopePlacement};
use crate::shared_test_support::placement_sized;
use crate::source::GhostTitles;
use crate::spawn::{GhostWindows, spawn_ghost_windows};
use crate::test_support::fake_handle;
use log_capture_kit::ExpectField;
use log_capture_kit::capture;
use wintf::ecs::FrameFinalize;

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
fn text_box_surface_alone_does_not_count_as_shown() {
    // `\s[-1]` で装着の面を隠した直後、箱の文字の面（可視のまま）だけが残っている窓は数えない。
    let (mut world, chars) = spawn_scopes(&[(0, 1301), (1, 500)]);
    attach_surface(&mut world, chars[0], true);
    attach_surface(&mut world, chars[1], false);
    world.spawn((
        Name::new("emo-text-box-surface"),
        Visual::default(),
        ChildOf(chars[1]),
    ));
    assert_eq!(shown_char_rects(&world), vec![(0, char_rect_at(1301))]);
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
    // spawn に渡す順を崩しても、列はスコープの昇順（`balloon-flip-side-rules` 要件 2.8）。
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

// ---------------------------------------------------------------------------
// 起動の並べ直しの後の判定し直し（`balloon-flip-side-rules` task 4.1・設計 RejudgeSystem）
// ---------------------------------------------------------------------------

/// 作業領域（開発者の机の主の画面）・キャラ窓の寸・バルーンの寸（`balloon_limit_flip_tests.rs` と同じ形）。
const WA: crate::resolver::RectPx = crate::resolver::RectPx {
    left: 0,
    top: 0,
    right: 2880,
    bottom: 1704,
};
const CSZ: crate::resolver::SizePx = crate::resolver::SizePx { w: 1108, h: 800 };
const BSZ: crate::resolver::SizePx = crate::resolver::SizePx { w: 872, h: 400 };

fn pt(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

fn pos_of(world: &World, entity: Entity) -> PointPx {
    let p = world
        .get::<WindowPos>(entity)
        .and_then(|wp| wp.position)
        .expect("位置がある");
    pt(p.x, p.y)
}

/// 本体（scope 0・左隣のバルーン・`limit` 指定）と相方（scope 1・`limit=0`・子は可視）の 2 スコープの
/// 実 spawn の World。判定し直しの系を `FrameFinalize` に載せてある。戻りは (World, 本体の窓, 本体のバルーン, 相方の窓)。
fn rejudge_world(
    own: PointPx,
    partner: PointPx,
    own_limit: bool,
) -> (World, Entity, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(super::MonitorSnapshot {
        work_areas: vec![WA],
    });
    world.init_resource::<Schedules>();
    super::register_balloon_rejudge(&mut world);
    let titles = GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]);
    let ps = [
        placement_sized(0, own, pt(-872, 0), own_limit, CSZ, BSZ),
        placement_sized(1, partner, pt(-872, 0), false, CSZ, BSZ),
    ];
    let gw = spawn_ghost_windows(&mut world, &ps, &titles);
    let (c0, b0, c1, b1) = (
        gw.char_window(0).expect("キャラ窓"),
        gw.balloon_window(0).expect("バルーン窓"),
        gw.char_window(1).expect("相方のキャラ窓"),
        gw.balloon_window(1).expect("相方のバルーン窓"),
    );
    for (i, e) in [c0, b0, c1, b1].into_iter().enumerate() {
        world.entity_mut(e).insert(fake_handle(0x1000 * (i + 1)));
    }
    attach_surface(&mut world, c0, true);
    attach_surface(&mut world, c1, true);
    (world, c0, b0, c1)
}

/// 1 巡ぶんの `FrameFinalize` を回す。
fn run_frame_finalize(world: &mut World) {
    world.run_schedule(FrameFinalize);
}

fn clamp_lines(events: &[log_capture_kit::CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.message().contains(BALLOON_LIMIT_CLAMP_TAG))
        .count()
}

/// (a)(b) 本体を左端へ動かしてバルーンを右隣（1108〜1980 × 904〜1304）へ並べ替えた後、相方の窓を
/// その映した先に重なる位置へ動かす。印を入れるまでは本体のバルーンは動かず（`balloon-flip-side-rules` 要件 2.7）、印を入れた巡で
/// 押し込みの位置に書かれ（`balloon-flip-side-rules` 要件 2.10）、印の後の 2 回目の巡では系が走らない。
#[test]
fn rejudge_after_chain_finalize_pushes_the_balloon_onto_the_moved_partner_once() {
    let (mut world, c0, b0, c1) = rejudge_world(pt(1200, 904), pt(1108, 0), true);
    move_window_to(&mut world, c0, 0, 904);
    assert_eq!(
        pos_of(&world, b0),
        pt(1108, 904),
        "探針: 相方が下に無いので並べ替える"
    );

    // (b) 印を入れずに相方を動かす（台本の `\![move]` と同じ道）→ 本体のバルーンは動かない。
    move_window_to(&mut world, c1, 1108, 904);
    run_frame_finalize(&mut world);
    assert_eq!(
        pos_of(&world, b0),
        pt(1108, 904),
        "印の無い巡では判定し直さない"
    );

    // (a) 印の入った巡で判定し直す → 押し込みの位置（x=0）。
    world.insert_resource(ChainFinalized);
    let (_, events) = capture(|| run_frame_finalize(&mut world));
    assert_eq!(pos_of(&world, b0), pt(0, 904), "押し込みの位置に書かれる");
    let clamp = log_capture_kit::expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"pushed\"");
    assert_eq!(clamp.expect_field("overlap_scope"), "Some(1)");
    assert_eq!(clamp.expect_field("context"), "\"runtime-gate\"");

    // (b) 印の後の 2 回目の巡: 相方を上へ戻しても系は走らない（走れば右隣へ並べ替わる）。
    move_window_to(&mut world, c1, 1108, 0);
    run_frame_finalize(&mut world);
    assert_eq!(pos_of(&world, b0), pt(0, 904), "2 回目の巡では走らない");
}

/// (c) どの窓も重ならず収まる形 → 系を回しても全バルーンの位置がビット同一で、`Clamp` の行が増えない（`balloon-flip-side-rules` 要件 6.4）。
#[test]
fn rejudge_without_overlap_keeps_every_balloon_bit_identical() {
    let (mut world, c0, b0, c1) = rejudge_world(pt(1200, 904), pt(1800, 0), true);
    let b1 = world
        .resource::<GhostWindows>()
        .balloon_window(1)
        .expect("相方のバルーン窓");
    let before: Vec<_> = [c0, b0, c1, b1]
        .iter()
        .map(|&e| world.get::<WindowPos>(e).cloned())
        .collect();
    world.insert_resource(ChainFinalized);
    let (_, events) = capture(|| run_frame_finalize(&mut world));
    let after: Vec<_> = [c0, b0, c1, b1]
        .iter()
        .map(|&e| world.get::<WindowPos>(e).cloned())
        .collect();
    assert_eq!(after, before);
    // 捕捉が生きていることを確かめてから「無い」を言う（判定し直しの書き込みは窓の移動として記録される）。
    assert!(
        events
            .iter()
            .any(|e| e.message().contains("[diag.window_move]")),
        "系が走った: {events:?}"
    );
    assert_eq!(clamp_lines(&events), 0);
}

/// (d) `windowposition.limit,0` のバルーンは系の対象に入らない（書き込みが 1 つも出ない・設計 RejudgeSystem）。
#[test]
fn rejudge_skips_balloons_with_limit_disabled() {
    let (mut world, _, b0, _) = rejudge_world(pt(0, 904), pt(1800, 0), false);
    let before = pos_of(&world, b0);
    world.insert_resource(ChainFinalized);
    let (_, events) = capture(|| run_frame_finalize(&mut world));
    assert_eq!(pos_of(&world, b0), before);
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains("[diag.window_move]")),
        "書き込みが出ない: {events:?}"
    );
}

/// キャラ窓の位置が決まらないスコープは `[balloon-limit] Unresolved` を残して飛ばし、ほかのスコープは続ける（`balloon-flip-side-rules` 要件 4.4）。
#[test]
fn rejudge_skips_a_scope_whose_char_position_is_undetermined() {
    let (mut world, c0, b0, _) = rejudge_world(pt(1200, 904), pt(1800, 0), true);
    let before = pos_of(&world, b0);
    world.entity_mut(c0).insert(WindowPos::default());
    world.insert_resource(ChainFinalized);
    let (_, events) = capture(|| run_frame_finalize(&mut world));
    assert_eq!(pos_of(&world, b0), before);
    let warn = log_capture_kit::expect_one(&events, super::BALLOON_LIMIT_UNRESOLVED_TAG);
    assert_eq!(warn.expect_field("scope"), "0");
    assert_eq!(warn.expect_field("context"), "\"runtime-gate\"");
}
