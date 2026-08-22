// =============================================================================
// apply_move_directive（UI スレッド適用）統合檻（task 7.4・R5.1/5.3/5.5/R6/9.5）
//
// 偽装境界: 実 HWND なしの headless World（`fake_handle` パターン・follow.rs 流儀）で
// `spawn_ghost_windows` により実 GhostWindows＋char/balloon 窓（Anchored/WindowPos/
// BalloonFollow 付き）を組み、既知 WindowPos に対し apply_move_directive を駆動して
// fixture 検算式どおりの物理座標・バルーン随伴 offset 維持・対象不在 warn+false・
// Anchored ビット同一（第二位置ライター非混入の構造檻）を固定する。
// =============================================================================

use super::*;
use areka_emo_compose::ScaleRatio;
use bevy_ecs::prelude::{Entity, World};
use windows::Win32::Foundation::{HINSTANCE, HWND};

use crate::placement::follow::{Anchored, BalloonFollow};
use crate::placement::resolver::{Anchor, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};
use wintf::ecs::{Point, WindowHandle, WindowPos};

/// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム）。
fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

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
        // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 各窓へ偽 WindowHandle を付与する（move_window_to の反映口 enqueue_window_set_pos が
/// WindowPos を書ける条件＝WindowHandle 実在）。
fn attach_fake_handles(world: &mut World, gw: &GhostWindows) {
    let mut raw = 0x100usize;
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).unwrap(),
            gw.balloon_window(scope).unwrap(),
        ] {
            world.entity_mut(e).insert(fake_handle(raw));
            raw += 0x10;
        }
    }
}

/// base=scope0 (1000,500,400,687)・target=scope1 (1200,800,300,434) の headless World＋
/// GhostWindows（全窓に偽 WindowHandle 付与済み）。
fn move_world() -> (World, GhostWindows) {
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
    attach_fake_handles(&mut world, &gw);
    (world, gw)
}

fn pos_of(world: &World, e: Entity) -> Point {
    world
        .get::<WindowPos>(e)
        .expect("WindowPos があるはず")
        .position
        .expect("position があるはず")
}

