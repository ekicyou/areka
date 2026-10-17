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

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};

use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use wintf::ecs::Point;

use crate::emo2_boot::move_cue::{MoveDirective, PositionCue, parse_move_directive};
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::emo2_boot::zorder_cue::ZOrderDirective;
use crate::placement::chain_finalize::ChainFinalized;

use super::test_support::{
    FrameHarness, PerTargetSizes, SPAWN_SIZE_0, capture_logs, pos_of, synth_assets, zero_clock,
};
use super::{Emo2Wiring, run_move_drain_phase};

/// scope1 の新しい表示の面の物理寸（spawn 時の 278×357 から変わる）。
const NEW_SIZE_1: (u32, u32) = (300, 400);

/// 位置の線の送り口と、それを受ける headless な結線。
fn wiring() -> (Sender<PositionCue>, Emo2Wiring) {
    let (tx, rx) = mpsc::channel::<PositionCue>();
    let wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        rx,
        mpsc::channel::<TalkLifecycleSignal>().1,
        mpsc::channel::<ZOrderDirective>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        zero_clock(),
        synth_assets(&[(0, 0)]),
    );
    (tx, wiring)
}

/// `\1\![move,--X=0,--Y=0,--base=0,--base-offset=right.bottom,--move-offset=right.bottom]`（scope1 の右下を
/// scope0 の右下へそろえる）。
fn align_right_bottom_to_scope0() -> MoveDirective {
    let tokens: Vec<String> = [
        "--X=0",
        "--Y=0",
        "--base=0",
        "--base-offset=right.bottom",
        "--move-offset=right.bottom",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    parse_move_directive(1, &tokens).directive
}

/// 起動の確定が済んだ定常の状態の、scope1 の面だけ大きさが変わったフレームの寸。
fn sizes_with_scope1_resized() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, Some(NEW_SIZE_1))])
}

/// 起動の確定が済んだ定常のハーネス（連鎖の確定は一度きりで、このフレームでは何もしない）。
fn steady_harness() -> FrameHarness {
    let mut harness = FrameHarness::new();
    harness.world.insert_resource(ChainFinalized);
    harness.advance_frame();
    harness
}

/// `emo2_frame_system` の再スナップから連鎖の確定までを本番と同じ順で 1 フレームぶん回す。
fn run_resnap_move_finalize(
    harness: &mut FrameHarness,
    wiring: &Emo2Wiring,
    sizes: &PerTargetSizes,
) {
    harness.run_resnap(sizes);
    run_move_drain_phase(wiring, &mut harness.world);
    harness.run_chain_finalize(sizes);
}

/// 期待値: scope0 の右下（1483＋434, 757＋687）から新しい大きさを引いた左上。
fn aligned_with_new_size() -> Point {
    Point {
        x: 1483 + 434 - NEW_SIZE_1.0 as i32,
        y: 757 + 687 - NEW_SIZE_1.1 as i32,
    }
}

/// 同じフレームで面の大きさの変化と `\![move]` が届くと、新しい大きさで右下どうしがそろう
/// （要件 2.5・2.12）。
#[test]
fn a_move_in_the_same_frame_as_a_resize_aligns_with_the_new_size() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring();
    let target = harness.char_window(1);
    tx.send(PositionCue::Move(align_right_bottom_to_scope0()))
        .expect("送出は成功する");

    run_resnap_move_finalize(&mut harness, &wiring, &sizes_with_scope1_resized());

    assert_eq!(
        pos_of(&harness.world, target),
        Some(aligned_with_new_size()),
        "移動が新しい大きさで計算されていない（右下がそろわない）"
    );
}

/// 対照: 移動を再スナップの前に回すと右端がずれる（上の檻が置き場を見分けていることの証し）。
#[test]
fn a_move_before_the_resnap_misses_the_right_edge() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring();
    let target = harness.char_window(1);
    tx.send(PositionCue::Move(align_right_bottom_to_scope0()))
        .expect("送出は成功する");

    let sizes = sizes_with_scope1_resized();
    run_move_drain_phase(&wiring, &mut harness.world);
    harness.run_resnap(&sizes);

    let pos = pos_of(&harness.world, target).expect("位置がある");
    assert_ne!(
        pos,
        aligned_with_new_size(),
        "旧い順でも右下がそろっている（大きさの差が檻に効いていない）"
    );
    // 古い幅 278 で右端をそろえてから、下端中央を保って 300 へ広げた位置。
    assert_eq!(pos.x, 1483 + 434 - 278 + 139 - 150);
}

/// 「並べ直し」は同じ並びの中で並べ直しの手続きへ届く（要件 8.7・`OnResetWindowPos` は送らない）。
///
/// ハーネスは並べ直しの材料（`DefaultPlacementBasis`）を置かないので、手続きは
/// `reset_window_pos_skipped` の WARN を 1 行出して何もしない——届いたことの証しとして使う。
#[test]
fn a_reset_reaches_the_reset_procedure_in_the_frame_order() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring();
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");

    let sizes = sizes_with_scope1_resized();
    let logs = capture_logs(|| run_resnap_move_finalize(&mut harness, &wiring, &sizes));

    assert_eq!(
        logs.iter()
            .filter(|l| l.contains("level=WARN") && l.contains("reset_window_pos_skipped"))
            .count(),
        1,
        "並べ直しが手続きへ届いていない: {logs:?}"
    );
    assert!(
        wiring.drain_move_directives().is_empty(),
        "指令が線に残っている（取り出されていない）"
    );
}

/// 本番の `emo2_frame_system` で、move drain は再スナップの後・連鎖の確定の前に 1 つだけ在る。
/// 重なりの指令の取り出しは置き場を変えない（報告寸の突合の後・再スナップの前）。
#[test]
fn the_frame_calls_the_move_drain_after_the_resnap_and_before_the_chain_finalize() {
    let raw = include_str!("frame.rs");
    let code: String = raw
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let at = |needle: &str| {
        code.find(needle)
            .unwrap_or_else(|| panic!("檻の錨が本文に見当たらない: {needle}"))
    };

    assert_eq!(
        code.matches("run_move_drain_phase(&wiring, world);")
            .count(),
        1,
        "move drain の呼出がちょうど 1 つではない"
    );
    let reconcile = at("reconcile_reported_sizes(&mut wiring.presenter, world);");
    let zorder = at("run_zorder_drain_phase(");
    let resnap = at("resnap_shell_targets(&wiring.presenter, world);");
    let mv = at("run_move_drain_phase(&wiring, world);");
    let finalize = at("finalize_chain_once(&wiring.presenter, world);");

    assert!(
        resnap < mv && mv < finalize,
        "move drain が再スナップの後・連鎖の確定の前に無い（resnap={resnap}・move={mv}・finalize={finalize}）"
    );
    assert!(
        reconcile < zorder && zorder < resnap,
        "重なりの指令の取り出しの置き場が変わった（reconcile={reconcile}・zorder={zorder}・resnap={resnap}）"
    );
}
