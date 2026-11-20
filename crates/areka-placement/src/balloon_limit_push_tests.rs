//! バルーンの反対の側への並べ替えの檻（window-work-area-clamp task 5.1・design「BalloonFlip」・
//! Testing ⑾・要件 5.3〜5.8・7.2・7.8）。
//!
//! 純関数 [`place_balloon_in_work_area`] の判断分岐と、それを呼ぶ 2 つの関門（起動の関門
//! [`apply_balloon_limit`]・書き込みの関門 `apply_balloon_limit_gate`＝`move_window_to` の随伴）と、
//! 並べ替えない解放の補正（`on_balloon_drag_end`）を固定する。
//!
//! 押し込みに倒れる場面の期待値は、檻の側で [`limit_correction`] を呼んで作る
//! （「今の押し込みと同じ位置」が主張そのもの）。並べ替えの期待値は檻の側の数で書く。

use bevy_ecs::prelude::*;
use std::collections::BTreeSet;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, SizeI, WindowPos};

use super::gate_tests::snapshot;
use super::*;
use crate::balloon_lock::{apply_script_balloon_lock, seed_balloon_lock};
use crate::follow::test_support::{drag_end_event_at, point_of};
use crate::follow::{move_window_to, on_balloon_drag_end};
use crate::shared_test_support::placement_sized;
use crate::source::GhostTitles;
use crate::spawn::spawn_ghost_windows;
use crate::test_support::{fake_handle, point};
use log_capture_kit::{CapturedEvent, ExpectField, capture, expect_one};

/// 開発者の机の主の画面の作業領域。
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

fn rect_of(pos: PointPx, size: SizePx) -> RectPx {
    RectPx {
        left: pos.x,
        top: pos.y,
        right: pos.x + size.w,
        bottom: pos.y + size.h,
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
    placement_sized(scope, char_pos, offset, limit, CSZ, BSZ)
}

// ---------------------------------------------------------------------------
// 純関数（要件 5.3〜5.5）
// ---------------------------------------------------------------------------

/// 左端の窓の左隣で外へ出るバルーンは右隣（x=1108）へ映され、y は変わらない。
#[test]
fn left_side_balloon_of_left_edge_window_flips_to_the_right_keeping_y() {
    let pos = point(-872, 904);
    let got = place_balloon_in_work_area(pos, BSZ, rect_of(left_edge_char(), CSZ), WA, &[]);
    assert_eq!(got, BalloonPlace::Flipped(point(1108, 904)));
    assert_eq!(got.how(), "flipped");
}

/// 右端の窓の右隣で外へ出るバルーンは左隣へ映される（シェルとの間合いは左右で同じ）。
#[test]
fn right_side_balloon_of_right_edge_window_flips_to_the_left() {
    let char_pos = point(2880 - 1108, 300);
    // 右隣から 20px 離れたバルーン（間合い 20）→ 左隣でも間合い 20。
    let pos = point(2880 + 20, 300);
    let got = place_balloon_in_work_area(pos, BSZ, rect_of(char_pos, CSZ), WA, &[]);
    assert_eq!(got, BalloonPlace::Flipped(point(1772 - 20 - 872, 300)));
}

/// 映した位置が縦にだけはみ出すときは、横の位置を保って縦だけ収める。
#[test]
fn flipped_position_overflowing_only_vertically_is_clamped_vertically() {
    let pos = point(-872, 1504); // 下端 1904 > 1704
    let got = place_balloon_in_work_area(pos, BSZ, rect_of(left_edge_char(), CSZ), WA, &[]);
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
        let got = place_balloon_in_work_area(pos, BSZ, char_rect, area, &[]);
        assert_eq!(got, BalloonPlace::Pushed(pushed), "{label}");
        assert_eq!(got.how(), "pushed", "{label}");
    }
}

/// 収まっていれば動かさない（`limit_correction` の `None` と同じ）。
#[test]
fn fitting_balloon_is_left_alone() {
    let got = place_balloon_in_work_area(
        point(1108, 904),
        BSZ,
        rect_of(left_edge_char(), CSZ),
        WA,
        &[],
    );
    assert_eq!(got, BalloonPlace::Fits);
}

// ---------------------------------------------------------------------------
// 映した先がほかのキャラ窓に重なるときの押し込み（balloon-flip-side-rules 要件 2.1〜2.9・5.3）
// ---------------------------------------------------------------------------

