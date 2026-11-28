use log_capture_kit::capture;
use wintf::ecs::drag::WindowDragging;
use wintf::ecs::{Point, SizeI, WindowPos};

use super::test_support::{BALLOON_SIZE, CHAR_SIZES, ScriptWorld, script_world};
use super::*;
use crate::balloon_lock::apply_script_balloon_lock;
use crate::follow::{BalloonFollow, BalloonFollowTrigger, follow_balloon};
use crate::reseed::test_support::position_of;
use crate::resolver::ScopePlacement;
use crate::resolver::SizePx;
use crate::shared_test_support::{events_with, one_event_with};

fn place(
    w: &mut ScriptWorld,
    scope: usize,
    offset: Option<ScriptOffset>,
    align: Option<ScriptAlign>,
) {
    let c = w.windows.char_window(scope).unwrap();
    w.world.entity_mut(c).insert(ScriptBalloonPlacement {
        scope,
        offset,
        align,
    });
}

fn standard(w: &ScriptWorld) -> Vec<ScopePlacement> {
    let basis = w.world.resource::<DefaultPlacementBasis>();
    resolve_standard_scopes(&w.world, &w.windows, &basis.source, &basis.balloon)
        .into_iter()
        .map(|s| s.placement)
        .collect()
}

fn descript(w: &ScriptWorld) -> Vec<ScopePlacement> {
    let basis = w.world.resource::<DefaultPlacementBasis>();
    resolve_descript_scopes(&w.world, &w.windows, &basis.source, &basis.balloon)
        .into_iter()
        .map(|s| s.placement)
        .collect()
}

fn abs(x: i32, y: i32) -> ScriptOffset {
    ScriptOffset {
        x: OffsetAxis::Absolute(x),
        y: OffsetAxis::Absolute(y),
    }
}

#[test]
fn scope_without_script_value_resolves_as_before() {
    let plain = script_world(&[]);
    let before = standard(&plain);
    assert_eq!(before, descript(&plain));

    let mut w = script_world(&[]);
    place(&mut w, 0, Some(abs(100, 30)), Some(ScriptAlign::Right));

    assert_eq!(standard(&w)[1], before[1]);
    assert_eq!(descript(&w), before);
}

#[test]
fn absolute_offset_resolves_like_the_same_descript_offset() {
    let mut w = script_world(&[]);
    place(&mut w, 0, Some(abs(100, 30)), None);
    let same = script_world(&[
        ("sakura.balloon.offsetx", "100"),
        ("sakura.balloon.offsety", "30"),
    ]);

    let got = standard(&w)[0];

    assert_eq!(got, descript(&same)[0]);
    assert_ne!(got, descript(&w)[0]);
}

#[test]
fn relative_offset_adds_to_the_descript_offset() {
    let mut w = script_world(&[("sakura.balloon.offsetx", "50")]);
    place(
        &mut w,
        0,
        Some(ScriptOffset {
            x: OffsetAxis::Relative(-10),
            y: OffsetAxis::Relative(5),
        }),
        None,
    );
    let same = script_world(&[
        ("sakura.balloon.offsetx", "40"),
        ("sakura.balloon.offsety", "5"),
    ]);

    assert_eq!(standard(&w)[0], descript(&same)[0]);
}

#[test]
fn align_resolves_like_the_same_descript_alignment() {
    let mut w = script_world(&[]);
    place(&mut w, 0, None, Some(ScriptAlign::Right));
    let same = script_world(&[("sakura.balloon.alignment", "right")]);

    let got = standard(&w)[0];

    assert_eq!(got, descript(&same)[0]);
    assert_ne!(got, descript(&w)[0]);
}

