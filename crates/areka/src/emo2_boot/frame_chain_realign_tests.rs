//! DPI／拡大率の遷移後に連鎖を一度だけ解き直す（task 5.6・設計 D8／C4・
//! 要件 6.1／6.2／6.3／6.6）の**観察可能な帰結**を、多フレーム駆動で固定する。
//!
//! # ここが押さえる是正
//!
//! 起動時の連鎖確定は一度きりで、確定後のサーフェス切替では駆動しない（`scope-chain-gap` 7.4）。
//! ところが拡大率が変わると全スコープの幅が k 倍に変わり、各窓は下端中央を保ったまま置き直される
//! ——隣接していた 2 体のあいだに**幅変化の半分の和**だけ隙間が開く。実機は 200%→100% で
//! **359px**（幅 764→382 と 672→336 の左端差 `191 + 168`）だった。
//!
//! 本ファイルの探針はその実測をそのまま決定論へ写したものである
//! （[`the_gap_returns_after_a_scale_change_and_the_realign_closes_it`]）——是正前は隙間 359 が
//! 残って赤くなり、遷移後の解き直しが入ると 0 になる。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 「会話中の表情差替では解き直さない」は、解き直しを丸ごと止めても恒真で通る。ゆえに
//! [`a_surface_swap_during_a_conversation_never_arms_the_realign`] は、**同じ駆動口**で
//! 拡大率が動けば武装することを同じテスト本体の中で続けて主張する。

use bevy_ecs::change_detection::DetectChangesMut;
use wintf::ecs::window::SetWindowPosCommand;
use wintf::ecs::{Point, WindowHandle, WindowPos};

use crate::placement::chain_finalize::{CHAIN_FINALIZE_STALL_FRAMES, ChainFinalized};
use crate::placement::chain_realign::ChainRealignPending;
use crate::placement::dpi_sync::DpiSyncHold;
use crate::placement::spawn::GhostWindows;

use super::shell_target;
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, capture_logs, count_level, pos_of, size_of,
};

// ---------------------------------------------------------------------------
// 実機 359px を決定論へ写した探針（design Testing「隙間 359→0 の決定論版」）
// ---------------------------------------------------------------------------

/// 遷移前の拡大率水準（実機の 200% に対応）。
const HIGH_DPI: u16 = 192;

/// 遷移後の拡大率水準（実機の 100% に対応＝幅がちょうど半分になる）。
const LOW_DPI: u16 = 96;

/// scope0 の実表示寸（高水準）。幅は実機の 764（`baseline` の physical 764x1094）。
const HIGH_SIZE_0: (u32, u32) = (764, 1094);
/// scope0 の実表示寸（低水準）＝ちょうど半分。
const LOW_SIZE_0: (u32, u32) = (382, 547);
/// scope1 の実表示寸（高水準）。幅は実機の 672。
const HIGH_SIZE_1: (u32, u32) = (672, 596);
/// scope1 の実表示寸（低水準）＝ちょうど半分。
const LOW_SIZE_1: (u32, u32) = (336, 298);

/// 高水準で連鎖を確定した直後の scope0 左上 X（連鎖の起点ゆえ以後も動かない）。
///
/// spawn 位置 `x=1483`・幅 434 の中央は 1700。幅 764 で `1700 − 382 = 1318`。
const SCOPE0_X_HIGH: i32 = 1318;

/// 高水準で連鎖を確定した直後の scope1 左上 X（`SCOPE0_X_HIGH − 672`＝隣接）。
const SCOPE1_X_HIGH: i32 = SCOPE0_X_HIGH - HIGH_SIZE_1.0 as i32;

/// 低水準へ遷移した直後の scope0 左上 X（下端中央保存の再射影後）。
///
/// 中央 `1318 + 382 = 1700`、幅 382 で `1700 − 191 = 1509`。
const SCOPE0_X_LOW: i32 = 1509;

