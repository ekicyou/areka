//! 既定位置の追跡（task 5.5・design D9／D16・要件 6.2）の**観察可能な帰結**を、多フレーム駆動で
//! 固定する。
//!
//! # ここが押さえる是正
//!
//! 連鎖の対象判定は「現在位置が既定位置と一致するか」で明示的な再配置を除外する
//! （`drain_resnap::collect_chain_states`＋`chain_finalize::finalize_chain`・
//! `scope-chain-gap` 7.3）。拡大率が変わるとキャラ窓は新しい寸で下端中央を保ったまま
//! 置き直され、左上 X が動く——既定位置が spawn 時のまま据え置かれると、**誰も触っていない
//! スコープ**が「明示的に動かされた」へ倒れて連鎖の解き直しが空振りする。要件 6.2 が
//! 名指しで禁じているのがこの空振りである。
//!
//! [`a_scale_change_leaves_the_scope_eligible_for_the_chain`] がその形そのものである。
//! 是正前は再射影後の X（910）に留まって赤くなり、既定位置がシステム由来の書込に追随すると
//! 連鎖が解けて隣接位置（710）へ収まる。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 「ドラッグ後のスコープは対象から外れたまま」は、連鎖を丸ごと止めても恒真で通る。ゆえに
//! [`a_dragged_scope_stays_out_of_the_chain_across_the_scale_change`] は、**同じ拡大率変更・
//! 同じ駆動口**でドラッグを行わない側が実際に動くことを同じテスト本体の中で続けて主張する。

use bevy_ecs::change_detection::DetectChangesMut;
use wintf::ecs::{Point, WindowPos};

use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::spawn::GhostWindows;

use super::shell_target;
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, pos_of,
};

/// 遷移前の拡大率水準。
const LOW_DPI: u16 = 96;

/// 遷移後の拡大率水準（等倍の 2 倍＝寸も作業領域下端も動く）。
const HIGH_DPI: u16 = 192;

/// [`HIGH_DPI`] で各スコープが報告する物理寸（等倍寸の 2 倍）。
const HIGH_SIZE_0: (u32, u32) = (SPAWN_SIZE_0.0 * 2, SPAWN_SIZE_0.1 * 2);
const HIGH_SIZE_1: (u32, u32) = (SPAWN_SIZE_1.0 * 2, SPAWN_SIZE_1.1 * 2);

/// 遷移後に scope1 が置かれる左上 X（下端中央保存の再射影後・連鎖はまだ解いていない）。
///
/// scope1 は spawn 時 `x=1049`・幅 278 ゆえ中央 1188。幅が 556 になると
/// `1188 − 556/2 = 910`。
const SCOPE1_X_AFTER_REPROJECT: i32 = 910;

/// scope1 の spawn 時の左上 X（`resnap_placements` の既定配置）。
const SCOPE1_SPAWN_X: i32 = 1049;

/// ドラッグで scope1 を右へずらす量。
const DRAG_DX: i32 = 120;

/// ドラッグ後（`x=1049+120=1169`・幅 278）に拡大率を上げた場合の再射影後 X。
///
/// 中央 `1169 + 139 = 1308`、幅 556 で `1308 − 278 = 1030`。連鎖から外れる限りここに留まる。
const SCOPE1_X_AFTER_DRAG_REPROJECT: i32 = 1030;

/// 連鎖が解けたときに scope1 が収まる左上 X。
///
/// scope0 は spawn 時 `x=1483`・幅 434 ゆえ中央 1700。幅 868 で `1700 − 434 = 1266`。
/// 連鎖規則 `new_x(n) = x(n−1) − w(n)` より `1266 − 556 = 710`。
const SCOPE1_X_AFTER_CHAIN: i32 = 710;

/// 遷移後の実表示寸（連鎖確定の駆動条件＝窓寸と一致していること）。
fn high_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(HIGH_SIZE_0)), (1, Some(HIGH_SIZE_1))])
}

/// 起動直後の整地——3 つの源と窓の拡大率をすべて [`LOW_DPI`] へ揃える。
///
/// 拡大率の相の初回 run は永続 `SystemState` の仕様で全窓へマッチするので、ここで 1 度
/// 空回しして消費する（以後のフレームでは真に変化した窓だけが対象になる）。
fn settle(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(LOW_DPI);
    harness.set_monitor_table_for_dpi(LOW_DPI);
    harness.set_window_dpi(LOW_DPI);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_none(),
        "前提が崩れている: 整地の時点で連鎖が確定済みになっている"
    );
}

/// OS 設定の拡大率変更を 1 フレームで流し込む（実行時のモニタ表・窓の拡大率・報告寸）。
///
/// 作業領域源は触らない——実行時のモニタ表から作り直すのが同期段の仕事である。
fn raise_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    source.refresh.insert(shell_target(0).0, HIGH_SIZE_0);
    source.refresh.insert(shell_target(1).0, HIGH_SIZE_1);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// 当該スコープの既定位置（台帳）。
