use bevy_ecs::prelude::*;
use wintf::ecs::drag::OnDrag;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, WindowPos};

use super::test_support::{fake_window_handle, titles};
use super::{GhostWindows, spawn_ghost_windows};
use crate::placement::follow::BalloonFollow;
use crate::placement::resolver::{PointPx, ScopePlacement, SizePx};

// -------------------------------------------------------------------------
// T-I4: follow 幾何（task 5.3・design Testing Strategy 4・要件 4.2）
//
// 実パイプライン統合: `build_placement_config`（KV 実経路）→
// `resolve_placement`（非 96 倍数の合成 work_area・原点非 (0,0)）→
// `spawn_ghost_windows` → 偽 WindowHandle 付与 → `move_window_to`。
// 期待値は resolver 出力（`ScopePlacement.balloon_offset`）から導出し、
// 手書き offset のコピー照合にしない（T-I1 との差分＝実パイプライン消費）。
//
// 置き場の判断: spawn は resolver 出力と follow API の両方を消費する合成根で
// あり、兄弟モジュールのテストは自ファイル内という repo 慣行に従いここに置く。
// -------------------------------------------------------------------------

use std::collections::BTreeMap;
use std::time::Instant;

use wintf::ecs::drag::DragEvent;

use crate::placement::config::build_placement_config;
use crate::placement::follow::move_window_to;
use crate::placement::resolver::{RectPx, ScopeInput, resolve_placement};

/// ドラッグイベント（wndproc 移動済み後の Bubble 配送を模す・follow.rs と同型）。
fn drag_event(target: Entity) -> DragEvent {
    DragEvent {
        target,
        start_position: Point::new(0, 0),
        position: Point::new(10, 10),
        is_primary: true,
        timestamp: Instant::now(),
    }
}

/// `(key, value)` ペア列 → `parse_kv` 出力相当の `BTreeMap`（config テストと同じ流儀）。
fn kv_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// 実パイプライン（KV → config → resolver → spawn）で組んだ World。
///
/// - work_area は原点非 (0,0)・全辺 96 の倍数を避けた合成値（隠れた `dpi/96`
///   再スケールがあれば完全一致が崩れる檻・repo 慣行）
/// - scope0＝バルーン left（既定）＋ `balloon.offsetx/offsety` 加算、
///   scope1＝バルーン right — 左右両変種を 1 パイプラインで通す
/// - scope0 のキャラ寸は emo2 実寸（434×687）・他は 96 非倍数の合成値
fn real_pipeline_world() -> (World, GhostWindows, Vec<ScopePlacement>) {
    let ghost_kv = kv_map(&[("sakura.name", "むらさき"), ("kero.name", "エモ")]);
    let shell_kv = kv_map(&[
        ("seriko.alignmenttodesktop", "bottom"),
        ("sakura.defaultx", "52"),
        ("sakura.balloon.offsetx", "29"),
        ("sakura.balloon.offsety", "-41"),
        ("kero.defaultx", "36"),
        ("kero.balloon.alignment", "right"),
    ]);
    let cfg = build_placement_config(&ghost_kv, &shell_kv);

    let work_area = RectPx {
        left: 31,
        top: 17,
        right: 2574,
        bottom: 1444,
    };
    let scopes = [
        ScopeInput {
            scope: 0,
            char_size: SizePx { w: 434, h: 687 },
            balloon_size: SizePx { w: 401, h: 223 },
        },
        ScopeInput {
            scope: 1,
            char_size: SizePx { w: 278, h: 357 },
            balloon_size: SizePx { w: 227, h: 159 },
        },
    ];
    let placements = resolve_placement(&cfg, work_area, &scopes);

    let mut world = World::new();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    (world, gw, placements)
}

/// 全 4 窓へ偽 WindowHandle を付与する（4.2 の偽装境界パターン）。
fn attach_fake_handles(world: &mut World, gw: &GhostWindows) {
    let mut raw = 0x100isize;
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).unwrap(),
            gw.balloon_window(scope).unwrap(),
        ] {
            world.entity_mut(e).insert(fake_window_handle(raw));
            raw += 0x10;
        }
    }
}

