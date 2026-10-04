//! ドラッグの開始・終了の知らせが届く・届かないの基本（areka-P0-mouse-drag-events・
//! design「Testing Strategy > areka」A1・A2・A3・A5・A8）。
//!
//! 土台は [`super::test_support::Rig`]（本番の順で組んだ窓と受け口）。期待する知らせは偽の当たり判定
//! （[`fake_hit`]）へ「押した位置 − 窓の位置」を渡して作る（受け口の座標・当たり判定から、
//! 渡した位置が正しいことを読み戻す）。

use areka_kanade::{MouseButton, MouseEventKind, MouseInput};
use wintf::ecs::Point;
use wintf::ecs::drag::{DraggingState, OnDragStart};
use wintf::ecs::pointer::{DoubleClick, Phase, PointerState};

use super::super::{MouseWiring, on_char_pointer_moved, on_char_pointer_pressed};
use super::test_support::{Rig, SCOPE, ended, fake_hit, started};

/// 押した位置の窓の中の点（当たり判定のある位置）。
const PRESS_LOCAL: Point = Point { x: 60, y: 100 };
/// 押した位置から離した位置までの動き（離した位置は当たり判定の無い位置になる）。
const MOVE_BY: Point = Point { x: 602, y: 10 };

/// `screen − window` を偽の当たり判定へ渡した結果としての、期待する知らせ。
fn expected(kind: MouseEventKind, screen: Point, window: Point) -> MouseInput {
    let scope = SCOPE as u32;
    let hit = fake_hit(
        scope,
        i64::from(screen.x - window.x),
        i64::from(screen.y - window.y),
    );
    MouseInput {
        scope,
        x: hit.surface_point.0,
        y: hit.surface_point.1,
        region: hit.region,
        kind,
    }
}

fn offset(p: Point, by: Point) -> Point {
    Point {
        x: p.x + by.x,
        y: p.y + by.y,
    }
}

/// A1（要件 1.1・1.2・1.3・2.1・2.3・2.4・4.4・4.5・9.1 ⑴⑷⑸）: 開始 → 配る → 離す終了 → 配る。
/// 受け口に `DragStart` 1 件 → `DragEnd` 1 件がこの順で届き、スコープ・座標・当たり判定
/// （開始はある位置・終了は無い位置）が期待どおり。
#[test]
fn drag_start_then_end_each_arrive_once_in_order() {
    let mut rig = Rig::new();
    let start_win = rig.position_of(rig.char_w);
    let press = offset(start_win, PRESS_LOCAL);
    let release = offset(press, MOVE_BY);

    rig.put(started(rig.char_w, press));
    rig.tick();
    let after_start = rig.drain();
    rig.put(ended(rig.char_w, release, false));
    rig.tick();
    let after_end = rig.drain();

    let end_win = rig.position_of(rig.char_w);
    let start_msg = expected(MouseEventKind::DragStart, press, start_win);
    let end_msg = expected(MouseEventKind::DragEnd, release, end_win);
    // 前提: 開始は当たり判定のある位置、終了は無い位置（両方の枝を通す）。
    assert!(start_msg.region.is_some() && end_msg.region.is_none());
    assert_eq!(after_start, vec![start_msg], "開始の配りで届いた知らせ");
    assert_eq!(after_end, vec![end_msg], "終了の配りで届いた知らせ");

    rig.close();
}

/// A2（要件 1.1・2.1・9.1 ⑴⑷）: 開始と終了を同じ配りに積む（速いドラッグ）。
/// 順は開始 → 終了・各 1 件。開始の座標は動く前の窓から引く。
#[test]
fn fast_drag_in_one_dispatch_sends_start_then_end_once() {
    let mut rig = Rig::new();
    let start_win = rig.position_of(rig.char_w);
    let press = offset(start_win, PRESS_LOCAL);
    let release = offset(press, MOVE_BY);

    rig.put(started(rig.char_w, press));
    rig.put(ended(rig.char_w, release, false));
    rig.tick();

    let end_win = rig.position_of(rig.char_w);
    assert_eq!(
        rig.drain(),
        vec![
            expected(MouseEventKind::DragStart, press, start_win),
            expected(MouseEventKind::DragEnd, release, end_win),
        ]
    );

    rig.close();
}