fn default_x(harness: &FrameHarness, scope: usize) -> Option<i32> {
    harness
        .world
        .get_resource::<GhostWindows>()
        .expect("GhostWindows がある")
        .default_char_pos(scope)
        .map(|p| p.x)
}

/// 当該スコープのキャラ窓の現在左上 X。
fn char_x(harness: &FrameHarness, scope: usize) -> i32 {
    pos_of(&harness.world, harness.char_window(scope))
        .expect("position がある")
        .x
}

/// 当該スコープのキャラ窓を右へずらす（利用者のドラッグが landing した後の状態）。
///
/// ドラッグ経路そのもの（`on_char_drag`）は `placement/follow_drag_tests.rs` が所有するので、
/// ここでは**結果の位置**だけを置く。既定位置は動かさない——ドラッグは route を持たない書込で
/// あり、D9 の追跡対象から構造的に外れているからである。
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

// ---------------------------------------------------------------------------
// 観察可能な完了条件（要件 6.2）
// ---------------------------------------------------------------------------

/// **是正前は赤・是正後は緑**: 拡大率の再射影のあとでも、連鎖の対象判定が「明示的に
/// 動かされた」へ倒れない。
///
/// 判定器そのもの（`collect_chain_states`＋`finalize_chain`）を通して観る——既定位置と現在位置を
/// 直接比べるだけだと、判定器が既定位置を読まなくなっても緑のままになる。
#[test]
fn a_scale_change_leaves_the_scope_eligible_for_the_chain() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    raise_the_scale(&mut harness, &mut source);

    // 前提: 再射影は実際に窓を動かしている（動いていなければ以下の主張は空虚）。
    assert_eq!(
        char_x(&harness, 1),
        SCOPE1_X_AFTER_REPROJECT,
        "前提が崩れている: 拡大率の再射影で scope1 のキャラ窓 X が動いていない"
    );
    assert_eq!(
        default_x(&harness, 1),
        Some(SCOPE1_X_AFTER_REPROJECT),
        "既定位置がシステム由来の再射影に追随していない（要件 6.2・D9）"
    );

    // 判定器を通す: 追随していれば scope1 は対象に残り、連鎖が隣接位置へ収める。
    harness.run_chain_finalize(&high_sizes());
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "連鎖の確定が駆動していない（窓寸と実表示寸が揃っていない可能性）"
    );
    assert_eq!(
        char_x(&harness, 1),
        SCOPE1_X_AFTER_CHAIN,
        "連鎖が scope1 を対象から外した（既定位置が追随せず「明示的に動かされた」へ倒れている）"
    );
}

/// ドラッグで動かしたスコープは、拡大率が変わっても対象から外れたままである。
///
/// 陽性の対（ドラッグしない側は動く）を同じテスト本体の末尾に置く——連鎖を丸ごと止めても
/// 前半だけなら緑になるからである。
#[test]
fn a_dragged_scope_stays_out_of_the_chain_across_the_scale_change() {
    // ハーネスは 1 つずつ生かす——同一スレッドで 2 つ同時に生かすと、どちらの `drain_writes`
    // も両者の書込を取り出す（`FrameHarness` の doc）。陽性の対は本体の後で組む。
    {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        settle(&mut harness, &mut source);

        drag_char_window_right(&mut harness, 1, DRAG_DX);
        assert_ne!(
            default_x(&harness, 1),
            Some(char_x(&harness, 1)),
            "前提が崩れている: ドラッグ後も既定位置と現在位置が一致している"
        );

        raise_the_scale(&mut harness, &mut source);

        assert_eq!(
            default_x(&harness, 1),
            Some(SCOPE1_SPAWN_X),
            "ドラッグ済みスコープの既定位置が拡大率変更で動いた（D9 の一致条件が効いていない）"
        );

        harness.run_chain_finalize(&high_sizes());
        assert_eq!(
            char_x(&harness, 1),
            SCOPE1_X_AFTER_DRAG_REPROJECT,
            "ドラッグ済みスコープが連鎖で動かされた（`scope-chain-gap` 7.3 の破壊・隣接位置は {SCOPE1_X_AFTER_CHAIN}）"
        );
    }

    // 陽性の対: ドラッグしなければ同じ駆動で連鎖が解ける。
    let mut fresh = FrameHarness::new();
    let mut fresh_source = FakeReports::default();
    settle(&mut fresh, &mut fresh_source);
    raise_the_scale(&mut fresh, &mut fresh_source);
    fresh.run_chain_finalize(&high_sizes());
    assert_eq!(
        char_x(&fresh, 1),
        SCOPE1_X_AFTER_CHAIN,
        "陽性の対が成立していない（駆動が死んでいれば上の主張は空虚である）"
    );
}
