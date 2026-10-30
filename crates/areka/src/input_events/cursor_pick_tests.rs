//! 形の状態と選びの純関数の檻（shell-mouse-cursor 要件 2〜5・design「CursorPick」の 13 行の表と pick_shape の決まり）。

use std::cell::Cell;

use wintf::ecs::cursor::CursorTrigger;

use super::{Hold, Origin, Shown, Spot, WindowCursor, pick_shape, plan_cursor};
use crate::emo2_boot::shell_cursor_table::{
    CursorRows, CursorScene, GhostCursorKeys, Shape, SystemCursor,
};

use CursorScene::{Down, Hover, RightDown, Up, Wheel};
use CursorTrigger::{Move, Press, Refresh, Release};

const P: (i32, i32) = (10, 20);
const Q: (i32, i32) = (11, 20);

fn region(r: &str) -> Spot {
    Spot::Region(r.to_owned())
}

fn shown(scene: CursorScene, r: &str) -> Shown {
    Shown::Region {
        scene,
        region: r.to_owned(),
    }
}

fn pressed(scene: CursorScene, r: &str) -> Hold {
    Hold::Pressed {
        scene,
        region: r.to_owned(),
    }
}

fn wc(client: (i32, i32), spot: Spot, hold: Hold) -> WindowCursor {
    WindowCursor { client, spot, hold }
}

/// plan_cursor を回し、当たり判定を引いたかも返す。
fn plan(
    prev: Option<&WindowCursor>,
    trigger: CursorTrigger,
    client: (i32, i32),
    buttons_down: bool,
    now: Spot,
) -> (WindowCursor, Shown, bool) {
    let called = Cell::new(false);
    let (next, s) = plan_cursor(prev, trigger, client, buttons_down, || {
        called.set(true);
        now
    });
    (next, s, called.get())
}

// ---- plan_cursor: 表の 13 行 ----

#[test]
fn row01_move_same_position_keeps_everything_without_resolving() {
    for prev in [
        wc(P, region("head"), Hold::Free),
        wc(P, region("head"), pressed(Down, "head")),
        wc(P, region("head"), Hold::Resting(Wheel)),
        wc(P, Spot::Plain, Hold::Free),
    ] {
        let (next, s, called) = plan(Some(&prev), Move, P, false, region("bust"));
        assert!(!called, "{prev:?}");
        assert_eq!(next, prev);
        let (_, want, _) = plan(Some(&prev), Refresh, P, false, Spot::Plain);
        assert_eq!(s, want);
    }
}

#[test]
fn row02_move_while_pressed_and_button_down_keeps_pressed_scene_and_region() {
    let prev = wc(P, region("head"), pressed(Down, "head"));
    let (next, s, called) = plan(Some(&prev), Move, Q, true, Spot::Plain);
    assert!(called);
    assert_eq!(next, wc(Q, Spot::Plain, pressed(Down, "head")));
    assert_eq!(s, shown(Down, "head"));
}

#[test]
fn row03_move_while_pressed_without_button_returns_to_free() {
    // 解放を落としたとき（メニューが食べた・窓の外で放した）の戻り（design・要件 3.4）。
    let prev = wc(P, region("head"), pressed(Down, "head"));
    let (next, s, called) = plan(Some(&prev), Move, Q, false, region("bust"));
    assert!(called);
    assert_eq!(next, wc(Q, region("bust"), Hold::Free));
    assert_eq!(s, shown(Up, "bust"));
}

#[test]
fn row04_move_from_free_or_resting_resolves_and_frees() {
    for hold in [Hold::Free, Hold::Resting(Hover), Hold::Resting(Wheel)] {
        let prev = wc(P, region("head"), hold);
        let (next, s, called) = plan(Some(&prev), Move, Q, false, region("head"));
        assert!(called);
        assert_eq!(next, wc(Q, region("head"), Hold::Free));
        assert_eq!(s, shown(Up, "head"), "要件 4.4");
    }
}

#[test]
fn row05_press_over_region_holds_down_or_right_down() {
    for (right, scene) in [(false, Down), (true, RightDown)] {
        for hold in [Hold::Free, Hold::Resting(Hover), Hold::Resting(Wheel)] {
            let prev = wc(P, region("head"), hold);
            let (next, s, called) = plan(Some(&prev), Press { right }, P, true, region("head"));
            assert!(called);
            assert_eq!(next, wc(P, region("head"), pressed(scene, "head")));
            assert_eq!(s, shown(scene, "head"));
        }
    }
}

