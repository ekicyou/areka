//! バルーンは作業領域内へ押し込むだけという決まりの檻。起動の関門・書き込みの関門・
//! 解放の補正に同じ形の表を通し、窓の左右どちらへ出す指定かの位置の式も固定する。

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

/// 窓の左隣に出すバルーンのずらし。
fn left_side_offset() -> PointPx {
    point(-872, 0)
}

fn placement(scope: usize, char_pos: PointPx, offset: PointPx, limit: bool) -> ScopePlacement {
    placement_sized(scope, char_pos, offset, limit, CSZ, BSZ)
}

// ---------------------------------------------------------------------------
// 書き込みの関門
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

/// キャラ窓を左端へ動かすと随伴のバルーンは作業領域の左端へ押し込まれ（`how="pushed"`・
/// `overlap_scope` は出さない）、中ほどへ戻すと元の側（左隣）へ戻る（焼き付けない）。
#[test]
fn write_gate_pushes_to_the_edge_and_returns_to_the_original_side() {
    let (mut world, char_window, balloon) = world_with(0, true);

    let (_, events) = capture(|| move_window_to(&mut world, char_window, 0, 904));
    assert_eq!(point_of(&world, char_window), point(0, 904));
    assert_eq!(point_of(&world, balloon), point(0, 904));
    let clamp = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(clamp.expect_field("how"), "\"pushed\"");
    assert_eq!(clamp.expect_field("context"), "\"runtime-gate\"");
    assert!(!format!("{clamp:?}").contains("overlap_scope"), "{clamp:?}");

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

/// 相方のキャラ窓の可視・不可視・寸の未確定のどれでも、自分のバルーンは押し込みの位置（0, 904）。
#[test]
fn write_gate_ignores_the_partner_window_state() {
    for (children, undetermined_size) in [
        (&[false, true][..], false),
        (&[false, false], false),
        (&[true], true),
    ] {
        let (mut world, c0, b0, c1) = world_with_partner(children);
        if undetermined_size {
            world.entity_mut(c1).insert(WindowPos::default());
        }
        let (_, events) = capture(|| move_window_to(&mut world, c0, 0, 904));
        assert_eq!(point_of(&world, b0), point(0, 904), "{children:?}");
        let clamp = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
        assert_eq!(clamp.expect_field("how"), "\"pushed\"");
    }
}

/// descript（`balloon.dontmove`）と台本（`\![lock,balloonmove]`）の動かせない指定があっても
/// 押し込む。
#[test]
fn balloon_move_locks_do_not_stop_the_push() {
    let (mut world, char_window, balloon) = world_with(1, true);
    seed_balloon_lock(&mut world, &BTreeSet::from([1]));
    apply_script_balloon_lock(&mut world, 1, true);
    move_window_to(&mut world, char_window, 0, 904);
    assert_eq!(point_of(&world, balloon), point(0, 904));
}

// ---------------------------------------------------------------------------
// 解放の補正
// ---------------------------------------------------------------------------

/// 利用者が左端の窓の左隣の外でバルーンを離したときは、並べ替えず今の押し込み（x=0）。
#[test]
fn release_correction_pushes_to_the_edge() {
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

/// 書き込みの関門にスコープを名指しした分岐が無いことの檻: スコープ 0・1・2 で同じ押し込み。
#[test]
fn write_gate_pushes_the_same_for_every_scope() {
    for scope in 0..=2 {
        let (mut world, char_window, balloon) = world_with(scope, true);
        move_window_to(&mut world, char_window, 0, 904);
        assert_eq!(point_of(&world, balloon), point(0, 904), "scope {scope}");
    }
}

// ---------------------------------------------------------------------------
// 右に出した位置の式
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

/// descript の分 d: 右に出したバルーンの x ＝ キャラ窓の右端 − d で、右の間合い（キャラ窓の右端から
/// バルーンの左端）は左に出したときの間合い（バルーンの右端からキャラ窓の左端）と等しい。
#[test]
fn right_side_x_is_char_right_edge_minus_descript_part_with_gap_equal_to_left() {
    use crate::config::BalloonSide::{Left, Right};
    for d in [30, 100] {
        let (char_rect, right) = resolved_at_left_edge(Right, d, d);
        let (_, left) = resolved_at_left_edge(Left, d, d);
        assert_eq!(right.x, char_rect.right - d, "d={d}");
        assert_eq!(
            right.x - char_rect.right,
            char_rect.left - (left.x + BSZ.w),
            "d={d}: 左右の間合い"
        );
    }
}

/// `windowposition.x` の分 −237（descript 0）: 左は窓の左 − 幅 − 237、右は窓の右 − 237。
#[test]
fn windowposition_part_offsets_left_and_right_sides_by_the_same_amount() {
    use crate::config::BalloonSide::{Left, Right};
    let (char_rect, left) = resolved_at_left_edge(Left, -237, 0);
    let (_, right) = resolved_at_left_edge(Right, -237, 0);
    assert_eq!(left.x, char_rect.left - BSZ.w - 237);
    assert_eq!(right.x, char_rect.right - 237);
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
        if let Some(tag) = alive_tag
            && !events.iter().any(|e| e.message().contains(tag))
        {
            fails.push(format!("{} [capture-alive]: no {tag}", row.name));
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