/// entity の WindowPos.position を読む（未設定は panic で検出）。
fn window_position(world: &World, e: Entity) -> Point {
    world
        .get::<WindowPos>(e)
        .expect("WindowPos")
        .position
        .expect("position")
}

/// T-I4: `spawn_ghost_windows` の `BalloonFollow.offset` が `resolve_placement`
/// の `balloon_offset` と一致する（実パイプライン・左右両変種・4.2）。
#[test]
fn t_i4_follow_offset_matches_resolver_output_through_real_pipeline() {
    let (world, gw, placements) = real_pipeline_world();

    assert_eq!(
        placements.len(),
        2,
        "空虚一致封じ: 2 スコープが解決されること"
    );

    // config 由来の左右両変種が実際に効いている檻（resolver 幾何の正値を固定し、
    // 「resolver 出力のコピー同士の照合」への退化を防ぐ）
    assert_eq!(
        placements[0].balloon_offset,
        PointPx {
            x: -401 + 29,
            y: -41
        },
        "scope0: left＝キャラ左隣（−balloon_w）＋balloon.offsetx/y 加算"
    );
    assert_eq!(
        placements[1].balloon_offset,
        PointPx { x: 278, y: 0 },
        "scope1: right＝キャラ右隣（＋char_w）・上端揃え"
    );

    for p in &placements {
        let char_e = gw.char_window(p.scope).expect("char window");
        let follow = world
            .get::<BalloonFollow>(char_e)
            .expect("char window BalloonFollow");
        assert_eq!(
            follow.offset, p.balloon_offset,
            "scope{}: BalloonFollow.offset は resolver 出力の転写",
            p.scope
        );
        assert_eq!(
            follow.balloon,
            gw.balloon_window(p.scope).expect("balloon window"),
            "scope{}: 追従先は自スコープのバルーン窓",
            p.scope
        );

        // 恒等式 balloon_offset ≡ balloon_pos − char_pos（design Postconditions）が
        // spawn 転記後の WindowPos 上でも観測できる
        let char_pos = window_position(&world, char_e);
        let balloon_pos = window_position(&world, follow.balloon);
        assert_eq!(
            PointPx {
                x: balloon_pos.x - char_pos.x,
                y: balloon_pos.y - char_pos.y
            },
            p.balloon_offset,
            "scope{}: 初期 WindowPos も恒等式を満たす",
            p.scope
        );
    }
}

/// T-I4: spawn 済み entity への `move_window_to` で、バルーンが resolver 由来
/// offset を保って追従する（複数回移動でも offset 静的・他スコープ不干渉・4.2）。
#[test]
fn t_i4_move_window_to_keeps_balloon_offset_across_multiple_moves() {
    let (mut world, gw, placements) = real_pipeline_world();
    attach_fake_handles(&mut world, &gw);

    let p1 = &placements[1];
    let char1 = gw.char_window(1).unwrap();
    let balloon1 = gw.balloon_window(1).unwrap();
    let scope1_initial = (
        window_position(&world, char1),
        window_position(&world, balloon1),
    );

    for p in &placements {
        let char_e = gw.char_window(p.scope).unwrap();
        let balloon_e = gw.balloon_window(p.scope).unwrap();
        // 96 の倍数を避けた移動先を複数回（offset は配置時確定で静的・4.4）
        let targets = [(1237 + p.scope as i32, 941), (533, 1189 + p.scope as i32)];
        for (x, y) in targets {
            assert!(move_window_to(&mut world, char_e, x, y));
            assert_eq!(
                window_position(&world, char_e),
                Point { x, y },
                "scope{}: 対象自身は指定座標へ（物理 px 素通し）",
                p.scope
            );
            assert_eq!(
                window_position(&world, balloon_e),
                Point {
                    x: x + p.balloon_offset.x,
                    y: y + p.balloon_offset.y
                },
                "scope{}: バルーンは resolver 由来 offset を保って追従",
                p.scope
            );
        }

        // scope0 の移動が scope1 の窓を動かしていない（追従は自スコープのみ）
        if p.scope == 0 {
            assert_eq!(window_position(&world, char1), scope1_initial.0);
            assert_eq!(window_position(&world, balloon1), scope1_initial.1);
            assert_eq!(
                scope1_initial.1,
                Point {
                    x: p1.char_pos.x + p1.balloon_offset.x,
                    y: p1.char_pos.y + p1.balloon_offset.y
                },
                "前提: scope1 初期位置は resolver 解決値"
            );
        }
    }
}

