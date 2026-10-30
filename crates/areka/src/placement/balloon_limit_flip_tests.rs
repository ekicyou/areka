//! バルーンの反対の側への並べ替えの檻（window-work-area-clamp task 5.1・design「BalloonFlip」・
//! Testing ⑾・要件 5.3〜5.8・7.2・7.8）。
//!
//! 純関数 [`place_balloon_in_work_area`] の判断分岐と、それを呼ぶ 2 つの関門（起動の関門
//! [`apply_balloon_limit`]・書き込みの関門 `apply_balloon_limit_gate`＝`move_window_to` の随伴）と、
//! 並べ替えない解放の補正（`on_balloon_drag_end`）を固定する。
//!
//! 押し込みに倒れる場面の期待値は、檻の側で [`limit_correction`] を呼んで作る
//! （「今の押し込みと同じ位置」が主張そのもの）。並べ替えの期待値は檻の側の数で書く。

use std::collections::BTreeSet;
use std::time::Instant;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::drag::DragEndEvent;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, SizeI, WindowHandle, WindowPos};

use super::super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};
use super::*;
use crate::placement::balloon_lock::{apply_script_balloon_lock, seed_balloon_lock};
use crate::placement::follow::{OffsetBase, move_window_to, on_balloon_drag_end};
use crate::placement::resolver::Anchor;
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;

const CLAMP_TAG: &str = "[balloon-limit] Clamp";

/// 開発者の机の主の画面の作業領域（4.4 の 1 回目と同じ形）。
const WA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 2880,
    bottom: 1704,
};
/// キャラ窓の寸（左端に置くと 0〜1108）。
const CSZ: SizePx = SizePx { w: 1108, h: 800 };
/// バルーンの寸（幅 872）。
const BSZ: SizePx = SizePx { w: 872, h: 400 };

fn point(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

fn rect_of(pos: PointPx, size: SizePx) -> RectPx {
    RectPx {
        left: pos.x,
        top: pos.y,
        right: pos.x + size.w,
        bottom: pos.y + size.h,
    }
}

fn snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![WA],
    }
}

/// 左端の窓（0〜1108・y=904）と、その左隣のバルーン（ずらし (−872, 0)）。
fn left_edge_char() -> PointPx {
    point(0, 904)
}
fn left_side_offset() -> PointPx {
    point(-872, 0)
}

fn placement(scope: usize, char_pos: PointPx, offset: PointPx, limit: bool) -> ScopePlacement {
    ScopePlacement {
        scope,
        char_pos,
        char_size: CSZ,
        balloon_pos: point(char_pos.x + offset.x, char_pos.y + offset.y),
        balloon_size: BSZ,
        balloon_offset: offset,
        balloon_offset_base: OffsetBase::unpinned(offset),
        balloon_limit: limit,
        anchor: Anchor::Free,
        balloon_keyword_base: None,
    }
}

// ---------------------------------------------------------------------------
// 純関数（要件 5.3〜5.5）
// ---------------------------------------------------------------------------

/// 左端の窓の左隣で外へ出るバルーンは右隣（x=1108）へ映され、y は変わらない。
#[test]
fn left_side_balloon_of_left_edge_window_flips_to_the_right_keeping_y() {
    let pos = point(-872, 904);
    let got = place_balloon_in_work_area(pos, BSZ, rect_of(left_edge_char(), CSZ), WA);
    assert_eq!(got, BalloonPlace::Flipped(point(1108, 904)));
    assert_eq!(got.how(), "flipped");
}

/// 右端の窓の右隣で外へ出るバルーンは左隣へ映される（シェルとの間合いは左右で同じ）。
#[test]
fn right_side_balloon_of_right_edge_window_flips_to_the_left() {
    let char_pos = point(2880 - 1108, 300);
    // 右隣から 20px 離れたバルーン（間合い 20）→ 左隣でも間合い 20。
    let pos = point(2880 + 20, 300);
    let got = place_balloon_in_work_area(pos, BSZ, rect_of(char_pos, CSZ), WA);
    assert_eq!(got, BalloonPlace::Flipped(point(1772 - 20 - 872, 300)));
}

/// 映した位置が縦にだけはみ出すときは、横の位置を保って縦だけ収める。
#[test]
fn flipped_position_overflowing_only_vertically_is_clamped_vertically() {
    let pos = point(-872, 1504); // 下端 1904 > 1704
    let got = place_balloon_in_work_area(pos, BSZ, rect_of(left_edge_char(), CSZ), WA);
    assert_eq!(got, BalloonPlace::Flipped(point(1108, 1704 - 400)));
}

