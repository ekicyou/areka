//! ドラッグの開始・終了の知らせが届く・届かないの基本と、終了の座標・位置の保存
//! （areka-P0-mouse-drag-events・design「Testing Strategy > areka」A1〜A8）と、送らない経路の記録（A9）。
//!
//! 土台は [`super::test_support::Rig`]（本番の順で組んだ窓と受け口）。期待する知らせは偽の当たり判定
//! （[`fake_hit`]）へ「押した位置 − 窓の位置」を渡して作る（受け口の座標・当たり判定から、
//! 渡した位置が正しいことを読み戻す）。
//!
//! 終了の座標を見るテスト（A4・A6）と保存を比べるテスト（A7）は、開始を配った直後に
//! `DraggingState.initial_inset` を開始時の窓位置で上書きし（偽の窓では `(0,0)` のまま入るため）、
//! ドラッグの間の窓の移動は窓の位置を直に書いて作る（[`begin_drag`]）。

use std::time::Instant;

use areka_kanade::{MouseButton, MouseEventKind, MouseInput};
use areka_sylphya::PersistKey;
use bevy_ecs::prelude::Entity;
use wintf::ecs::drag::{DragStartEvent, DraggingState, OnDragStart};
use wintf::ecs::pointer::{DoubleClick, Phase, PointerState};
use wintf::ecs::{PhysicalPoint, Point, WindowPos};

use super::super::{MouseWiring, on_char_pointer_moved, on_char_pointer_pressed};
use super::on_char_drag_start;
use super::test_support::{Rig, SCOPE, ended, fake_hit, started};
use crate::placement::test_support::{LogEvent, capture_logs};

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

/// ドラッグの間に窓を動かす量（下端へ寄せる置き方なので横へだけ）。
const WINDOW_SHIFT: Point = Point { x: 300, y: 0 };
/// A6 で押した位置から離した位置までの動き（横と縦）。
const RELEASE_BY: Point = Point { x: 150, y: 40 };

/// 開始を配り、`initial_inset` を開始時の窓位置にしてから、窓を [`WINDOW_SHIFT`] だけ動かす
/// （最後の移動の知らせが配られず 1 つ古い位置に残った窓も兼ねる）。
/// 開始時の窓位置・押した位置・開始の配りで届いた知らせを返す。
fn begin_drag(rig: &mut Rig) -> (Point, Point, Vec<MouseInput>) {
    let char_w = rig.char_w;
    let start_win = rig.position_of(char_w);
    let press = offset(start_win, PRESS_LOCAL);
    rig.put(started(char_w, press));
    rig.tick();
    rig.set_initial_inset(char_w, start_win);
    rig.move_window(char_w, offset(start_win, WINDOW_SHIFT));
    (start_win, press, rig.drain())
}

/// A4（要件 2.2・4.2・9.1 ⑶⑸）: 開始 → 窓を動かす → 取り消しの終了（位置＝押した位置）。
/// 窓が開始の位置へ戻り、`DragEnd` が 1 件届き、座標と当たり判定が `DragStart` と同じ値。
/// 保存の前の窓の位置から引くと、動かした先の窓から見た値になって赤。
#[test]
fn cancelled_drag_end_reports_the_start_coordinates() {
    let mut rig = Rig::new();
    let (start_win, press, after_start) = begin_drag(&mut rig);
    assert_ne!(
        rig.position_of(rig.char_w),
        start_win,
        "前提: ドラッグの間に窓が動いている"
    );

    rig.put(ended(rig.char_w, press, true));
    rig.tick();

    assert_eq!(
        rig.position_of(rig.char_w),
        start_win,
        "取り消しで窓が開始の位置へ戻っていない"
    );
    assert_eq!(
        after_start,
        vec![expected(MouseEventKind::DragStart, press, start_win)]
    );
    assert_eq!(
        rig.drain(),
        vec![expected(MouseEventKind::DragEnd, press, start_win)],
        "取り消しの終了は開始と同じ座標・当たり判定で 1 件"
    );

    rig.close();
}

/// A6（要件 4.2・9.1 ⑸・決定 D1）: 下端へ寄せる置き方の窓で、押した位置から横と縦へ動かして離す
/// （窓は横へだけ動く）。`DragEnd` の座標は「離した位置 − 保存の後の窓の位置」と一致し、
/// 「離した位置 − 保存の前の窓の位置」とは違う。
#[test]
fn drag_end_coordinates_use_the_window_position_after_the_save() {
    let mut rig = Rig::new();
    let (start_win, press, _) = begin_drag(&mut rig);
    let before_save = rig.position_of(rig.char_w);
    let release = offset(press, RELEASE_BY);

    rig.put(ended(rig.char_w, release, false));
    rig.tick();

    let after_save = rig.position_of(rig.char_w);
    assert_eq!(
        after_save,
        Point {
            x: start_win.x + RELEASE_BY.x,
            y: start_win.y
        },
        "前提: 下端へ寄せる置き方の窓は横へだけ動く"
    );
    let ends = rig.drain();
    assert_eq!(
        ends,
        vec![expected(MouseEventKind::DragEnd, release, after_save)]
    );
    assert_ne!(
        ends,
        vec![expected(MouseEventKind::DragEnd, release, before_save)],
        "前提: 保存の前と後で引いた値が区別できる"
    );

    rig.close();
}

