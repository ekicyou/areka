// apply_move_directive（UI スレッド適用）の統合テスト。実 HWND なしの headless World に
// 実 GhostWindows を組み、物理座標・バルーン随伴 offset・対象不在・Anchored 不変を固定する。

use super::*;
use areka_emo_compose::ScaleRatio;
use areka_placement::follow::OffsetBase;
use bevy_ecs::prelude::{Entity, World};

use areka_placement::follow::{Anchored, BalloonFollow, MonitorSnapshot};
use areka_placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use areka_placement::source::GhostTitles;
use areka_placement::spawn::{GhostWindows, spawn_ghost_windows};
use areka_placement::test_support::give_fake_handles;
use wintf::ecs::{Point, WindowPos};

/// 既知位置・寸法の解決済み配置（balloon_offset は char 窓へ BalloonFollow.offset として転写）。
fn placement(scope: usize, cx: i32, cy: i32, cw: i32, ch: i32, boff: PointPx) -> ScopePlacement {
    ScopePlacement {
        scope,
        char_pos: PointPx { x: cx, y: cy },
        char_size: SizePx { w: cw, h: ch },
        balloon_pos: PointPx {
            x: cx + boff.x,
            y: cy + boff.y,
        },
        balloon_size: SizePx { w: 200, h: 150 },
        balloon_offset: boff,
        balloon_offset_base: OffsetBase::unpinned(boff),
        // 正典既定（有効）。limit の判定はここでは対象外。
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// base=scope0 (1000,500,400,687)・target=scope1 (1200,800,300,434) の headless World＋
/// GhostWindows（全窓に偽 WindowHandle 付与済み）。
pub(super) fn move_world() -> (World, GhostWindows) {
    let placements = vec![
        placement(0, 1000, 500, 400, 687, PointPx { x: 285, y: -19 }),
        placement(1, 1200, 800, 300, 434, PointPx { x: 285, y: -19 }),
    ];
    let mut world = World::new();
    let gw = spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]),
    );
    give_fake_handles(&mut world, &gw);
    (world, gw)
}

pub(super) fn pos_of(world: &World, e: Entity) -> Point {
    world
        .get::<WindowPos>(e)
        .expect("WindowPos があるはず")
        .position
        .expect("position があるはず")
}

