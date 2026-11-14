//! 面の `move` のずれを窓の位置へ当てる口の檻（seriko-pattern-calls・task 6.1・要件 7.1・7.3・
//! 7.5・7.8・8.1）。
//!
//! 作業領域は 1 枚 `0..1920 × 0..1043`、キャラ窓は 200×300。窓の寸は書かない（7.5）ので、
//! どの件でも寸は変わらないことを見る。

use areka_emo_compose::ScaleRatio;
use bevy_ecs::prelude::*;
use wintf::ecs::{Point, SizeI};

use wintf::ecs::WindowPos;
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::window::transition_diag::FIELD_ORIGIN;

use super::move_shift::MoveShiftApplied;
use super::test_support::{
    OFFSCREEN_PULL_TAG, assert_deferred, drag_end_event_at, drag_event, dragging_state,
    fake_handle, guard_events, position_of, single_monitor_snapshot, size_of, window_pos_at,
    window_pos_sized,
};
use super::{
    Anchored, BalloonFollow, DpiSyncHold, MonitorSnapshot, OffsetBase, PlacementRoute,
    apply_balloon_shift, apply_char_shift, begin_char_drag, move_window_to, on_balloon_drag,
    on_balloon_drag_end, on_char_drag_end, rebase_shift_axes, shift_is_current,
};
use crate::resolver::{Anchor, PointPx};
use log_capture_kit::{capture, expect_one};

const W: i32 = 200;
const H: i32 = 300;
/// 下揃えの y（作業領域の下端 1043 − 300）。
const BOTTOM_Y: i32 = 1043 - H;

fn k(num: u32, den: u32) -> ScaleRatio {
    ScaleRatio::new(num, den).expect("正の倍率")
}

/// キャラ窓 1 枚（バルーンなし）。
fn char_world(anchor: Anchor, x: i32, y: i32) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(x, y, W, H),
            Anchored(anchor),
        ))
        .id();
    (world, window)
}

/// 倍率 k をかける（揃えない置き方では x・y とも動く）。
#[test]
fn the_shift_is_multiplied_by_the_scale() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);

    assert!(apply_char_shift(&mut world, window, (10, 20), k(3, 2)));

    assert_eq!(position_of(&world, window), Point { x: 115, y: 130 });
    assert_eq!(
        size_of(&world, window),
        SizeI::new(W, H),
        "窓の寸は変えない"
    );
}

/// 下揃えでは y が揃いに負ける（下へのずれは見えない・M4 の付記 2）。x は動く。
#[test]
fn bottom_aligned_y_loses_to_the_alignment() {
    let (mut world, window) = char_world(Anchor::Bottom, 100, BOTTOM_Y);

    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 20),
        ScaleRatio::ONE
    ));

    assert_eq!(
        position_of(&world, window),
        Point {
            x: 110,
            y: BOTTOM_Y
        }
    );
}

/// `move,0,10,0,9999` の形: 揃えない置き方でも作業領域の中へ寄る。0 で元へ戻る。
#[test]
fn a_huge_shift_is_pulled_into_the_work_area_and_zero_returns_to_base() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);

    assert!(apply_char_shift(
        &mut world,
        window,
        (0, 10),
        ScaleRatio::ONE
    ));
    assert_eq!(position_of(&world, window), Point { x: 100, y: 110 });

    assert!(apply_char_shift(
        &mut world,
        window,
        (0, 9999),
        ScaleRatio::ONE
    ));
    assert_eq!(
        position_of(&world, window),
        Point {
            x: 100,
            y: BOTTOM_Y
        },
        "作業領域の下端へ寄る"
    );

    assert!(apply_char_shift(
        &mut world,
        window,
        (0, 0),
        ScaleRatio::ONE
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 100, y: 100 },
        "当てた分（寄せで縮んだ分も）を引いて元へ戻る"
    );
}

/// 作業領域の右端からはみ出して置いた窓（x=1820・幅 200）は、寄せる先が作業領域と元の矩形を
/// 合わせた範囲なので、ずれ (5,0) でも (0,0) でも中へ引き寄せられない（監査の指摘 5）。
#[test]
fn an_edge_window_is_not_pulled_inward_by_a_shift() {
    let (mut world, window) = char_world(Anchor::Free, 1820, 100);

    assert!(apply_char_shift(
        &mut world,
        window,
        (5, 0),
        ScaleRatio::ONE
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 1820, y: 100 },
        "元の矩形の範囲までは寄せない（右へのずれは元の右端で止まる）"
    );

    assert!(apply_char_shift(
        &mut world,
        window,
        (0, 0),
        ScaleRatio::ONE
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 1820, y: 100 },
        "0 で元の位置のまま"
    );
}

