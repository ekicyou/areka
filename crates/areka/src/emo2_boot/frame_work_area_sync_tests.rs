//! 作業領域源の実行時同期（task 5.1・設計 C6・要件 5.1／5.4／5.5／5.6／5.7）の決定論テスト。
//!
//! # ここが押さえる是正
//!
//! 作業領域源は起動時に 1 度だけ作られ、以後どのフレームでも作り直されなかった（確定台帳
//! L3）。タスクバーの高さは論理寸で宣言され物理 px では拡大率に比例するので、拡大率を
//! 下げると真の作業領域下端は**下がる**。起動時の値が焼き付いたままだと下端吸着のキャラ窓は
//! 古い下端へ接地し続け、実機の 200%→100% は 6 遷移すべてで接地点 −48px の浮きを出した。
//!
//! [`lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write`] が
//! その形そのものである。同期を入れる前は接地点が旧下端に留まって差 −48px で赤くなり、
//! 入れた後は同一フレームの拡大率の相が新しい下端へ**1 回の書込で**接地させて緑になる
//! （要件 7.3 が求める対テスト）。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 「表に変化のないフレームでは作り直さない」は零件の主張であり、同期を丸ごと無操作にしても
//! 恒真で通る。同じ駆動口が**変化があれば作り直す**ことを
//! [`a_changed_monitor_table_rebuilds_both_sources`] が別に固定する。順序だけの違い・
//! モニタ 0 台・窓書込ゼロの各主張にも、同じ理由でそれぞれ陽性の対がある。

use crate::placement::WORK_AREA_SYNC_CONTEXT;
use crate::placement::follow::{MonitorDpiTable, MonitorSnapshot, MonitorSources};
use crate::placement::test_support::capture_logs;
use wintf::ecs::window::monitor::Monitor;

use super::test_support::{
    FakeReports, FrameHarness, s2_monitors, s2_sources, s2_work_area_for_dpi,
};

/// 起動時の拡大率（実機の採取と同じ向き＝高い側から始める）。
const BOOT_DPI: u16 = 192;
/// 遷移先の拡大率（下げる向き＝作業領域下端が**下がる**側）。
const LOWERED_DPI: u16 = 96;

/// 是正前に実機が出した接地点の差（`ground_y − wa_bottom`・浮きゆえ負）。
///
/// 合成レイアウトの値から導く（要件 5.6: 絶対 px を判定に直書きしない）。実機の −48px と
/// 一致するのは、どちらもタスクバー論理高 48px が 2 倍と等倍の間で動くためである。
fn stale_source_float() -> i32 {
    s2_work_area_for_dpi(BOOT_DPI).bottom - s2_work_area_for_dpi(LOWERED_DPI).bottom
}

/// 当該スコープのキャラ窓の接地点 Y（下端）。
fn ground_y(harness: &FrameHarness, scope: usize) -> i32 {
    harness.ground_point(scope).1
}

/// 起動直後の整地——2 源と実行時のモニタ表と窓の拡大率をすべて `BOOT_DPI` へ揃え、
/// キャラ窓を当該水準の作業領域下端へ接地させる。
///
/// 拡大率の相の初回 run は永続 `SystemState` の仕様で全窓へマッチするので、ここで 1 度
/// 空回しして消費する（以後のフレームでは真に変化した窓だけが対象になる）。
fn settle_at_boot_dpi(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table_for_dpi(BOOT_DPI);
    harness.set_window_dpi(BOOT_DPI);
    harness.advance_frame();
    harness.run_work_area_sync();
    harness.run_dpi_phase(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    assert_eq!(
        ground_y(harness, 0),
        s2_work_area_for_dpi(BOOT_DPI).bottom,
        "前提が崩れている: 起動水準でキャラ窓が作業領域下端へ接地していない"
    );
}

// ---------------------------------------------------------------------------
// 是正（要件 5.1・5.8）
// ---------------------------------------------------------------------------

/// **是正前は赤・是正後は緑**（要件 7.3）: 拡大率を下げた遷移で、キャラ窓が 1 回の書込のまま
/// 新しい作業領域下端へ接地する。
///
/// 是正前は同期段が無いので作業領域源が起動時（192）のまま残り、接地点は旧下端に留まって
/// 実際の下端との差が [`stale_source_float`]（−48px）になる。
#[test]
fn lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at_boot_dpi(&mut harness, &mut source);

    // OS 設定の拡大率変更（経路 (b)＝モニタ表の更新が先・窓の拡大率が続く）。
    // 作業領域源は**触らない**——それを実行時のモニタ表から作り直すのが同期段の仕事である。
    harness.set_monitor_table_for_dpi(LOWERED_DPI);
    harness.set_window_dpi(LOWERED_DPI);
    harness.advance_frame();
    harness.run_work_area_sync();
    harness.run_dpi_phase(&mut source);

    let new_bottom = s2_work_area_for_dpi(LOWERED_DPI).bottom;
    let diff = ground_y(&harness, 0) - new_bottom;
    assert_eq!(
        diff,
        0,
        "接地点が新しい作業領域下端に載っていない（差 {diff}px・是正前の値は {float}px）",
        float = stale_source_float(),
    );

    // 一度書き（要件 5.8）: 当該キャラ窓への窓書込は 1 件だけ。
    let writes = harness.drain_writes();
    let char_writes = writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(0) && cmd.tag.kind == "char")
        .count();
    assert_eq!(
        char_writes, 1,
        "キャラ窓の書込が 1 回ではない（中間矩形を挟んでいる）: {writes:?}"
    );
}

