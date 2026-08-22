//! 遷移後の連鎖再解決の**武装条件**そのものの檻（task 5.6・設計 C4・要件 6.1／6.6）。
//!
//! # ここが押さえるもの
//!
//! 武装は `dpi.rs` の 3 連言——⑴ 窓書込が起きた ⑵ キャラ窓である ⑶ 寸が変わった——で決まる。
//! 3 つとも設計 C4 が名指しした条件（「キャラ窓の `DpiReproject` が**寸変化を伴って**書込を
//! 発生させたとき」）であり、どれか 1 つでも落ちると意味が変わる:
//!
//! - ⑶ を落とすと、拡大率を変えずに**作業領域だけが動いた**フレーム（タスクバーの表示切替等）で
//!   毎回武装する。幅は 1px も変わっていないので隣接は崩れておらず、解き直しは無駄な書込になる。
//! - ⑵ を落とすと、バルーン窓の寸変化で武装する。バルーンは連鎖の構成要素ではない。
//! - ⑴ を落とすと、反映口が縮退（報告寸に 0 軸がある等）で**何も書かなかった**フレームでも
//!   武装する。位置は動いていないので解き直す理由が無い。
//!
//! 隣接の是正そのもの（359→0）は `frame_chain_realign_tests.rs` が持つ。本ファイルは
//! **武装の特異性**だけを主題にする——あちらの檻は 3 連言のどれを外しても緑のままだからである。
//!
//! # 純関数の直接呼び（`size_changed`）
//!
//! 寸比較の 3 分岐のうち **i32 変換失敗の腕は駆動からは踏めない**（`reconcile_window_size` が
//! 先に超過を弾いて `false` を返し、武装条件の `&&` が短絡する）。届かない腕を無検査で残さない
//! ため、`size_changed` を `pub(super)` へ上げて単位で呼ぶ。

use bevy_ecs::entity::Entity;
use wintf::ecs::SizeI;

use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::chain_realign::ChainRealignPending;

use super::dpi::size_changed;
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, WRITER_WITNESS, arrangement_offset_of,
    assert_no_write, s2_monitors_with_work_area, s2_taskbar_hidden_work_area, size_of,
};
use super::{balloon_target, shell_target};

/// 本ファイルは拡大率を**動かさない**（動かすと寸変化と作業領域変化が同時に来て、
/// どちらが武装させたのか判別できなくなる）。
const DPI_LEVEL: u16 = 192;

/// 起動時に landing させる scope0 の実表示寸。
const SIZE_0: (u32, u32) = (764, 1094);
/// 起動時に landing させる scope1 の実表示寸。
const SIZE_1: (u32, u32) = (672, 596);

/// 遷移後の寸（ちょうど半分＝幅が変わる側）。
const HALVED_0: (u32, u32) = (382, 547);
/// 同上（scope1）。
const HALVED_1: (u32, u32) = (336, 298);

/// バルーン窓が取る新しい寸（spawn 時 223x158 の 2 倍）。
const BALLOON_SIZE: (u32, u32) = (446, 316);

/// 起動時の実表示寸（連鎖確定の駆動条件＝窓寸と一致していること）。
fn settled_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SIZE_0)), (1, Some(SIZE_1))])
}

/// 窓寸を読む補助（未付与は前提崩れとして落とす）。
fn expect_size(harness: &FrameHarness, window: Entity) -> SizeI {
    size_of(&harness.world, window).expect("size がある")
}

/// 報告寸（`u32` の対）を窓寸の通貨（`i32`）へ。
fn as_window_size(reported: (u32, u32)) -> SizeI {
    SizeI::new(reported.0 as i32, reported.1 as i32)
}

/// 起動を済ませ、**連鎖確定済み・未武装**の状態を作る（武装の前提が揃った地点）。
///
/// 起動フレームの拡大率の相でも寸は変わるが、そこではまだ [`ChainFinalized`] が無いので
/// 武装しない（解き直す連鎖がそもそも無い）。その事実もここで固定する。
fn boot_and_finalize(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(DPI_LEVEL);
    harness.set_monitor_table_for_dpi(DPI_LEVEL);
    harness.set_window_dpi(DPI_LEVEL);
    source.refresh.insert(shell_target(0).0, SIZE_0);
    source.refresh.insert(shell_target(1).0, SIZE_1);
    harness.advance_frame();
    harness.run_placement_phases(source);
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_none(),
        "起動時の確定がまだ無いフレームで武装した（解き直す連鎖が存在しない）"
    );

    harness.run_chain_finalize(&settled_sizes());
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "前提が崩れている: 起動時の連鎖確定が駆動していない"
    );
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_none(),
        "前提が崩れている: 武装の前提地点で既に武装している"
    );
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
}

