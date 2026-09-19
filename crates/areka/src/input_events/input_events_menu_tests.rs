//! 右ダブルクリックの預かりと、終了指示に載るスコープのテスト（areka-P0-popup-menu-minimal 6.1）。
//!
//! 合成した `PointerState` で押下ハンドラを直接呼び、kanade 宛ての受信端で「届いた／届かない」を
//! 観測する（実窓も GPU も要らない）。既存の `input_events_tests.rs` と同じ観測方法である。

use super::*;
use crate::placement::spawn::{CharWindowMarker, GhostWindowMarker};
use areka_kanade::MouseInput;
use std::sync::mpsc;
use wintf::ecs::Point;

/// 当たり判定の代役。面座標は窓座標の半分にする——恒等だと「面座標を預かったのか窓座標を
/// 預かったのか」が数値で区別できない。
fn half_scale_bust(_scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope: 0,
        region: Some("Bust".to_string()),
        surface_point: (x / 2, y / 2),
    }
}

/// 代役の当たり判定を持つ `MouseWiring` を挿入した World と、kanade 宛ての受信端を組む。
fn world_with_wiring() -> (World, mpsc::Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::with_clock(
        tx,
        RegionSource::Mock(half_scale_bust),
        Box::new(|| 1000),
    ));
    (world, rx)
}

/// Bubble 相の押下を組む（Ctrl／Shift は呼び手が決める）。
fn pressed(
    x: i32,
    y: i32,
    double_click: DoubleClick,
    ctrl_down: bool,
    shift_down: bool,
) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        double_click,
        ctrl_down,
        shift_down,
        ..Default::default()
    })
}

fn take_pending(world: &mut World) -> Option<PendingDoubleClick> {
    world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .take_pending_right_double_click()
}

fn ghost_count(world: &mut World) -> usize {
    world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .count()
}

/// 右ダブルクリックは SHIORI へ何も届けず、スコープ・面座標・領域を預かる（要件 1.10）。
///
/// 押下ハンドラが即送出へ戻れば `rx` の表明が落ち、預かりを置かなければ `take` の表明が落ちる。
#[test]
fn right_double_click_sends_nothing_and_keeps_the_material() {
    let (mut world, rx) = world_with_wiring();
    let partner = world.spawn(CharWindowMarker { scope: 1 }).id();

    let ev = pressed(14, 8, DoubleClick::Right, false, false);
    assert!(on_char_pointer_pressed(&mut world, partner, partner, &ev));

    assert!(
        rx.try_recv().is_err(),
        "右ダブルクリックは押下の時点では何も送らない"
    );
    let pending = take_pending(&mut world).expect("預かりが残る");
    assert_eq!(pending.scope, 1, "操作した窓のスコープ");
    assert_eq!(pending.surface_pos, (7, 4), "窓座標ではなく面座標");
    assert_eq!(pending.region, Some("Bust".to_string()));
}

/// 預かりは高々 1 件。2 度目の右ダブルクリックは前の預かりを捨てて上書きする。
#[test]
fn second_right_double_click_overwrites_the_pending_one() {
    let (mut world, rx) = world_with_wiring();
    let main = world.spawn(CharWindowMarker { scope: 0 }).id();
    let partner = world.spawn(CharWindowMarker { scope: 1 }).id();

    let first = pressed(14, 8, DoubleClick::Right, false, false);
    assert!(on_char_pointer_pressed(&mut world, main, main, &first));
    let second = pressed(30, 40, DoubleClick::Right, false, false);
    assert!(on_char_pointer_pressed(
        &mut world, partner, partner, &second
    ));

    let pending = take_pending(&mut world).expect("最新の預かりが残る");
    assert_eq!((pending.scope, pending.surface_pos), (1, (15, 20)));
    assert!(
        take_pending(&mut world).is_none(),
        "前の預かりは積まれていない"
    );
    assert!(rx.try_recv().is_err(), "どちらの押下も何も送らない");
}