/// 上の対照——**探針が退化していない**こと。是正前の差はちょうど −48px であり、
/// 合成レイアウトが「どの水準でも下端が同じ」へ縮退していたら本テストが先に落ちる。
#[test]
fn the_two_scale_levels_really_move_the_work_area_bottom() {
    assert_eq!(
        stale_source_float(),
        -48,
        "合成レイアウトが実機の −48px と同じ量を作っていない（探針が退化している）"
    );
}

// ---------------------------------------------------------------------------
// 変化したフレームだけ作り直す（要件 5.4・4.7）
// ---------------------------------------------------------------------------

/// 零件の主張: モニタ表に変化のないフレームでは 2 源を作り直さない。
#[test]
fn an_unchanged_monitor_table_does_not_rebuild_the_sources() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table_for_dpi(BOOT_DPI);
    harness.advance_frame();

    assert!(
        harness.run_work_area_sync().is_none(),
        "同じ表なのに作業領域源を作り直している（定常フレームの無操作契約に反する）"
    );
    assert_eq!(
        harness.work_area_source(),
        Some(&s2_sources(BOOT_DPI).snapshot),
        "作り直していないのに作業領域源が変わっている"
    );
}

/// 上の**陽性の対**: 同じ駆動口が、表が変われば 2 源とも作り直す。
///
/// これが無いと、同期を丸ごと無操作にしても零件の主張は恒真で通る。
#[test]
fn a_changed_monitor_table_rebuilds_both_sources() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table_for_dpi(LOWERED_DPI);
    harness.advance_frame();

    let change = harness
        .run_work_area_sync()
        .expect("表が変わったのに作り直していない");
    let expected = s2_sources(LOWERED_DPI);
    assert_eq!(
        change.previous,
        s2_sources(BOOT_DPI).snapshot,
        "差し替え前の値が記録されていない（task 5.2 が動いた作業領域を特定できない）"
    );
    assert_eq!(change.current, expected.snapshot);
    assert_eq!(
        harness.work_area_source(),
        Some(&expected.snapshot),
        "作業領域源が新しい表から作り直されていない"
    );
    assert_eq!(
        harness.monitor_dpi_table(),
        Some(&expected.dpi_table),
        "モニタ別拡大率表が作業領域源と同時に作り直されていない（片方だけ古い運転になる）"
    );
}

/// 拡大率だけが変わって作業領域が動かない構成変更でも作り直す。
///
/// 比較が作業領域だけを見ていると、この変化を静かに取りこぼして表だけが古いまま残る
/// （整合待ち＝task 5.4 が引く拡大率が永久に古くなる）。
#[test]
fn a_monitor_whose_scale_changed_without_moving_its_work_area_still_rebuilds() {
    let mut harness = FrameHarness::new();
    let mut monitors = s2_monitors(BOOT_DPI);
    harness.set_work_area_source(MonitorSources::from_monitors(&monitors).snapshot);
    harness.set_monitor_dpi_table(MonitorDpiTable::from_monitors(&monitors));
    // 作業領域も矩形も動かさず、拡大率だけを変える。
    monitors[0].dpi += 24;
    harness.set_monitor_table(monitors.clone());
    harness.advance_frame();

    assert!(
        harness.run_work_area_sync().is_some(),
        "拡大率だけの変化を取りこぼしている"
    );
    assert_eq!(
        harness.monitor_dpi_table(),
        Some(&MonitorDpiTable::from_monitors(&monitors)),
        "モニタ別拡大率表に新しい拡大率が載っていない"
    );
}