/// 押し込みに倒れる場面は、どれも今の押し込み（`limit_correction`）と同じ位置。
#[test]
fn fallback_cases_return_the_current_push_in() {
    let narrow = RectPx { right: 1500, ..WA };
    let cases: [(&str, PointPx, RectPx, RectPx); 4] = [
        // 作業領域が窓とバルーン 2 つ分より狭く、両方の側に入らない。
        (
            "both-sides",
            point(-872, 904),
            rect_of(left_edge_char(), CSZ),
            narrow,
        ),
        // 縦だけはみ出す。
        (
            "vertical-only",
            point(1108, 1504),
            rect_of(left_edge_char(), CSZ),
            WA,
        ),
        // 真上の中央揃え（バルーンがキャラ窓より広い）: 映しても x が変わらない。
        (
            "centered",
            point(-100, 100),
            rect_of(point(0, 500), SizePx { w: 672, h: 800 }),
            WA,
        ),
        // キャラ窓が作業領域からはみ出している。
        (
            "char-overflows",
            point(-200 - 872, 904),
            rect_of(point(-200, 904), CSZ),
            WA,
        ),
    ];
    for (label, pos, char_rect, area) in cases {
        let pushed = limit_correction(pos, BSZ, area).expect("探針: はみ出している");
        let got = place_balloon_in_work_area(pos, BSZ, char_rect, area);
        assert_eq!(got, BalloonPlace::Pushed(pushed), "{label}");
        assert_eq!(got.how(), "pushed", "{label}");
    }
}

/// 収まっていれば動かさない（`limit_correction` の `None` と同じ）。
#[test]
fn fitting_balloon_is_left_alone() {
    let got = place_balloon_in_work_area(point(1108, 904), BSZ, rect_of(left_edge_char(), CSZ), WA);
    assert_eq!(got, BalloonPlace::Fits);
}

// ---------------------------------------------------------------------------
// 起動の関門（要件 5.3・5.6・5.7・5.8）
// ---------------------------------------------------------------------------

/// 相方の形（スコープ 1）: 右隣へ並べ替え、`balloon_offset` は変えず、`how="flipped"` を残す。
/// 対照: 同じ捕捉の中の押し込みの場面（キャラ窓がはみ出している）は `how="pushed"`。
#[test]
fn boot_gate_flips_keeps_offset_and_logs_how() {
    let flip = placement(1, left_edge_char(), left_side_offset(), true);
    let push = placement(0, point(-200, 100), left_side_offset(), true);
    let (out, events) = capture_logs(|| apply_balloon_limit(vec![flip, push], &snapshot()));

    assert_eq!(out[0].balloon_pos, point(1108, 904));
    assert_eq!(
        out[0].balloon_offset,
        left_side_offset(),
        "並べ替えを相対の位置に焼き付けない（要件 5.7）"
    );
    assert_eq!(out[0].balloon_offset_base, flip.balloon_offset_base);
    assert_eq!(out[1].balloon_pos, point(0, 100), "押し込み");

    let clamps: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.message().contains(CLAMP_TAG))
        .collect();
    assert_eq!(clamps.len(), 2, "{events:?}");
    assert_eq!(clamps[0].expect_field("scope"), "1");
    assert_eq!(clamps[0].expect_field("how"), "\"flipped\"");
    assert_eq!(clamps[1].expect_field("scope"), "0");
    assert_eq!(clamps[1].expect_field("how"), "\"pushed\"");
}

/// `limit=0` のスコープは並べ替えも押し込みもしない（要件 5.6）。
#[test]
fn boot_gate_does_not_flip_when_limit_is_disabled() {
    let p = placement(1, left_edge_char(), left_side_offset(), false);
    let (out, events) = capture_logs(|| apply_balloon_limit(vec![p], &snapshot()));
    assert_eq!(out[0].balloon_pos, point(-872, 904));
    assert!(
        !events.iter().any(|e| e.message().contains(CLAMP_TAG)),
        "{events:?}"
    );
}

// ---------------------------------------------------------------------------
// 書き込みの関門（要件 5.3・5.6・5.7・5.8）
// ---------------------------------------------------------------------------

fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

fn point_of(world: &World, entity: Entity) -> PointPx {
    let p = world
        .get::<WindowPos>(entity)
        .and_then(|wp| wp.position)
        .expect("位置がある");
    point(p.x, p.y)
}

/// 実 spawn の World（キャラ窓は作業領域の中ほど・バルーンは左隣）。
fn world_with(scope: usize, limit: bool) -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(snapshot());
    let titles = GhostTitles::from_scope_titles([(scope, "むらさき".to_string())]);
    let p = placement(scope, point(1200, 904), left_side_offset(), limit);
    let gw = spawn_ghost_windows(&mut world, &[p], &titles);
    let char_window = gw.char_window(scope).expect("キャラ窓");
    let balloon = gw.balloon_window(scope).expect("バルーン窓");
    world.entity_mut(char_window).insert(fake_handle(0x1000));
    world.entity_mut(balloon).insert(fake_handle(0x2000));
    (world, char_window, balloon)
}