/// 左端の窓の左隣のバルーン（映した先は 1108〜1980 × 904〜1304）。
fn left_side_pos() -> PointPx {
    point(-872, 904)
}

/// 映した先が相方の窓と縦にも横にも重なる（相方の上端 904 がバルーンの下端 1304 より上）→
/// 並べ替えず今の押し込み（要件 2.1・5.3 の赤の 1 件）。
#[test]
fn flip_target_overlapping_partner_window_is_pushed_instead() {
    let own = rect_of(left_edge_char(), CSZ);
    let partner = rect_of(point(1108, 904), CSZ);
    let pushed = limit_correction(left_side_pos(), BSZ, WA).expect("探針: はみ出している");
    let got = place_balloon_in_work_area(left_side_pos(), BSZ, own, WA, &[(0, own), (1, partner)]);
    assert_eq!(
        got,
        BalloonPlace::PushedOverlap {
            to: pushed,
            scope: 1
        }
    );
    assert_eq!(got.how(), "pushed");
    assert_eq!(got.overlap_scope(), Some(1));
}

/// `window-work-area-clamp` 4.3 ⑴ の形: 映した先 232〜1032 × 352〜800 は相方の窓
/// （764〜1436 × 904〜1704）と縦に重ならないので並べ替える（要件 2.2・5.3）。
#[test]
fn shape_4_3_1_flips_because_partner_is_below() {
    let own = rect_of(point(0, 610), SizePx { w: 764, h: 1094 });
    let partner = rect_of(point(764, 904), SizePx { w: 672, h: 800 });
    let got = place_balloon_in_work_area(
        point(-268, 352),
        SizePx { w: 800, h: 448 },
        own,
        WA,
        &[(0, own), (1, partner)],
    );
    assert_eq!(got, BalloonPlace::Flipped(point(232, 352)));
    assert_eq!(got.overlap_scope(), None);
}

/// 本体と相方が同じ位置に重なる形（配布版のぱすた・バルーンがシェルに 476 食い込む）:
/// 映した先と相方の交わりは自分の窓に収まるので数えない（要件 2.4）。
#[test]
fn partner_at_same_position_as_own_window_is_not_counted() {
    let own = rect_of(left_edge_char(), CSZ);
    let got =
        place_balloon_in_work_area(point(-872 + 476, 904), BSZ, own, WA, &[(0, own), (1, own)]);
    assert_eq!(got, BalloonPlace::Flipped(point(1108 - 476, 904)));
}

/// 映した先の右端 1980 に相方の左端が接するだけなら重ならない（半開）。
#[test]
fn partner_only_touching_the_flip_target_is_not_counted() {
    let own = rect_of(left_edge_char(), CSZ);
    let partner = rect_of(point(1980, 904), SizePx { w: 900, h: 800 });
    let got = place_balloon_in_work_area(left_side_pos(), BSZ, own, WA, &[(0, own), (1, partner)]);
    assert_eq!(got, BalloonPlace::Flipped(point(1108, 904)));
}

/// 3 スコープの列で 2 番目は重ならず 3 番目が重なる → 重なったスコープは 2（要件 2.8）。
#[test]
fn first_overlapping_scope_is_reported_across_three_scopes() {
    let own = rect_of(left_edge_char(), CSZ);
    let above = rect_of(point(1108, 0), SizePx { w: 1108, h: 500 });
    let beside = rect_of(point(1108, 904), CSZ);
    let got = place_balloon_in_work_area(
        left_side_pos(),
        BSZ,
        own,
        WA,
        &[(0, own), (1, above), (2, beside)],
    );
    assert_eq!(got.overlap_scope(), Some(2));
}

/// 押し込んだ位置（0〜872）もなお相方の窓（500〜1600）に重なるが、位置は押し込みのまま（要件 2.5）。
#[test]
fn pushed_position_is_kept_even_if_it_still_overlaps() {
    let own = rect_of(left_edge_char(), CSZ);
    let partner = rect_of(point(500, 904), SizePx { w: 1100, h: 800 });
    let pushed = limit_correction(left_side_pos(), BSZ, WA).expect("探針: はみ出している");
    let got = place_balloon_in_work_area(left_side_pos(), BSZ, own, WA, &[(0, own), (1, partner)]);
    assert_eq!(
        got,
        BalloonPlace::PushedOverlap {
            to: pushed,
            scope: 1
        }
    );
}

