//! 台本の値の寿命のテスト（シェル・面・初期化・窓の捨て・本体のドラッグ・DPI の変化と、押し戻しの 3 か所が同じ画面を選ぶこと）。

use areka_emo_compose::ScaleRatio;
use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};
use log_capture_kit::{CapturedEvent, ExpectField, capture, expect_one};
use std::time::Instant;

use wintf::ecs::drag::{DragEvent, DraggingState};
use wintf::ecs::pointer::Phase;
use wintf::ecs::{DPI, Point, WindowPos};

use super::test_support::{BALLOON_SIZE, CHAR_SIZES, MONITORS, ScriptWorld, script_world};
use super::*;
use crate::balloon_limit::{BALLOON_LIMIT_CLAMP_TAG, apply_balloon_limit};
use crate::face_windowposition::apply_face_windowposition;
use crate::follow::test_support::{drag_end_event_at, window_pos_sized};
use crate::follow::{
    BalloonFollow, MonitorSnapshot, OffsetRescale, move_window_to, on_balloon_drag_end,
    on_char_drag, rescale_follow_offset,
};
use crate::persist::balloon_offset_entries;
use crate::reseed::test_support::new_shell;
use crate::reseed::{BalloonPlacementInputs, apply_shell_descript};
use crate::reset_balloon_pos::{BalloonResetTrigger, reset_balloon_positions};
use crate::shared_test_support::{assert_no_event, events_with, one_event_with, placement_sized};
use crate::source::DescriptSource;
use crate::spawn::BalloonLimit;
use crate::test_support::point;
use crate::windowposition::scale_signed;

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

fn abs(x: i32, y: i32) -> ScriptOffset {
    ScriptOffset {
        x: OffsetAxis::Absolute(x),
        y: OffsetAxis::Absolute(y),
    }
}

fn add(a: PointPx, b: PointPx) -> PointPx {
    point(a.x + b.x, a.y + b.y)
}

fn no_balloon() -> BalloonPlacementInputs {
    BalloonPlacementInputs {
        author_dpi: 96,
        windowpositions: Default::default(),
    }
}

fn offset_of(solved: Vec<StandardScope>, scope: usize) -> PointPx {
    solved
        .into_iter()
        .find(|s| s.placement.scope == scope)
        .unwrap()
        .placement
        .balloon_offset
}

fn script_standard(w: &ScriptWorld, src: &DescriptSource, scope: usize) -> PointPx {
    offset_of(
        resolve_standard_scopes(&w.world, &w.windows, src, &no_balloon()),
        scope,
    )
}

fn descript_standard(w: &ScriptWorld, src: &DescriptSource, scope: usize) -> PointPx {
    offset_of(
        resolve_descript_scopes(&w.world, &w.windows, src, &no_balloon()),
        scope,
    )
}

fn basis_source(w: &ScriptWorld) -> DescriptSource {
    w.world.resource::<DefaultPlacementBasis>().source.clone()
}

fn follow_of(w: &ScriptWorld, scope: usize) -> BalloonFollow {
    let c = w.windows.char_window(scope).unwrap();
    *w.world.get::<BalloonFollow>(c).unwrap()
}

fn user_moved(w: &ScriptWorld) -> bool {
    let c = w.windows.char_window(0).unwrap();
    w.world.entity(c).contains::<BalloonUserMoved>()
}

fn script_value(w: &ScriptWorld) -> Option<ScriptBalloonPlacement> {
    let c = w.windows.char_window(0).unwrap();
    w.world.get::<ScriptBalloonPlacement>(c).cloned()
}

/// sakura のバルーンが本体の左上から `rel` の所に居ること。
fn assert_balloon_at(w: &ScriptWorld, rel: PointPx) {
    let c = pos_of(&w.world, w.windows.char_window(0).unwrap());
    let b = pos_of(&w.world, w.windows.balloon_window(0).unwrap());
    assert_eq!(b, add(c, rel));
}