#[test]
fn share_is_script_standard_minus_descript_standard() {
    let mut w = script_world(&[]);
    assert_eq!(script_share(&w.world, 0), ScriptShare::NotPlaced);

    place(&mut w, 0, Some(abs(100, 30)), None);
    let (s, d) = (standard(&w)[0], descript(&w)[0]);

    let ScriptShare::Share(share) = script_share(&w.world, 0) else {
        panic!("分が解ける");
    };
    assert_eq!(
        share,
        PointPx {
            x: s.balloon_offset.x - d.balloon_offset.x,
            y: s.balloon_offset.y - d.balloon_offset.y,
        }
    );
    assert_ne!(share, PointPx { x: 0, y: 0 });
    assert_eq!(script_share(&w.world, 1), ScriptShare::NotPlaced);
}

#[test]
fn share_without_default_basis_is_unresolved() {
    let mut w = script_world(&[]);
    place(&mut w, 0, Some(abs(100, 30)), None);
    w.world.remove_resource::<DefaultPlacementBasis>();

    assert_eq!(
        script_share(&w.world, 0),
        ScriptShare::Unresolved("no_default_basis")
    );
}

fn char_at(w: &ScriptWorld, scope: usize) -> Point {
    position_of(&w.world, w.windows.char_window(scope).unwrap())
}

fn balloon_at(w: &ScriptWorld, scope: usize) -> Point {
    position_of(&w.world, w.windows.balloon_window(scope).unwrap())
}

fn shifted(p: Point, dx: i32, dy: i32) -> Point {
    Point {
        x: p.x + dx,
        y: p.y + dy,
    }
}

#[test]
fn absolute_offset_reseats_shown_balloon_in_the_same_pass() {
    let mut w = script_world(&[]);
    let c = char_at(&w, 0);

    let (applied, events) =
        capture(|| apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true));

    assert!(applied);
    assert_eq!(
        balloon_at(&w, 0),
        shifted(c, -BALLOON_SIZE.w + 100, 30),
        "左に出すバルーンは本体の左隣＋換算した値（作者と窓の DPI は同じ 96）"
    );
    let set = one_event_with(&events, "script_balloon_offset_set");
    assert_eq!(set.level, tracing::Level::INFO);
    assert_eq!(set.field_str("x_kind"), Some("absolute"));
    assert_eq!(set.field("x"), Some("100"));
    assert_eq!(set.field_str("y_kind"), Some("absolute"));
    assert_eq!(set.field("y"), Some("30"));
}

#[test]
fn relative_axis_is_logged_as_relative() {
    let mut w = script_world(&[]);

    let (_, events) = capture(|| {
        apply_script_balloon_offset(
            &mut w.world,
            0,
            ScriptOffset {
                x: OffsetAxis::Relative(-10),
                y: OffsetAxis::Absolute(0),
            },
            true,
        )
    });

    let set = one_event_with(&events, "script_balloon_offset_set");
    assert_eq!(set.field_str("x_kind"), Some("relative"));
    assert_eq!(set.field("x"), Some("-10"));
    assert_eq!(set.field_str("y_kind"), Some("absolute"));
}

#[test]
fn positive_x_points_toward_the_body_on_the_right() {
    let mut w = script_world(&[]);
    let c = char_at(&w, 0);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true);

    apply_script_balloon_offset(&mut w.world, 0, abs(100, 0), true);

    assert_eq!(balloon_at(&w, 0), shifted(c, CHAR_SIZES[0].w - 100, 0));
}

#[test]
fn positive_x_points_right_for_center_and_bottom() {
    let center_x = (CHAR_SIZES[0].w - BALLOON_SIZE.w) / 2 + 100;
    for (align, dy, y) in [
        (ScriptAlign::Center, 20, -BALLOON_SIZE.h + 20),
        (ScriptAlign::Bottom, -300, CHAR_SIZES[0].h - 300),
    ] {
        let mut w = script_world(&[]);
        let c = char_at(&w, 0);
        apply_script_balloon_align(&mut w.world, 0, align.clone(), true);

        apply_script_balloon_offset(&mut w.world, 0, abs(100, dy), true);

        assert_eq!(balloon_at(&w, 0), shifted(c, center_x, y), "{align:?}");
    }
}