/// キャラ窓を左端へ動かすと随伴のバルーンが右隣へ並べ替えられ（`how="flipped"`）、
/// 作業領域の中ほどへ戻すと元の側（左隣）へ戻る（焼き付けない・要件 5.7）。
#[test]
fn write_gate_flips_and_returns_to_the_original_side() {
    let (mut world, char_window, balloon) = world_with(0, true);

    let (_, events) = capture_logs(|| move_window_to(&mut world, char_window, 0, 904));
    assert_eq!(point_of(&world, char_window), point(0, 904));
    assert_eq!(point_of(&world, balloon), point(1108, 904));
    let clamp = expect_one(&events, CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"flipped\"");
    assert_eq!(clamp.expect_field("context"), "\"runtime-gate\"");

    // 対照: キャラ窓ごと左へはみ出すと押し込み。
    let (_, events) = capture_logs(|| move_window_to(&mut world, char_window, -200, 904));
    assert_eq!(point_of(&world, balloon), point(0, 904));
    assert_eq!(
        expect_one(&events, CLAMP_TAG).expect_field("how"),
        "\"pushed\""
    );

    let (_, events) = capture_logs(|| move_window_to(&mut world, char_window, 1200, 904));
    assert_eq!(
        point_of(&world, balloon),
        point(1200 - 872, 904),
        "中ほどへ戻ると元の側"
    );
    // 捕捉が生きていることを確かめてから「無い」を言う（窓の移動の記録はキャラ窓とバルーンの 2 件）。
    assert!(
        events
            .iter()
            .any(|e| e.message().contains("[diag.window_move]"))
    );
    assert!(!events.iter().any(|e| e.message().contains(CLAMP_TAG)));
}

/// `limit=0` のバルーンは書き込みの関門でも並べ替えない。
#[test]
fn write_gate_does_not_flip_when_limit_is_disabled() {
    let (mut world, char_window, balloon) = world_with(0, false);
    move_window_to(&mut world, char_window, 0, 904);
    assert_eq!(point_of(&world, balloon), point(-872, 904));
}

/// descript（`balloon.dontmove`）と台本（`\![lock,balloonmove]`）の動かせない指定があっても
/// 並べ替える（要件 5.7）。
#[test]
fn balloon_move_locks_do_not_stop_the_flip() {
    let (mut world, char_window, balloon) = world_with(1, true);
    seed_balloon_lock(&mut world, &BTreeSet::from([1]));
    apply_script_balloon_lock(&mut world, 1, true);
    move_window_to(&mut world, char_window, 0, 904);
    assert_eq!(point_of(&world, balloon), point(1108, 904));
}

// ---------------------------------------------------------------------------
// 解放の補正（要件 5.6）
// ---------------------------------------------------------------------------

/// 利用者が左端の窓の左隣の外でバルーンを離したときは、並べ替えず今の押し込み（x=0）。
#[test]
fn release_correction_never_flips() {
    let (mut world, char_window, balloon) = world_with(0, true);
    move_window_to(&mut world, char_window, 0, 904);
    // wndproc がドラッグでバルーンを左隣の外へ動かし切った状態。
    world.entity_mut(balloon).insert(WindowPos {
        position: Some(Point { x: -872, y: 904 }),
        size: Some(SizeI::new(BSZ.w, BSZ.h)),
        ..Default::default()
    });
    let ev = DragEndEvent {
        target: balloon,
        position: Point::new(0, 0),
        cancelled: false,
        is_primary: true,
        timestamp: Instant::now(),
    };
    let (_, events) =
        capture_logs(|| on_balloon_drag_end(&mut world, balloon, balloon, &Phase::Bubble(ev)));
    assert_eq!(point_of(&world, balloon), point(0, 904));
    let clamp = expect_one(&events, CLAMP_TAG);
    assert_eq!(clamp.expect_field("context"), "\"release-gate\"");
}

/// 差分にゴーストやスコープを名指しした分岐が無いことの檻: スコープ 0 と 1 で同じ置き方。
#[test]
fn flip_is_the_same_for_every_scope() {
    let out = apply_balloon_limit(
        vec![
            placement(0, left_edge_char(), left_side_offset(), true),
            placement(1, left_edge_char(), left_side_offset(), true),
            placement(2, left_edge_char(), left_side_offset(), true),
        ],
        &snapshot(),
    );
    assert!(out.iter().all(|p| p.balloon_pos == point(1108, 904)));
}