/// 作者の大きなずれを自分の寄せで作業領域へ戻すのは意図どおりで、勝手に飛んだ窓ではない。
/// 揃えの有無を問わず `[visibility-guard] OffscreenPull` を出さない（task 8.3 の実機で見つけた件・要件 9.4）。
#[test]
fn a_huge_shift_pulled_back_by_our_own_clamp_does_not_warn_offscreen_pull() {
    for anchor in [Anchor::Bottom, Anchor::Free] {
        let (mut world, window) = char_world(anchor, 100, BOTTOM_Y);

        let (applied, events) =
            capture(|| apply_char_shift(&mut world, window, (0, 9999), ScaleRatio::ONE));

        assert!(applied, "{anchor:?}: 当てる");
        // 捕捉の陽性の対: この書込の遷移の行（経路 PatternMove）は 1 行在る（0 行が捕捉の
        // 取りこぼしでない証・印の無い窓なので窓移動レコードは出ない）。
        expect_one(
            &events,
            &format!("{FIELD_ORIGIN}={}", PlacementRoute::PatternMove.as_str()),
        );
        assert!(
            guard_events(&events, OFFSCREEN_PULL_TAG).is_empty(),
            "{anchor:?}: 自分の寄せを画面外からの引き寄せとして報告している: {events:?}"
        );
        assert_eq!(
            position_of(&world, window),
            Point {
                x: 100,
                y: BOTTOM_Y
            },
            "{anchor:?}: 作業領域の中に収まる"
        );
    }
}

/// `i64` の極端な値でも溢れずに作業領域の中へ寄る（`i32` へ飽和）。
#[test]
fn extreme_values_saturate_without_overflow() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);

    assert!(apply_char_shift(
        &mut world,
        window,
        (i64::MIN, i64::MAX),
        k(3, 2)
    ));

    assert_eq!(position_of(&world, window), Point { x: 0, y: BOTTOM_Y });
}

/// ほかの書き手が窓を動かしても、ずれは動いた後の窓に残り、0 で動いた後の元へ戻る。
#[test]
fn another_writer_moves_the_base_and_zero_returns_to_the_new_base() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);

    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 20),
        ScaleRatio::ONE
    ));
    assert!(move_window_to(&mut world, window, 510, 420));

    assert!(apply_char_shift(
        &mut world,
        window,
        (0, 0),
        ScaleRatio::ONE
    ));
    assert_eq!(position_of(&world, window), Point { x: 500, y: 400 });
}

/// 窓の位置・作業領域が無ければ当てない（次のフレームで当て直す）。
#[test]
fn missing_inputs_do_not_apply() {
    // 位置はあるが寸が無い（`None`）。
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let mut wp = window_pos_at(100, 100);
    wp.size = None;
    let no_size = world.spawn((fake_handle(0x1000), wp)).id();
    assert!(!apply_char_shift(
        &mut world,
        no_size,
        (10, 20),
        ScaleRatio::ONE
    ));
    assert_eq!(position_of(&world, no_size), Point { x: 100, y: 100 });

    // 作業領域が無い。
    let mut world = World::new();
    let no_area = world
        .spawn((fake_handle(0x1000), window_pos_sized(100, 100, W, H)))
        .id();
    assert!(!apply_char_shift(
        &mut world,
        no_area,
        (10, 20),
        ScaleRatio::ONE
    ));
    assert_eq!(position_of(&world, no_area), Point { x: 100, y: 100 });

    // 窓が無い（破棄済み）。
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let gone = world.spawn(window_pos_sized(0, 0, W, H)).id();
    world.despawn(gone);
    assert!(!apply_char_shift(
        &mut world,
        gone,
        (10, 20),
        ScaleRatio::ONE
    ));
    assert!(!apply_balloon_shift(
        &mut world,
        gone,
        (10, 20),
        ScaleRatio::ONE
    ));

    // 書けない（`WindowHandle` が無い）: 当てた分の記録を書かない（指摘 11 ⑷）。
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let refused = world.spawn(window_pos_sized(100, 100, W, H)).id();
    let (ok, events) = capture(|| apply_char_shift(&mut world, refused, (10, 20), ScaleRatio::ONE));
    assert!(!ok);
    assert_eq!(position_of(&world, refused), Point { x: 100, y: 100 });
    assert!(world.get::<MoveShiftApplied>(refused).is_none());
    assert_skipped(&events, "write_refused");

    // 持ち主のキャラ窓が無いバルーン: 記録を書かない（指摘 11 ⑷）。
    let mut world = World::new();
    let orphan = world
        .spawn((fake_handle(0x2000), window_pos_sized(0, 0, 150, 100)))
        .id();
    let (ok, events) =
        capture(|| apply_balloon_shift(&mut world, orphan, (10, 20), ScaleRatio::ONE));
    assert!(!ok);
    assert!(world.get::<MoveShiftApplied>(orphan).is_none());
    assert_skipped(&events, "no_char_window_position");
}