#[test]
fn changing_align_moves_to_the_new_side_in_the_same_pass() {
    let mut w = script_world(&[]);
    let c = char_at(&w, 0);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    assert_eq!(balloon_at(&w, 0), shifted(c, -BALLOON_SIZE.w, 0));

    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true);

    assert_eq!(balloon_at(&w, 0), shifted(c, CHAR_SIZES[0].w, 0));
}

#[test]
fn fixed_align_keeps_its_side_after_the_body_moves() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true);
    let c = w.windows.char_window(0).unwrap();
    let moved = Point {
        x: 100,
        y: char_at(&w, 0).y,
    };
    w.world.get_mut::<WindowPos>(c).unwrap().position = Some(moved);
    follow_balloon(&mut w.world, c, moved, BalloonFollowTrigger::Drag);

    apply_script_balloon_offset(&mut w.world, 0, abs(0, 0), true);

    assert_eq!(balloon_at(&w, 0), shifted(moved, CHAR_SIZES[0].w, 0));
}

#[test]
fn unknown_align_places_like_none_and_warns_once() {
    let mut none = script_world(&[]);
    apply_script_balloon_align(&mut none.world, 0, ScriptAlign::Right, true);
    apply_script_balloon_align(&mut none.world, 0, ScriptAlign::None, true);
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true);

    let (applied, events) = capture(|| {
        apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Unknown("foo".into()), true)
    });

    assert!(applied);
    assert_eq!(balloon_at(&w, 0), balloon_at(&none, 0));
    assert_ne!(
        balloon_at(&w, 0),
        shifted(char_at(&w, 0), CHAR_SIZES[0].w, 0)
    );
    let warn = one_event_with(&events, "script_balloon_align_unknown");
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.field_str("raw"), Some("foo"));
    let c = w.windows.char_window(0).unwrap();
    assert_eq!(
        w.world.get::<ScriptBalloonPlacement>(c).unwrap().align,
        Some(ScriptAlign::None)
    );
}

#[test]
fn applies_even_when_balloon_move_is_locked() {
    let mut w = script_world(&[]);
    let c = char_at(&w, 0);
    apply_script_balloon_lock(&mut w.world, 0, true);

    assert!(apply_script_balloon_offset(
        &mut w.world,
        0,
        abs(100, 30),
        true
    ));

    assert_eq!(balloon_at(&w, 0), shifted(c, -BALLOON_SIZE.w + 100, 30));
}

#[test]
fn other_scope_follow_is_unchanged() {
    let mut w = script_world(&[]);
    let kero = w.windows.char_window(1).unwrap();
    let follow = *w.world.get::<BalloonFollow>(kero).unwrap();
    let pos = balloon_at(&w, 1);

    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true);

    assert_eq!(*w.world.get::<BalloonFollow>(kero).unwrap(), follow);
    assert_eq!(balloon_at(&w, 1), pos);
    assert!(w.world.get::<ScriptBalloonPlacement>(kero).is_none());
}

#[test]
fn scope_without_char_window_warns_once_and_changes_nothing() {
    let mut w = script_world(&[]);
    let all = |w: &ScriptWorld| -> Vec<Point> {
        (0..2)
            .flat_map(|s| [char_at(w, s), balloon_at(w, s)])
            .collect()
    };
    let before = all(&w);

    let (applied, events) =
        capture(|| apply_script_balloon_offset(&mut w.world, 5, abs(100, 30), true));

    assert!(!applied);
    let warn = one_event_with(&events, "script_balloon_skipped");
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.field_str("reason"), Some("no_char_window"));
    assert_eq!(events_with(&events, "script_balloon_offset_set").len(), 0);
    assert_eq!(all(&w), before);
    let mut placed = w.world.query::<&ScriptBalloonPlacement>();
    assert_eq!(placed.iter(&w.world).count(), 0);
}

