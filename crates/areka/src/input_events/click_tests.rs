//! 押下の控えと新しい知らせの送出の口の檻（mouse-click-wheel-events 要件 1.5・2.1・3.4・3.5・4.4・design「PressNote・ClickJudge」）。

use super::super::{MouseWiring, RegionSource, on_char_pointer_pressed};
use super::{
    LeftDrag, NotClickReason, PressNote, ReleaseVerdict, judge_release, note_press,
    on_char_pointer_released,
};
use crate::emo2_boot::hit_region::HitRegion;
use crate::placement::spawn::CharWindowMarker;
use areka_kanade::{ClickButton, KanadeMsg, MouseButton, MouseEventKind, MouseInput};
use bevy_ecs::prelude::*;
use std::sync::mpsc;
use windows::Win32::Foundation::HWND;
use wintf::ecs::drag::{end_dragging_on_release, start_dragging, start_preparing};
use wintf::ecs::pointer::{ButtonReleased, DoubleClick, Phase, PointerState};
use wintf::ecs::{PhysicalPoint, SizeI, WindowPos};

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

// --- 解放の判定（task 3.4） -------------------------------------------------

/// 判定の表: 控え無し → 別の窓／箱 → 箱の選択肢／左のドラッグ → ドラッグ／窓の外 → 外／
/// それ以外 → クリック（2 回目の印は控えの `double`）。先の理由が後の理由に勝つ（要件 3.1・3.3・3.4）。
#[test]
fn judge_release_follows_the_table_in_order() {
    use LeftDrag::*;
    use NotClickReason::*;
    let boxed = PressNote {
        double: false,
        by_box: true,
    };
    let second = PressNote {
        double: true,
        by_box: false,
    };
    let not = ReleaseVerdict::NotClick;
    let click = |second_of_double| ReleaseVerdict::Click { second_of_double };
    let cases = [
        (None, Clean, true, not(OtherWindow)),
        (None, Dragged, false, not(OtherWindow)),
        (Some(boxed), Dragged, false, not(BoxChoice)),
        (Some(plain()), Dragged, false, not(Drag)),
        (Some(plain()), Clean, false, not(Outside)),
        (Some(plain()), NotLeft, false, not(Outside)),
        (Some(plain()), Clean, true, click(false)),
        (Some(plain()), NotLeft, true, click(false)),
        (Some(second), Clean, true, click(true)),
        (Some(second), NotLeft, true, click(true)),
    ];
    for (note, left, inside, want) in cases {
        assert_eq!(
            judge_release(note, left, inside),
            want,
            "{note:?} {left:?} inside={inside}"
        );
    }
}

/// 窓の中の点を区別できるサーフェスの点へ写す偽の当たり判定（受信端の座標が surface_point だと読める）。
fn scaled(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: (x < 100).then(|| "Head".to_owned()),
        surface_point: (x * 2, y * 3),
    }
}

/// 結線済みの World・大きさ 200×300 のキャラ窓（scope 1）・受信端。
fn sized() -> (World, Entity, mpsc::Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::new(tx, RegionSource::Mock(scaled)));
    let entity = world
        .spawn((
            CharWindowMarker { scope: 1 },
            WindowPos {
                size: Some(SizeI {
                    width: 200,
                    height: 300,
                }),
                ..Default::default()
            },
        ))
        .id();
    (world, entity, rx)
}

fn all_released() -> ButtonReleased {
    ButtonReleased {
        left: true,
        right: true,
        middle: true,
        xbutton1: true,
        xbutton2: true,
    }
}

fn press(world: &mut World, entity: Entity, state: PointerState) {
    on_char_pointer_pressed(world, entity, entity, &Phase::Bubble(state));
}

fn release(world: &mut World, entity: Entity, x: i32, y: i32, released: ButtonReleased) -> bool {
    let state = PointerState {
        client_point: PhysicalPoint::new(x, y),
        released,
        ..Default::default()
    };
    on_char_pointer_released(world, entity, entity, &Phase::Bubble(state))
}

fn drain(rx: &mpsc::Receiver<KanadeMsg>) -> Vec<MouseInput> {
    rx.try_iter()
        .map(|m| match m {
            KanadeMsg::Mouse(input) => input,
            _ => panic!("マウスの知らせ以外"),
        })
        .collect()
}

fn up(x: i64, y: i64, region: Option<&str>, button: ClickButton, second: bool) -> MouseInput {
    MouseInput {
        scope: 1,
        x,
        y,
        region: region.map(str::to_owned),
        kind: MouseEventKind::ButtonUp {
            button,
            second_of_double: second,
        },
    }
}

