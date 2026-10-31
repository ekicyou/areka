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

use super::super::test_support::capture_logs;
use super::test_support::{
    OFFSCREEN_PULL_TAG, drag_end_event_at, drag_event, fake_handle, guard_events, position_of,
    single_monitor_snapshot, size_of, window_pos_at, window_pos_sized,
};
use super::{
    Anchored, BalloonFollow, OffsetBase, apply_balloon_shift, apply_char_shift, move_window_to,
    on_balloon_drag, on_balloon_drag_end, on_char_drag_end,
};
use crate::placement::resolver::{Anchor, PointPx};

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

/// 作者の大きなずれを自分の寄せで作業領域へ戻すのは意図どおりで、勝手に飛んだ窓ではない。
/// 揃えの有無を問わず `[visibility-guard] OffscreenPull` を出さない（task 8.3 の実機で見つけた件・要件 9.4）。
#[test]
fn a_huge_shift_pulled_back_by_our_own_clamp_does_not_warn_offscreen_pull() {
    for anchor in [Anchor::Bottom, Anchor::Free] {
        let (mut world, window) = char_world(anchor, 100, BOTTOM_Y);

        let (applied, events) =
            capture_logs(|| apply_char_shift(&mut world, window, (0, 9999), ScaleRatio::ONE));

        assert!(applied, "{anchor:?}: 当てる");
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
    // 位置はあるが寸が無い。
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let no_size = world
        .spawn((fake_handle(0x1000), window_pos_at(100, 100)))
        .id();
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
    assert!(apply_char_shift(
        &mut world,
        window,
        (10, 0),
        ScaleRatio::ONE
    ));
    assert!(move_window_to(&mut world, window, 700, 450));
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