#[test]
fn dragging_balloon_takes_the_value_without_writing_position() {
    let mut w = script_world(&[]);
    let b = w.windows.balloon_window(0).unwrap();
    w.world.entity_mut(b).insert(WindowDragging);
    let pos = balloon_at(&w, 0);

    let (applied, events) =
        capture(|| apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true));

    assert!(applied);
    assert_eq!(balloon_at(&w, 0), pos);
    let skipped = one_event_with(&events, "script_balloon_reanchor_skipped");
    assert_eq!(skipped.field_str("reason"), Some("balloon_dragging"));
    let c = w.windows.char_window(0).unwrap();
    assert_eq!(
        w.world.get::<ScriptBalloonPlacement>(c).unwrap().offset,
        Some(abs(100, 30))
    );
}

#[test]
fn missing_default_basis_keeps_the_value_and_warns() {
    let mut w = script_world(&[]);
    w.world.remove_resource::<DefaultPlacementBasis>();
    let pos = balloon_at(&w, 0);

    let (applied, events) =
        capture(|| apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true));

    assert!(applied);
    assert_eq!(balloon_at(&w, 0), pos);
    let warn = one_event_with(&events, "script_balloon_place_deferred");
    assert_eq!(warn.field_str("reason"), Some("no_default_basis"));
    let c = w.windows.char_window(0).unwrap();
    assert_eq!(
        w.world.get::<ScriptBalloonPlacement>(c).unwrap().align,
        Some(ScriptAlign::Right)
    );
}

const SHIFT: PointPx = PointPx { x: 30, y: 20 };
const LANDED_W: i32 = 600;
const BALLOON_BEFORE: SizeI = SizeI {
    width: BALLOON_SIZE.w,
    height: BALLOON_SIZE.h,
};
const BALLOON_AFTER: SizeI = SizeI {
    width: 334,
    height: 237,
};

fn set_size(w: &mut ScriptWorld, e: Entity, width: i32, height: i32) {
    w.world.get_mut::<WindowPos>(e).unwrap().size = Some(SizeI { width, height });
}

fn land_sakura(w: &mut ScriptWorld) {
    let c = w.windows.char_window(0).unwrap();
    set_size(w, c, LANDED_W, CHAR_SIZES[0].h);
}

fn resize_sakura_balloon(w: &mut ScriptWorld) {
    let b = w.windows.balloon_window(0).unwrap();
    set_size(w, b, BALLOON_AFTER.width, BALLOON_AFTER.height);
}

fn follow_of(w: &ScriptWorld, scope: usize) -> PointPx {
    let c = w.windows.char_window(scope).unwrap();
    w.world.get::<BalloonFollow>(c).unwrap().offset()
}

/// sakura のバルーンを今の追従の値から `SHIFT` だけ動かした形にする。
fn shift_by_user(w: &mut ScriptWorld) {
    let c = w.windows.char_window(0).unwrap();
    let b = w.windows.balloon_window(0).unwrap();
    let base = follow_of(w, 0);
    let offset = PointPx {
        x: base.x + SHIFT.x,
        y: base.y + SHIFT.y,
    };
    w.world
        .entity_mut(c)
        .insert(BalloonFollow::new(b, OffsetBase { offset, dpi: None }));
    mark_user_moved(&mut w.world, c);
}

fn unsettled(w: &ScriptWorld) -> bool {
    let c = w.windows.char_window(0).unwrap();
    w.world.entity(c).contains::<ScriptBalloonUnsettled>()
}

#[test]
fn applying_before_landing_resolves_again_at_the_landed_size() {
    let mut w = script_world(&[]);
    let c = char_at(&w, 0);

    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, false);

    assert_eq!(balloon_at(&w, 0), shifted(c, CHAR_SIZES[0].w, 0));
    assert!(unsettled(&w));

    land_sakura(&mut w);
    resettle_script_balloons(&mut w.world, &|_| false);
    assert_eq!(balloon_at(&w, 0), shifted(c, CHAR_SIZES[0].w, 0));
    assert!(unsettled(&w), "着地していないスコープは印を残す");

    let (_, events) = capture(|| resettle_script_balloons(&mut w.world, &|s| s == 0));

    assert_eq!(balloon_at(&w, 0), shifted(c, LANDED_W, 0));
    assert!(!unsettled(&w));
    one_event_with(&events, "script_balloon_resettled");
}