/// 順序に依存しない比較（設計 C6）: 内容が同じで**並びだけ**違う表では作り直さない。
///
/// 実行時のモニタ表の走査順は列挙順と一致する保証が無い。素の `==` で比べると、中身が
/// 同じなのに毎フレーム作り直す運転になり、定常フレームの無操作契約が並びという無関係な
/// 理由で壊れる。
#[test]
fn reordering_the_monitor_table_does_not_rebuild_the_sources() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    let mut reordered = s2_monitors(BOOT_DPI);
    reordered.reverse();
    harness.set_monitor_table(reordered);
    harness.advance_frame();

    assert!(
        harness.run_work_area_sync().is_none(),
        "並びが違うだけの同一構成で作業領域源を作り直している"
    );
}

/// 上の**陽性の対**: 並びが同じでも中身が違えば作り直す（比較が恒真に潰れていない）。
#[test]
fn the_order_independent_comparison_still_sees_a_real_difference() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    let mut changed = s2_monitors(BOOT_DPI);
    changed[0].work_area.bottom -= 1;
    harness.set_monitor_table(changed);
    harness.advance_frame();

    assert!(
        harness.run_work_area_sync().is_some(),
        "1px の違いを「同じ」と見なしている（順序非依存の比較が中身まで潰している）"
    );
}

// ---------------------------------------------------------------------------
// 解決できないときは現状維持＋警告（要件 5.5）
// ---------------------------------------------------------------------------

/// モニタ 0 台（列挙異常）では源を差し替えず、警告を 1 件残す。
///
/// 空の源で今の源を潰すと、下端が引けなくなって全キャラ窓の射影が縮退する——架空の矩形を
/// 発明しないのと同じ理由で、**無い表で在る源を上書きしない**。
#[test]
fn an_empty_monitor_table_keeps_the_sources_and_warns() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table(Vec::new());
    harness.advance_frame();

    let (change, events) = capture_logs(|| harness.run_work_area_sync());
    assert!(change.is_none(), "空の表で源を差し替えている");
    assert_eq!(
        harness.work_area_source(),
        Some(&s2_sources(BOOT_DPI).snapshot),
        "空の表で作業領域源が潰れている"
    );
    let warnings: Vec<&str> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| e.message())
        .filter(|m| m.contains(WORK_AREA_SYNC_CONTEXT))
        .collect();
    assert_eq!(
        warnings.len(),
        1,
        "モニタ 0 台が無言で素通りしている（ログ無し失敗経路）: {events:?}"
    );
}

/// 上の**陽性の対**: 表が 1 台でもあれば警告は出ない（警告が常時鳴っていない）。
#[test]
fn a_non_empty_monitor_table_does_not_warn() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table_for_dpi(LOWERED_DPI);
    harness.advance_frame();

    let (change, events) = capture_logs(|| harness.run_work_area_sync());
    assert!(change.is_some(), "前提: 差し替えが起きる表を渡している");
    assert!(
        events.iter().all(|e| e.level != tracing::Level::WARN),
        "正常な表更新で警告が出ている: {events:?}"
    );
}

// ---------------------------------------------------------------------------
// 同期そのものは窓を動かさない（要件 5.7・task 5.2 との境界）
// ---------------------------------------------------------------------------