/// 低水準へ遷移した直後の scope1 左上 X（解き直しを入れる前）。
///
/// 中央 `646 + 336 = 982`、幅 336 で `982 − 168 = 814`。
const SCOPE1_X_LOW_BEFORE_REALIGN: i32 = 814;

/// **是正前に残る隙間**（実機 359px の決定論版）。
///
/// `1509 − (814 + 336) = 359`＝左端差 `191 + 168` の和そのものである。
const GAP_WITHOUT_REALIGN: i32 = 359;

/// 解き直しが入ったときに scope1 が収まる左上 X（`SCOPE0_X_LOW − 336`＝隣接）。
const SCOPE1_X_AFTER_REALIGN: i32 = SCOPE0_X_LOW - LOW_SIZE_1.0 as i32;

/// ドラッグで scope1 を右へずらす量。
const DRAG_DX: i32 = 120;

/// ドラッグ後（`x=646+120=766`・幅 672）に低水準へ遷移した場合の再射影後 X。
///
/// 中央 `766 + 336 = 1102`、幅 336 で `1102 − 168 = 934`。解き直しから外れる限りここに留まる
/// （解き直しの対象に入っていれば隣接位置 1173 へ動く）。
const SCOPE1_X_AFTER_DRAG_LOW: i32 = 934;

/// 会話中の表情差替でシェルが取る別寸（高水準・幅だけが変わる）。
const SWAPPED_SIZE_0: (u32, u32) = (800, 1094);

/// 高水準の実表示寸（連鎖の駆動条件＝窓寸と一致していること）。
fn high_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(HIGH_SIZE_0)), (1, Some(HIGH_SIZE_1))])
}

/// 低水準の実表示寸。
fn low_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(LOW_SIZE_0)), (1, Some(LOW_SIZE_1))])
}

// ---------------------------------------------------------------------------
// 駆動の型（起動 → 遷移）
// ---------------------------------------------------------------------------

/// 高水準で起動し、起動時の連鎖確定まで済ませる（＝二体が隣接した状態を作る）。
///
/// 拡大率の相の初回 run は永続 `SystemState` の仕様で全窓へマッチするので、ここで実表示寸を
/// 報告させて高水準の寸へ landing させる。窓書込のキューと witness は末尾で掃除するので、
/// 以後の主張は**遷移が起こした書込だけ**を見る。
fn boot_at_high(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(HIGH_DPI);
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    source.refresh.insert(shell_target(0).0, HIGH_SIZE_0);
    source.refresh.insert(shell_target(1).0, HIGH_SIZE_1);
    harness.advance_frame();
    harness.run_placement_phases(source);
    harness.run_chain_finalize(&high_sizes());
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "前提が崩れている: 起動時の連鎖確定が駆動していない"
    );
    assert_eq!(
        (char_x(harness, 0), char_x(harness, 1)),
        (SCOPE0_X_HIGH, SCOPE1_X_HIGH),
        "前提が崩れている: 高水準で二体が隣接していない（この状態からでないと隙間の復活を観測できない）"
    );
    assert_eq!(gap(harness), 0, "前提が崩れている: 高水準の隙間が 0 でない");
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
}

/// OS 設定の拡大率変更（高→低）を 1 フレームで流し込む。
///
/// 作業領域源は触らない——実行時のモニタ表から作り直すのが同期段の仕事である。
fn lower_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_table_for_dpi(LOW_DPI);
    harness.set_window_dpi(LOW_DPI);
    source.refresh.insert(shell_target(0).0, LOW_SIZE_0);
    source.refresh.insert(shell_target(1).0, LOW_SIZE_1);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// OS 設定の拡大率変更（低→高）を 1 フレームで流し込む（2 度目の遷移を作る口）。
fn raise_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    source.refresh.insert(shell_target(0).0, HIGH_SIZE_0);
    source.refresh.insert(shell_target(1).0, HIGH_SIZE_1);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// 当該スコープのキャラ窓の現在左上 X。
fn char_x(harness: &FrameHarness, scope: usize) -> i32 {
    pos_of(&harness.world, harness.char_window(scope))
        .expect("position がある")
        .x
}