/// 5 つのボタンを押して同じ tick に放すと、左・右・中・拡張 1・拡張 2 の順に解放の知らせが届き、
/// 座標は放した点のサーフェスの点・当たり判定はその点のもの。控えは全部取り出される（要件 1.1・1.4・2.1）。
#[test]
fn release_of_five_buttons_sends_five_button_ups_at_the_surface_point() {
    let (mut world, entity, rx) = sized();
    press(
        &mut world,
        entity,
        PointerState {
            left_down: true,
            right_down: true,
            middle_down: true,
            xbutton1_down: true,
            xbutton2_down: true,
            ..Default::default()
        },
    );
    assert!(!release(&mut world, entity, 40, 50, all_released()));
    let head = Some("Head");
    assert_eq!(
        drain(&rx),
        vec![
            up(80, 150, head, ClickButton::Left, false),
            up(80, 150, head, ClickButton::Right, false),
            up(80, 150, head, ClickButton::Middle, false),
            up(80, 150, head, ClickButton::XButton1, false),
            up(80, 150, head, ClickButton::XButton2, false),
        ]
    );
    assert!(notes(&world, entity).is_empty(), "控えは必ず取り出す");
}

/// 押下の無い解放・窓の外で放した解放はどれも送らず、控えはどちらでも取り出される（要件 3.3）。
#[test]
fn release_without_note_or_outside_the_window_sends_nothing() {
    let (mut world, entity, rx) = sized();
    assert!(!release(&mut world, entity, 40, 50, all_released()));
    assert!(drain(&rx).is_empty());

    press(
        &mut world,
        entity,
        PointerState {
            left_down: true,
            middle_down: true,
            ..Default::default()
        },
    );
    // 幅 200 の右の端は外（0 ≤ x < 幅）。
    assert!(!release(&mut world, entity, 200, 50, all_released()));
    assert!(drain(&rx).is_empty());
    assert!(notes(&world, entity).is_empty());
}

/// 左のドラッグの後の解放は送らない。同じ tick の右の解放はクリックのまま（要件 3.1）。
#[test]
fn left_release_after_a_drag_is_not_a_click_but_right_is() {
    let (mut world, entity, rx) = sized();
    press(
        &mut world,
        entity,
        PointerState {
            left_down: true,
            right_down: true,
            ..Default::default()
        },
    );
    start_preparing(entity, PhysicalPoint::new(1, 1), HWND::default());
    start_dragging(PhysicalPoint::new(30, 1));
    end_dragging_on_release(HWND::default(), None);
    // ドラッグの解放と次の左の押下が同じ tick に入っても印は残る（今の状態は見ない）。
    start_preparing(entity, PhysicalPoint::new(2, 2), HWND::default());
    let released = ButtonReleased {
        left: true,
        right: true,
        ..Default::default()
    };
    assert!(!release(&mut world, entity, 150, 50, released));
    assert_eq!(
        drain(&rx),
        vec![up(300, 150, None, ClickButton::Right, false)]
    );
    assert!(notes(&world, entity).is_empty());
}

/// 箱が使った押下の解放は送らない（要件 3.4）。
#[test]
fn release_of_a_box_press_is_not_a_click() {
    let (mut world, entity, rx) = sized();
    note_press(
        &mut world,
        entity,
        &PointerState {
            left_down: true,
            ..Default::default()
        },
        true,
    );
    let released = ButtonReleased {
        left: true,
        ..Default::default()
    };
    assert!(!release(&mut world, entity, 10, 10, released));
    assert!(drain(&rx).is_empty());
}

/// 左の押下 → 解放 → ダブルクリックの押下 → 解放の順に、受信端へ
/// 「解放（ふつう）→ OnMouseDoubleClick → 解放（2 回目）」が届く（要件 4.1・3.2）。
#[test]
fn left_double_click_delivers_up_double_click_up_in_order() {
    let (mut world, entity, rx) = sized();
    let left = ButtonReleased {
        left: true,
        ..Default::default()
    };
    press(
        &mut world,
        entity,
        PointerState {
            left_down: true,
            ..Default::default()
        },
    );
    release(&mut world, entity, 10, 20, left);
    press(
        &mut world,
        entity,
        PointerState {
            client_point: PhysicalPoint::new(10, 20),
            left_down: true,
            double_click: DoubleClick::Left,
            ..Default::default()
        },
    );
    release(&mut world, entity, 10, 20, left);
    assert_eq!(
        drain(&rx),
        vec![
            up(20, 60, Some("Head"), ClickButton::Left, false),
            MouseInput {
                scope: 1,
                x: 20,
                y: 60,
                region: Some("Head".to_owned()),
                kind: MouseEventKind::DoubleClick {
                    button: MouseButton::Left
                },
            },
            up(20, 60, Some("Head"), ClickButton::Left, true),
        ]
    );
}

/// 結線の前は何もしない（落ちもしない）。Tunnel も何もしない。
#[test]
fn release_before_wiring_or_in_tunnel_is_a_no_op() {
    let mut world = World::new();
    let entity = world.spawn(CharWindowMarker { scope: 0 }).id();
    assert!(!release(&mut world, entity, 1, 1, all_released()));

    let (mut world, entity, rx) = sized();
    press(
        &mut world,
        entity,
        PointerState {
            left_down: true,
            ..Default::default()
        },
    );
    let state = PointerState {
        released: all_released(),
        ..Default::default()
    };
    assert!(!on_char_pointer_released(
        &mut world,
        entity,
        entity,
        &Phase::Tunnel(state)
    ));
    assert!(drain(&rx).is_empty());
    assert_eq!(
        notes(&world, entity).len(),
        1,
        "Tunnel では控えを取り出さない"
    );
}