/// 空の列なら今の並べ替えと同一（要件 6.4）。
#[test]
fn empty_char_list_keeps_the_current_flip() {
    let own = rect_of(left_edge_char(), CSZ);
    let got = place_balloon_in_work_area(left_side_pos(), BSZ, own, WA, &[]);
    assert_eq!(got, BalloonPlace::Flipped(point(1108, 904)));
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
    let (out, events) = capture(|| apply_balloon_limit(vec![flip, push], &snapshot(&[WA])));

    assert_eq!(out[0].balloon_pos, point(1108, 904));
    assert_eq!(
        out[0].balloon_offset,
        left_side_offset(),
        "並べ替えを相対の位置に焼き付けない（要件 5.7）"
    );
    assert_eq!(out[0].balloon_offset_base, flip.balloon_offset_base);
    assert_eq!(out[1].balloon_pos, point(0, 100), "押し込み");

    let clamps: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(BALLOON_LIMIT_CLAMP_TAG))
        .collect();
    assert_eq!(clamps.len(), 2, "{events:?}");
    assert_eq!(clamps[0].expect_field("scope"), "1");
    assert_eq!(clamps[0].expect_field("how"), "\"flipped\"");
    assert_eq!(clamps[1].expect_field("scope"), "0");
    assert_eq!(clamps[1].expect_field("how"), "\"pushed\"");
}

/// 捕捉した中の `[balloon-limit] Clamp` の行（ちょうど 1 行）。
fn the_clamp(events: &[CapturedEvent]) -> &CapturedEvent {
    let v: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(BALLOON_LIMIT_CLAMP_TAG))
        .collect();
    assert_eq!(v.len(), 1, "{events:?}");
    v[0]
}

/// 2 スコープで本体（0）の映した先が相方（1）の窓に重なる → 押し込みの位置・
/// `how="pushed"`・`overlap_scope=Some(1)`・`context="boot-gate"` の 1 行（要件 2.3・4.1・4.2）。
/// 重ならない形（相方が遠い）は `how="flipped"`・`overlap_scope=None`。
#[test]
fn boot_gate_pushes_on_overlap_and_logs_the_scope() {
    let main = placement(0, left_edge_char(), left_side_offset(), true);
    let partner = placement(1, point(1108, 904), left_side_offset(), true);
    let pushed = limit_correction(left_side_pos(), BSZ, WA).expect("探針: はみ出している");
    let (out, events) = capture(|| apply_balloon_limit(vec![main, partner], &snapshot(&[WA])));
    assert_eq!(out[0].balloon_pos, pushed);
    assert_eq!(out[0].balloon_offset, left_side_offset());
    let ev = the_clamp(&events);
    assert_eq!(ev.expect_field("how"), "\"pushed\"");
    assert_eq!(ev.expect_field("overlap_scope"), "Some(1)");
    assert_eq!(ev.expect_field("context"), "\"boot-gate\"");

    let far = placement(1, point(2000, 0), left_side_offset(), true);
    let (out, events) = capture(|| apply_balloon_limit(vec![main, far], &snapshot(&[WA])));
    assert_eq!(out[0].balloon_pos, point(1108, 904));
    let ev = the_clamp(&events);
    assert_eq!(ev.expect_field("how"), "\"flipped\"");
    assert_eq!(ev.expect_field("overlap_scope"), "None");
}

/// `limit=0` のスコープは並べ替えも押し込みもしない（要件 5.6）。
#[test]
fn boot_gate_does_not_flip_when_limit_is_disabled() {
    let p = placement(1, left_edge_char(), left_side_offset(), false);
    let (out, events) = capture(|| apply_balloon_limit(vec![p], &snapshot(&[WA])));
    assert_eq!(out[0].balloon_pos, point(-872, 904));
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains(BALLOON_LIMIT_CLAMP_TAG)),
        "{events:?}"
    );
}

// ---------------------------------------------------------------------------
// 書き込みの関門（要件 5.3・5.6・5.7・5.8）
// ---------------------------------------------------------------------------

