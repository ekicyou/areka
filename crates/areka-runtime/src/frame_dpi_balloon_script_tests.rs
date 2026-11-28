//! バルーン窓だけの拡大率が変わり、DPI の相が窓の寸を書き換えたとき、台本のバルーンの位置が保たれる檻。
//!
//! 作業領域は 2 つの画面（本体の居る画面と、その右隣の画面）。本体の拡大率は変えない。

use std::collections::BTreeMap;

use areka_placement::diag::WindowKind;
use areka_placement::follow::{BalloonFollow, on_balloon_drag};
use areka_placement::reset_balloon_pos::{BalloonResetTrigger, reset_balloon_positions};
use areka_placement::script_balloon::{
    OffsetAxis, ScriptAlign, ScriptOffset, apply_script_balloon_align, apply_script_balloon_offset,
};
use areka_placement::spawn::BalloonLimit;
use wintf::ecs::drag::DragEvent;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{DPI, Point, SizeI, WindowPos};

use super::balloon_target;
use super::shared_test_support::steady_harness;
use super::test_support::{
    FakeReports, FrameHarness, capture_logs, placement_basis, pos_of, s2_neighbor_work_area,
    size_of,
};
use super::transition_test_support::{
    assert_writes_per_window_within_bound, neighbor_at, writes_for,
};

/// 隣の画面へ移ったバルーン窓の拡大率。
const BALLOON_DPI: u16 = 144;

/// 拡大率の変化で書き換わるバルーン窓の寸（窓を作ったときは 223×158）。
const RESIZED: (u32, u32) = (335, 237);

const SHIFT: Point = Point { x: 30, y: 20 };

fn script_harness() -> FrameHarness {
    let mut harness = steady_harness();
    harness.set_work_area_source_for_dpi(96);
    harness
        .world
        .insert_resource(placement_basis(BTreeMap::new(), 2));
    reset_balloon_positions(&mut harness.world, BalloonResetTrigger::Script);
    // 初回の全窓の一致を先に済ませ、以後は拡大率を変えた窓だけが DPI の相に入る。
    harness.run_dpi_phase(&mut FakeReports::default());
    let _ = harness.drain_writes();
    harness
}

fn align(harness: &mut FrameHarness, align: ScriptAlign) {
    assert!(apply_script_balloon_align(
        &mut harness.world,
        0,
        align,
        true
    ));
}

/// スコープ 0 のバルーン窓だけ拡大率を変え、DPI の相が窓の寸を `size` へ書き換える巡を 1 回回す。
fn resize_balloon_by_dpi(harness: &mut FrameHarness, size: (u32, u32)) {
    let balloon = harness.balloon_window(0);
    harness
        .world
        .entity_mut(balloon)
        .insert(DPI::from_dpi(BALLOON_DPI, BALLOON_DPI));
    let mut source = FakeReports::default();
    source.refresh.insert(balloon_target(0).0, size);
    harness.advance_frame();
    harness.run_dpi_phase(&mut source);
    assert_eq!(
        size_of(&harness.world, balloon),
        Some(SizeI::new(size.0 as i32, size.1 as i32)),
        "前提: DPI の相がバルーン窓の寸を書き換えた"
    );
}

/// 利用者がバルーンを今の位置から `SHIFT` だけ動かした形にする（単独ドラッグのハンドラを本番の引数で呼ぶ）。
fn drag_balloon_by_shift(harness: &mut FrameHarness) {
    let balloon = harness.balloon_window(0);
    let at = balloon_at(harness);
    harness
        .world
        .get_mut::<WindowPos>(balloon)
        .expect("バルーン窓に WindowPos がある")
        .position = Some(Point {
        x: at.x + SHIFT.x,
        y: at.y + SHIFT.y,
    });
    let ev = Phase::Bubble(DragEvent {
        target: balloon,
        start_position: Point::new(0, 0),
        position: Point::new(0, 0),
        timestamp: std::time::Instant::now(),
    });
    assert!(!on_balloon_drag(&mut harness.world, balloon, balloon, &ev));
}

fn char_at(harness: &FrameHarness) -> Point {
    pos_of(&harness.world, harness.char_window(0)).expect("キャラ窓の位置がある")
}