#[test]
fn shell_switch_keeps_both_values_and_relative_follows_the_new_descript() {
    let mut w = script_world(&[]);
    apply_script_balloon_offset(
        &mut w.world,
        0,
        ScriptOffset {
            x: OffsetAxis::Relative(-10),
            y: OffsetAxis::Relative(5),
        },
        true,
    );
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    let before = script_value(&w);

    let (_, events) = capture(|| {
        apply_shell_descript(
            &mut w.world,
            &w.windows,
            &new_shell(&[]),
            &no_balloon(),
            &[],
        )
    });

    assert_eq!(script_value(&w), before, "シェルの切り替えで台本の値を保つ");
    assert!(events_with(&events, "script_balloon_dropped").is_empty());
    // 新しいシェルの sakura は (30,40)・右。台本の `@-10,@5`・left を重ねたものは、descript に (20,45)・左を書いたのと同じ。
    let written = new_shell(&[
        ("sakura.balloon.offsetx", "20"),
        ("sakura.balloon.offsety", "45"),
        ("sakura.balloon.alignment", "left"),
    ]);
    let expected = descript_standard(&w, &written, 0);
    assert_eq!(follow_of(&w, 0).offset(), expected);
    assert_balloon_at(&w, expected);
}

#[test]
fn shell_switch_adds_the_share_to_a_restored_offset() {
    let mut w = script_world(&[]);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    let persisted = point(-260, 15);
    let src = new_shell(&[]);

    apply_shell_descript(
        &mut w.world,
        &w.windows,
        &src,
        &no_balloon(),
        &balloon_offset_entries(0, persisted),
    );

    let s = script_standard(&w, &src, 0);
    let d = descript_standard(&w, &src, 0);
    let share = point(s.x - d.x, s.y - d.y);
    assert_ne!(share, point(0, 0), "前提: 分が 0 でない");
    let f = follow_of(&w, 0);
    assert_eq!(f.offset(), add(persisted, share));
    assert_eq!(f.base().offset, add(persisted, share));
    assert_eq!(f.base().dpi, None, "記憶から戻した値は未係留のまま");
    assert!(user_moved(&w));
    assert_balloon_at(&w, add(persisted, share));
    assert_eq!(
        follow_of(&w, 1).offset(),
        descript_standard(&w, &src, 1),
        "台本の値の無いスコープは新しいシェルの値のまま"
    );
}

#[test]
fn restored_offset_without_script_value_is_used_as_is_on_shell_switch() {
    let mut w = script_world(&[]);
    let persisted = point(-260, 15);

    apply_shell_descript(
        &mut w.world,
        &w.windows,
        &new_shell(&[]),
        &no_balloon(),
        &balloon_offset_entries(0, persisted),
    );

    assert_eq!(follow_of(&w, 0).offset(), persisted);
}

#[test]
fn face_windowposition_rewrite_keeps_the_script_position() {
    let mut w = script_world(&[]);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);

    assert!(apply_face_windowposition(
        &mut w.world,
        0,
        (
            WindowPosition::new(Some(41), Some(-23)),
            WindowPositionRaw::new(Some("41".to_string()), None),
        ),
    ));

    let basis = w.world.resource::<DefaultPlacementBasis>().clone();
    let expected = offset_of(
        resolve_standard_scopes(&w.world, &w.windows, &basis.source, &basis.balloon),
        0,
    );
    let descript = offset_of(
        resolve_descript_scopes(&w.world, &w.windows, &basis.source, &basis.balloon),
        0,
    );
    assert_ne!(
        expected, descript,
        "前提: 台本の位置と descript の位置が違う"
    );
    assert!(script_value(&w).is_some());
    assert_eq!(follow_of(&w, 0).offset(), expected);
    assert_balloon_at(&w, expected);
}