/// `WindowHandle` がまだ無いバルーン（窓の生成前）へは当てたと記録せず、理由つきの `warn!` で
/// 戻る。窓ができた後の当て直しでずれが当たる（記録が無いので毎フレームの相が当て直す）。
#[test]
fn a_balloon_without_its_window_is_not_recorded_until_the_window_exists() {
    let (mut world, window) = char_world(Anchor::Free, 600, 500);
    let balloon = world.spawn(window_pos_sized(0, 0, 150, 100)).id();
    let offset = PointPx { x: -170, y: -40 };
    world
        .entity_mut(window)
        .insert(BalloonFollow::new(balloon, OffsetBase::unpinned(offset)));

    let (ok, events) = capture(|| apply_balloon_shift(&mut world, balloon, (4, 6), k(3, 2)));
    assert!(!ok);
    assert!(world.get::<MoveShiftApplied>(balloon).is_none());
    assert!(!shift_is_current(&world, balloon, (4, 6), k(3, 2)));
    assert_skipped(&events, "no_balloon_window");

    world.entity_mut(balloon).insert(fake_handle(0x2000));
    assert!(apply_balloon_shift(&mut world, balloon, (4, 6), k(3, 2)));
    assert!(shift_is_current(&world, balloon, (4, 6), k(3, 2)));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 600 - 170 + 6,
            y: 500 - 40 + 9
        }
    );
    let _residue = drain_window_pos_commands();
}

/// `move_shift_skipped` の `warn!` が理由 `reason` で 1 行出ている。
fn assert_skipped(events: &[log_capture_kit::CapturedEvent], reason: &str) {
    let skipped: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("move_shift_skipped"))
        .collect();
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, tracing::Level::WARN);
    assert_eq!(skipped[0].field_str("reason"), Some(reason), "{events:?}");
}

/// バルーンのずれは倍率をかけてキャラ窓の今の位置から追い直し、キャラ窓が動いても残る。
#[test]
fn the_balloon_follows_with_its_own_shift() {
    let (mut world, window) = char_world(Anchor::Free, 600, 500);
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_sized(0, 0, 150, 100)))
        .id();
    let offset = PointPx { x: -170, y: -40 };
    world
        .entity_mut(window)
        .insert(BalloonFollow::new(balloon, OffsetBase::unpinned(offset)));

    assert!(apply_balloon_shift(&mut world, balloon, (4, 6), k(3, 2)));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 600 - 170 + 6,
            y: 500 - 40 + 9
        }
    );

    // キャラ窓のずれ・ほかの書き手の移動でも、バルーンのずれは足されたまま追う。
    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 0),
        ScaleRatio::ONE
    ));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 610 - 170 + 6,
            y: 500 - 40 + 9
        }
    );
    assert!(move_window_to(&mut world, window, 700, 450));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 700 - 170 + 6,
            y: 450 - 40 + 9
        }
    );

    // 0 で相対の位置へ戻る。
    assert!(apply_balloon_shift(&mut world, balloon, (0, 0), k(3, 2)));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 700 - 170,
            y: 450 - 40
        }
    );
}