/// 当該スコープの既定位置（台帳）の X。
fn default_x(harness: &FrameHarness, scope: usize) -> Option<i32> {
    harness
        .world
        .get_resource::<GhostWindows>()
        .expect("GhostWindows がある")
        .default_char_pos(scope)
        .map(|p| p.x)
}

/// 隣接ペアの**隙間**＝`scope0 の左端 − scope1 の右端`（連鎖規則 `x(n−1) = x(n) + w(n)`）。
///
/// 実機サインオフが目視する量そのものである（200% で 0・100% で 359 だった）。
fn gap(harness: &FrameHarness) -> i32 {
    let right_of_scope1 = char_x(harness, 1)
        + size_of(&harness.world, harness.char_window(1))
            .expect("size がある")
            .width;
    char_x(harness, 0) - right_of_scope1
}

/// 当該スコープのキャラ窓を右へずらす（利用者のドラッグが landing した後の状態）。
///
/// ドラッグ経路そのもの（`on_char_drag`）は `placement/follow_drag_tests.rs` が所有するので、
/// ここでは**結果の位置**だけを置く。既定位置は動かさない——ドラッグは route を持たない書込で
/// あり、既定位置の追跡（D9）の対象から構造的に外れているからである。
fn drag_char_window_right(harness: &mut FrameHarness, scope: usize, dx: i32) {
    let window = harness.char_window(scope);
    let mut wp = harness
        .world
        .get_mut::<WindowPos>(window)
        .expect("WindowPos がある");
    let wp = wp.bypass_change_detection();
    let moved = wp.position.expect("position がある");
    wp.position = Some(Point {
        x: moved.x + dx,
        y: moved.y,
    });
}

/// 当該スコープのキャラ窓へ整合待ちの札を置く（拡大率と表が未整合のまま止まっている状態）。
fn hold_char_window(harness: &mut FrameHarness, scope: usize) {
    let window = harness.char_window(scope);
    let since_frame = harness.frame();
    harness
        .world
        .entity_mut(window)
        .insert(DpiSyncHold { since_frame });
}

/// 整合待ちの札を外す。
fn release_char_window(harness: &mut FrameHarness, scope: usize) {
    let window = harness.char_window(scope);
    harness.world.entity_mut(window).remove::<DpiSyncHold>();
}

// ---------------------------------------------------------------------------
// 観察可能な完了条件（要件 6.1／6.2）
// ---------------------------------------------------------------------------

/// **是正前は赤・是正後は緑**: 幅が半分になる遷移のあと、隣接ペアの隙間が 0 に戻る。
///
/// 是正前は左端差の和（実機 359px の決定論版）が残る。
#[test]
fn the_gap_returns_after_a_scale_change_and_the_realign_closes_it() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_at_high(&mut harness, &mut source);

    lower_the_scale(&mut harness, &mut source);

    // 前提: 遷移は実際に幅を半分にし、隙間を開けている（開いていなければ以下は空虚）。
    assert_eq!(
        (char_x(&harness, 0), char_x(&harness, 1)),
        (SCOPE0_X_LOW, SCOPE1_X_LOW_BEFORE_REALIGN),
        "前提が崩れている: 低水準への再射影が想定の位置へ landing していない"
    );
    assert_eq!(
        gap(&harness),
        GAP_WITHOUT_REALIGN,
        "前提が崩れている: 遷移で隙間が開いていない（この探針では欠陥を観測できない）"
    );

    harness.run_chain_realign(&low_sizes());

    assert_eq!(
        gap(&harness),
        0,
        "遷移後の連鎖の解き直しが効いていない（隣接ペアの隙間が残っている・要件 6.1）"
    );
    assert_eq!(
        char_x(&harness, 1),
        SCOPE1_X_AFTER_REALIGN,
        "解き直しが scope1 を隣接位置へ収めていない"
    );
    assert_eq!(
        char_x(&harness, 0),
        SCOPE0_X_LOW,
        "連鎖の起点（先頭スコープ）が動いた（解き直しは先頭を動かさない）"
    );
    // 既定位置は**単一の窓書込口**が運ぶ（D9／D16）——`ChainRealign` はシステム由来ゆえ
    // 追跡が発火する。ここが追随しないと、次の遷移で当該スコープが「明示的に動かされた」へ
    // 倒れて 2 度目の解き直しが空振りする。
    assert_eq!(
        default_x(&harness, 1),
        Some(SCOPE1_X_AFTER_REALIGN),
        "解き直し先へ既定位置が追随していない（次の遷移で解き直しが空振りする・D9／D16）"
    );
}