#[test]
fn reset_drops_only_the_user_shift_and_returns_to_the_script_position() {
    let mut w = script_world(&[]);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    let script = script_standard(&w, &basis_source(&w), 0);
    let c = w.windows.char_window(0).unwrap();
    let b = w.windows.balloon_window(0).unwrap();
    let char_pos = pos_of(&w.world, c);
    w.world.entity_mut(b).insert(window_pos_sized(
        char_pos.x + script.x + 30,
        char_pos.y + script.y + 20,
        BALLOON_SIZE.w,
        BALLOON_SIZE.h,
    ));
    let ev = Phase::Bubble(drag_end_event_at(b, (0, 0)));
    on_balloon_drag_end(&mut w.world, b, b, &ev);
    assert!(user_moved(&w), "前提: 利用者が動かした");

    assert!(reset_balloon_positions(
        &mut w.world,
        BalloonResetTrigger::Menu
    ));

    assert!(script_value(&w).is_some(), "初期化は台本の値に触らない");
    assert!(!user_moved(&w));
    assert_eq!(follow_of(&w, 0).offset(), script);
    assert_balloon_at(&w, script);
}

#[test]
fn despawning_the_char_window_logs_one_drop_line() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true);
    let c = w.windows.char_window(0).unwrap();

    let (_, events) = capture(|| w.world.despawn(c));

    let dropped = one_event_with(&events, "script_balloon_dropped");
    assert_eq!(dropped.level, tracing::Level::INFO);
    assert_eq!(dropped.field("scope"), Some("0"));
}

#[test]
fn shell_reload_that_keeps_the_window_keeps_the_script_value() {
    let mut w = script_world(&[]);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    let before = script_value(&w);
    let src = basis_source(&w);
    let script = script_standard(&w, &src, 0);

    let (_, events) =
        capture(|| apply_shell_descript(&mut w.world, &w.windows, &src, &no_balloon(), &[]));

    assert_eq!(script_value(&w), before, "向きも落とさない");
    assert!(events_with(&events, "script_balloon_dropped").is_empty());
    assert_eq!(follow_of(&w, 0).offset(), script);
}

#[test]
fn body_drag_keeps_the_script_relative_position() {
    let mut w = script_world(&[]);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    let script = script_standard(&w, &basis_source(&w), 0);
    let c = w.windows.char_window(0).unwrap();
    let from = pos_of(&w.world, c);
    let start = Point { x: 1400, y: 600 };
    w.world.entity_mut(c).insert(DraggingState {
        drag_start_pos: start,
        initial_window_pos: Some((from.x as f32, from.y as f32)),
    });
    let ev = DragEvent {
        target: c,
        start_position: start,
        position: Point { x: 1100, y: 400 },
        timestamp: Instant::now(),
    };

    assert!(!on_char_drag(&mut w.world, c, c, &Phase::Bubble(ev)));

    assert_eq!(
        pos_of(&w.world, c),
        point(from.x - 300, from.y),
        "前提: 下端に吸着したまま横へ動く"
    );
    assert_balloon_at(&w, script);
}

#[test]
fn dpi_change_rederives_from_the_seated_basis() {
    let mut w = script_world(&[]);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    let script = script_standard(&w, &basis_source(&w), 0);
    let base = follow_of(&w, 0).base();
    assert_eq!(base.offset, script, "据えた基準は台本の位置");
    assert_eq!(base.dpi, Some(DPI::from_dpi(96, 96)));

    let k = ScaleRatio::new(144, 96).unwrap();
    assert_eq!(
        rescale_follow_offset(base, DPI::from_dpi(144, 144)),
        OffsetRescale::Rescaled {
            offset: point(scale_signed(script.x, k), scale_signed(script.y, k)),
            saturated: false,
        }
    );
    assert_eq!(
        rescale_follow_offset(base, DPI::from_dpi(96, 96)),
        OffsetRescale::Unchanged,
        "戻れば据えた基準のまま"
    );
}