/// A3（要件 3.1・3.2・9.1 ⑵）: 動かさないクリック（開始の無い終了）とダブルクリックの押下では
/// ドラッグの知らせは 0 件。ダブルクリックは今どおり 1 件。
#[test]
fn click_without_move_and_double_click_send_no_drag() {
    let mut rig = Rig::new();
    let win = rig.position_of(rig.char_w);
    let press = offset(win, PRESS_LOCAL);

    rig.put(ended(rig.char_w, press, false));
    rig.tick();
    let pressed = Phase::Bubble(PointerState {
        client_point: PRESS_LOCAL,
        double_click: DoubleClick::Left,
        ..Default::default()
    });
    let char_w = rig.char_w;
    assert!(on_char_pointer_pressed(
        &mut rig.world,
        char_w,
        char_w,
        &pressed
    ));

    assert_eq!(
        rig.drain(),
        vec![expected(
            MouseEventKind::DoubleClick {
                button: MouseButton::Left
            },
            press,
            win
        )],
        "ダブルクリックだけが 1 件"
    );

    rig.close();
}

/// A5（要件 3.3・9.1 ⑵）: バルーン窓を対象にした開始 → 終了。マウスの知らせは 0 件。
#[test]
fn balloon_drag_sends_nothing() {
    let mut rig = Rig::new();
    let press = offset(rig.position_of(rig.balloon), PRESS_LOCAL);

    rig.put(started(rig.balloon, press));
    rig.tick();
    // 前提: 開始はバルーン窓まで配られている（wintf が DraggingState を入れた）。
    assert!(rig.world.get::<DraggingState>(rig.balloon).is_some());
    rig.put(ended(rig.balloon, offset(press, MOVE_BY), false));
    rig.tick();

    assert_eq!(rig.drain(), vec![]);
    // 付けるのはキャラクター窓だけ（送り手の no_scope の防御に頼らずに見る）。
    assert!(rig.world.get::<OnDragStart>(rig.balloon).is_none());

    rig.close();
}

/// A8（要件 1.4・2.5・9.1 ⑻）: 直前に移動を 1 件送って間引きが閉じている状態（時計は止めたまま）で
/// 開始 → 終了。2 件とも届き、間引きの状態は変わらない。移動は開始と同じ窓の中の位置に置く
/// （開始が間引きを通ると、同じ位置ゆえ抑えられて赤になる）。
#[test]
fn drag_bypasses_closed_move_throttle_and_leaves_it_untouched() {
    let mut rig = Rig::new();
    let char_w = rig.char_w;
    let start_win = rig.position_of(char_w);
    let press = offset(start_win, PRESS_LOCAL);
    let release = offset(press, MOVE_BY);
    let moved = Phase::Bubble(PointerState {
        client_point: PRESS_LOCAL,
        ..Default::default()
    });

    on_char_pointer_moved(&mut rig.world, char_w, char_w, &moved);
    assert_eq!(rig.drain().len(), 1, "前提: 最初の移動は届く");
    // 前提: 間引きが閉じている（同じ位置の移動は届かない）。
    on_char_pointer_moved(&mut rig.world, char_w, char_w, &moved);
    assert_eq!(rig.drain(), vec![], "前提: 同じ位置の移動は抑えられる");
    let throttle = |rig: &Rig| rig.world.non_send::<MouseWiring>().throttle.clone();
    let before = throttle(&rig);

    rig.put(started(char_w, press));
    rig.tick();
    rig.put(ended(char_w, release, false));
    rig.tick();

    let end_win = rig.position_of(char_w);
    assert_eq!(
        rig.drain(),
        vec![
            expected(MouseEventKind::DragStart, press, start_win),
            expected(MouseEventKind::DragEnd, release, end_win),
        ]
    );
    assert_eq!(throttle(&rig), before, "間引きの状態が変わった");

    rig.close();
}
