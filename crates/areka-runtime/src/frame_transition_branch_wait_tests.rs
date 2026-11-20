//! 拡大率の通知が表の更新より先に届く順序（経路 (a)）の整合待ちを、要件 7.2 が名指しする
//! 2 水準（120 と 192）で多フレーム駆動して固定する。
//!
//! 是正前の赤はここが持つ: 待ちが無いと、窓は新しい寸のまま旧作業領域下端へ接地した中間矩形で
//! 1 度書かれ、表が追いついた次のフレームでもう 1 度書き直される（要件 5.8 の 2 段書込）。
//!
//! 零件（「書込 0」「警告 0」）の主張には、同じテスト本体の内側に陽性の対を置く。駆動が
//! 死んでいれば零件はどれも空虚に緑になるからである。

use log_capture_kit::capture;

use crate::placement_tests::transition_judge::WRITES_PER_WINDOW_MAX;
use areka_placement::diag::WindowKind;
use areka_placement::dpi_sync::{DPI_SYNC_HOLD_MAX_FRAMES, evaluate};

use super::shared_test_support::{BALLOON_SPAWN_SIZE, HIGH_DPI, LOW_DPI, SCALE_120};
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1,
    s2_assert_work_area_bottom_moves, s2_work_area_for_dpi, size_of,
};
use super::transition_test_support::{
    deliver_the_scale_notice_first, holds_of, scaled, settle_at, warnings_containing, writes_for,
};
use super::{balloon_target, shell_target};

/// 待ち → 解除 → 当該窓への書込 1 回。旧作業領域下端の中間矩形は 1 度も可視化しない
/// （要件 5.8・設計 D15）。
///
/// 零件（待ちフレームの書込 0）の**陽性の対**は同じ本体の後半——解除フレームで 4 本の指令が
/// 出ることである。待ちが起きていなければ前半で書かれ、駆動が死んでいれば後半で 0 本になる。
fn scale_notice_first_lands_in_one_write_at(dpi: u16) {
    // 探針の非退化: この 2 水準で作業領域下端が実際に動く（動かなければ中間矩形は観測できない）。
    s2_assert_work_area_bottom_moves(LOW_DPI, dpi);

    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);

    let old_bottom = s2_work_area_for_dpi(LOW_DPI).bottom;
    let new_bottom = s2_work_area_for_dpi(dpi).bottom;

    // ── 待ちフレーム: 拡大率だけが新しく、表はまだ旧水準 ──────────────────
    deliver_the_scale_notice_first(&mut harness, &mut source, dpi);
    // 零件を主張する**前に**、このフレームで本当に食い違いが起きていることを固定する。
    for scope in harness.scopes().to_vec() {
        let outcome = evaluate(&harness.world, harness.char_window(scope), harness.frame());
        assert_eq!(
            (outcome.window_dpi, outcome.table_dpi),
            (u32::from(dpi), Some(u32::from(LOW_DPI))),
            "dpi={dpi} scope={scope}: 拡大率通知が先に届いた状態になっていない（探針が退化している）"
        );
        assert_eq!(
            holds_of(&harness, scope),
            (true, true),
            "dpi={dpi} scope={scope}: 拡大率と表が食い違うのに待ち札が付いていない（ゲートが走っていない）"
        );
    }
    let waiting = harness.drain_writes();
    assert!(
        waiting.is_empty(),
        "dpi={dpi}: 表が追いつく前に窓書込が出ている（旧下端 {old_bottom} の中間矩形）: {waiting:?}"
    );

    // ── 解除フレーム: 表が追いつく ───────────────────────────────────────
    harness.set_monitor_table_for_dpi(dpi);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let released = harness.drain_writes();

    // 陽性の対: 同じ駆動口が、待ちが解ければ 4 本（2 スコープ × キャラ／バルーン）書く。
    assert_eq!(
        released.len(),
        harness.scopes().len() * 2,
        "dpi={dpi}: 解除フレームの窓書込が 4 本ではない（駆動が死んでいる／合流が効いていない）: {released:?}"
    );
    for scope in harness.scopes().to_vec() {
        let scope32 = scope as u32;
        for kind in [WindowKind::Char, WindowKind::Balloon] {
            assert_eq!(
                writes_for(&released, scope32, kind).len() as u32,
                WRITES_PER_WINDOW_MAX,
                "dpi={dpi} scope={scope} {kind}: 窓あたりの書込が {WRITES_PER_WINDOW_MAX} 本ではない: {released:?}"
            );
        }
        let char_write = writes_for(&released, scope32, WindowKind::Char)
            .pop()
            .expect("キャラ窓の書込がある");
        assert_eq!(
            char_write.y + char_write.height,
            new_bottom,
            "dpi={dpi} scope={scope}: キャラ窓の書込が新しい作業領域下端に載っていない（旧下端 {old_bottom} の中間矩形）: {released:?}"
        );
        assert_eq!(
            harness.ground_point(scope).1,
            new_bottom,
            "dpi={dpi} scope={scope}: 遷移後の接地点が新しい作業領域下端に載っていない"
        );
        assert_eq!(
            holds_of(&harness, scope),
            (false, false),
            "dpi={dpi} scope={scope}: 表が追いついたのに待ち札が外れていない"
        );
    }
}