/// fixture `\1\![move,-353,,,0,base,base]`（scope 1・base scope0）。
fn fixture_directive() -> MoveDirective {
    parse_move_directive(
        1,
        &["-353", "", "", "0", "base", "base"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
    .expect("fixture move は Ok")
}

/// R5.1: fixture の即時移動が対象窓へ反映される。
/// x' = pos0.x + w0/2 − 353 − w1/2 = 1000 + 200 − 353 − 150 = 697・y は Fix ゆえ現状維持（800）。
#[test]
fn apply_moves_target_to_fixture_position() {
    let (mut world, gw) = move_world();
    let target = gw.char_window(1).unwrap();

    assert!(
        apply_move_directive(&mut world, &fixture_directive(), ScaleRatio::ONE),
        "対象・基準窓が揃うので適用は成功する"
    );
    assert_eq!(
        pos_of(&world, target),
        Point { x: 697, y: 800 },
        "x'=pos0.x+w0/2−353−w1/2=697・y=Fix は現状維持"
    );
}

/// R5.3: バルーン随伴——移動後も balloon_pos − char_pos ≡ offset（move_window_to が内包）。
#[test]
fn apply_keeps_balloon_offset() {
    let (mut world, gw) = move_world();
    let target = gw.char_window(1).unwrap();
    let balloon = gw.balloon_window(1).unwrap();
    let offset = world
        .get::<BalloonFollow>(target)
        .copied()
        .expect("target に BalloonFollow")
        .offset;

    assert!(apply_move_directive(
        &mut world,
        &fixture_directive(),
        ScaleRatio::ONE
    ));

    let cpos = pos_of(&world, target);
    let bpos = pos_of(&world, balloon);
    assert_eq!(bpos.x - cpos.x, offset.x, "offset x が移動後も維持される");
    assert_eq!(bpos.y - cpos.y, offset.y, "offset y が移動後も維持される");
}

/// R5.5: 対象 scope の窓が GhostWindows に無い→warn＋false・state 不変（panic しない）。
#[test]
fn apply_target_absent_returns_false_without_mutation() {
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
    attach_fake_handles(&mut world, &gw);
    let base = gw.char_window(0).unwrap();
    let before = pos_of(&world, base);

    assert!(
        !apply_move_directive(&mut world, &fixture_directive(), ScaleRatio::ONE),
        "対象 scope1 不在は false（R5.5）"
    );
    assert_eq!(pos_of(&world, base), before, "適用不成立で state は不変");
}

/// R6/9.5: apply の前後で対象・基準窓の `Anchored`（ドラッグ確定系の単一真実源）が
/// ビット同一であること（apply が move_window_to のみを呼び第二の位置ライターを混入しない構造檻）。
#[test]
fn apply_leaves_anchored_bit_identical() {
    let (mut world, gw) = move_world();
    let target = gw.char_window(1).unwrap();
    let base = gw.char_window(0).unwrap();
    let target_before = world
        .get::<Anchored>(target)
        .copied()
        .expect("target に Anchored（spawn が付与）");
    let base_before = world
        .get::<Anchored>(base)
        .copied()
        .expect("base に Anchored（spawn が付与）");

    assert!(apply_move_directive(
        &mut world,
        &fixture_directive(),
        ScaleRatio::ONE
    ));

    assert_eq!(
        world.get::<Anchored>(target).copied(),
        Some(target_before),
        "対象窓の Anchored はビット同一（永続確定系へ触れない・R6/9.5）"
    );
    assert_eq!(
        world.get::<Anchored>(base).copied(),
        Some(base_before),
        "基準窓の Anchored もビット同一"
    );
}

/// R5.5: 非スコープ基準（M1 非実導出）は warn＋false（座標算出へ進まない）。
#[test]
fn apply_non_scope_base_returns_false() {
    let (mut world, gw) = move_world();
    let target = gw.char_window(1).unwrap();
    let before = pos_of(&world, target);
    // base=screen（非スコープ・M1 縮退）。x=Px(-353)・y=Fix。
    let directive = parse_move_directive(
        1,
        &["-353", "", "", "screen", "base", "base"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
    .expect("screen 基準は語彙保持で Ok");
    assert_eq!(directive.base, MoveBase::Screen);

    assert!(
        !apply_move_directive(&mut world, &directive, ScaleRatio::ONE),
        "非スコープ基準は M1 非実導出のため false（R5.5）"
    );
    assert_eq!(pos_of(&world, target), before, "適用不成立で対象窓は不動");
}

// -------------------------------------------------------------------------
// task 10.4 — MoveCueSink→channel→apply_move_directive→move_window_to の
// 末端 pipeline を **一続きの unit** として駆動する統合檻（R5.3/5.5/6.1/6.2/9.5）。
//
// 既存檻との差分（重複回避）:
//   - 7.4 `apply_move_tests`: `apply_move_directive` を **単体**で駆動（directive を
//     parse で直接構築・sink/channel を経ない apply-in-isolation）。
//   - 9.1 `move_cue_sink_reaches_emo2_wiring_receiver`（frame.rs）: sink→channel→
//     `drain_move_directives` まで（**apply を呼ばない**＝窓を動かさない）。
//   - 9.2 `run_move_drain_phase_applies_directive_when_ghost_windows_present`（frame.rs）:
//     **生 `tx.send(directive)`**（sink を経ない）→drain→apply（channel→apply 結線）。
//   - 9.3 `spine_move_cue_drives_window_move_end_to_end`（spine.rs）: full async spine
//     （cue script→compile→dispatch→broadcast→実 MoveCueSink→channel→drain→apply）だが
//     **座標のみ** assert（balloon 随伴・Anchored ビット同一・対象不在は檻に入れていない）。
//
// 10.4 の固有価値: `MoveCueSink::emit(キャリア cue)` の **名前選別＋actor→scope＋parse＋
// channel handoff** と `apply_move_directive` の **座標算出＋move_window_to 反映** が
// headless 単一スレッドで **正しく合成する** ことを、R5.3（balloon 随伴 offset 維持）・
// R6/9.5（Anchored ビット同一）・R5.5（対象不在 warn+false・no mutation）まで **pipeline 越し**
// に固定する（9.3 の full boot を要さない move-only の焦点檻）。

use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use std::sync::mpsc::channel;

/// `\![move]` キャリア cue を **実 `MoveCueSink`→mpsc channel** に通し、drain した
/// `MoveDirective` を返す（sink の名前選別＋`cue.actor`→scope＋`parse_move_directive`＋
/// channel handoff を実経路で通す＝末端 pipeline の前段そのもの）。
fn directive_via_sink(actor: &str, tokens: &[&str]) -> MoveDirective {
    let (tx, rx) = channel::<MoveDirective>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::command_carrier(
            "move",
            tokens.iter().map(|s| s.to_string()).collect(),
        ),
        duration: 0.0,
    });
    rx.try_recv()
        .expect("move キャリアは sink→channel を通って MoveDirective を送出する")
}

/// R5.3/6.1/6.2/9.5: fixture `\1\![move,-353,,,0,base,base]` を **sink→channel→apply**
/// の一続きに通し、①対象窓が fixture 検算座標へ移動②バルーン随伴 offset 維持③対象・基準の
/// `Anchored` がビット同一——を **pipeline 越し** に同時固定する（sink 名前選別＋parse＋
/// handoff と apply の座標算出＋move_window_to が正しく合成する証明）。
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
        .offset;
    let target_anchored_before = world
        .get::<Anchored>(target)
        .copied()
        .expect("target に Anchored（spawn が付与）");
    let base_anchored_before = world
        .get::<Anchored>(base)
        .copied()
        .expect("base に Anchored（spawn が付与）");

    // 末端 pipeline を一続きに駆動: キャリア cue→MoveCueSink::emit→channel→drain→apply。
    let directive = directive_via_sink("1", &["-353", "", "", "0", "base", "base"]);
    assert_eq!(
        directive.scope, 1,
        "sink が cue.actor（\\1）から scope=1 を導出する（pipeline 前段）"
    );
    assert!(
        apply_move_directive(&mut world, &directive, ScaleRatio::ONE),
        "対象・基準窓が揃うので pipeline 越しの適用は成功する"
    );

    // ① fixture 検算座標（x'=1000+200−353−150=697・y=Fix は現状維持 800）。
    assert_eq!(
        pos_of(&world, target),
        Point { x: 697, y: 800 },
        "sink→channel→apply の一続きで対象窓が fixture 座標へ移動する"
    );

    // ② バルーン随伴 offset 維持（R5.3）——pipeline 越しに balloon_pos − char_pos ≡ offset。
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

    // ③ Anchored ビット同一（R6.1/6.2/9.5）——sink 経由でも永続確定系へ触れない。
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

/// R5.5: 対象 scope 不在の move キャリアを **sink→channel→apply** に通しても、apply は
/// warn＋`false` を返し World は不変（Anchored 含む）——非 panic・talk を殺さない縮退が
/// pipeline 越しに成立する。sink 段は正常に scope=1 の directive を送出し（名前選別・parse は
/// 通る）、対象不在の縮退は apply 段でのみ起きる分離を固定する。
#[test]
fn pipeline_target_absent_warns_false_no_mutation() {
    // GhostWindows に scope0（基準）のみ挿入・fixture の対象 scope1 は不在。
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
    attach_fake_handles(&mut world, &gw);
    let base = gw.char_window(0).unwrap();
    let before = pos_of(&world, base);
    let base_anchored_before = world
        .get::<Anchored>(base)
        .copied()
        .expect("base に Anchored");

    // sink 段は正常送出（actor=1・parse は通る）。対象不在の縮退は apply 段でのみ起きる。
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