/// バルーンのずれが `i64` の極端な値でも、追う足し算が溢れない（飽和する）。
#[test]
fn extreme_balloon_shift_does_not_overflow() {
    let (mut world, window) = char_world(Anchor::Free, 600, 500);
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_sized(0, 0, 150, 100)))
        .id();
    // 両軸とも足すと端を越える向きの相対位置。
    let offset = PointPx { x: 170, y: -600 };
    world
        .entity_mut(window)
        .insert(BalloonFollow::new(balloon, OffsetBase::unpinned(offset)));

    assert!(apply_balloon_shift(
        &mut world,
        balloon,
        (i64::MAX, i64::MIN),
        ScaleRatio::ONE
    ));
    assert_eq!(
        position_of(&world, balloon).y,
        i32::MIN,
        "500 − 600 + i64::MIN は i32 の下端で止まる"
    );
    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 0),
        ScaleRatio::ONE
    ));
    assert!(move_window_to(&mut world, window, 700, 450));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: i32::MAX,
            y: i32::MIN
        },
        "追う足し算は両軸とも i32 の端で止まる（回り込まない）"
    );
}

/// キャラ窓のドラッグの終わりで当てた分は 0 になり、利用者が置いた所が新しい元の位置になる
/// （task 6.2・要件 7.5）。
#[test]
fn char_drag_end_resets_the_applied_shift() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);
    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 20),
        ScaleRatio::ONE
    ));
    // 利用者がずれたままの窓を 300,200 へ置いた（wndproc が書いた最終位置）。
    world.get_mut::<WindowPos>(window).unwrap().position = Some(Point { x: 300, y: 200 });
    on_char_drag_end(
        &mut world,
        window,
        window,
        &Phase::Bubble(drag_end_event_at(window, (0, 0))),
    );

    // 同じずれを当て直しても、置いた所から引かず・足さない（0 を当てれば置いた所のまま）。
    assert!(apply_char_shift(
        &mut world,
        window,
        (0, 0),
        ScaleRatio::ONE
    ));
    assert_eq!(position_of(&world, window), Point { x: 300, y: 200 });
    let _residue = drain_window_pos_commands();
}

/// バルーン窓のドラッグの終わりで当てた分は 0 になり、追い直してもずれを二重に足さない
/// （task 6.2・設計の検証の指摘 2）。
#[test]
fn balloon_drag_end_resets_the_shift_without_double_counting() {
    let (mut world, window) = char_world(Anchor::Free, 600, 500);
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_sized(0, 0, 150, 100)))
        .id();
    world.entity_mut(window).insert(BalloonFollow::new(
        balloon,
        OffsetBase::unpinned(PointPx { x: -170, y: -40 }),
    ));
    assert!(apply_balloon_shift(
        &mut world,
        balloon,
        (4, 6),
        ScaleRatio::ONE
    ));

    // 利用者がバルーンを 300,300 へ置いた（相対の位置はずれを含んだ窓の位置から作られる）。
    world.get_mut::<WindowPos>(balloon).unwrap().position = Some(Point { x: 300, y: 300 });
    on_balloon_drag(
        &mut world,
        balloon,
        balloon,
        &Phase::Bubble(drag_event(balloon)),
    );
    on_balloon_drag_end(
        &mut world,
        balloon,
        balloon,
        &Phase::Bubble(drag_end_event_at(balloon, (0, 0))),
    );

    // キャラ窓が動いて追い直しても、置いた相対の位置のまま（ずれ 4,6 をもう 1 度足さない）。
    assert!(move_window_to(&mut world, window, 700, 450));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 300 + 100,
            y: 300 - 50
        }
    );
    let _residue = drain_window_pos_commands();
}

/// 毎フレームのずれの相（本体の `runtime/frame/move_shift.rs`）の判定をなぞる: 今の値でなく札も
/// 無ければ当てる。
fn run_frame_phase(world: &mut World, window: Entity, want: (i64, i64)) {
    if !shift_is_current(world, window, want, ScaleRatio::ONE)
        && world.get::<DpiSyncHold>(window).is_none()
    {
        apply_char_shift(world, window, want, ScaleRatio::ONE);
    }
}

/// ずれ (10,20) を当てた後に `\![move]` が x を絶対の位置 500 へ書き、整合待ちの札が付いた窓。
fn held_after_absolute_x_move() -> (World, Entity) {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);
    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 20),
        ScaleRatio::ONE
    ));
    world
        .entity_mut(window)
        .insert(DpiSyncHold { since_frame: 3 });
    // `\![move]` は見送らない書き手（`MoveCue`・札の監視の対象外）。
    assert!(move_window_to(&mut world, window, 500, 120));
    let _residue = drain_window_pos_commands();
    (world, window)
}