/// 要件 7.2 の低い側（120）。
#[test]
fn scale_notice_first_lands_in_one_write_at_120() {
    scale_notice_first_lands_in_one_write_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn scale_notice_first_lands_in_one_write_at_192() {
    scale_notice_first_lands_in_one_write_at(HIGH_DPI);
}

/// 待ちのあいだに表情差替（drain 相の `ShowSurface` が積む新 k・寸変化の窓寸要求）が来ても、
/// 報告寸の突合と再スナップの**どちらからも**当該窓への書込は 0 のままである
/// （設計 C5 議題 1・D15 の「4 点すべて」のうち、この 2 点）。
///
/// 陽性の対は同じ本体の後半——待ちが解けた後は、**同じ 2 つの駆動口**が書く。
fn a_surface_swap_during_the_wait_writes_nothing_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);
    deliver_the_scale_notice_first(&mut harness, &mut source, dpi);
    let _waiting = harness.drain_writes();

    let swapped_0 = scaled(SPAWN_SIZE_0, dpi);
    let swapped_1 = scaled(SPAWN_SIZE_1, dpi);
    assert_ne!(
        swapped_0, SPAWN_SIZE_0,
        "探針が退化している: dpi={dpi} の表情差替が寸を動かさない"
    );
    let size_before = size_of(&harness.world, harness.char_window(0)).expect("窓寸がある");

    // 待ち中の表情差替: drain 相が積んだ未消費の窓寸要求と、実表示寸の食い違い。
    source.pending.insert(shell_target(0).0, swapped_0);
    source
        .pending
        .insert(balloon_target(0).0, scaled(BALLOON_SPAWN_SIZE, dpi));
    harness.run_reconcile(&mut source);
    harness.run_resnap(&PerTargetSizes::new([
        (0, Some(swapped_0)),
        (1, Some(swapped_1)),
    ]));

    let writes = harness.drain_writes();
    assert!(
        writes.is_empty(),
        "dpi={dpi}: 待ち札のある窓へ報告寸の突合・再スナップから窓書込が届いている: {writes:?}"
    );
    assert!(
        source.pending.contains_key(&shell_target(0).0),
        "dpi={dpi}: 報告を消費してしまっている（待ち中は消費せず次フレームへ持ち越す）"
    );
    assert_eq!(
        size_of(&harness.world, harness.char_window(0)),
        Some(size_before),
        "dpi={dpi}: 待ち中に窓寸が動いている（見送りは寸にも及ぶ）"
    );

    // ── 陽性の対（同じ本体・同じ 2 つの駆動口）: 待ちが解ければ書く ──────
    harness.set_monitor_table_for_dpi(dpi);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let _released = harness.drain_writes();
    assert_eq!(
        holds_of(&harness, 0),
        (false, false),
        "dpi={dpi}: 表が追いついたのに待ち札が外れていない（陽性の対が成立しない）"
    );

    // ⑴ 報告寸の突合。
    source
        .pending
        .insert(shell_target(0).0, (swapped_0.0 + 8, swapped_0.1));
    harness.run_reconcile(&mut source);
    assert!(
        !harness.drain_writes().is_empty(),
        "dpi={dpi}: 待ちが解けても報告寸の突合が書いていない（駆動口が死んでいる）"
    );
    // ⑵ 再スナップ。
    harness.run_resnap(&PerTargetSizes::new([
        (0, Some((swapped_0.0 + 16, swapped_0.1))),
        (1, None),
    ]));
    assert!(
        !harness.drain_writes().is_empty(),
        "dpi={dpi}: 待ちが解けても再スナップが書いていない（駆動口が死んでいる）"
    );
}

