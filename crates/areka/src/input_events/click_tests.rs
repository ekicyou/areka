//! 押下の控えと新しい知らせの送出の口の檻（mouse-click-wheel-events 要件 1.5・2.1・3.4・3.5・4.4・design「PressNote・ClickJudge」）。

use super::super::{MouseWiring, RegionSource, on_char_pointer_pressed};
use super::{PressNote, note_press};
use crate::emo2_boot::hit_region::HitRegion;
use crate::placement::spawn::CharWindowMarker;
use areka_kanade::{ClickButton, KanadeMsg, MouseEventKind, MouseInput};
use bevy_ecs::prelude::*;
use std::sync::mpsc;
use wintf::ecs::pointer::{DoubleClick, Phase, PointerState};

fn mock(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: Some("Head".to_owned()),
        surface_point: (x, y),
    }
}

/// 結線済みの World・キャラ窓 1 つ・受信端。
fn wired() -> (World, Entity, mpsc::Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::new(tx, RegionSource::Mock(mock)));
    let entity = world.spawn(CharWindowMarker { scope: 0 }).id();
    (world, entity, rx)
}

fn notes(world: &World, entity: Entity) -> Vec<(ClickButton, PressNote)> {
    let wiring = world.get_non_send::<MouseWiring>().expect("結線済み");
    let mut out: Vec<_> = wiring
        .presses
        .iter()
        .filter(|((e, _), _)| *e == entity)
        .map(|((_, b), n)| (*b, *n))
        .collect();
    out.sort_by_key(|(b, _)| *b as u8);
    out
}

fn plain() -> PressNote {
    PressNote {
        double: false,
        by_box: false,
    }
}

/// その tick に押されたボタンごとに控えが在り、押されていないボタンには無い（要件 2.1）。
#[test]
fn note_press_writes_one_note_per_pressed_button() {
    let (mut world, entity, _rx) = wired();
    let state = PointerState {
        left_down: true,
        middle_down: true,
        xbutton2_down: true,
        ..Default::default()
    };
    note_press(&mut world, entity, &state, false);
    assert_eq!(
        notes(&world, entity),
        vec![
            (ClickButton::Left, plain()),
            (ClickButton::Middle, plain()),
            (ClickButton::XButton2, plain()),
        ]
    );
}

/// ダブルクリックとして届いたボタンだけ「2 回目」。同じ鍵の新しい押下は上書きする（要件 4.4）。
#[test]
fn only_the_double_clicked_button_is_marked_second() {
    let (mut world, entity, _rx) = wired();
    let first = PointerState {
        right_down: true,
        ..Default::default()
    };
    note_press(&mut world, entity, &first, false);
    let second = PointerState {
        left_down: true,
        right_down: true,
        double_click: DoubleClick::Right,
        ..Default::default()
    };
    note_press(&mut world, entity, &second, false);
    assert_eq!(
        notes(&world, entity),
        vec![
            (ClickButton::Left, plain()),
            (
                ClickButton::Right,
                PressNote {
                    double: true,
                    by_box: false
                }
            ),
        ]
    );
}

/// 箱の押下は箱の印（要件 3.4）。
#[test]
fn box_press_is_marked_by_box() {
    let (mut world, entity, _rx) = wired();
    let state = PointerState {
        xbutton1_down: true,
        double_click: DoubleClick::XButton1,
        ..Default::default()
    };
    note_press(&mut world, entity, &state, true);
    assert_eq!(
        notes(&world, entity),
        vec![(
            ClickButton::XButton1,
            PressNote {
                double: true,
                by_box: true
            }
        )]
    );
}

/// 結線の前は控えを書かない（落ちもしない）。
#[test]
fn note_press_before_wiring_is_a_no_op() {
    let mut world = World::new();
    let entity = world.spawn(CharWindowMarker { scope: 0 }).id();
    let state = PointerState {
        left_down: true,
        ..Default::default()
    };
    note_press(&mut world, entity, &state, false);
    assert!(world.get_non_send::<MouseWiring>().is_none());
}

/// 押下の受け手が控えを書く。戻り値は今のまま（単発は false・右ダブルクリックは true）。
#[test]
fn pressed_handler_writes_notes_and_keeps_its_return_value() {
    let (mut world, entity, _rx) = wired();
    let single = PointerState {
        middle_down: true,
        ..Default::default()
    };
    assert!(!on_char_pointer_pressed(
        &mut world,
        entity,
        entity,
        &Phase::Bubble(single)
    ));
    let double = PointerState {
        right_down: true,
        double_click: DoubleClick::Right,
        ..Default::default()
    };
    assert!(on_char_pointer_pressed(
        &mut world,
        entity,
        entity,
        &Phase::Bubble(double)
    ));
    assert_eq!(
        notes(&world, entity),
        vec![
            (
                ClickButton::Right,
                PressNote {
                    double: true,
                    ..plain()
                }
            ),
            (ClickButton::Middle, plain()),
        ]
    );
}

/// 新しい知らせの送出の口: 送れたら true で受信端に届き、受信端が無ければ false（warn だけ）。
#[test]
fn send_mouse_reports_whether_it_was_sent() {
    let (world, _entity, rx) = wired();
    let input = MouseInput {
        scope: 1,
        x: 3,
        y: 4,
        region: None,
        kind: MouseEventKind::Wheel { delta: 120 },
    };
    let wiring = world.get_non_send::<MouseWiring>().expect("結線済み");
    assert!(wiring.send_mouse(input.clone(), "wheel"));
    assert!(matches!(
        rx.try_recv(),
        Ok(KanadeMsg::Mouse(MouseInput {
            scope: 1,
            kind: MouseEventKind::Wheel { delta: 120 },
            ..
        }))
    ));
    drop(rx);
    assert!(!wiring.send_mouse(input, "wheel"));
}
