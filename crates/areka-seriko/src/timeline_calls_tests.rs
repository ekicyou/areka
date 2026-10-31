//! 働きのコマと今のコマの計算の決定論テスト（spec: seriko-pattern-calls 要件 1.3・7.7）。
//!
//! 働きのコマ（`move`・呼び出し・`insert`）は番号の欄が `-1` でも止まらず、欄に置くのは
//! [`visible_frame`]（その位置以下で最後の描くコマ）。描くコマが無ければ欄は外れる。

use super::*;
use crate::table::FrameAct;
use areka_emo_compose::ComposeMethod;
use std::num::NonZeroU64;

/// 描くコマ。
fn draw(surface_id: i64, wait_ms: u32) -> LoopFrame {
    LoopFrame {
        surface_id,
        picture: None,
        method: ComposeMethod::Overlay,
        act: FrameAct::Draw,
        wait_ms,
        x: 0,
        y: 0,
    }
}

/// 働きのコマ（番号の欄は読み込みのまま残る）。
fn act(act: FrameAct, surface_id: i64, wait_ms: u32) -> LoopFrame {
    LoopFrame {
        act,
        ..draw(surface_id, wait_ms)
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
    let frames = vec![draw(10, 0), act(FrameAct::Move, -1, 50), draw(11, 50)];
    assert_eq!(frame_at(&frames, 50), FrameStatus::Active(1));
    assert_eq!(frame_at(&frames, 100), FrameStatus::FinishedResidual(2));
    // 末尾の働きのコマも止まらず残る。
    let frames = vec![draw(10, 0), act(FrameAct::Move, -1, 50)];
    assert_eq!(frame_at(&frames, 50), FrameStatus::FinishedResidual(1));
    // 描くコマの -1 は今までどおり止まる。
    let frames = vec![draw(10, 0), draw(-1, 50)];
    assert_eq!(frame_at(&frames, 50), FrameStatus::Stopped);
}

/// `move` や呼び出しのコマが今のコマでも、前の描くコマが選ばれる（`base,2` を含む・7.7）。
#[test]
fn action_frame_keeps_the_previous_draw_frame() {
    let base = LoopFrame {
        method: ComposeMethod::Base,
        ..draw(2, 0)
    };
    let frames = vec![
        base,
        act(FrameAct::Move, -1, 50),
        act(start(3), 0, 0),
        draw(7, 50),
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
    let frames = vec![act(FrameAct::Move, -1, 0), act(start(1), 0, 0), draw(7, 50)];
    assert_eq!(visible_frame(&frames, 0), None);
    assert_eq!(visible_frame(&frames, 1), None);
    assert_eq!(visible_frame(&frames, 2), Some(2));
    assert_eq!(visible_frame(&[], 0), None);
}

/// `always` でも働きのコマの `-1` は何も出さない扱いにしない（その周の絵が残る）。
#[test]
fn always_action_frame_with_minus_one_is_a_frame() {
    let frames = vec![draw(10, 0), act(FrameAct::Move, -1, 50), draw(11, 50)];
    let period = NonZeroU64::new(100).unwrap();
    assert_eq!(always_at(&frames, period, None, 60), AlwaysView::Frame(1));
    assert_eq!(visible_frame(&frames, 1), Some(0));
}