/// A7 の操作。
#[derive(Clone, Copy, Debug)]
enum Gesture {
    /// 動かして離す。
    Release,
    /// 動かしてから取り消す。
    Cancel,
    /// 動かさないクリック（開始の無い終了）。
    Click,
}

/// A7 の包みを付けた側の 3 通り。
#[derive(Clone, Copy, Debug)]
enum Wrapped {
    /// 送り先あり。
    Sends,
    /// `MouseWiring` なし。
    NoWiring,
    /// 受け口を落として送出に失敗。
    SendFails,
}

/// 操作をして、窓の位置・記憶へ書かれた組・届いた知らせの件数を返す（土台は閉じる）。
fn perform(mut rig: Rig, gesture: Gesture) -> (Point, Vec<(PersistKey, String)>, usize) {
    let char_w = rig.char_w;
    let mut sent = 0;
    match gesture {
        Gesture::Click => {
            let press = offset(rig.position_of(char_w), PRESS_LOCAL);
            rig.put(ended(char_w, press, false));
        }
        Gesture::Release | Gesture::Cancel => {
            let (_, press, after_start) = begin_drag(&mut rig);
            sent += after_start.len();
            let cancelled = matches!(gesture, Gesture::Cancel);
            let end = if cancelled {
                press
            } else {
                offset(press, RELEASE_BY)
            };
            rig.put(ended(char_w, end, cancelled));
        }
    }
    rig.tick();
    sent += rig.drain().len();
    let result = (rig.position_of(char_w), rig.saved(), sent);
    rig.close();
    result
}

/// A7（要件 6.1・6.2・6.3・9.1 ⑺）: 包みを付けない窓（本 spec の前）と包みを付けた窓で同じ操作
/// （離す・取り消す・動かさないクリック）をすると、窓の位置と記憶へ書かれた組が一致する。
/// 包みの側は送り先あり・`MouseWiring` なし・送出に失敗の 3 通り。動かさないクリックでは
/// どちらも保存 0 件。保存を呼ばない・送り先が無いと戻る、にすると赤。
#[test]
fn wrapper_keeps_window_position_and_saved_pairs_unchanged() {
    for gesture in [Gesture::Release, Gesture::Cancel, Gesture::Click] {
        let (base_pos, base_saved, base_sent) = perform(Rig::without_wrapper(), gesture);
        assert_eq!(base_sent, 0, "前提: 包みの無い窓は知らせを送らない");
        assert_eq!(
            base_saved.is_empty(),
            matches!(gesture, Gesture::Click),
            "前提: ドラッグは保存し、動かさないクリックは保存しない（{gesture:?}: {base_saved:?}）"
        );

        for wrapped in [Wrapped::Sends, Wrapped::NoWiring, Wrapped::SendFails] {
            let mut rig = Rig::new();
            match wrapped {
                Wrapped::Sends => {}
                Wrapped::NoWiring => {
                    rig.world
                        .remove_non_send::<MouseWiring>()
                        .expect("前提: MouseWiring がある");
                }
                Wrapped::SendFails => rig.drop_receiver(),
            }
            let (pos, saved, sent) = perform(rig, gesture);

            assert_eq!(
                (pos, &saved),
                (base_pos, &base_saved),
                "{gesture:?}・{wrapped:?}: 窓の位置か記憶へ書かれた組が包みの無い窓と違う"
            );
            // 前提: 包みが付いている（送り先があればドラッグは開始と終了の 2 件・クリックは 0 件）。
            let want = match (wrapped, gesture) {
                (Wrapped::Sends, Gesture::Release | Gesture::Cancel) => 2,
                _ => 0,
            };
            assert_eq!(sent, want, "{gesture:?}・{wrapped:?}: 届いた知らせの件数");
        }
    }
}