/// 取り出した後の預かりは空になる。
#[test]
fn take_empties_the_pending_slot() {
    let (mut world, _rx) = world_with_wiring();
    let main = world.spawn(CharWindowMarker { scope: 0 }).id();
    assert!(take_pending(&mut world).is_none(), "押下前は空");

    let ev = pressed(2, 2, DoubleClick::Right, false, false);
    assert!(on_char_pointer_pressed(&mut world, main, main, &ev));
    assert!(take_pending(&mut world).is_some());
    assert!(take_pending(&mut world).is_none(), "2 度目の取り出しは空");
}

/// 預かりを送ると、以前の即送出と同じ形の `OnMouseDoubleClick` がちょうど 1 件届く。
///
/// 以前の即送出は `MouseInput{scope, x, y, region, kind: DoubleClick{Right}}` を面座標で送っていた
/// （kanade がこれを Ref0〜Ref5 へ写し、右ボタンは Ref5＝1 になる）。同じ押下から同じ値が届く。
#[test]
fn sending_the_pending_one_delivers_the_same_message_as_the_old_immediate_path() {
    let (mut world, rx) = world_with_wiring();
    let partner = world.spawn(CharWindowMarker { scope: 1 }).id();
    let ev = pressed(14, 8, DoubleClick::Right, false, false);
    assert!(on_char_pointer_pressed(&mut world, partner, partner, &ev));

    let pending = take_pending(&mut world).expect("預かりが残る");
    world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .send_pending_right_double_click(pending);

    match rx.try_recv().expect("1 件届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            m,
            MouseInput {
                scope: 1,
                x: 7,
                y: 4,
                region: Some("Bust".to_string()),
                kind: MouseEventKind::DoubleClick {
                    button: MouseButton::Right
                },
            }
        ),
        _ => panic!("Mouse(DoubleClick) を期待"),
    }
    assert!(rx.try_recv().is_err(), "ちょうど 1 件");
    assert!(take_pending(&mut world).is_none(), "送った預かりは残らない");
}

/// 結線済みで Shift を伴わない Ctrl＋左ダブルクリックは終了指示を送らず、相方側の窓でも
/// 左ダブルクリックとして届く（要件 5.3・開発者裁定 2026-09-19）。窓は閉じない。
///
/// 相方側（スコープ 1）で見る——Ctrl を見て早期に戻れば `rx` が空で落ち、強制退避の腕へ
/// 落ちれば窓の件数が合わない。
///
/// 「終了指示に操作した窓のスコープが載る」性質はメニュー側の檻が持つ
/// （`menu::mod_wiring_tests::the_close_action_sends_exactly_one_close_request_with_the_window_scope`
/// と `menu::trigger_wired_tests`）。ここでは重ねない。
#[test]
fn wired_ctrl_left_double_click_sends_no_close_request_on_the_partner_window() {
    let (mut world, rx) = world_with_wiring();
    world.spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }));
    let partner = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 1 }))
        .id();

    let ev = pressed(10, 20, DoubleClick::Left, true, false);
    assert!(on_char_pointer_pressed(&mut world, partner, partner, &ev));

    match rx.try_recv().expect("左ダブルクリックが届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            m,
            MouseInput {
                scope: 1,
                x: 5,
                y: 10,
                region: Some("Bust".to_string()),
                kind: MouseEventKind::DoubleClick {
                    button: MouseButton::Left
                },
            },
            "相方側の窓のスコープ 1・面座標で届く"
        ),
        _ => panic!("終了指示ではなく左ダブルクリック（Mouse）を期待"),
    }
    assert!(
        rx.try_recv().is_err(),
        "届くのは左ダブルクリック 1 件だけ（終了指示は 0 件）"
    );
    assert_eq!(ghost_count(&mut world), 2, "窓は 1 枚も閉じない");
    assert!(
        take_pending(&mut world).is_none(),
        "左ダブルクリックは預からない"
    );
}