/// 札のある窓で絶対の軸を当て直しても書かず（監視が鳴らない）、「今の値」は偽で残る。札を外すと
/// 当て直しが当たる（監査の指摘 1・要件 3.1）。
#[test]
fn rebase_on_a_held_window_waits_and_reapplies_after_the_hold_lifts() {
    let (mut world, window) = held_after_absolute_x_move();

    rebase_shift_axes(&mut world, window, true, false);

    assert!(drain_window_pos_commands().is_empty(), "札のある窓へ書いた");
    assert!(
        !shift_is_current(&world, window, (10, 20), ScaleRatio::ONE),
        "当て直しを待っているのに今の値と見なした"
    );
    world.entity_mut(window).remove::<DpiSyncHold>();
    run_frame_phase(&mut world, window, (10, 20));
    assert_eq!(position_of(&world, window), Point { x: 510, y: 120 });
    assert!(shift_is_current(&world, window, (10, 20), ScaleRatio::ONE));
    let _residue = drain_window_pos_commands();
}

/// 作業領域の無い World で当て直しが書けなかったら、「今の値」は偽で残る（次で当て直す）。
#[test]
fn rebase_that_cannot_write_is_not_current() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);
    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 20),
        ScaleRatio::ONE
    ));
    world.remove_resource::<MonitorSnapshot>();

    rebase_shift_axes(&mut world, window, true, false);

    assert!(!shift_is_current(&world, window, (10, 20), ScaleRatio::ONE));
    let _residue = drain_window_pos_commands();
}

/// 当て直し待ちの窓を利用者がドラッグで離すと置いた所が元の位置になり、毎フレームの相は書かない
/// （完了した `seriko-pattern-calls` の task 6.2 を壊さない）。
#[test]
fn drag_end_clears_the_awaiting_reapply_mark() {
    let (mut world, window) = held_after_absolute_x_move();
    rebase_shift_axes(&mut world, window, true, false);
    world.entity_mut(window).remove::<DpiSyncHold>();
    world.get_mut::<WindowPos>(window).unwrap().position = Some(Point { x: 300, y: 200 });

    on_char_drag_end(
        &mut world,
        window,
        window,
        &Phase::Bubble(drag_end_event_at(window, (0, 0))),
    );
    let _residue = drain_window_pos_commands();

    assert!(shift_is_current(&world, window, (10, 20), ScaleRatio::ONE));
    run_frame_phase(&mut world, window, (10, 20));
    assert!(
        drain_window_pos_commands().is_empty(),
        "置いた所へずれを足し直した"
    );
    assert_eq!(position_of(&world, window), Point { x: 300, y: 200 });
}

// -------------------------------------------------------------------------
// 位置が未確定の印（`CW_USEDEFAULT`）・ドラッグ中の窓へは当てない（audit のかけ直し 2 回目 A3・A4）
// -------------------------------------------------------------------------

/// 位置が `CW_USEDEFAULT` の印（窓生成前）のキャラ窓へは当てず、位置が無いのと同じ理由で
/// `warn!` する。持ち主のキャラ窓の位置が印のバルーンも同じ（記録を書かない）。
#[test]
fn a_sentinel_position_is_not_shifted() {
    use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;

    let (mut world, window) = char_world(Anchor::Free, CW_USEDEFAULT, CW_USEDEFAULT);
    let _residue = drain_window_pos_commands();
    let (ok, events) = capture(|| apply_char_shift(&mut world, window, (10, 20), ScaleRatio::ONE));
    assert!(!ok, "印の位置へ当ててはならない");
    assert!(drain_window_pos_commands().is_empty(), "書込の列が空でない");
    assert!(world.get::<MoveShiftApplied>(window).is_none());
    assert_skipped(&events, "no_window_position");

    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_sized(0, 0, 150, 100)))
        .id();
    world.entity_mut(window).insert(BalloonFollow::new(
        balloon,
        OffsetBase::unpinned(PointPx { x: -170, y: -40 }),
    ));
    let (ok, events) =
        capture(|| apply_balloon_shift(&mut world, balloon, (4, 6), ScaleRatio::ONE));
    assert!(!ok, "持ち主の位置が印のバルーンへ当ててはならない");
    assert!(drain_window_pos_commands().is_empty(), "書込の列が空でない");
    assert!(world.get::<MoveShiftApplied>(balloon).is_none());
    assert_skipped(&events, "no_char_window_position");
}