#[test]
fn applying_after_landing_clears_the_unsettled_mark() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, false);
    assert!(unsettled(&w));

    apply_script_balloon_offset(&mut w.world, 0, abs(0, 0), true);

    assert!(!unsettled(&w));
}

#[test]
fn resettle_keeps_the_user_shift() {
    let mut w = script_world(&[]);
    shift_by_user(&mut w);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, false);
    assert_eq!(
        follow_of(&w, 0),
        PointPx {
            x: CHAR_SIZES[0].w + SHIFT.x,
            y: SHIFT.y,
        }
    );

    land_sakura(&mut w);
    resettle_script_balloons(&mut w.world, &|_| true);

    assert_eq!(
        follow_of(&w, 0),
        PointPx {
            x: LANDED_W + SHIFT.x,
            y: SHIFT.y,
        }
    );
    let c = w.windows.char_window(0).unwrap();
    assert!(w.world.entity(c).contains::<BalloonUserMoved>());
}

#[test]
fn balloon_size_replacement_resolves_that_scope_at_the_given_size() {
    let w = script_world(&[]);
    let basis = w.world.resource::<DefaultPlacementBasis>();

    let got: Vec<ScopePlacement> = resolve_scopes(
        &w.world,
        &w.windows,
        &basis.source,
        &basis.balloon,
        Layer::Script,
        Some((0, BALLOON_AFTER)),
    )
    .into_iter()
    .map(|s| s.placement)
    .collect();

    assert_eq!(
        got[0].balloon_size,
        SizePx {
            w: BALLOON_AFTER.width,
            h: BALLOON_AFTER.height,
        }
    );
    assert_eq!(
        got[0].balloon_offset,
        PointPx {
            x: -BALLOON_AFTER.width,
            y: 0,
        }
    );
    assert_eq!(got[1].balloon_size, BALLOON_SIZE);
}

#[test]
fn balloon_resize_keeps_a_left_balloon_touching_the_body() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    let c = char_at(&w, 0);
    resize_sakura_balloon(&mut w);

    let (_, events) =
        capture(|| reanchor_after_balloon_resize(&mut w.world, 0, BALLOON_BEFORE, BALLOON_AFTER));

    let b = balloon_at(&w, 0);
    assert_eq!(
        b.x + BALLOON_AFTER.width,
        c.x,
        "バルーンの右端が本体の左端に接する"
    );
    assert_eq!(b.y, c.y);
    one_event_with(&events, "script_balloon_reanchored");
}

#[test]
fn balloon_resize_keeps_a_center_balloon_centered() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Center, true);
    let c = char_at(&w, 0);
    resize_sakura_balloon(&mut w);

    reanchor_after_balloon_resize(&mut w.world, 0, BALLOON_BEFORE, BALLOON_AFTER);

    assert_eq!(
        balloon_at(&w, 0),
        shifted(
            c,
            (CHAR_SIZES[0].w - BALLOON_AFTER.width) / 2,
            -BALLOON_AFTER.height
        )
    );
}

#[test]
fn balloon_resize_keeps_the_user_shift() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    shift_by_user(&mut w);
    let c = char_at(&w, 0);
    resize_sakura_balloon(&mut w);

    reanchor_after_balloon_resize(&mut w.world, 0, BALLOON_BEFORE, BALLOON_AFTER);

    let expected = PointPx {
        x: -BALLOON_AFTER.width + SHIFT.x,
        y: SHIFT.y,
    };
    assert_eq!(follow_of(&w, 0), expected);
    assert_eq!(balloon_at(&w, 0), shifted(c, expected.x, expected.y));
    let char_window = w.windows.char_window(0).unwrap();
    assert!(w.world.entity(char_window).contains::<BalloonUserMoved>());
}