/// 要件 7.2 の低い側（120）。
#[test]
fn a_surface_swap_during_the_wait_writes_nothing_at_120() {
    a_surface_swap_during_the_wait_writes_nothing_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn a_surface_swap_during_the_wait_writes_nothing_at_192() {
    a_surface_swap_during_the_wait_writes_nothing_at(HIGH_DPI);
}

/// 上限フレームを超えたら、**警告の上で**現在の源のまま進む（設計 C5・D15・要件 4.4）。
///
/// 上限の値は本番の定数（[`DPI_SYNC_HOLD_MAX_FRAMES`]）を引く——回帰テストが自前の数字を
/// 持つと、片方だけ緩めたときに静かに食い違う。
///
/// 警告 0 件（待っているあいだ）と警告 1 件以上（上限フレーム）を**同じ本体で**問う
/// ——上限判定を丸ごと外しても、後半だけを見ていれば「常に鳴っている」で緑になる。
fn the_wait_gives_up_after_the_bounded_number_of_frames_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);
    deliver_the_scale_notice_first(&mut harness, &mut source, dpi);
    let _waiting = harness.drain_writes();

    // 表は永遠に追いつかない。上限に達するまでは 1 件も書かず、警告も出さない。
    for _ in 0..DPI_SYNC_HOLD_MAX_FRAMES - 1 {
        harness.advance_frame();
        let (_, events) = capture(|| {
            harness.run_placement_phases(&mut source);
        });
        let writes = harness.drain_writes();
        assert!(
            writes.is_empty(),
            "dpi={dpi}: 上限に達する前に書いている（frame={}）: {writes:?}",
            harness.frame()
        );
        assert_eq!(
            warnings_containing(&events, "dpi sync:"),
            0,
            "dpi={dpi}: 上限に達する前に打ち切りの警告が出ている（frame={}）: {events:?}",
            harness.frame()
        );
    }

    harness.advance_frame();
    let (_, events) = capture(|| {
        harness.run_placement_phases(&mut source);
    });
    let writes = harness.drain_writes();
    assert!(
        !writes.is_empty(),
        "dpi={dpi}: 上限（{DPI_SYNC_HOLD_MAX_FRAMES} フレーム）を超えても待ち続けている（有界でない）"
    );
    assert!(
        warnings_containing(&events, "dpi sync:") > 0,
        "dpi={dpi}: 上限を超えて進んだのに警告が 1 件も出ていない（ログ無し失敗経路）: {events:?}"
    );
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            holds_of(&harness, scope),
            (false, false),
            "dpi={dpi} scope={scope}: 上限を超えたのに待ち札が残っている"
        );
    }
}

/// 要件 7.2 の低い側（120）。
#[test]
fn the_wait_gives_up_after_the_bounded_number_of_frames_at_120() {
    the_wait_gives_up_after_the_bounded_number_of_frames_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn the_wait_gives_up_after_the_bounded_number_of_frames_at_192() {
    the_wait_gives_up_after_the_bounded_number_of_frames_at(HIGH_DPI);
}

/// 待ち札が見送るのは**窓書込の 5 つの点（＋シェルの切り替えの見送り）だけ**であり、描画の相
/// （`run_drain_phase`）は素通しである（設計 D15「描画そのものは止めない」）。
///
/// 決定論テストが買える射程はここまでである——本ハーネスは GPU を持たないので `apply_show` が
/// 実際に画を出したことは検証できない。ゆえに問うのは**構造**——描画の相の本文に整合ゲートの
/// 参照が 1 つも無く、逆に 5 つの見送り点にはあること（陽性の対）である。発話・アニメが実機で
/// 遅れていないことの持ち分は実機サインオフ（要件 8.4 の目視所見併記）にある。
#[test]
fn the_wait_defers_window_writes_only_and_leaves_the_drawing_phase_untouched() {
    let drain_resnap = include_str!("frame/drain_resnap.rs");
    let start = drain_resnap
        .find("pub fn run_drain_phase")
        .expect("描画（指令適用）の相が frame/drain_resnap.rs に無い");
    let end = drain_resnap[start..]
        .find("pub fn run_move_drain_phase")
        .expect("描画の相の次の関数が見つからない（切片の終端が取れない）")
        + start;
    let drawing_phase = &drain_resnap[start..end];

    // 陽性の対: この切片が本当に描画（present 指令の適用）の相である。
    assert!(
        drawing_phase.contains("wiring.presenter.apply(world, cmd)"),
        "切片が描画の相になっていない（`apply` の呼出が無い）: {drawing_phase}"
    );
    for gate in ["dpi_sync", "DpiSyncHold"] {
        assert!(
            !drawing_phase.contains(gate),
            "描画の相が整合ゲート（`{gate}`）を参照している——待ちが描画を止めている（設計 D15 違反）"
        );
    }

    // 陽性の対: 5 つの見送り点にはゲートがある（無ければ上の「無い」は空虚）。
    assert!(
        include_str!("frame/dpi.rs").contains("dpi_sync::apply_dpi_phase_gate(world, window, now)"),
        "拡大率の相にゲートが無い（見送り点の 1 つ目）"
    );
    assert!(
        include_str!("frame/scale_text.rs")
            .contains("dpi_sync::defers_window_write(world, window, HoldSite::Reconcile)"),
        "報告寸の突合にゲートが無い（見送り点の 2 つ目）"
    );
    assert!(
        drain_resnap
            .contains("dpi_sync::defers_window_write(world, char_window, HoldSite::Resnap)"),
        "実表示寸の再スナップにゲートが無い（見送り点の 3 つ目）"
    );
    assert!(
        include_str!("frame/work_area_sync.rs")
            .contains("dpi_sync::defers_window_write(world, window, HoldSite::WorkAreaResnap)"),
        "作業領域変化を契機とする再スナップにゲートが無い（見送り点の 4 つ目）"
    );
    assert!(
        include_str!("frame/move_shift.rs")
            .contains("dpi_sync::defers_window_write(world, window, HoldSite::PatternMove)"),
        "型の移動にゲートが無い（見送り点の 5 つ目）"
    );
}