/// 起動時の確定標識（[`ChainFinalized`]）は解き直しでも解除されない（設計 D8）。
///
/// 解除してしまうと「起動時の確定は一度きり」という意味そのものが失われ、会話中の表情差替で
/// 起動時確定が再駆動し得る（`scope-chain-gap` 7.4 の破壊）。
#[test]
fn the_startup_finalized_marker_survives_the_realign() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_at_high(&mut harness, &mut source);
    lower_the_scale(&mut harness, &mut source);
    harness.run_chain_realign(&low_sizes());

    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "遷移後の解き直しが起動時の確定標識を解除した（設計 D8 は解除しないと定める）"
    );
    // 解き直しそのものは成立していること（標識が残っただけの空虚な緑にしない）。
    assert_eq!(
        gap(&harness),
        0,
        "前提が崩れている: 解き直しが駆動していない"
    );
}

// ---------------------------------------------------------------------------
// 一度きり（要件 6.6）
// ---------------------------------------------------------------------------

/// 遷移 1 回につき解き直しは**ちょうど 1 回**である（要件 6.6）。
///
/// 2 度目以降のフレームでは武装が解けており、窓書込を 1 本も出さない（要件 4.7 の側面）。
#[test]
fn one_transition_realigns_exactly_once() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_at_high(&mut harness, &mut source);
    lower_the_scale(&mut harness, &mut source);
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_some(),
        "前提が崩れている: 拡大率の遷移で解き直しが武装していない（要件 6.1）"
    );

    let _transition_writes = harness.drain_writes();
    harness.run_chain_realign(&low_sizes());
    let first = harness.drain_writes();
    assert!(
        !first.is_empty(),
        "前提が崩れている: 1 度目の解き直しが窓書込を出していない"
    );
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_none(),
        "解き直しの後も武装が残っている（次のフレームでも解き直す＝一度きりではない）"
    );

    // 定常フレームを 3 つ回す——武装が解けている以上、解き直しは 1 本も書かない。
    for _ in 0..3 {
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        harness.run_chain_realign(&low_sizes());
    }
    assert!(
        harness.drain_writes().is_empty(),
        "解き直しが遷移 1 回につき 2 回以上駆動している（要件 6.6）"
    );
    assert_eq!(
        char_x(&harness, 1),
        SCOPE1_X_AFTER_REALIGN,
        "定常フレームで scope1 が動いた（解き直しが繰り返し駆動している）"
    );
}

/// 武装と解決が同一 tick に落ちるので、キャラ窓の書込は**窓あたり 1 回のまま**である
/// （設計 C4 の「C2 の合流で `DpiReproject` 指令へ畳まれる」・要件 4.5 の側面）。
///
/// 解き直しが窓書込の回数を増やしたら、遷移の逐次性を減らすという本仕様の主目的と衝突する。
#[test]
fn the_realign_write_coalesces_into_the_reprojection_of_the_same_tick() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_at_high(&mut harness, &mut source);

    lower_the_scale(&mut harness, &mut source);
    harness.run_chain_realign(&low_sizes());

    let scope1_char = harness
        .world
        .get::<WindowHandle>(harness.char_window(1))
        .expect("WindowHandle がある")
        .hwnd;
    let writes = harness.drain_writes();
    let for_scope1: Vec<&SetWindowPosCommand> =
        writes.iter().filter(|c| c.hwnd == scope1_char).collect();
    assert_eq!(
        for_scope1.len(),
        1,
        "同一 tick の再射影と解き直しが 1 本に畳まれていない（窓ごとの書込が増えた・要件 4.5）: {writes:?}"
    );
    assert_eq!(
        (for_scope1[0].x, for_scope1[0].width),
        (SCOPE1_X_AFTER_REALIGN, LOW_SIZE_1.0 as i32),
        "畳まれた 1 本が最終ジオメトリ（解き直し後の位置＋遷移後の寸）を持っていない"
    );
}