#[test]
fn row06_press_off_region_keeps_the_place_shape() {
    // 要件 3.5: 当たり判定の無い所で押しても形を変えない。
    let prev = wc(P, Spot::Plain, Hold::Free);
    let (next, s, called) = plan(Some(&prev), Press { right: false }, P, true, Spot::Plain);
    assert!(called);
    assert_eq!(next, wc(P, Spot::Plain, Hold::Free));
    assert_eq!(s, Shown::Arrow);
    let (next, s, _) = plan(
        Some(&prev),
        Press { right: true },
        P,
        true,
        Spot::SeeThrough,
    );
    assert_eq!(next.hold, Hold::Free);
    assert_eq!(s, Shown::Pass);
}

#[test]
fn row07_second_press_keeps_the_first() {
    let prev = wc(P, region("head"), pressed(Down, "head"));
    let (next, s, called) = plan(Some(&prev), Press { right: true }, Q, true, region("bust"));
    assert!(called);
    assert_eq!(next, wc(Q, region("bust"), pressed(Down, "head")));
    assert_eq!(s, shown(Down, "head"));
}

#[test]
fn row08_release_resolves_and_frees_from_any_hold() {
    for hold in [
        Hold::Free,
        pressed(Down, "head"),
        pressed(RightDown, "head"),
        Hold::Resting(Hover),
    ] {
        let prev = wc(P, region("head"), hold);
        let (next, s, called) = plan(Some(&prev), Release, P, false, region("bust"));
        assert!(called);
        assert_eq!(next, wc(P, region("bust"), Hold::Free));
        assert_eq!(s, shown(Up, "bust"), "要件 3.4");
    }
    let prev = wc(P, region("head"), pressed(Down, "head"));
    let (_, s, _) = plan(Some(&prev), Release, P, false, Spot::Plain);
    assert_eq!(s, Shown::Arrow);
}

#[test]
fn row09_hover_from_free_over_region_rests_without_resolving() {
    let prev = wc(P, region("head"), Hold::Free);
    let (next, s, called) = plan(Some(&prev), CursorTrigger::Hover, P, false, Spot::Plain);
    assert!(!called);
    assert_eq!(next, wc(P, region("head"), Hold::Resting(Hover)));
    assert_eq!(s, shown(Hover, "head"));
}

#[test]
fn row10_hover_is_ignored_when_resting_pressed_or_off_region() {
    for prev in [
        // ホイールの形を止まったで上書きしない。
        wc(P, region("head"), Hold::Resting(Wheel)),
        wc(P, region("head"), Hold::Resting(Hover)),
        // 要件 4.5: 押している間は受けない。
        wc(P, region("head"), pressed(Down, "head")),
        wc(P, Spot::Plain, Hold::Free),
        wc(P, Spot::SeeThrough, Hold::Free),
    ] {
        let (next, s, called) = plan(Some(&prev), CursorTrigger::Hover, P, false, region("x"));
        assert!(!called);
        assert_eq!(next, prev);
        let (_, want, _) = plan(Some(&prev), Refresh, P, false, Spot::Plain);
        assert_eq!(s, want);
    }
}

#[test]
fn row11_wheel_over_region_rests_with_wheel() {
    for hold in [Hold::Free, Hold::Resting(Hover), Hold::Resting(Wheel)] {
        let prev = wc(P, region("head"), hold);
        let (next, s, called) = plan(Some(&prev), CursorTrigger::Wheel, P, false, region("head"));
        assert!(called);
        assert_eq!(next, wc(P, region("head"), Hold::Resting(Wheel)));
        assert_eq!(s, shown(Wheel, "head"));
    }
}

#[test]
fn row12_wheel_while_pressed_or_off_region_keeps_hold() {
    let prev = wc(P, region("head"), pressed(Down, "head"));
    let (next, s, called) = plan(Some(&prev), CursorTrigger::Wheel, P, true, region("head"));
    assert!(called);
    assert_eq!(next, prev);
    assert_eq!(s, shown(Down, "head"), "要件 4.5");
    let prev = wc(P, Spot::Plain, Hold::Free);
    let (next, s, called) = plan(Some(&prev), CursorTrigger::Wheel, P, false, Spot::Plain);
    assert!(called);
    assert_eq!(next, prev);
    assert_eq!(s, Shown::Arrow);
}

#[test]
fn row13_refresh_keeps_state_and_resolves_only_without_prev() {
    for (prev, want) in [
        (wc(P, region("head"), Hold::Free), shown(Up, "head")),
        (
            wc(P, region("head"), Hold::Resting(Wheel)),
            shown(Wheel, "head"),
        ),
        (
            wc(Q, Spot::SeeThrough, pressed(Down, "bust")),
            shown(Down, "bust"),
        ),
        (wc(P, Spot::Plain, Hold::Free), Shown::Arrow),
        (wc(P, Spot::SeeThrough, Hold::Free), Shown::Pass),
    ] {
        let (next, s, called) = plan(Some(&prev), Refresh, P, false, region("x"));
        assert!(!called);
        assert_eq!(next, prev);
        assert_eq!(s, want);
    }
    let (next, s, called) = plan(None, Refresh, P, false, region("head"));
    assert!(called);
    assert_eq!(next, wc(P, region("head"), Hold::Free));
    assert_eq!(s, shown(Up, "head"));
}