#[test]
fn balloon_resize_before_landing_keeps_the_user_shift_after_landing() {
    let mut w = script_world(&[]);
    shift_by_user(&mut w);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Center, false);
    resize_sakura_balloon(&mut w);
    reanchor_after_balloon_resize(&mut w.world, 0, BALLOON_BEFORE, BALLOON_AFTER);

    land_sakura(&mut w);
    resettle_script_balloons(&mut w.world, &|_| true);

    assert_eq!(
        follow_of(&w, 0),
        PointPx {
            x: (LANDED_W - BALLOON_AFTER.width) / 2 + SHIFT.x,
            y: -BALLOON_AFTER.height + SHIFT.y,
        }
    );
}

#[test]
fn balloon_resize_leaves_a_scope_without_script_value_untouched() {
    let mut w = script_world(&[]);
    let c = w.windows.char_window(0).unwrap();
    let follow = *w.world.get::<BalloonFollow>(c).unwrap();
    let pos = balloon_at(&w, 0);
    resize_sakura_balloon(&mut w);

    let (_, events) =
        capture(|| reanchor_after_balloon_resize(&mut w.world, 0, BALLOON_BEFORE, BALLOON_AFTER));

    assert_eq!(*w.world.get::<BalloonFollow>(c).unwrap(), follow);
    assert_eq!(balloon_at(&w, 0), pos);
    assert!(!w.world.entity(c).contains::<BalloonUserMoved>());
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn balloon_resize_while_dragging_does_not_move_the_balloon() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    let b = w.windows.balloon_window(0).unwrap();
    w.world.entity_mut(b).insert(WindowDragging);
    let follow = follow_of(&w, 0);
    let pos = balloon_at(&w, 0);
    resize_sakura_balloon(&mut w);

    let (_, events) =
        capture(|| reanchor_after_balloon_resize(&mut w.world, 0, BALLOON_BEFORE, BALLOON_AFTER));

    assert_eq!(balloon_at(&w, 0), pos);
    assert_eq!(follow_of(&w, 0), follow);
    let skipped = one_event_with(&events, "script_balloon_reanchor_skipped");
    assert_eq!(skipped.field_str("reason"), Some("balloon_dragging"));
    assert_eq!(events_with(&events, "script_balloon_reanchored").len(), 0);
}

#[test]
fn balloon_resize_without_default_basis_warns_and_does_not_move_the_balloon() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    w.world.remove_resource::<DefaultPlacementBasis>();
    let follow = follow_of(&w, 0);
    let pos = balloon_at(&w, 0);
    resize_sakura_balloon(&mut w);

    let (_, events) =
        capture(|| reanchor_after_balloon_resize(&mut w.world, 0, BALLOON_BEFORE, BALLOON_AFTER));

    assert_eq!(balloon_at(&w, 0), pos);
    assert_eq!(follow_of(&w, 0), follow);
    let warn = one_event_with(&events, "script_balloon_reanchor_skipped");
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.field_str("reason"), Some("no_default_basis"));
}

#[test]
fn resettle_that_cannot_resolve_drops_the_mark_and_keeps_the_provisional_position() {
    let mut w = script_world(&[]);
    let c = char_at(&w, 0);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, false);
    assert!(unsettled(&w));
    land_sakura(&mut w);
    w.world.remove_resource::<DefaultPlacementBasis>();

    let (_, events) = capture(|| resettle_script_balloons(&mut w.world, &|_| true));

    assert!(!unsettled(&w));
    assert_eq!(balloon_at(&w, 0), shifted(c, CHAR_SIZES[0].w, 0));
    let warn = one_event_with(&events, "script_balloon_resettle_skipped");
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.field_str("reason"), Some("no_default_basis"));
    assert_eq!(events_with(&events, "script_balloon_resettled").len(), 0);
}