/// ドラッグ中の窓（`DraggingState` が付いている）へは当てず記録も変えない（次のドラッグの
/// 出来事が古いずれで書き戻してちらつかない）。見送りは失敗でないので `warn!` を出さず
/// `debug!` を 1 行残す。ドラッグが終われば当たる。
#[test]
fn a_dragged_window_is_not_shifted_until_the_drag_ends() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_sized(0, 0, 150, 100)))
        .id();
    world.entity_mut(window).insert(BalloonFollow::new(
        balloon,
        OffsetBase::unpinned(PointPx { x: -170, y: -40 }),
    ));
    let _residue = drain_window_pos_commands();

    for (target, shift) in [
        (
            window,
            apply_char_shift as fn(&mut World, Entity, (i64, i64), ScaleRatio) -> bool,
        ),
        (balloon, apply_balloon_shift),
    ] {
        world
            .entity_mut(target)
            .insert(dragging_state((100, 100), (0, 0)));
        let (ok, events) = capture(|| shift(&mut world, target, (10, 20), ScaleRatio::ONE));
        assert!(!ok, "ドラッグ中の窓へ当ててはならない");
        assert!(drain_window_pos_commands().is_empty(), "書込の列が空でない");
        assert!(
            world.get::<MoveShiftApplied>(target).is_none(),
            "記録を変えた"
        );
        assert_deferred(&events, "dragging");

        world
            .entity_mut(target)
            .remove::<wintf::ecs::drag::DraggingState>();
        assert!(
            shift(&mut world, target, (10, 20), ScaleRatio::ONE),
            "ドラッグの後は当たる"
        );
        let _residue = drain_window_pos_commands();
    }
}

/// 多窓のとき dispatch が DragEnd の前に `DraggingState` を落としても、ドラッグの控え
/// （`CharDragSession`）が残る窓はドラッグ中と見て当てない（ずれを記憶に焼き付けない・要件 7.8）。
#[test]
fn a_dragged_window_whose_dragging_state_was_dropped_is_still_deferred() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);
    begin_char_drag(&mut world, window);
    let _residue = drain_window_pos_commands();

    let (ok, events) = capture(|| apply_char_shift(&mut world, window, (10, 20), ScaleRatio::ONE));

    assert!(!ok, "ドラッグの控えが残る窓へ当ててはならない");
    assert!(drain_window_pos_commands().is_empty(), "書込の列が空でない");
    assert!(
        world.get::<MoveShiftApplied>(window).is_none(),
        "記録を変えた"
    );
    assert_deferred(&events, "dragging");
}

/// 持ち主のバルーンがドラッグ中なら、キャラ窓へも当てない（追いがドラッグ中のバルーンへ書いて
/// 跳ばさない）。記録は変えないので、バルーンのドラッグの後に当たる。
#[test]
fn a_char_window_whose_balloon_is_dragged_is_not_shifted() {
    let (mut world, window) = char_world(Anchor::Free, 100, 100);
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_sized(0, 0, 150, 100),
            dragging_state((0, 0), (0, 0)),
        ))
        .id();
    world.entity_mut(window).insert(BalloonFollow::new(
        balloon,
        OffsetBase::unpinned(PointPx { x: -170, y: -40 }),
    ));
    let _residue = drain_window_pos_commands();

    let (ok, events) = capture(|| apply_char_shift(&mut world, window, (10, 20), ScaleRatio::ONE));

    assert!(!ok, "バルーンがドラッグ中のキャラ窓へ当ててはならない");
    assert!(drain_window_pos_commands().is_empty(), "書込の列が空でない");
    assert!(
        world.get::<MoveShiftApplied>(window).is_none(),
        "記録を変えた"
    );
    assert_deferred(&events, "balloon_dragging");

    world
        .entity_mut(balloon)
        .remove::<wintf::ecs::drag::DraggingState>();
    assert!(
        apply_char_shift(&mut world, window, (10, 20), ScaleRatio::ONE),
        "バルーンのドラッグの後は当たる"
    );
}