/// 作業領域だけが変わったフレームで、同期は窓書込を 1 件も出さない。
///
/// 同期は資源を差し替えるだけである。変化した作業領域に属する窓を現寸で射影し直すのは
/// 作業領域変化を契機とする再スナップ（task 5.2）の仕事であり、ここでそれを先取りすると
/// 保存位置の復元経路にも効いてしまう（要件 5.7 が禁じる拡大率をまたぐ追従）。
#[test]
fn the_sync_alone_writes_no_windows() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at_boot_dpi(&mut harness, &mut source);

    // 窓の拡大率は据え置き、作業領域（＝モニタ表）だけを動かす。
    harness.set_monitor_table_for_dpi(LOWERED_DPI);
    harness.advance_frame();
    assert!(
        harness.run_work_area_sync().is_some(),
        "前提: 作業領域が動くフレームを組めている"
    );

    let writes = harness.drain_writes();
    assert!(
        writes.is_empty(),
        "同期段が窓を動かしている（再スナップは task 5.2 の持ち物）: {writes:?}"
    );
    assert_eq!(
        ground_y(&harness, 0),
        s2_work_area_for_dpi(BOOT_DPI).bottom,
        "同期段が接地点を動かしている"
    );
}

/// 上の**陽性の対**: 同じ土台で拡大率まで動けば、拡大率の相が窓を書く。
///
/// これが無いと「窓書込 0」は駆動そのものが死んでいても緑になる。
#[test]
fn the_same_frame_writes_windows_once_the_scale_moves_too() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at_boot_dpi(&mut harness, &mut source);

    harness.set_monitor_table_for_dpi(LOWERED_DPI);
    harness.set_window_dpi(LOWERED_DPI);
    harness.advance_frame();
    harness.run_work_area_sync();
    harness.run_dpi_phase(&mut source);

    assert!(
        !harness.drain_writes().is_empty(),
        "拡大率が動いたのに窓書込が 1 件も出ない（駆動が死んでいる）"
    );
}

// ---------------------------------------------------------------------------
// 置き場（拡大率の相より前・同一フレーム）
// ---------------------------------------------------------------------------

/// 同期の呼出は毎フレームの相の**前**に置かれている（設計 C6・要件 5.8）。
///
/// 順序そのものは
/// [`lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write`]
/// が挙動で示すが、あちらはテストが自分で順に呼ぶ形ゆえ、本番の相順が入れ替わっても赤に
/// ならない。担い手は `frame.rs` の呼出順そのものなので、その形を本文で名指しする。
#[test]
fn the_sync_is_called_before_the_scale_phase_in_the_frame_system() {
    let code = include_str!("frame.rs");
    let sync = code
        .find("work_area_sync::sync_monitor_snapshot(world)")
        .expect("毎フレームの同期呼出が frame.rs に無い");
    let dpi = code
        .find("run_dpi_phase(&mut wiring, world)")
        .expect("拡大率の相の呼出が frame.rs に無い");
    assert!(
        sync < dpi,
        "同期が拡大率の相より後に置かれている（相が旧下端へ書いてから源が新しくなる＝2 段書込）"
    );
}

// ---------------------------------------------------------------------------
// 遷移観測レコード（`kind=snapshot` の発行点）
// ---------------------------------------------------------------------------

/// 同期を 1 回回し、遷移観測の行だけを拾う。
///
/// 捕捉窓の内側は観測 target が点いた状態である（既定 OFF そのものは
/// `placement/follow_transition_diag_tests.rs` が directive 単位で所有する）。
fn transition_lines(harness: &mut FrameHarness) -> Vec<String> {
    let (_, events) = capture_logs(|| {
        harness.run_work_area_sync();
    });
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with("[transition]"))
        .collect()
}

/// 差し替えたフレームでは `kind=snapshot` が 1 行出て、台数と各台の拡大率・作業領域が載る。
#[test]
fn a_rebuilt_source_emits_one_snapshot_record() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table_for_dpi(LOWERED_DPI);
    harness.advance_frame();
    let frame = harness.frame();

    let lines = transition_lines(&mut harness);
    let snapshots: Vec<&String> = lines
        .iter()
        .filter(|m| m.contains("kind=snapshot"))
        .collect();
    assert_eq!(
        snapshots.len(),
        1,
        "作業領域源を作り直したフレームの記録が 1 行ではない: {lines:?}"
    );
    let line = snapshots[0];
    let wa = s2_work_area_for_dpi(LOWERED_DPI);
    assert!(
        line.contains(&format!("frame={frame} ")),
        "記録が同期したフレームを名乗っていない: {line}"
    );
    assert!(line.contains("monitors=2"), "台数が載っていない: {line}");
    assert!(
        line.contains(&format!(
            "m0={dpi}:{left},{top},{right},{bottom}",
            dpi = u32::from(LOWERED_DPI),
            left = wa.left,
            top = wa.top,
            right = wa.right,
            bottom = wa.bottom,
        )),
        "新しい作業領域と拡大率が載っていない: {line}"
    );
}

