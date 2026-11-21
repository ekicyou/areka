//! 台本の `\![execute,resetwindowpos]` が kanade へ何も送らないことの檻。
//!
//! move drain の相の順序（再スナップの後・連鎖の確定の前）と、同じフレームに届いた面の指令と
//! `\![move]` の扱いは `areka-runtime` の `frame_move_order_tests.rs` が持つ。ここは本体の
//! `GhostSlot`（kanade への送り口）が要るこの 1 本だけを置く。

use std::path::PathBuf;
use std::sync::mpsc::{self};

use crate::session::ghost_session::{GhostSession, GhostSlot};
use areka_runtime::move_cue::PositionCue;

use super::run_move_drain_phase;
use super::shared_test_support::{steady_harness, wiring};
use log_capture_kit::{LineFormat, capture_lines};

/// 台本の `\![execute,resetwindowpos]` は並べ直しの手続きへ届くが、kanade へは何も送らない
/// （`OnResetWindowPos` を送るのはメニューから選んだときだけ）。メニューの経路が読むのと
/// 同じ送り口（`GhostSlot` の会話の kanade）を World に置き、その受け手が空のままであることを見る。
#[test]
fn a_script_reset_sends_nothing_to_kanade() {
    let mut harness = steady_harness();
    crate::session::hooks::install(&mut harness.world);
    let (kanade_tx, kanade_rx) = mpsc::channel();
    harness
        .world
        .insert_non_send(GhostSlot(Some(GhostSession::for_test(
            Some(kanade_tx),
            PathBuf::from("ghost/A"),
        ))));
    let (tx, wiring) = wiring();
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");

    let logs = capture_lines(LineFormat::LevelTargetFields, || {
        run_move_drain_phase(&wiring, &mut harness.world)
    })
    .1;

    assert_eq!(
        logs.iter()
            .filter(|l| l.contains("reset_window_pos_skipped"))
            .count(),
        1,
        "並べ直しが手続きへ届いていない: {logs:?}"
    );
    assert!(
        matches!(kanade_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "台本の並べ直しが kanade へ何かを送った（OnResetWindowPos はメニューの経路だけ）"
    );
}