fn char_width(harness: &FrameHarness) -> i32 {
    size_of(&harness.world, harness.char_window(0))
        .expect("キャラ窓の寸がある")
        .width
}

fn balloon_at(harness: &FrameHarness) -> Point {
    pos_of(&harness.world, harness.balloon_window(0)).expect("バルーンの位置がある")
}

fn follow_of(harness: &FrameHarness) -> BalloonFollow {
    *harness
        .world
        .get::<BalloonFollow>(harness.char_window(0))
        .expect("キャラ窓に BalloonFollow がある")
}

#[test]
fn a_left_balloon_keeps_touching_the_body_after_a_dpi_resize() {
    let mut harness = script_harness();
    align(&mut harness, ScriptAlign::Left);
    let c = char_at(&harness);

    resize_balloon_by_dpi(&mut harness, RESIZED);

    let b = balloon_at(&harness);
    assert_eq!(
        b.x + RESIZED.0 as i32,
        c.x,
        "バルーンの右端が本体の左端に接する"
    );
    assert_eq!(b.y, c.y);
}

#[test]
fn a_center_balloon_stays_centered_after_a_dpi_resize() {
    let mut harness = script_harness();
    align(&mut harness, ScriptAlign::Center);
    let c = char_at(&harness);

    resize_balloon_by_dpi(&mut harness, RESIZED);

    assert_eq!(
        balloon_at(&harness),
        Point {
            x: c.x + (char_width(&harness) - RESIZED.0 as i32) / 2,
            y: c.y - RESIZED.1 as i32,
        }
    );
}

#[test]
fn the_user_shift_remains_after_a_dpi_resize() {
    let mut harness = script_harness();
    align(&mut harness, ScriptAlign::Left);
    drag_balloon_by_shift(&mut harness);
    let c = char_at(&harness);

    resize_balloon_by_dpi(&mut harness, RESIZED);

    assert_eq!(
        balloon_at(&harness),
        Point {
            x: c.x - RESIZED.0 as i32 + SHIFT.x,
            y: c.y + SHIFT.y,
        }
    );
}

/// 本体の右から 1000 px 離したバルーンは隣の画面に居る。寸が広がって隣の画面の右端からはみ出すと、
/// 隣の画面の中へ押し込まれ、当て直しても同じ画面を選んで止まる。
#[test]
fn a_pushed_in_balloon_keeps_its_monitor_when_applied_again() {
    let mut harness = script_harness();
    let balloon = harness.balloon_window(0);
    harness.world.entity_mut(balloon).insert(BalloonLimit(true));
    let away = ScriptOffset {
        x: OffsetAxis::Absolute(-1000),
        y: OffsetAxis::Absolute(0),
    };
    align(&mut harness, ScriptAlign::Right);
    assert!(apply_script_balloon_offset(
        &mut harness.world,
        0,
        away,
        true
    ));
    let neighbor = s2_neighbor_work_area();
    assert!(
        balloon_at(&harness).x >= neighbor.left,
        "前提: バルーンが隣の画面に居る"
    );
    let wide = (1000, 200);

    let logs = capture_logs(|| resize_balloon_by_dpi(&mut harness, wide));

    let pushed = Point {
        x: neighbor.right - wide.0 as i32,
        y: char_at(&harness).y,
    };
    assert_eq!(balloon_at(&harness), pushed, "隣の画面の右端へ押し込まれる");
    let clamps: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("[balloon-limit] Clamp"))
        .collect();
    assert!(!clamps.is_empty(), "前提: 押し込みが起きた");
    assert!(
        logs.iter().any(|l| l.contains("script_balloon_reanchored")),
        "前提: 寸の変化で据え直した"
    );
    assert!(
        clamps.iter().all(|l| l.contains("monitor=\"balloon\"")),
        "{clamps:?}"
    );
    let _ = harness.drain_writes();

    assert!(apply_script_balloon_offset(
        &mut harness.world,
        0,
        away,
        true
    ));
    harness.advance_frame();
    harness.run_dpi_phase(&mut FakeReports::default());

    assert_eq!(balloon_at(&harness), pushed, "当て直しで選ぶ画面が変わった");
    assert_eq!(
        size_of(&harness.world, balloon),
        Some(SizeI::new(wide.0 as i32, wide.1 as i32))
    );
}

