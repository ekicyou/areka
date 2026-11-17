//! `\![move]` の取り出しの相の置き場（areka-P0-move-canon task 3.3・design D4・要件 2.5／2.12／8.7）の檻。
//!
//! move drain は再スナップ（`resnap_shell_targets`）の**後**・連鎖の確定（`finalize_chain_once`）の
//! **前**に走る。同じフレームで表示の面の大きさが変わる指令と `\![move]` が届いたとき、移動は
//! 再スナップが窓へ書いた**新しい大きさ**で計算される（右下どうしがそろう）。前に置くと古い大きさで
//! 位置を決め、続く再スナップが下端中央を保って大きさだけ変えるので、右端がずれる。
//!
//! # 扱わない隙間
//!
//! 面の指令（表示の線）と `\![move]`（位置の線）は別々の線で届く。2 つが**別のフレームに割れて**
//! 届くと、移動は古い大きさで計算され、次のフレームの再スナップが下端中央を保って大きさを変える。
//! この隙間は今の作りにもあり、本 spec では扱わない（design Testing Strategy）。本ファイルが守るのは
//! 同じフレームに届いた場合だけである。
//!
//! 挙動の檻はハーネスの上で相を本番と同じ順に呼ぶ形なので、本番の並びが入れ替わっても赤にならない。
//! 本番の並びそのものは [`the_frame_calls_the_move_drain_after_the_resnap_and_before_the_chain_finalize`]
//! が `frame.rs` の本文で押さえる。

use std::path::PathBuf;
use std::sync::mpsc::{self};

use crate::ghost_session::{GhostSession, GhostSlot};
use areka_runtime::move_cue::PositionCue;

use super::run_move_drain_phase;
use super::shared_test_support::{steady_harness, wiring};
use log_capture_kit::{LineFormat, capture_lines};

/// 台本の `\![execute,resetwindowpos]` は並べ直しの手続きへ届くが、kanade へは何も送らない
/// （要件 8.7・`OnResetWindowPos` を送るのはメニューから選んだときだけ）。メニューの経路が読むのと
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
    assert!(
        harness
            .world
            .get_non_send::<crate::menu::reset_pos_frame::PendingResetRaise>()
            .is_none(),
        "答えの受け手が置かれている（メニューの経路を通った）"
    );
}
