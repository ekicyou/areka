//! `\![move]` の取り出しの相の置き場の檻。
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
//! この隙間は今の作りにもあり、ここでは扱わない。本ファイルが守るのは
//! 同じフレームに届いた場合だけである。
//!
//! 挙動の檻はハーネスの上で相を本番と同じ順に呼ぶ形なので、本番の並びが入れ替わっても赤にならない。
//! 本番の並びそのものは [`the_frame_calls_the_move_drain_after_the_resnap_and_before_the_chain_finalize`]
//! が `frame.rs` の本文で押さえる。同じ本文の走査で、装置の作り直しの出し直しの口
//! （`refresh_after_device_change`）が装着の相の後・拡大率の相の前に 1 回だけ在ることも見る。

use crate::move_cue::{MoveDirective, PositionCue, parse_move_directive};
use wintf::ecs::Point;

use super::super::source_text_test_support::{code_only, index_of};
use super::shared_test_support::{steady_harness, wiring};
use super::test_support::{FrameHarness, PerTargetSizes, SPAWN_SIZE_0, all_shown, pos_of};
use super::{RuntimeWiring, run_move_drain_with};

/// scope1 の新しい表示の面の物理寸（spawn 時の 278×357 から変わる）。
const NEW_SIZE_1: (u32, u32) = (300, 400);

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

/// `frame_system` の再スナップから連鎖の確定までを本番と同じ順で 1 フレームぶん回す。
fn run_resnap_move_finalize(
    harness: &mut FrameHarness,
    wiring: &RuntimeWiring,
    sizes: &PerTargetSizes,
) {
    harness.run_resnap(sizes);
    // 前提: 窓はどれも最初の面を表示済み（定常のフレーム・位置の指令を預からない）。
    run_move_drain_with(wiring, &all_shown, &mut harness.world);
    harness.run_chain_finalize(sizes);
}

/// 期待値: scope0 の右下（1483＋434, 757＋687）から新しい大きさを引いた左上。
fn aligned_with_new_size() -> Point {
    Point {
        x: 1483 + 434 - NEW_SIZE_1.0 as i32,
        y: 757 + 687 - NEW_SIZE_1.1 as i32,
    }
}

/// 同じフレームで面の大きさの変化と `\![move]` が届くと、新しい大きさで右下どうしがそろう。
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
    run_move_drain_with(&wiring, &all_shown, &mut harness.world);
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

/// 本番の `frame_system` で、move drain は再スナップの後・連鎖の確定の前に 1 つだけ在る。
/// 重なりの指令の取り出しは置き場を変えない（報告寸の突合の後・再スナップの前）。
#[test]
fn the_frame_calls_the_move_drain_after_the_resnap_and_before_the_chain_finalize() {
    let code = code_only(include_str!("frame.rs"));
    assert_eq!(
        code.matches("run_move_drain_phase(&wiring, world);")
            .count(),
        1,
        "move drain の呼出がちょうど 1 つではない"
    );
    let reconcile = index_of(
        &code,
        "reconcile_reported_sizes(&mut wiring.presenter, world);",
    );
    let zorder = index_of(&code, "run_zorder_drain_phase(");
    let resnap = index_of(&code, "resnap_shell_targets(&wiring.presenter, world);");
    let mv = index_of(&code, "run_move_drain_phase(&wiring, world);");
    let finalize = index_of(&code, "finalize_chain_once(&wiring.presenter, world);");

    assert!(
        resnap < mv && mv < finalize,
        "move drain が再スナップの後・連鎖の確定の前に無い（resnap={resnap}・move={mv}・finalize={finalize}）"
    );
    assert!(
        reconcile < zorder && zorder < resnap,
        "重なりの指令の取り出しの置き場が変わった（reconcile={reconcile}・zorder={zorder}・resnap={resnap}）"
    );
}

/// 装置の作り直しで emo-present が控えを捨てて出し直す口を、
/// 本番の `frame_system` が毎フレーム 1 回、装着の相の後・拡大率の相の前に呼ぶ。
#[test]
fn the_frame_calls_the_device_change_refresh_once_between_attach_and_dpi() {
    let code = code_only(include_str!("frame.rs"));
    let call = "wiring.presenter.refresh_after_device_change(world);";
    assert_eq!(code.matches(call).count(), 1, "呼出がちょうど 1 つではない");
    let (attach, refresh, dpi) = (
        index_of(&code, "run_attach_phase(&mut wiring, world);"),
        index_of(&code, call),
        index_of(&code, "run_dpi_phase(&mut wiring, world);"),
    );
    assert!(
        attach < refresh && refresh < dpi,
        "装着の相の後・拡大率の相の前に無い（attach={attach}・refresh={refresh}・dpi={dpi}）"
    );
}
