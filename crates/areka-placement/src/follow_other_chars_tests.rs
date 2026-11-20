//! 起動の並べ直しの後の判定し直しの系の檻。World は本番の `spawn_ghost_windows` で組む。

use bevy_ecs::prelude::*;
use wintf::ecs::{Visual, WindowPos};

use super::{BALLOON_LIMIT_CLAMP_TAG, move_window_to};
use crate::chain_finalize::ChainFinalized;
use crate::resolver::PointPx;
use crate::shared_test_support::placement_sized;
use crate::source::GhostTitles;
use crate::spawn::{GhostWindows, spawn_ghost_windows};
use crate::test_support::fake_handle;
use log_capture_kit::ExpectField;
use log_capture_kit::capture;
use wintf::ecs::FrameFinalize;

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

/// 作業領域（開発者の机の主の画面）・キャラ窓の寸・バルーンの寸。
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

/// 本体を左端へ動かしてバルーンを (0,904) に置いた後、相方を重なる位置へ動かす。位置は系が走っても走らなくても同じなので、
/// 走ったかは `Clamp` の行で見る: 印の無い巡は 0 行、印の入った巡は押し込みの 1 行、印の後の巡は 0 行。
#[test]
fn rejudge_runs_once_on_the_chain_finalize_mark() {
    let (mut world, c0, b0, c1) = rejudge_world(pt(1200, 904), pt(1108, 0), true);
    move_window_to(&mut world, c0, 0, 904);
    assert_eq!(pos_of(&world, b0), pt(0, 904), "探針: 左端へ寄せた位置");

    let (_, events) = capture(|| {
        move_window_to(&mut world, c1, 1108, 904);
        run_frame_finalize(&mut world);
    });
    assert!(
        events
            .iter()
            .any(|e| e.message().contains("[diag.window_move]")),
        "捕捉が生きている: {events:?}"
    );
    assert_eq!(clamp_lines(&events), 0, "印の無い巡では判定し直さない");

    world.insert_resource(ChainFinalized);
    let (_, events) = capture(|| run_frame_finalize(&mut world));
    assert_eq!(pos_of(&world, b0), pt(0, 904), "押し込みの位置のまま");
    let clamp = log_capture_kit::expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"pushed\"");
    assert_eq!(
        clamp.field("overlap_scope"),
        None,
        "欄 overlap_scope は無い"
    );
    assert_eq!(clamp.expect_field("context"), "\"runtime-gate\"");

    move_window_to(&mut world, c1, 1108, 0);
    let (_, events) = capture(|| run_frame_finalize(&mut world));
    assert_eq!(clamp_lines(&events), 0, "印の後の巡では走らない");
}

/// (c) どの窓も重ならず収まる形 → 系を回しても全バルーンの位置がビット同一で、`Clamp` の行が増えない。
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

/// (d) `windowposition.limit,0` のバルーンは系の対象に入らない（書き込みが 1 つも出ない）。
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

/// キャラ窓の位置が決まらないスコープは `[balloon-limit] Unresolved` を残して飛ばし、ほかのスコープは続ける。
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