/// 実 spawn の World（キャラ窓は作業領域の中ほど・バルーンは左隣）。
fn world_with(scope: usize, limit: bool) -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(snapshot(&[WA]));
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

    let (_, events) = capture(|| move_window_to(&mut world, char_window, 0, 904));
    assert_eq!(point_of(&world, char_window), point(0, 904));
    assert_eq!(point_of(&world, balloon), point(1108, 904));
    let clamp = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"flipped\"");
    assert_eq!(clamp.expect_field("context"), "\"runtime-gate\"");

    // 対照: キャラ窓ごと左へはみ出すと押し込み。
    let (_, events) = capture(|| move_window_to(&mut world, char_window, -200, 904));
    assert_eq!(point_of(&world, balloon), point(0, 904));
    assert_eq!(
        expect_one(&events, BALLOON_LIMIT_CLAMP_TAG).expect_field("how"),
        "\"pushed\""
    );

    let (_, events) = capture(|| move_window_to(&mut world, char_window, 1200, 904));
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
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains(BALLOON_LIMIT_CLAMP_TAG))
    );
}

/// 2 スコープの実 spawn の World（スコープ 1 の相方のキャラ窓は x=1108・本体の映した先の右隣）。
/// 相方のキャラ窓には、子の描画面を `visible` の列どおりに付ける。
fn world_with_partner(partner_children: &[bool]) -> (World, Entity, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(snapshot(&[WA]));
    let titles = GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]);
    let ps = [
        placement(0, point(1200, 904), left_side_offset(), true),
        placement(1, point(1108, 904), left_side_offset(), false),
    ];
    let gw = spawn_ghost_windows(&mut world, &ps, &titles);
    let (c0, b0, c1) = (
        gw.char_window(0).expect("キャラ窓"),
        gw.balloon_window(0).expect("バルーン窓"),
        gw.char_window(1).expect("相方のキャラ窓"),
    );
    world.entity_mut(c0).insert(fake_handle(0x1000));
    world.entity_mut(b0).insert(fake_handle(0x2000));
    for &is_visible in partner_children {
        world.spawn((
            wintf::ecs::Visual {
                is_visible,
                ..Default::default()
            },
            ChildOf(c1),
        ));
    }
    (world, c0, b0, c1)
}

/// 相方の子の描画面が可視で映した先に重なる → 押し込み＋`overlap_scope=Some(1)`（要件 2.3・4.1・4.2）。
#[test]
fn write_gate_pushes_on_overlap_with_a_shown_partner() {
    let (mut world, c0, b0, _) = world_with_partner(&[false, true]);
    let (_, events) = capture(|| move_window_to(&mut world, c0, 0, 904));
    assert_eq!(point_of(&world, b0), point(0, 904), "押し込みの位置");
    let clamp = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"pushed\"");
    assert_eq!(clamp.expect_field("overlap_scope"), "Some(1)");
    assert_eq!(clamp.expect_field("context"), "\"runtime-gate\"");
}

/// 同じ形で相方の子が全部不可視、または相方の寸が未確定 → 並べ替え（要件 2.6）。
#[test]
fn write_gate_flips_when_the_partner_is_hidden_or_undetermined() {
    let (mut world, c0, b0, _) = world_with_partner(&[false, false]);
    let (_, events) = capture(|| move_window_to(&mut world, c0, 0, 904));
    assert_eq!(point_of(&world, b0), point(1108, 904));
    let clamp = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"flipped\"");
    assert_eq!(clamp.expect_field("overlap_scope"), "None");

    let (mut world, c0, b0, c1) = world_with_partner(&[true]);
    world.entity_mut(c1).insert(WindowPos::default());
    let (_, events) = capture(|| move_window_to(&mut world, c0, 0, 904));
    assert_eq!(point_of(&world, b0), point(1108, 904));
    let clamp = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"flipped\"");
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
    let ev = drag_end_event_at(balloon, (0, 0));
    let (_, events) =
        capture(|| on_balloon_drag_end(&mut world, balloon, balloon, &Phase::Bubble(ev)));
    assert_eq!(point_of(&world, balloon), point(0, 904));
    let clamp = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
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
        &snapshot(&[WA]),
    );
    assert!(out.iter().all(|p| p.balloon_pos == point(1108, 904)));
}

// ---------------------------------------------------------------------------
// 初めから右に出した位置と、左から映して右に来た位置（balloon-flip-side-rules 要件 3.3・3.8・5.1）
// ---------------------------------------------------------------------------