/// 会話中の表情差替（拡大率不変）では解き直しを **1 度も武装しない**（要件 6.6）。
///
/// 陽性の対（拡大率が動けば武装する）を同じテスト本体の末尾に置く——武装を丸ごと止めても
/// 前半だけなら恒真で緑になるからである。
#[test]
fn a_surface_swap_during_a_conversation_never_arms_the_realign() {
    {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        boot_at_high(&mut harness, &mut source);

        // 表情差替: 拡大率は動かさず、表示成立点の状態照合だけが新しい寸を報告する。
        source.pending.insert(shell_target(0).0, SWAPPED_SIZE_0);
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        harness.run_reconcile(&mut source);
        assert_eq!(
            size_of(&harness.world, harness.char_window(0))
                .expect("size がある")
                .width,
            SWAPPED_SIZE_0.0 as i32,
            "前提が崩れている: 表情差替の寸が窓へ landing していない（この後の主張が空虚になる）"
        );

        assert!(
            harness
                .world
                .get_resource::<ChainRealignPending>()
                .is_none(),
            "表情差替で解き直しが武装した（`scope-chain-gap` 7.4 の破壊・要件 6.6）"
        );
        let before = char_x(&harness, 1);
        harness.run_chain_realign(&PerTargetSizes::new([
            (0, Some(SWAPPED_SIZE_0)),
            (1, Some(HIGH_SIZE_1)),
        ]));
        assert_eq!(
            char_x(&harness, 1),
            before,
            "表情差替で連鎖が解き直された（会話中にキャラが横へ動く）"
        );
    }

    // 陽性の対: 同じ駆動口でも拡大率が動けば武装する（上の 0 件が空虚でないことの担保）。
    let mut fresh = FrameHarness::new();
    let mut fresh_source = FakeReports::default();
    boot_at_high(&mut fresh, &mut fresh_source);
    lower_the_scale(&mut fresh, &mut fresh_source);
    assert!(
        fresh.world.get_resource::<ChainRealignPending>().is_some(),
        "陽性の対が成立していない（武装が死んでいれば上の 0 件は空虚である）"
    );
}

// ---------------------------------------------------------------------------
// 明示的に再配置されたスコープの除外（要件 6.2）
// ---------------------------------------------------------------------------

/// ドラッグで動かしたスコープは、遷移後の解き直しでも対象から外れたままである（要件 6.2）。
#[test]
fn a_dragged_scope_stays_out_of_the_realign() {
    {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        boot_at_high(&mut harness, &mut source);

        drag_char_window_right(&mut harness, 1, DRAG_DX);
        assert_ne!(
            default_x(&harness, 1),
            Some(char_x(&harness, 1)),
            "前提が崩れている: ドラッグ後も既定位置と現在位置が一致している"
        );

        lower_the_scale(&mut harness, &mut source);
        harness.run_chain_realign(&low_sizes());

        assert_eq!(
            char_x(&harness, 1),
            SCOPE1_X_AFTER_DRAG_LOW,
            "ドラッグ済みスコープが解き直しで動かされた（`scope-chain-gap` 7.3 の破壊）"
        );
    }

    // 陽性の対: ドラッグしなければ同じ駆動で解き直しが効く。
    let mut fresh = FrameHarness::new();
    let mut fresh_source = FakeReports::default();
    boot_at_high(&mut fresh, &mut fresh_source);
    lower_the_scale(&mut fresh, &mut fresh_source);
    fresh.run_chain_realign(&low_sizes());
    assert_eq!(
        char_x(&fresh, 1),
        SCOPE1_X_AFTER_REALIGN,
        "陽性の対が成立していない（解き直しが死んでいれば上の主張は空虚である）"
    );
}