/// 対（零件）: 作り直していないフレームでは 1 行も出ない。
///
/// 毎フレーム出す形にすると、判定側が遷移を切り出すときの雑音になるうえ、定常状態で
/// 確保が走る（要件 10.4）。
#[test]
fn an_unchanged_frame_emits_no_snapshot_record() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table_for_dpi(BOOT_DPI);
    harness.advance_frame();

    let lines = transition_lines(&mut harness);
    assert!(
        lines.iter().all(|m| !m.contains("kind=snapshot")),
        "作り直していないフレームで記録が出ている: {lines:?}"
    );
}

/// 差し替えたときは人が読む側の構成ログも同じフレームで出て、**呼出点タグで出所が判る**
/// （起動時の構築点と同じ共有ヘルパ・タグだけが違う）。
#[test]
fn a_rebuilt_source_also_logs_the_monitor_configuration_with_its_own_call_site_tag() {
    let mut harness = FrameHarness::new();
    harness.set_monitor_sources_for_dpi(BOOT_DPI);
    harness.set_monitor_table_for_dpi(LOWERED_DPI);
    harness.advance_frame();

    let (_, events) = capture_logs(|| harness.run_work_area_sync());
    let headers: Vec<&str> = events
        .iter()
        .map(|e| e.message())
        .filter(|m| m.starts_with("[diag.monitor_snapshot]"))
        .collect();
    assert_eq!(
        headers.len(),
        1,
        "構成ログの見出しが 1 行ではない: {events:?}"
    );
    assert!(
        headers[0].contains(&format!("context={WORK_AREA_SYNC_CONTEXT}")),
        "同期段が起動時の構築点と同じ呼出点タグを名乗っている（出所を弁別できない）: {}",
        headers[0]
    );
}

/// 記録の**材料を組むより外側に**前置ガードが在る（要件 10.4）。
///
/// 観測が消えている運転で確保が走らないことは、発行が `debug!` である以上、濾過テストでは
/// 検出できない（組んでから捨てても緑になる）。担い手は本文の形そのものなので、ここで名指しする
/// ——`placement/follow_transition_diag_tests.rs` の接地点レコードと同じ流儀である。
#[test]
fn the_snapshot_record_is_built_behind_the_front_guard() {
    let code = include_str!("frame/work_area_sync.rs");
    let guard = code
        .find("if transition_diag::is_enabled()")
        .expect("前置ガードが無い");
    let build = code
        .find("let record: Vec<MonitorEntry>")
        .expect("記録の材料を組む箇所が無い");
    assert!(
        guard < build,
        "記録の材料をガードの外で組んでいる（観測が消えていても確保が走る）"
    );
    assert_eq!(
        code.matches("transition_diag::log_monitor_snapshot_sync(")
            .count(),
        1,
        "発行点が 1 箇所ではない（ガードの掛かっていない発行が増えていないか）"
    );
}

// ---------------------------------------------------------------------------
// 起動時と実行時が同じ構築関数を通る（要件 5.1）
// ---------------------------------------------------------------------------

/// 同期段が作る 2 源は、起動シームが同じモニタ列から作る 2 源と**同一の値**である。
///
/// 起動時だけが別の作り方をすると、同期が入った後も起動時の値だけが違う形になり得る。
#[test]
fn the_sync_builds_the_same_sources_the_boot_seam_builds() {
    let monitors: Vec<Monitor> = s2_monitors(LOWERED_DPI);
    let boot = MonitorSources::from_monitors(&monitors);

    let mut harness = FrameHarness::new();
    harness.set_work_area_source(MonitorSnapshot {
        work_areas: Vec::new(),
    });
    harness.set_monitor_dpi_table(MonitorDpiTable::default());
    harness.set_monitor_table(monitors);
    harness.advance_frame();
    harness.run_work_area_sync();

    assert_eq!(harness.work_area_source(), Some(&boot.snapshot));
    assert_eq!(harness.monitor_dpi_table(), Some(&boot.dpi_table));
}