/// 左端のキャラ窓（`default_x` を大きくして左端へクランプ）に `side` で出したバルーンの
/// （キャラ窓の矩形, バルーンの位置）。合流後のずらしの総量 `total_x` のうち descript の分が `descript_x`。
fn resolved_at_left_edge(
    side: crate::config::BalloonSide,
    total_x: i32,
    descript_x: i32,
) -> (RectPx, PointPx) {
    let cfg = crate::config::ScopeConfig {
        alignment: crate::config::Alignment::Bottom,
        default_x: Some(40000),
        balloon_alignment: side,
        balloon_offset: Some((total_x, 0)),
        balloon_offset_descript_x: descript_x,
        ..Default::default()
    };
    let out = crate::resolver::resolve_placement(
        &crate::shared_test_support::cfg_of(vec![(0, cfg)]),
        WA,
        &[crate::resolver::ScopeInput {
            scope: 0,
            char_size: CSZ,
            balloon_size: BSZ,
        }],
        crate::shared_test_support::MEASURE_DPI,
    );
    (rect_of(out[0].char_pos, CSZ), out[0].balloon_pos)
}

/// descript の分 d: 初めから右に出した x ＝ 左に出して左から右へ映した x（要件 3.3・5.1）。
#[test]
fn right_from_start_equals_left_mirrored_for_descript_part() {
    use crate::config::BalloonSide::{Left, Right};
    for d in [30, 100] {
        let (_, right) = resolved_at_left_edge(Right, d, d);
        let (char_rect, left) = resolved_at_left_edge(Left, d, d);
        let BalloonPlace::Flipped(mirrored) =
            place_balloon_in_work_area(left, BSZ, char_rect, WA, &[])
        else {
            panic!("探針: 左端の窓の左に出したバルーンは映される (d={d})");
        };
        assert_eq!(mirrored.x, right.x, "d={d}");
        assert_eq!(right.x, char_rect.right - d, "d={d}: 設計の式");
    }
}

/// `windowposition.x` の分 −237（descript 0）: 映しの結果は今の式（キャラ窓の右端 ＋ 237）のまま（要件 3.8）。
#[test]
fn mirror_of_windowposition_part_keeps_current_formula() {
    use crate::config::BalloonSide::Left;
    let (char_rect, left) = resolved_at_left_edge(Left, -237, 0);
    assert_eq!(left.x, char_rect.left - BSZ.w - 237, "探針");
    let got = place_balloon_in_work_area(left, BSZ, char_rect, WA, &[]);
    // 設計の式: 映した x ＝ 窓の左 ＋ 窓の右 − (元の x ＋ バルーンの幅)。
    assert_eq!(
        got,
        BalloonPlace::Flipped(point(char_rect.right + 237, left.y))
    );
}

// ---------------------------------------------------------------------------
// 押し込みだけの形の表（両関門に同じ表を通す）
// ---------------------------------------------------------------------------

struct PushRow {
    name: &'static str,
    scope: usize,
    char_pos: PointPx,
    char_size: SizePx,
    offset: PointPx,
    balloon_size: SizePx,
    limit: bool,
    expected: PointPx,
    recorded: bool,
    /// 書き込みの関門の檻でキャラ窓を最初に置く位置（バルーンが収まる）。
    spawn_pos: PointPx,
}

fn push_rows() -> Vec<PushRow> {
    let std_char = SizePx { w: 1108, h: 800 };
    let std_balloon = SizePx { w: 872, h: 400 };
    vec![
        PushRow {
            name: "10-30",
            scope: 1,
            char_pos: point(0, 336),
            char_size: SizePx { w: 1108, h: 1368 },
            offset: point(-872, 0),
            balloon_size: SizePx { w: 872, h: 900 },
            limit: true,
            expected: point(0, 336),
            recorded: true,
            spawn_pos: point(1200, 336),
        },
        PushRow {
            name: "11-16-after-drag",
            scope: 1,
            char_pos: point(2524, 1304),
            char_size: SizePx { w: 336, h: 400 },
            offset: point(131, -116),
            balloon_size: SizePx { w: 288, h: 203 },
            limit: true,
            expected: point(2592, 1188),
            recorded: true,
            spawn_pos: point(1200, 904),
        },
        PushRow {
            name: "fits",
            scope: 0,
            char_pos: point(1200, 904),
            char_size: std_char,
            offset: point(-872, 0),
            balloon_size: std_balloon,
            limit: true,
            expected: point(328, 904),
            recorded: false,
            spawn_pos: point(1300, 904),
        },
        PushRow {
            name: "limit-0",
            scope: 0,
            char_pos: point(0, 904),
            char_size: std_char,
            offset: point(-872, 0),
            balloon_size: std_balloon,
            limit: false,
            expected: point(-872, 904),
            recorded: false,
            spawn_pos: point(1200, 904),
        },
        PushRow {
            name: "char-overflows",
            scope: 0,
            char_pos: point(-200, 904),
            char_size: std_char,
            offset: point(-872, 0),
            balloon_size: std_balloon,
            limit: true,
            expected: point(0, 904),
            recorded: true,
            spawn_pos: point(1200, 904),
        },
        PushRow {
            name: "vertical-overflow",
            scope: 0,
            char_pos: point(1200, 904),
            char_size: std_char,
            offset: point(-872, 1000),
            balloon_size: std_balloon,
            limit: true,
            expected: point(328, 1304),
            recorded: true,
            spawn_pos: point(1200, 300),
        },
    ]
}