/// T-I4 補: バルーン単独ドラッグの相対位置記憶（4.8・DD16・task 8.3）。
///
/// 仕様退役: 2026-07-11 要件 4.8 —— 本テストの旧版
/// `t_i4_char_move_restores_initial_offset_after_balloon_solo_move` が檻に
/// していた「次のキャラ窓移動で初期 offset へスナップバック」は仕様として
/// 退役し、調整後 offset の記憶・追従が正となった（記憶挙動の檻へ書き換え）。
///
/// 実パイプライン（KV → config → resolver → spawn）で組んだ World 上で、
/// spawn が付けた**実際の** `OnDrag` ハンドラ（バルーン窓の
/// `on_balloon_drag`）を呼んで検証する＝結線の檻を兼ねる。
#[test]
fn t_i4_char_move_follows_adjusted_offset_after_balloon_solo_drag() {
    let (mut world, gw, placements) = real_pipeline_world();
    attach_fake_handles(&mut world, &gw);
    let p = &placements[0];
    let char_e = gw.char_window(0).unwrap();
    let balloon_e = gw.balloon_window(0).unwrap();

    // バルーン単独ドラッグ: wndproc がバルーンを (613, 407) へ移動済みの状態を
    // 模し、spawn が付けた実 OnDrag ハンドラを Bubble で呼ぶ
    world.get_mut::<WindowPos>(balloon_e).unwrap().position = Some(Point { x: 613, y: 407 });
    let handler = world.get::<OnDrag>(balloon_e).expect("balloon OnDrag").0;
    let ev = Phase::Bubble(drag_event(balloon_e));
    assert!(!handler(&mut world, balloon_e, balloon_e, &ev));

    // キャラ窓は不動（4.8: バルーンのみ移動）
    assert_eq!(
        window_position(&world, char_e),
        Point {
            x: p.char_pos.x,
            y: p.char_pos.y
        },
        "バルーンドラッグでキャラ窓は動かない"
    );

    // 調整後 offset = balloon_pos − char_pos が記憶される
    let adjusted = PointPx {
        x: 613 - p.char_pos.x,
        y: 407 - p.char_pos.y,
    };
    assert_ne!(
        adjusted, p.balloon_offset,
        "檻の前提: 調整後 offset は resolver 由来の初期 offset と異なる"
    );
    assert_eq!(
        world.get::<BalloonFollow>(char_e).unwrap().offset,
        adjusted,
        "バルーン単独ドラッグで offset が記憶更新される（4.8）"
    );

    // 次のキャラ窓移動は**調整後** offset で追従（初期 offset へ戻らない）
    assert!(move_window_to(&mut world, char_e, 1751, 893));
    assert_eq!(
        window_position(&world, balloon_e),
        Point {
            x: 1751 + adjusted.x,
            y: 893 + adjusted.y
        },
        "キャラ窓移動でバルーンは調整後 offset 位置へ追従する"
    );

    // 他スコープ（scope1）の offset は不干渉
    let char1 = gw.char_window(1).unwrap();
    assert_eq!(
        world.get::<BalloonFollow>(char1).unwrap().offset,
        placements[1].balloon_offset,
        "scope0 バルーンのドラッグは scope1 の offset を変えない"
    );
}
