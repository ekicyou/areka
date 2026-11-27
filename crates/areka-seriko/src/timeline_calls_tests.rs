//! 働きのコマと今のコマの計算の決定論テスト。
//!
//! 働きのコマ（`move`・呼び出し・`insert`）は番号の欄が `-1` でも止まらず、欄に置くのは
//! [`visible_frame`]（その位置以下で最後の描くコマ）。描くコマが無ければ欄は外れる。

use super::*;
use crate::table::FrameAct;
use areka_emo_compose::ComposeMethod;
use dola::time::testing::ms;

// 描くコマ。
use crate::test_support::draw_frame as draw;

/// 働きのコマ（番号の欄は読み込みのまま残る）。
fn act(act: FrameAct, surface_id: i64, wait_ms: u32) -> LoopFrame {
    LoopFrame {
        act,
        ..draw(surface_id, ms(wait_ms))
    }
}

fn start(target: u32) -> FrameAct {
    FrameAct::Start {
        targets: vec![target].into_boxed_slice(),
        pick_one: false,
    }
}

/// `move,-1,…` が今のコマでも止まらない（`-1` で止まるのは描くコマだけ）。
#[test]
fn move_with_minus_one_does_not_stop() {
    let frames = vec![
        draw(10, ms(0)),
        act(FrameAct::Move, -1, 50),
        draw(11, ms(50)),
    ];
    assert_eq!(frame_at(&frames, ms(50)), FrameStatus::Active(1));
    assert_eq!(frame_at(&frames, ms(100)), FrameStatus::FinishedResidual(2));
    // 末尾の働きのコマも止まらず残る。
    let frames = vec![draw(10, ms(0)), act(FrameAct::Move, -1, 50)];
    assert_eq!(frame_at(&frames, ms(50)), FrameStatus::FinishedResidual(1));
    // 描くコマの -1 は止まる。
    let frames = vec![draw(10, ms(0)), draw(-1, ms(50))];
    assert_eq!(frame_at(&frames, ms(50)), FrameStatus::Stopped);
}

/// `move` や呼び出しのコマが今のコマでも、前の描くコマが選ばれる（`base,2` を含む）。
#[test]
fn action_frame_keeps_the_previous_draw_frame() {
    let base = LoopFrame {
        method: ComposeMethod::Base,
        ..draw(2, ms(0))
    };
    let frames = vec![
        base,
        act(FrameAct::Move, -1, 50),
        act(start(3), 0, 0),
        draw(7, ms(50)),
        act(FrameAct::Inert, 5, 0),
    ];
    assert_eq!(visible_frame(&frames, 0), Some(0));
    assert_eq!(visible_frame(&frames, 1), Some(0), "move は base,2 を残す");
    assert_eq!(
        visible_frame(&frames, 2),
        Some(0),
        "呼び出しも base,2 を残す"
    );
    assert_eq!(visible_frame(&frames, 3), Some(3));
    assert_eq!(visible_frame(&frames, 4), Some(3));
    // 位置が表の外でも末尾までで答える。
    assert_eq!(visible_frame(&frames, 99), Some(3));
}

/// 描くコマが前に無ければ欄が外れる（None）。
#[test]
fn no_draw_frame_clears_the_slot() {
    let frames = vec![
        act(FrameAct::Move, -1, 0),
        act(start(1), 0, 0),
        draw(7, ms(50)),
    ];
    assert_eq!(visible_frame(&frames, 0), None);
    assert_eq!(visible_frame(&frames, 1), None);
    assert_eq!(visible_frame(&frames, 2), Some(2));
    assert_eq!(visible_frame(&[], 0), None);
}

/// `always` でも働きのコマの `-1` は何も出さない扱いにしない（その周の絵が残る）。
#[test]
fn always_action_frame_with_minus_one_is_a_frame() {
    let frames = vec![
        draw(10, ms(0)),
        act(FrameAct::Move, -1, 50),
        draw(11, ms(50)),
    ];
    let period = ms(100);
    assert_eq!(
        always_at(&frames, period, None, ms(60)),
        AlwaysView::Frame(1)
    );
    assert_eq!(visible_frame(&frames, 1), Some(0));
}

/// 描くコマの `-1` のすぐ後に待ち 0 の働きのコマが続いても、`-1` で止まる（今のコマは働きのコマでも、
/// 欄に置くはずのコマが `-1` だから・ukadoc pattern の項）。
#[test]
fn minus_one_followed_by_zero_wait_action_frame_stops() {
    let frames = vec![
        draw(100, ms(0)),
        draw(-1, ms(50)),
        act(FrameAct::Inert, 0, 0),
    ];
    assert_eq!(frame_at(&frames, ms(49)), FrameStatus::Active(0));
    assert_eq!(frame_at(&frames, ms(50)), FrameStatus::Stopped);
    let frames = vec![
        draw(100, ms(0)),
        draw(-1, ms(50)),
        act(start(3), 0, 0),
        draw(101, ms(50)),
    ];
    assert_eq!(frame_at(&frames, ms(50)), FrameStatus::Stopped);
    // 描くコマが前に無い働きのコマは止めない（欄を外すだけ）。
    let frames = vec![act(start(3), 0, 0), draw(101, ms(50))];
    assert_eq!(frame_at(&frames, ms(0)), FrameStatus::Active(0));
}

/// `-1` のすぐ後に待ち 0 の描くコマがあっても、`-1` に着いた時点で止まる（後ろの描くコマへ進まない・
/// ukadoc「サーフェス番号に-1でそのアニメーションの停止」）。
#[test]
fn minus_one_followed_by_a_zero_wait_draw_frame_stops() {
    let frames = vec![draw(100, ms(0)), draw(-1, ms(50)), draw(101, ms(0))];
    assert_eq!(frame_at(&frames, ms(0)), FrameStatus::Active(0));
    assert_eq!(frame_at(&frames, ms(50)), FrameStatus::Stopped);
    assert_eq!(
        frame_at(&frames, TimeSpan::MAX),
        FrameStatus::Stopped,
        "恒久"
    );
    // 刻みが `-1` の短い窓 [50,55) を飛び越えても止まる。
    let frames = vec![draw(100, ms(0)), draw(-1, ms(50)), draw(101, ms(5))];
    assert_eq!(frame_at(&frames, ms(60)), FrameStatus::Stopped);
}