// ---- plan_cursor: 道すじ ----

#[test]
fn first_sighting_resolves_for_every_trigger() {
    for t in [
        Move,
        Press { right: false },
        Release,
        CursorTrigger::Hover,
        CursorTrigger::Wheel,
        Refresh,
    ] {
        let (_, _, called) = plan(None, t, P, false, region("head"));
        assert!(called, "{t:?}");
    }
    let (next, s, _) = plan(None, Press { right: false }, P, true, region("head"));
    assert_eq!(next.hold, pressed(Down, "head"));
    assert_eq!(s, shown(Down, "head"));
    let (next, _, _) = plan(None, CursorTrigger::Hover, P, false, region("head"));
    assert_eq!(next.hold, Hold::Resting(Hover));
}

#[test]
fn walk_region_to_region_to_plain_to_see_through() {
    // 要件 2.1・2.3・2.4・2.5: 移ったその場で替わる。
    let (a, s, _) = plan(None, Move, (0, 0), false, region("head"));
    assert_eq!(s, shown(Up, "head"));
    let (b, s, _) = plan(Some(&a), Move, (1, 0), false, region("face"));
    assert_eq!(s, shown(Up, "face"));
    let (c, s, _) = plan(Some(&b), Move, (2, 0), false, Spot::Plain);
    assert_eq!(s, Shown::Arrow);
    let (_, s, _) = plan(Some(&c), Move, (3, 0), false, Spot::SeeThrough);
    assert_eq!(s, Shown::Pass);
}

#[test]
fn walk_press_drag_out_to_see_through_then_release() {
    // 要件 3.3・3.4: 押したまま外へ・透けた所へ出ても押したときの形、放せばその位置の形。
    let (a, _, _) = plan(None, Move, (0, 0), false, region("bust"));
    let (b, s, _) = plan(
        Some(&a),
        Press { right: false },
        (0, 0),
        true,
        region("bust"),
    );
    assert_eq!(s, shown(Down, "bust"));
    let (c, s, _) = plan(Some(&b), Move, (5, 0), true, Spot::Plain);
    assert_eq!(s, shown(Down, "bust"));
    let (d, s, _) = plan(Some(&c), Move, (9, 0), true, Spot::SeeThrough);
    assert_eq!(s, shown(Down, "bust"));
    let (e, s, _) = plan(Some(&d), Release, (9, 0), false, Spot::SeeThrough);
    assert_eq!(e.hold, Hold::Free);
    assert_eq!(s, Shown::Pass);
}

#[test]
fn surface_change_without_motion_waits_for_the_next_move() {
    // 要件 2.6: 動かない間は前の当たり判定のまま、動いた時点で引き直す。
    let (a, _, _) = plan(None, Move, P, false, region("head"));
    let (b, s, called) = plan(Some(&a), Move, P, false, region("face"));
    assert!(!called);
    assert_eq!(s, shown(Up, "head"));
    let (_, s, called) = plan(Some(&b), Move, Q, false, region("face"));
    assert!(called);
    assert_eq!(s, shown(Up, "face"));
}

// ---- pick_shape ----

fn rows(list: &[(u32, &str, CursorScene, Shape<String>)]) -> CursorRows<String> {
    CursorRows {
        rows: list
            .iter()
            .map(|(s, r, c, shape)| ((*s, (*r).to_owned(), *c), shape.clone()))
            .collect(),
    }
}

fn file(name: &str) -> Shape<String> {
    Shape::File(name.to_owned())
}

fn sys(c: SystemCursor) -> Shape<String> {
    Shape::System(c)
}

fn no_ghost() -> GhostCursorKeys<String> {
    GhostCursorKeys::default()
}

fn ghost_all() -> GhostCursorKeys<String> {
    GhostCursorKeys {
        finger: Some(file("finger.cur")),
        hand: Some(file("hand.cur")),
        grip: Some(file("grip.cur")),
        arrow: Some(file("arrow.cur")),
        unused: Vec::new(),
    }
}

fn pick(
    scope: u32,
    s: &Shown,
    r: &CursorRows<String>,
    g: &GhostCursorKeys<String>,
) -> Option<(Shape<String>, Origin)> {
    pick_shape(scope, s, r, g)
}