/// 非スコープの基準でも動く。
///
/// `screen` は対象窓の中心のモニタの作業領域 (0,0)-(1920,1040)。`base` は `center` へ置き換わり
/// x' = 960 − 353 − 300/2 = 457。Y は `fix` だが窓は下端に吸い付く（spawn の `Anchor::Bottom`）
/// ので作業領域の下端 − 高さ = 1040 − 434 = 606。
#[test]
fn apply_non_scope_base_moves() {
    let (mut world, gw) = move_world();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        }],
    });
    let target = gw.char_window(1).unwrap();
    let directive = parse_move_directive(
        1,
        &["-353", "", "", "screen", "base", "base"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
    .directive;
    assert_eq!(directive.base, MoveBase::Screen);

    assert!(
        apply_move_directive(&mut world, &directive, ScaleRatio::ONE),
        "非スコープの基準でも動く"
    );
    assert_eq!(pos_of(&world, target), Point { x: 457, y: 606 });
}

// MoveCueSink→channel→apply_move_directive→move_window_to の一続きを通すテスト。
// 隣の段は `move_cue_move_sink_tests.rs`（sink→drain）・`run_move_drain_phase_applies_directive_when_ghost_windows_present`（channel→apply）・`spine_move_cue_drives_window_move_end_to_end`（全経路・座標だけ）が見る。

use crate::consumer_ledger::test_support::actor_carrier_cue;
use dola::cue::CueSink;
use std::sync::mpsc::channel;

/// `\![move]` キャリア cue を実 `MoveCueSink`→mpsc channel に通し、drain した
/// `MoveDirective` を返す。
fn directive_via_sink(actor: &str, tokens: &[&str]) -> MoveDirective {
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(actor_carrier_cue(actor, "move", tokens));
    match rx.try_recv() {
        Ok(PositionCue::Move(directive)) => directive,
        other => panic!("move キャリアは sink→channel を通って移動の指令を送出する: {other:?}"),
    }
}

/// fixture `\1\![move,-353,,,0,base,base]` を sink→channel→apply の一続きに通し、
/// 対象窓の移動・バルーン随伴 offset 維持・対象と基準の `Anchored` ビット同一を同時に固定する。
#[test]
fn pipeline_sink_to_apply_moves_keeps_balloon_and_anchored() {
    let (mut world, gw) = move_world();
    let target = gw.char_window(1).unwrap();
    let base = gw.char_window(0).unwrap();
    let balloon = gw.balloon_window(1).unwrap();

    let offset = world
        .get::<BalloonFollow>(target)
        .copied()
        .expect("target に BalloonFollow")
        .offset();
    let target_anchored_before = world
        .get::<Anchored>(target)
        .copied()
        .expect("target に Anchored（spawn が付与）");
    let base_anchored_before = world
        .get::<Anchored>(base)
        .copied()
        .expect("base に Anchored（spawn が付与）");

    let directive = directive_via_sink("1", &["-353", "", "", "0", "base", "base"]);
    assert_eq!(
        directive.scope, 1,
        "sink が cue.actor（\\1）から scope=1 を導出する（pipeline 前段）"
    );
    assert!(
        apply_move_directive(&mut world, &directive, ScaleRatio::ONE),
        "対象・基準窓が揃うので pipeline 越しの適用は成功する"
    );

    // fixture 検算座標（x'=1000+200−353−150=697・y=Fix は現状維持 800）。
    assert_eq!(
        pos_of(&world, target),
        Point { x: 697, y: 800 },
        "sink→channel→apply の一続きで対象窓が fixture 座標へ移動する"
    );

    // バルーン随伴 offset 維持。
    let cpos = pos_of(&world, target);
    let bpos = pos_of(&world, balloon);
    assert_eq!(
        bpos.x - cpos.x,
        offset.x,
        "offset x が pipeline 越しに維持される"
    );
    assert_eq!(
        bpos.y - cpos.y,
        offset.y,
        "offset y が pipeline 越しに維持される"
    );

    // Anchored ビット同一（sink 経由でも永続確定系へ触れない）。
    assert_eq!(
        world.get::<Anchored>(target).copied(),
        Some(target_anchored_before),
        "対象窓の Anchored はビット同一（pipeline 越しに永続確定系へ触れない）"
    );
    assert_eq!(
        world.get::<Anchored>(base).copied(),
        Some(base_anchored_before),
        "基準窓の Anchored もビット同一"
    );
}

/// 対象 scope 不在の move キャリアを sink→channel→apply に通しても、apply は warn＋`false` を返し
/// World は不変。sink 段は正常に directive を送出し、縮退は apply 段でのみ起きる。
#[test]
fn pipeline_target_absent_warns_false_no_mutation() {
    // GhostWindows に scope0（基準）のみ。fixture の対象 scope1 は不在。
    let placements = vec![placement(
        0,
        1000,
        500,
        400,
        687,
        PointPx { x: 285, y: -19 },
    )];
    let mut world = World::new();
    let gw = spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles([(0, "a".to_string())]),
    );
    give_fake_handles(&mut world, &gw);
    let base = gw.char_window(0).unwrap();
    let before = pos_of(&world, base);
    let base_anchored_before = world
        .get::<Anchored>(base)
        .copied()
        .expect("base に Anchored");

    let directive = directive_via_sink("1", &["-353", "", "", "0", "base", "base"]);
    assert_eq!(
        directive.scope, 1,
        "sink 段は対象不在を知らず directive を送出する"
    );

    assert!(
        !apply_move_directive(&mut world, &directive, ScaleRatio::ONE),
        "対象 scope1 不在は pipeline 越しでも warn＋false（R5.5）"
    );
    assert_eq!(
        pos_of(&world, base),
        before,
        "適用不成立で基準窓の位置は不変（no mutation）"
    );
    assert_eq!(
        world.get::<Anchored>(base).copied(),
        Some(base_anchored_before),
        "適用不成立で Anchored も不変（永続確定系へ触れない・R6/9.5）"
    );
}
