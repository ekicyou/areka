//! 帯の働きの純関数の全分岐（要件 2.4・2.5・2.10・7.3）。

use super::*;

fn e(index: u32) -> Entity {
    Entity::from_raw_u32(index).expect("有効な entity の番号")
}

fn h(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

/// 順位どおりに奥から手前へ並ぶ（0 が最上なので、順位の大きい窓が先）。
#[test]
fn back_to_front_orders_by_rank_from_back_to_front() {
    let windows = [(e(1), h(0x10)), (e(2), h(0x20)), (e(3), h(0x30))];
    let rank = |w: HWND| match w.0 as isize {
        0x10 => Some(1),
        0x20 => Some(0),
        0x30 => Some(5),
        _ => unreachable!(),
    };
    assert_eq!(
        back_to_front(&windows, rank),
        vec![(e(3), h(0x30)), (e(1), h(0x10)), (e(2), h(0x20))]
    );
}

/// 測れない窓は奥の端へ置き、測れない窓どうしは元の順を保つ。
#[test]
fn back_to_front_puts_unmeasured_windows_at_the_back_in_original_order() {
    let windows = [
        (e(1), h(0x10)),
        (e(2), h(0x20)),
        (e(3), h(0x30)),
        (e(4), h(0x40)),
    ];
    let rank = |w: HWND| match w.0 as isize {
        0x10 => Some(0),
        0x30 => Some(2),
        _ => None,
    };
    assert_eq!(
        back_to_front(&windows, rank),
        vec![
            (e(2), h(0x20)),
            (e(4), h(0x40)),
            (e(3), h(0x30)),
            (e(1), h(0x10)),
        ]
    );
}

/// 同じ順位の窓どうしは元の順を保つ（並べ替えが安定）。空の列は空のまま。
#[test]
fn back_to_front_keeps_original_order_for_equal_ranks_and_empty_input() {
    let windows = [(e(1), h(0x10)), (e(2), h(0x20))];
    assert_eq!(back_to_front(&windows, |_| Some(3)), windows.to_vec());
    assert!(back_to_front(&[], |_| Some(0)).is_empty());
}

/// 3 つの指令の挿入位置。
#[test]
fn band_insert_after_maps_each_op_to_its_insert_position() {
    assert_eq!(band_insert_after(BandOp::Enter), HWND_TOPMOST);
    assert_eq!(band_insert_after(BandOp::Leave), HWND_NOTOPMOST);
    assert_eq!(band_insert_after(BandOp::Raise), HWND_TOP);
}

/// 入る・出るは帯の印を確かめ、手前へ出すは確かめない。
#[test]
fn expected_band_checks_enter_and_leave_but_not_raise() {
    assert_eq!(expected_band(BandOp::Enter), Some(true));
    assert_eq!(expected_band(BandOp::Leave), Some(false));
    assert_eq!(expected_band(BandOp::Raise), None);
}

/// 列は積まれた順に指令と窓の列を持つ。
#[test]
fn window_band_queue_keeps_pushed_order() {
    let mut queue = WindowBandQueue::default();
    queue.push(BandOp::Enter, vec![e(1), e(2)]);
    queue.push(BandOp::Raise, vec![e(2)]);
    assert_eq!(
        queue.ops,
        vec![
            (BandOp::Enter, vec![e(1), e(2)]),
            (BandOp::Raise, vec![e(2)])
        ]
    );
}