#[test]
fn a_scope_without_script_value_only_resizes_in_place() {
    let mut harness = script_harness();
    let follow = follow_of(&harness);
    let before = balloon_at(&harness);

    let logs = capture_logs(|| resize_balloon_by_dpi(&mut harness, RESIZED));

    assert_eq!(balloon_at(&harness), before);
    assert_eq!(follow_of(&harness), follow);
    let writes = harness.drain_writes();
    assert_eq!(
        writes_for(&writes, 0, WindowKind::Balloon).len(),
        1,
        "寸の書き換えの 1 本だけ"
    );
    assert!(
        !logs.iter().any(|l| l.contains("script_balloon")),
        "{logs:?}"
    );
}

#[test]
fn a_dpi_resize_on_the_other_monitor_writes_the_balloon_once() {
    let mut harness = script_harness();
    harness.set_monitor_table(neighbor_at(BALLOON_DPI));
    harness.advance_frame();
    harness.run_placement_phases(&mut FakeReports::default());
    align(&mut harness, ScriptAlign::Center);
    assert!(apply_script_balloon_offset(
        &mut harness.world,
        0,
        ScriptOffset {
            x: OffsetAxis::Absolute(1500),
            y: OffsetAxis::Absolute(0),
        },
        true
    ));
    let neighbor = s2_neighbor_work_area();
    let before = balloon_at(&harness);
    assert!(before.x >= neighbor.left, "前提: バルーンが隣の画面に居る");
    let _ = harness.drain_writes();

    let balloon = harness.balloon_window(0);
    harness
        .world
        .entity_mut(balloon)
        .insert(DPI::from_dpi(BALLOON_DPI, BALLOON_DPI));
    let mut source = FakeReports::default();
    source.refresh.insert(balloon_target(0).0, RESIZED);
    let logs = capture_logs(|| {
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        harness.run_reconcile(&mut source);
    });
    let writes = harness.drain_writes();

    assert!(
        logs.iter().any(|l| l.contains("script_balloon_reanchored")),
        "前提: 寸の変化で据え直した: {logs:?}"
    );
    assert_ne!(balloon_at(&harness), before, "前提: 据え直しで位置が動いた");
    assert_writes_per_window_within_bound(&writes, 0, WindowKind::Balloon, "隣の画面のバルーン");
    let write = writes_for(&writes, 0, WindowKind::Balloon)
        .pop()
        .expect("バルーン窓の書き込みがある");
    let at = balloon_at(&harness);
    assert_eq!(
        (write.x, write.y, write.width, write.height),
        (at.x, at.y, RESIZED.0 as i32, RESIZED.1 as i32),
        "畳まれた 1 本が据え直しの位置と新しい寸を持つ"
    );
}

/// DPI の相がバルーンを書き直さなかった巡（拡大率が変わったときバルーンが出ていなかった）は、変わった寸が drain の後段の報告の回収で窓へ着く。
#[test]
fn a_left_balloon_keeps_touching_the_body_when_the_resize_lands_in_the_reported_size_reconcile() {
    let mut harness = script_harness();
    align(&mut harness, ScriptAlign::Left);
    let c = char_at(&harness);
    let balloon = harness.balloon_window(0);
    harness
        .world
        .entity_mut(balloon)
        .insert(DPI::from_dpi(BALLOON_DPI, BALLOON_DPI));
    let mut source = FakeReports::default();
    source.pending.insert(balloon_target(0).0, RESIZED);
    harness.advance_frame();
    harness.run_dpi_phase(&mut source);
    assert_eq!(
        source.pending.get(&balloon_target(0).0),
        Some(&RESIZED),
        "前提: DPI の相は報告を取らない"
    );

    let logs = capture_logs(|| harness.run_reconcile(&mut source));

    assert_eq!(
        size_of(&harness.world, balloon),
        Some(SizeI::new(RESIZED.0 as i32, RESIZED.1 as i32)),
        "前提: 報告の回収がバルーン窓の寸を書き換えた"
    );
    let b = balloon_at(&harness);
    assert_eq!(
        b.x + RESIZED.0 as i32,
        c.x,
        "バルーンの右端が本体の左端に接する"
    );
    assert_eq!(b.y, c.y);
    assert!(
        logs.iter().any(|l| l.contains("script_balloon_reanchored")),
        "{logs:?}"
    );
}
