//! 右ダブルクリックの預かりと、結線済みの Ctrl＋左ダブルクリックのテスト。
//!
//! 合成した `PointerState` で押下ハンドラを直接呼び、kanade 宛ての受信端で「届いた／届かない」を
//! 観測する（実窓も GPU も要らない）。`char_pointer_tests.rs` と同じ観測方法である。

use super::pointer_test_support::*;
use super::*;
use areka_kanade::MouseInput;
use areka_placement::spawn::CharWindowMarker;
use std::sync::mpsc;

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
fn world_with_half_scale_bust() -> (World, mpsc::Receiver<KanadeMsg>) {
    world_with_wiring(half_scale_bust, Box::new(|| 1000))
}

/// 右ダブルクリックは SHIORI へ何も届けず、スコープ・面座標・領域を預かる。
///
/// 押下ハンドラが即送出へ戻れば `rx` の表明が落ち、預かりを置かなければ `take` の表明が落ちる。
#[test]
fn right_double_click_sends_nothing_and_keeps_the_material() {
    let (mut world, rx) = world_with_half_scale_bust();
    let partner = world.spawn(CharWindowMarker { scope: 1 }).id();

    let ev = bubble_pointer_mods(14, 8, DoubleClick::Right, false, false);
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
    let (mut world, rx) = world_with_half_scale_bust();
    let main = world.spawn(CharWindowMarker { scope: 0 }).id();
    let partner = world.spawn(CharWindowMarker { scope: 1 }).id();

    let first = bubble_pointer_mods(14, 8, DoubleClick::Right, false, false);
    assert!(on_char_pointer_pressed(&mut world, main, main, &first));
    let second = bubble_pointer_mods(30, 40, DoubleClick::Right, false, false);
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

/// 預かりを送ると、右ダブルクリックの `OnMouseDoubleClick` がちょうど 1 件届く。
///
/// 届くのは `MouseInput{scope, x, y, region, kind: DoubleClick{Right}}` を面座標にした値
/// （kanade がこれを Ref0〜Ref5 へ写し、右ボタンは Ref5＝1 になる）。
#[test]
fn sending_the_pending_one_delivers_one_right_double_click_in_surface_coordinates() {
    let (mut world, rx) = world_with_half_scale_bust();
    let partner = world.spawn(CharWindowMarker { scope: 1 }).id();
    let ev = bubble_pointer_mods(14, 8, DoubleClick::Right, false, false);
    assert!(on_char_pointer_pressed(&mut world, partner, partner, &ev));

    let pending = take_pending(&mut world).expect("預かりが残る");
    send_pending(&mut world, pending);

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
/// 左ダブルクリックとして届く。終了の指示は立たない。
///
/// 相方側（スコープ 1）で見る——Ctrl を見て早期に戻れば `rx` が空で落ち、強制退避の腕へ
/// 落ちれば終了の指示が立って表明が落ちる。
///
/// 「終了指示に操作した窓のスコープが載る」性質は本体のメニューのテストが持つ。ここでは重ねない。
#[test]
fn wired_ctrl_left_double_click_sends_no_close_request_on_the_partner_window() {
    let (mut world, rx) = world_with_half_scale_bust();
    let partner = world.spawn(CharWindowMarker { scope: 1 }).id();

    let ev = bubble_pointer_mods(10, 20, DoubleClick::Left, true, false);
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
    assert!(
        !exit_requested(&world),
        "強制退避へ落ちない——終了の指示は立たない"
    );
    assert!(
        take_pending(&mut world).is_none(),
        "左ダブルクリックは預からない"
    );
}