fn check_clamp_lines(
    row: &PushRow,
    events: &[CapturedEvent],
    context: &str,
    alive_tag: Option<&str>,
    fails: &mut Vec<String>,
) {
    let clamps: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(BALLOON_LIMIT_CLAMP_TAG))
        .collect();
    if row.recorded {
        if clamps.len() != 1 {
            fails.push(format!(
                "{} [clamp-count]: {} lines",
                row.name,
                clamps.len()
            ));
            return;
        }
        let c = clamps[0];
        if c.field("how") != Some("\"pushed\"") {
            fails.push(format!("{} [how]: {:?}", row.name, c.field("how")));
        }
        if c.field("context") != Some(context) {
            fails.push(format!("{} [context]: {:?}", row.name, c.field("context")));
        }
        if c.field("overlap_scope").is_some() {
            fails.push(format!(
                "{} [overlap_scope-field]: {:?}",
                row.name,
                c.field("overlap_scope")
            ));
        }
    } else {
        if let Some(tag) = alive_tag {
            if !events.iter().any(|e| e.message().contains(tag)) {
                fails.push(format!("{} [capture-alive]: no {tag}", row.name));
            }
        }
        if !clamps.is_empty() {
            fails.push(format!(
                "{} [clamp-count]: {} lines, want 0",
                row.name,
                clamps.len()
            ));
        }
    }
}

#[test]
fn push_table_boot_gate() {
    let mut fails = Vec::new();
    for row in push_rows() {
        let p = placement_sized(
            row.scope,
            row.char_pos,
            row.offset,
            row.limit,
            row.char_size,
            row.balloon_size,
        );
        let (out, events) = capture(|| apply_balloon_limit(vec![p], &snapshot(&[WA])));
        if out[0].balloon_pos != row.expected {
            fails.push(format!(
                "{} [position]: got {:?}, want {:?}",
                row.name, out[0].balloon_pos, row.expected
            ));
        }
        if out[0].balloon_offset != row.offset {
            fails.push(format!(
                "{} [balloon_offset]: {:?}",
                row.name, out[0].balloon_offset
            ));
        }
        if out[0].balloon_offset_base != p.balloon_offset_base {
            fails.push(format!("{} [balloon_offset_base]", row.name));
        }
        check_clamp_lines(&row, &events, "\"boot-gate\"", None, &mut fails);
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

#[test]
fn push_table_write_gate() {
    let mut fails = Vec::new();
    for row in push_rows() {
        let mut world = World::new();
        world.insert_resource(snapshot(&[WA]));
        let titles = GhostTitles::from_scope_titles([(row.scope, "むらさき".to_string())]);
        let p = placement_sized(
            row.scope,
            row.spawn_pos,
            row.offset,
            row.limit,
            row.char_size,
            row.balloon_size,
        );
        let gw = spawn_ghost_windows(&mut world, &[p], &titles);
        let char_window = gw.char_window(row.scope).expect("キャラ窓");
        let balloon = gw.balloon_window(row.scope).expect("バルーン窓");
        world.entity_mut(char_window).insert(fake_handle(0x1000));
        world.entity_mut(balloon).insert(fake_handle(0x2000));

        let (_, events) =
            capture(|| move_window_to(&mut world, char_window, row.char_pos.x, row.char_pos.y));
        let got = point_of(&world, balloon);
        if got != row.expected {
            fails.push(format!(
                "{} [position]: got {:?}, want {:?}",
                row.name, got, row.expected
            ));
        }
        check_clamp_lines(
            &row,
            &events,
            "\"runtime-gate\"",
            Some("[diag.window_move]"),
            &mut fails,
        );
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}