/// **同一の駆動口**——作業領域を動かし（＝位置書込が必ず起きる）、拡大率の相を 1 フレーム回す。
///
/// 引数で変えるのは**報告する実表示寸だけ**である。ほかの入力（作業領域の動き・窓の拡大率の
/// 再挿入・相の並び）は呼出ごとに bit 同一ゆえ、観測された差は寸変化の有無**だけ**に帰せられる。
fn drive_reprojection(
    harness: &mut FrameHarness,
    source: &mut FakeReports,
    reported: [(u32, u32); 2],
) {
    // 作業領域だけを動かす（タスクバーを隠した構成）。拡大率（`Monitor.dpi`）は据え置き。
    harness.set_monitor_table(s2_monitors_with_work_area(
        DPI_LEVEL,
        s2_taskbar_hidden_work_area(DPI_LEVEL),
    ));
    // 同値の再挿入でも `Changed<DPI>` は立つ（拡大率の相の対象に入れるための駆動）。
    harness.set_window_dpi(DPI_LEVEL);
    source.refresh.insert(shell_target(0).0, reported[0]);
    source.refresh.insert(shell_target(1).0, reported[1]);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

// ---------------------------------------------------------------------------
// ⑶ 寸変化の有無（設計 C4「寸変化を伴って」）
// ---------------------------------------------------------------------------

/// 寸が変わらない `DpiReproject` 書込では武装せず、**同じ駆動口で寸が変われば**武装する。
///
/// 前半だけだと武装を丸ごと止めても恒真で通る。陽性の対を同じ本体に置き、変えた入力を
/// 「報告寸」1 つに絞ってあるので、差が寸変化に帰せられることまで含めて固定される。
#[test]
fn a_reprojection_without_a_size_change_never_arms_but_one_with_it_does() {
    // 零件の側: 幅も高さも変わらないが、作業領域が動いたので位置は書かれる。
    {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        boot_and_finalize(&mut harness, &mut source);

        drive_reprojection(&mut harness, &mut source, [SIZE_0, SIZE_1]);

        // 前提: 書込は実際に起きている（起きていなければ ⑴ が効いただけで主張が空虚になる）。
        assert!(
            !harness.drain_writes().is_empty(),
            "前提が崩れている: 作業領域が動いたのに窓書込が 1 本も出ていない"
        );
        assert_eq!(
            expect_size(&harness, harness.char_window(0)),
            as_window_size(SIZE_0),
            "前提が崩れている: 寸が変わってしまっている（この駆動では寸は不変であるべき）"
        );
        assert!(
            harness
                .world
                .get_resource::<ChainRealignPending>()
                .is_none(),
            "寸が変わらない再射影で武装した（作業領域が動いただけでは隣接は崩れない・設計 C4）"
        );
    }

    // 陽性の対: 変えた入力は報告寸だけ。これで武装する。
    let mut fresh = FrameHarness::new();
    let mut fresh_source = FakeReports::default();
    boot_and_finalize(&mut fresh, &mut fresh_source);

    drive_reprojection(&mut fresh, &mut fresh_source, [HALVED_0, HALVED_1]);

    assert_eq!(
        expect_size(&fresh, fresh.char_window(0)),
        as_window_size(HALVED_0),
        "前提が崩れている: 陽性の対で寸が landing していない"
    );
    assert!(
        fresh.world.get_resource::<ChainRealignPending>().is_some(),
        "寸が変わる再射影で武装していない（陽性の対が成立しなければ上の 0 件は空虚である）"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 窓種別（キャラ窓だけが連鎖の構成要素）
// ---------------------------------------------------------------------------

/// バルーン窓だけが寸変化を起こしても武装しない（バルーンは連鎖の構成要素ではない）。
///
/// 陽性の対（キャラ窓が寸変化を起こせば武装する）を同じ本体の末尾に置く。
#[test]
fn a_balloon_only_size_change_never_arms_the_realign() {
    {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        boot_and_finalize(&mut harness, &mut source);

        // シェルには報告を置かない（キャラ窓は `None` の腕＝現寸のまま再射影）。
        harness.set_window_dpi(DPI_LEVEL);
        source.refresh.insert(balloon_target(0).0, BALLOON_SIZE);
        harness.advance_frame();
        harness.run_placement_phases(&mut source);

        // 前提: バルーン窓の寸は実際に変わっている（変わらなければ主張が空虚）。
        assert_eq!(
            expect_size(&harness, harness.balloon_window(0)),
            as_window_size(BALLOON_SIZE),
            "前提が崩れている: バルーン窓の寸が landing していない"
        );
        // 前提: キャラ窓の寸は動いていない（動いていたら ⑵ ではなく ⑶ を見てしまう）。
        assert_eq!(
            expect_size(&harness, harness.char_window(0)),
            as_window_size(SIZE_0),
            "前提が崩れている: キャラ窓の寸まで動いている"
        );
        assert!(
            harness
                .world
                .get_resource::<ChainRealignPending>()
                .is_none(),
            "バルーン窓の寸変化で武装した（バルーンは連鎖の構成要素ではない・設計 C4）"
        );
    }

    // 陽性の対: 同じフレーム構成でキャラ窓側が寸変化を起こせば武装する。
    let mut fresh = FrameHarness::new();
    let mut fresh_source = FakeReports::default();
    boot_and_finalize(&mut fresh, &mut fresh_source);
    fresh.set_window_dpi(DPI_LEVEL);
    fresh_source.refresh.insert(shell_target(0).0, HALVED_0);
    fresh.advance_frame();
    fresh.run_placement_phases(&mut fresh_source);
    assert!(
        fresh.world.get_resource::<ChainRealignPending>().is_some(),
        "陽性の対が成立していない（武装が死んでいれば上の 0 件は空虚である）"
    );
}

// ---------------------------------------------------------------------------
// ⑴ 書込が起きたか（縮退で何も書かなかったフレーム）
// ---------------------------------------------------------------------------

/// 反映口が縮退して**何も書かなかった**フレームでは武装しない。
///
/// 報告寸に 0 軸があると `reconcile_window_size` は `warn!` して窓寸を触らずに `false` を返す
/// （前寸維持）。寸の値としては前寸と違うので、書込の有無を見ていなければここで武装して
/// しまう——位置は 1px も動いていないのに解き直す理由は無い。
///
/// 陽性の対（同じ報告経路で正しい寸なら武装する）を同じ本体の末尾に置く。
#[test]
fn a_degenerate_report_that_writes_nothing_never_arms_the_realign() {
    {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        boot_and_finalize(&mut harness, &mut source);

        harness.set_window_dpi(DPI_LEVEL);
        source.refresh.insert(shell_target(0).0, (0, SIZE_0.1));
        harness.advance_frame();
        harness.run_placement_phases(&mut source);

        // 前提: 窓書込は 1 本も出ていない（単一ライター経路の witness が sentinel のまま）。
        assert_no_write(
            &harness.world,
            harness.char_window(0),
            "0 軸の報告寸で窓が書かれた",
        );
        assert!(
            harness.drain_writes().is_empty(),
            "前提が崩れている: 何も書かないはずのフレームで窓書込が出ている"
        );
        assert_eq!(
            expect_size(&harness, harness.char_window(0)),
            as_window_size(SIZE_0),
            "前提が崩れている: 0 軸の報告で窓寸が書き換わった（前寸維持のはず）"
        );
        assert!(
            harness
                .world
                .get_resource::<ChainRealignPending>()
                .is_none(),
            "書込が起きていないフレームで武装した（位置が動いていないのに解き直す理由は無い・設計 C4）"
        );
    }

    // 陽性の対: 同じ報告経路でも正しい寸なら書込が起きて武装する。
    let mut fresh = FrameHarness::new();
    let mut fresh_source = FakeReports::default();
    boot_and_finalize(&mut fresh, &mut fresh_source);
    fresh.set_window_dpi(DPI_LEVEL);
    fresh_source.refresh.insert(shell_target(0).0, HALVED_0);
    fresh.advance_frame();
    fresh.run_placement_phases(&mut fresh_source);
    assert_ne!(
        arrangement_offset_of(&fresh.world, fresh.char_window(0)),
        WRITER_WITNESS,
        "前提が崩れている: 陽性の対で窓書込が起きていない"
    );
    assert!(
        fresh.world.get_resource::<ChainRealignPending>().is_some(),
        "陽性の対が成立していない（武装が死んでいれば上の 0 件は空虚である）"
    );
}

// ---------------------------------------------------------------------------
// 寸比較の純関数（3 分岐すべて）
// ---------------------------------------------------------------------------

/// [`size_changed`] の 3 分岐を全網羅する。
///
/// i32 変換失敗の腕は駆動からは踏めない（`reconcile_window_size` が先に超過を弾き、武装条件の
/// `&&` が短絡する）ので、届かない腕を無検査にしないためここで直接呼ぶ。
#[test]
fn size_changed_covers_the_missing_size_the_overflow_and_the_comparison() {
    let before = Some(as_window_size(SIZE_0));

    // 比較の腕: 片軸だけの違いも「変わった」。同寸は「変わっていない」。
    assert!(size_changed(before, HALVED_0), "両軸が変われば変化");
    assert!(
        size_changed(before, (HALVED_0.0, SIZE_0.1)),
        "幅だけ変わっても変化"
    );
    assert!(
        size_changed(before, (SIZE_0.0, HALVED_0.1)),
        "高さだけ変わっても変化"
    );
    assert!(!size_changed(before, SIZE_0), "同寸は変化ではない");
    assert!(
        size_changed(Some(as_window_size(SIZE_1)), HALVED_1),
        "scope1 の寸の対でも同じ判定になる"
    );

    // 窓寸が引けない腕: 比較の相手が無い状態を変化と数えない（起動直後の初回 landing で
    // 毎回武装しないための腕）。
    assert!(
        !size_changed(None, HALVED_0),
        "窓寸が引けないとき（窓生成前）は「変わった」と言わない"
    );

    // i32 変換失敗の腕: 超過した報告寸は変化と数えない（値を捏造しない）。
    assert!(
        !size_changed(before, (u32::MAX, SIZE_0.1)),
        "幅が i32 域を超える報告は「変わった」と言わない"
    );
    assert!(
        !size_changed(before, (SIZE_0.0, u32::MAX)),
        "高さが i32 域を超える報告は「変わった」と言わない"
    );
}