#[test]
fn defaults_by_scene_and_bust_case_insensitive() {
    let r = rows(&[]);
    let g = no_ghost();
    for bust in ["bust", "Bust", "BUST"] {
        assert_eq!(
            pick(0, &shown(Up, bust), &r, &g),
            Some((sys(SystemCursor::Hand), Origin::Default))
        );
        assert_eq!(
            pick(0, &shown(Down, bust), &r, &g),
            Some((sys(SystemCursor::Grip), Origin::Default))
        );
        assert_eq!(
            pick(0, &shown(RightDown, bust), &r, &g),
            Some((sys(SystemCursor::Finger), Origin::Default))
        );
    }
    for scene in [Up, Down, RightDown] {
        assert_eq!(
            pick(0, &shown(scene, "head"), &r, &g),
            Some((sys(SystemCursor::Finger), Origin::Default))
        );
    }
    assert_eq!(
        pick(0, &Shown::Arrow, &r, &g),
        Some((sys(SystemCursor::Arrow), Origin::Default))
    );
    assert_eq!(pick(0, &Shown::Pass, &r, &g), None);
}

#[test]
fn ghost_descript_replaces_defaults() {
    // 要件 5.1〜5.4。
    let r = rows(&[]);
    let g = ghost_all();
    let gd = |n: &str| Some((file(n), Origin::GhostDescript));
    assert_eq!(pick(0, &shown(Up, "Bust"), &r, &g), gd("hand.cur"));
    assert_eq!(pick(0, &shown(Down, "bust"), &r, &g), gd("grip.cur"));
    assert_eq!(pick(0, &shown(RightDown, "bust"), &r, &g), gd("finger.cur"));
    assert_eq!(pick(0, &shown(Up, "head"), &r, &g), gd("finger.cur"));
    assert_eq!(pick(0, &shown(Down, "head"), &r, &g), gd("finger.cur"));
    assert_eq!(pick(0, &Shown::Arrow, &r, &g), gd("arrow.cur"));
    assert_eq!(pick(0, &Shown::Pass, &r, &g), None);
}

#[test]
fn brace_row_beats_ghost_and_default() {
    // 要件 5.5: 行がある場面は行の形。
    let r = rows(&[
        (0, "bust", Up, sys(SystemCursor::Help)),
        (0, "bust", Down, file("d.cur")),
        (0, "head", RightDown, sys(SystemCursor::No)),
    ]);
    for g in [no_ghost(), ghost_all()] {
        assert_eq!(
            pick(0, &shown(Up, "bust"), &r, &g),
            Some((sys(SystemCursor::Help), Origin::Brace))
        );
        assert_eq!(
            pick(0, &shown(Down, "bust"), &r, &g),
            Some((file("d.cur"), Origin::Brace))
        );
        assert_eq!(
            pick(0, &shown(RightDown, "head"), &r, &g),
            Some((sys(SystemCursor::No), Origin::Brace))
        );
    }
    // 行は綴りどおり（Bust の行は無い）なので既定へ落ちる。
    assert_eq!(
        pick(0, &shown(Up, "Bust"), &r, &no_ghost()),
        Some((sys(SystemCursor::Hand), Origin::Default))
    );
}

#[test]
fn hover_and_wheel_use_their_row_or_stay_on_the_up_shape() {
    // 要件 4.1・4.3。
    let r = rows(&[
        (0, "head", Hover, sys(SystemCursor::Help)),
        (0, "head", Wheel, sys(SystemCursor::Move)),
        (0, "face", Up, sys(SystemCursor::Cross)),
    ]);
    let g = no_ghost();
    assert_eq!(
        pick(0, &shown(Hover, "head"), &r, &g),
        Some((sys(SystemCursor::Help), Origin::Brace))
    );
    assert_eq!(
        pick(0, &shown(Wheel, "head"), &r, &g),
        Some((sys(SystemCursor::Move), Origin::Brace))
    );
    for scene in [Hover, Wheel] {
        assert_eq!(
            pick(0, &shown(scene, "face"), &r, &g),
            Some((sys(SystemCursor::Cross), Origin::Brace))
        );
        assert_eq!(
            pick(0, &shown(scene, "bust"), &r, &g),
            Some((sys(SystemCursor::Hand), Origin::Default))
        );
        assert_eq!(
            pick(0, &shown(scene, "bust"), &r, &ghost_all()),
            Some((file("hand.cur"), Origin::GhostDescript))
        );
        assert_eq!(
            pick(0, &shown(scene, "arm"), &r, &g),
            Some((sys(SystemCursor::Finger), Origin::Default))
        );
    }
}

#[test]
fn scopes_are_separate() {
    // 要件 2.7: スコープ 0 の行はスコープ 1 に効かない。
    let r = rows(&[(0, "head", Up, sys(SystemCursor::Help))]);
    let g = no_ghost();
    assert_eq!(
        pick(0, &shown(Up, "head"), &r, &g),
        Some((sys(SystemCursor::Help), Origin::Brace))
    );
    assert_eq!(
        pick(1, &shown(Up, "head"), &r, &g),
        Some((sys(SystemCursor::Finger), Origin::Default))
    );
}