/// A9 の 1 件: 送らない経路の記録（`mouse_drag_dropped`／`mouse_send_failed`）がちょうど 1 件で、
/// 理由・種類・重さが期待どおり。
fn assert_one_record(
    events: &[LogEvent],
    event: &str,
    reason: Option<&str>,
    level: tracing::Level,
) {
    let records: Vec<&LogEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e.field_str("event"),
                Some("mouse_drag_dropped" | "mouse_send_failed")
            )
        })
        .collect();
    assert_eq!(
        records.len(),
        1,
        "送らない経路の記録が 1 件ではない: {events:?}"
    );
    let got = records[0];
    assert_eq!(got.field_str("event"), Some(event), "{got:?}");
    assert_eq!(got.field_str("reason"), reason, "{got:?}");
    assert_eq!(got.field_str("kind"), Some("drag_start"), "{got:?}");
    assert_eq!(got.level, level, "{got:?}");
}

/// 本番の配りで開始を 1 回配り、その間の記録を返す。
fn dispatch_start_capturing(rig: &mut Rig) -> Vec<LogEvent> {
    let press = offset(rig.position_of(rig.char_w), PRESS_LOCAL);
    rig.put(started(rig.char_w, press));
    capture_logs(|| rig.tick()).1
}

/// 受け手を直に呼んで開始を 1 回知らせ、その間の記録を返す（本番の配りでは作れない防御の枝）。
fn call_start_capturing(rig: &mut Rig, entity: Entity, target: Entity) -> Vec<LogEvent> {
    let ev = Phase::Bubble(DragStartEvent {
        target,
        position: PhysicalPoint::new(0, 0),
        is_primary: true,
        timestamp: Instant::now(),
    });
    capture_logs(|| on_char_drag_start(&mut rig.world, entity, entity, &ev)).1
}

/// A9（要件 8.2・9.2）: `MouseWiring` なし → `mouse_drag_dropped`（`no_wiring`・debug）1 件・知らせ 0 件。
#[test]
fn no_wiring_records_one_drop() {
    let mut rig = Rig::new();
    rig.world
        .remove_non_send::<MouseWiring>()
        .expect("前提: MouseWiring がある");

    let events = dispatch_start_capturing(&mut rig);

    assert_one_record(
        &events,
        "mouse_drag_dropped",
        Some("no_wiring"),
        tracing::Level::DEBUG,
    );
    assert_eq!(rig.drain(), vec![]);
    rig.close();
}

/// A9（要件 8.2・9.2）: 対象が別の窓 → `mouse_drag_dropped`（`target_mismatch`・warn）1 件・知らせ 0 件。
#[test]
fn target_mismatch_records_one_drop() {
    let mut rig = Rig::new();
    let (char_w, balloon) = (rig.char_w, rig.balloon);

    let events = call_start_capturing(&mut rig, char_w, balloon);

    assert_one_record(
        &events,
        "mouse_drag_dropped",
        Some("target_mismatch"),
        tracing::Level::WARN,
    );
    assert_eq!(rig.drain(), vec![]);
    rig.close();
}

/// A9（要件 8.2・9.2）: `CharWindowMarker` なし（バルーン窓）→ `mouse_drag_dropped`（`no_scope`・warn）
/// 1 件・知らせ 0 件。
#[test]
fn no_scope_records_one_drop() {
    let mut rig = Rig::new();
    let balloon = rig.balloon;

    let events = call_start_capturing(&mut rig, balloon, balloon);

    assert_one_record(
        &events,
        "mouse_drag_dropped",
        Some("no_scope"),
        tracing::Level::WARN,
    );
    assert_eq!(rig.drain(), vec![]);
    rig.close();
}

/// A9（要件 8.2・9.2）: `WindowPos.position` なし → `mouse_drag_dropped`（`no_window_pos`・warn）
/// 1 件・知らせ 0 件。
#[test]
fn no_window_pos_records_one_drop() {
    let mut rig = Rig::new();
    let press = offset(rig.position_of(rig.char_w), PRESS_LOCAL);
    rig.world
        .get_mut::<WindowPos>(rig.char_w)
        .expect("前提: WindowPos がある")
        .position = None;
    rig.put(started(rig.char_w, press));

    let events = capture_logs(|| rig.tick()).1;

    assert_one_record(
        &events,
        "mouse_drag_dropped",
        Some("no_window_pos"),
        tracing::Level::WARN,
    );
    assert_eq!(rig.drain(), vec![]);
    rig.close();
}

/// A9（要件 8.2・9.2）: 受け口を落とす → `mouse_send_failed`（warn）1 件・知らせ 0 件。
#[test]
fn send_failure_records_one_failure() {
    let mut rig = Rig::new();
    rig.drop_receiver();

    let events = dispatch_start_capturing(&mut rig);

    assert_one_record(&events, "mouse_send_failed", None, tracing::Level::WARN);
    assert_eq!(rig.drain(), vec![]);
    rig.close();
}