// ---------------------------------------------------------------------------
// 見送りと停滞診断（要件 6.3）
// ---------------------------------------------------------------------------

/// 整合待ちの札が残っているあいだは解き直さず、札が外れたフレームで解決する。
#[test]
fn a_pending_dpi_sync_hold_defers_the_realign_until_it_is_released() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_at_high(&mut harness, &mut source);
    lower_the_scale(&mut harness, &mut source);
    let _transition_writes = harness.drain_writes();

    hold_char_window(&mut harness, 0);
    harness.run_chain_realign(&low_sizes());
    assert!(
        harness.drain_writes().is_empty(),
        "整合待ちの札がある窓が残っているのに解き直しが窓を書いた（要件 5.8 の 2 段書込）"
    );
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_some(),
        "見送りで武装が解けている（次のフレームで解き直せなくなる）"
    );
    assert_eq!(
        gap(&harness),
        GAP_WITHOUT_REALIGN,
        "見送りのはずが位置が動いている"
    );

    release_char_window(&mut harness, 0);
    harness.run_chain_realign(&low_sizes());
    assert_eq!(
        gap(&harness),
        0,
        "札が外れたフレームで解き直しが走っていない（見送りが恒久化している）"
    );
}

/// 見送りの警告は**有界を超えたところで一度だけ**出て、武装のたびに数え直される（要件 6.3）。
///
/// 捕捉はスレッド局所の subscriber で行い、駆動も同一スレッドの直接呼出である
/// （多スレッド実行器の相では 1 行も捕捉できない＝要件 7.6）。
#[test]
fn the_stall_warning_is_emitted_once_per_arming() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_at_high(&mut harness, &mut source);
    lower_the_scale(&mut harness, &mut source);
    hold_char_window(&mut harness, 0);

    // 1 度目の待ち: 有界（600 フレーム）に到達したところで 1 行だけ出る。
    let first_episode = capture_logs(|| {
        for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
            harness.run_chain_realign(&low_sizes());
        }
    });
    assert_eq!(
        count_level(&first_episode, "WARN"),
        1,
        "有界の待ちを超えた見送りの警告が一度だけ出ていない（1 度目・要件 6.3）"
    );
    assert!(
        first_episode
            .iter()
            .any(|line| line.contains("reason=\"dpi-sync-held\"")),
        "見送り理由が記録されていない（捕捉行: {first_episode:?}）"
    );

    // 待ち続けても 2 行目は出ない（同じ停滞で溢れさせない）。
    let still_waiting = capture_logs(|| {
        for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
            harness.run_chain_realign(&low_sizes());
        }
    });
    assert_eq!(
        count_level(&still_waiting, "WARN"),
        0,
        "同じ停滞で警告が繰り返し出ている"
    );

    // 待ちを解いて 1 度目の遷移を解決し、2 度目の遷移で武装し直す。
    release_char_window(&mut harness, 0);
    harness.run_chain_realign(&low_sizes());
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_none(),
        "前提が崩れている: 1 度目の遷移が解決していない"
    );
    raise_the_scale(&mut harness, &mut source);
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_some(),
        "前提が崩れている: 2 度目の遷移で武装していない"
    );
    hold_char_window(&mut harness, 0);

    // 2 度目の待ち: 武装時に計数と一発フラグが初期化されているので、また一度だけ出る。
    let second_episode = capture_logs(|| {
        for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
            harness.run_chain_realign(&high_sizes());
        }
    });
    assert_eq!(
        count_level(&second_episode, "WARN"),
        1,
        "2 度目の待ちで警告が出ていない（武装時の停滞診断の初期化が効いていない・要件 6.3）"
    );
}
