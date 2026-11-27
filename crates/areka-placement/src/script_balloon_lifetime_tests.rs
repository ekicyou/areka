//! 台本の値の寿命のテスト（押し戻しの 3 か所が同じ画面を選ぶこと・記録の欄）。

use log_capture_kit::{CapturedEvent, ExpectField, capture, expect_one};
use wintf::ecs::WindowPos;
use wintf::ecs::pointer::Phase;

use super::test_support::{BALLOON_SIZE, CHAR_SIZES, MONITORS, ScriptWorld, script_world};
use super::*;
use crate::balloon_limit::{BALLOON_LIMIT_CLAMP_TAG, apply_balloon_limit};
use crate::follow::test_support::{drag_end_event_at, window_pos_sized};
use crate::follow::{BalloonFollow, MonitorSnapshot, move_window_to, on_balloon_drag_end};
use crate::shared_test_support::{assert_no_event, placement_sized};
use crate::spawn::BalloonLimit;
use crate::test_support::point;

/// 左右の画面の境をまたぐ位置（中心は左の画面）。
fn straddling() -> PointPx {
    point(-150, 500)
}

fn sakura(script: bool, limit: bool) -> (ScriptWorld, Entity, Entity) {
    let mut w = script_world(&[]);
    let c = w.windows.char_window(0).unwrap();
    let b = w.windows.balloon_window(0).unwrap();
    if script {
        w.world.entity_mut(c).insert(ScriptBalloonPlacement {
            scope: 0,
            offset: None,
            align: Some(ScriptAlign::Left),
        });
    }
    w.world.entity_mut(b).insert(BalloonLimit(limit));
    (w, c, b)
}

fn pos_of(world: &World, window: Entity) -> PointPx {
    let p = world.get::<WindowPos>(window).unwrap().position.unwrap();
    point(p.x, p.y)
}

fn boot(limit: bool) -> (PointPx, Vec<CapturedEvent>) {
    let char_pos = point(2000, MONITORS[1].0.bottom - CHAR_SIZES[0].h);
    let to = straddling();
    let offset = point(to.x - char_pos.x, to.y - char_pos.y);
    let p = placement_sized(0, char_pos, offset, limit, CHAR_SIZES[0], BALLOON_SIZE);
    let snapshot = MonitorSnapshot {
        work_areas: MONITORS.iter().map(|(a, _)| *a).collect(),
    };
    let (out, events) = capture(|| apply_balloon_limit(vec![p], &snapshot));
    (out[0].balloon_pos, events)
}

fn write(script: bool, limit: bool) -> (PointPx, Vec<CapturedEvent>) {
    let (mut w, _, b) = sakura(script, limit);
    let to = straddling();
    let (ok, events) = capture(|| move_window_to(&mut w.world, b, to.x, to.y));
    assert!(ok);
    (pos_of(&w.world, b), events)
}

fn release(script: bool, limit: bool) -> (PointPx, Vec<CapturedEvent>) {
    let (mut w, c, b) = sakura(script, limit);
    let to = straddling();
    let char_pos = pos_of(&w.world, c);
    w.world
        .entity_mut(b)
        .insert(window_pos_sized(to.x, to.y, BALLOON_SIZE.w, BALLOON_SIZE.h));
    let ev = Phase::Bubble(drag_end_event_at(b, (0, 0)));
    let (_, events) = capture(|| on_balloon_drag_end(&mut w.world, b, b, &ev));
    let follow = w.world.get::<BalloonFollow>(c).unwrap().offset();
    assert_eq!(
        follow,
        point(to.x - char_pos.x, to.y - char_pos.y),
        "押し込みを追従の相対の位置へ焼き付けない"
    );
    (pos_of(&w.world, b), events)
}

fn clamp_fields(events: &[CapturedEvent]) -> (String, String) {
    let hit = expect_one(events, BALLOON_LIMIT_CLAMP_TAG);
    (
        hit.expect_field("monitor").trim_matches('"').to_string(),
        hit.expect_field("script").to_string(),
    )
}

#[test]
fn the_three_gates_without_script_push_back_to_the_body_monitor() {
    let body = point(MONITORS[1].0.left, straddling().y);

    let (boot_to, boot_events) = boot(true);
    let (write_to, write_events) = write(false, true);
    let (release_to, release_events) = release(false, true);

    for (to, events) in [
        (boot_to, &boot_events),
        (write_to, &write_events),
        (release_to, &release_events),
    ] {
        assert_eq!(to, body);
        assert_eq!(
            clamp_fields(events),
            ("shell".to_string(), "false".to_string())
        );
    }
}

#[test]
fn write_and_release_with_script_push_into_the_balloon_monitor() {
    let left = point(MONITORS[0].0.right - BALLOON_SIZE.w, straddling().y);

    let (write_to, write_events) = write(true, true);
    let (release_to, release_events) = release(true, true);

    for (to, events) in [(write_to, &write_events), (release_to, &release_events)] {
        assert_eq!(to, left, "中心の画面（左）へ押し込む");
        assert_eq!(
            clamp_fields(events),
            ("balloon".to_string(), "true".to_string())
        );
    }
}

#[test]
fn limit_zero_passes_through_at_the_three_gates() {
    let (boot_to, boot_events) = boot(false);
    let (write_to, write_events) = write(true, false);
    let (release_to, release_events) = release(true, false);

    for (to, events) in [
        (boot_to, &boot_events),
        (write_to, &write_events),
        (release_to, &release_events),
    ] {
        assert_eq!(to, straddling());
        assert_no_event(events, "[balloon-limit]");
    }
}
