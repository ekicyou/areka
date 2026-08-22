//! 多フレーム駆動ハーネス（[`FrameHarness`]）**そのもの**の檻。
//!
//! 本ファイルが固定するのはゴースト窓の挙動ではなく、**後続タスクの統合テストが乗る土台の
//! 性質**である——フレームを進める口・3 つの源（作業領域源／実行時のモニタ表／窓の拡大率）を
//! 差し替える口・二体ぶんのスコープを同時に駆動できること・そして
//! **テスト間で状態が残らないこと**（要件 7.7）。
//!
//! # なぜ「構造から明らか」では足りないか
//!
//! 本仕様は既に一度、テスト間の状態汚染を実測で出している（`command.rs` のプロセス共有
//! カウンタ＝60 回中 11 失敗）。**残留は「隔離されているように見える」形からも出る**ので、
//! 隔離は構成で主張せず実行で示す:
//!
//! - [`a_fresh_harness_starts_from_a_known_state_regardless_of_what_ran_before`] は
//!   残留を**その場でわざと作ってから**ハーネスを組み、既知の状態から始まることを見る。
//! - [`two_scenarios_in_sequence_reach_the_same_verdict`] は同一プロセス・同一スレッドで
//!   2 つのシナリオを続けて走らせ、後のシナリオの判定が前の残留で変わらないことを見る。
//!
//! いずれも [`FrameHarness::new`] の初期化（キューの取り出し・観測の写しの初期化）を外すと
//! 赤になる。
//!
//! # 実行形態の明示（要件 7.5／7.6）
//!
//! - **x64 のみ**: 本モジュールの接続宣言（`frame.rs`）が `target_pointer_width = "64"` で
//!   守られている。[`the_always_on_harness_tests_run_on_x64_only`] がその形を名指しする。
//! - **単一スレッド実行器**: ログ捕捉に依る検証は
//!   [`a_log_capture_verification_names_the_single_threaded_executor`] のとおり
//!   [`single_threaded_schedule`] で走らせる。既定の多スレッド実行器ではシステムがワーカー
//!   スレッドへ載り、スレッドローカルの捕捉が 1 行も拾えないまま緑になる。

use std::collections::BTreeSet;
use std::time::Instant;

use bevy_ecs::prelude::*;

/// 単一スレッド実行器の供給元（`frame_test_support::single_threaded_schedule`）の逐語。
/// bevy 0.19 で `Schedule::get_executor_kind` が撤去されたため、「捕捉できる実行形態を
/// 名指しする」要件 7.6 の要請を字面で見張る。
const FRAME_TEST_SUPPORT_SRC: &str = include_str!("frame_test_support.rs");
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};
use wintf::ecs::FrameCount;
use wintf::ecs::window::SetWindowPosCommand;
use wintf::ecs::window::transition_diag::{self, TickStart};

use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::follow::MonitorSnapshot;
use crate::placement::resolver::RectPx;
use crate::placement::test_support::capture_logs as capture_diag_logs;
use crate::placement::transition_diag as placement_diag;

use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, assert_no_write,
    s2_work_area_for_dpi, settled_sizes, single_threaded_schedule,
};

/// 残留を作るための偽指令（実窓を持たない・実行はされない）。
fn residue_command() -> SetWindowPosCommand {
    SetWindowPosCommand::new(
        HWND(0x9999 as *mut _),
        1,
        2,
        3,
        4,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    )
}

/// 1 シナリオ: 96 で整地してから 192 へ拡大率と作業領域源を差し替え、DPI 相を 1 回回す。
///
/// 戻り値は当該シナリオが積んだ窓書込の**要求語彙タグ**（スコープと窓種別）の列＝
/// 「このシナリオの判定」。残留があれば列の中身か長さが変わる。
fn run_scale_up_scenario(harness: &mut FrameHarness) -> Vec<(Option<u32>, &'static str)> {
    let mut source = FakeReports::default();

    // 96 の作業領域へ整地した状態で 1 フレーム回し、`SystemState` の初回全窓マッチを
    // 消費する（fixture は 96 の下端へ接地済みゆえ書込は出ないが、出ても捨てる）。
    harness.set_work_area_source_for_dpi(96);
    harness.advance_frame();
    harness.run_dpi_phase(&mut source);
    let _priming = harness.drain_writes();

    // OS 側の拡大率変更（作業領域源と窓の拡大率が同時に動く＝経路 (b)）。
    harness.set_work_area_source_for_dpi(192);
    harness.set_window_dpi(192);
    harness.advance_frame();
    harness.run_dpi_phase(&mut source);

    harness
        .drain_writes()
        .iter()
        .map(|cmd| (cmd.tag.scope, cmd.tag.kind))
        .collect()
}

// ---------------------------------------------------------------------------
// 残留を持ち越さない（要件 7.7）
// ---------------------------------------------------------------------------

/// **残留をわざと作ってから**組んだハーネスが、既知の状態から始まる。
///
/// 作る残留は 2 種類——観測の写し（フレーム番号）と窓書込キュー。どちらも
/// [`FrameHarness::new`] の初期化を外すと assert が落ちる。
#[test]
fn a_fresh_harness_starts_from_a_known_state_regardless_of_what_ran_before() {
    transition_diag::begin_tick(999, Instant::now());
    SetWindowPosCommand::enqueue(residue_command());
    assert_eq!(
        transition_diag::current_frame(),
        999,
        "前提: 観測の写しに残留を作れている（作れていなければ本檻は何も見ていない）"
    );

    let mut harness = FrameHarness::new();

    assert_eq!(harness.frame(), 0, "ハーネスのフレーム番号は 0 から始まる");
    assert_eq!(
        transition_diag::current_frame(),
        0,
        "観測の写しが初期化されていない（前のテストのフレーム番号が残る＝要件 7.7 違反）"
    );
    assert!(
        harness.drain_writes().is_empty(),
        "窓書込キューが初期化されていない（前のテストの指令が残る＝要件 7.7 違反）"
    );
}

/// 同一プロセス・同一スレッドで 2 つのシナリオを続けて走らせても、**後のシナリオの判定が
/// 前の残留で変わらない**（本タスクの観察可能な完了条件）。
///
/// 1 つ目のハーネスは**生かしたまま**（`Drop` に頼らない）2 つ目を組む。後始末が
/// [`FrameHarness::new`] 側にあることを、この形が要求している。
#[test]
fn two_scenarios_in_sequence_reach_the_same_verdict() {
    let mut first = FrameHarness::new();
    let first_writes = run_scale_up_scenario(&mut first);
    assert!(
        !first_writes.is_empty(),
        "非空虚性: シナリオが窓書込を 1 本も積んでいない（判定の比較が空虚になる）"
    );
    assert_ne!(first.frame(), 0, "前提: 持ち越すフレーム番号がある");

    // 1 つ目が残していくもの: 進んだフレーム番号（観測の写し）と、取り出されないままの指令。
    SetWindowPosCommand::enqueue(residue_command());
    assert_eq!(
        transition_diag::current_frame(),
        first.frame(),
        "前提: 観測の写しに 1 つ目のフレーム番号が残っている"
    );

    let mut second = FrameHarness::new();
    assert_eq!(second.frame(), 0, "2 つ目は 0 から始まる");
    assert_eq!(
        transition_diag::current_frame(),
        0,
        "1 つ目のフレーム番号が 2 つ目へ持ち越されている（要件 7.7 違反）"
    );
    assert!(
        second.drain_writes().is_empty(),
        "1 つ目が残した窓書込指令が 2 つ目のキューに載っている（要件 7.7 違反）"
    );

    let second_writes = run_scale_up_scenario(&mut second);
    assert_eq!(
        second_writes, first_writes,
        "同じシナリオの判定が、先行シナリオの残留で変わっている（要件 7.7 違反）"
    );
}

// ---------------------------------------------------------------------------
// フレームを進める口
// ---------------------------------------------------------------------------

/// [`FrameHarness::advance_frame`] は刻印の 2 つの権威——World 資源（`FrameCount`＋
/// `TickStart`）とスレッド局所の写し——を**同一点で同じ値**へ進める（D1・`try_tick_world`
/// と同じ規律）。片方だけ進めると、World を借りる観測点と借りない観測点で系列が割れる。
#[test]
fn advancing_frames_stamps_the_world_resources_and_the_mirror_alike() {
    let mut harness = FrameHarness::new();

    assert_eq!(harness.advance_frame(), 1, "1 周目は frame=1");
    assert_eq!(harness.advance_frame(), 2, "2 周目は frame=2");
    assert_eq!(harness.frame(), 2);
    assert_eq!(
        harness.world.resource::<FrameCount>().0,
        2,
        "World 資源のフレーム番号が進んでいない"
    );
    assert_eq!(
        placement_diag::stamp_of(&harness.world).frame,
        2,
        "World 資源から組んだ刻印がフレーム番号を持たない（`TickStart` 未挿入だと 0 へ潰れる）"
    );
    assert_eq!(
        transition_diag::current_frame(),
        2,
        "スレッド局所の写しが World 資源と食い違っている（D1 の 1 系列が割れる）"
    );
}

// ---------------------------------------------------------------------------
// 3 つの源の差し替え
// ---------------------------------------------------------------------------

/// 作業領域源（[`MonitorSnapshot`] 資源）と実行時のモニタ表（`Monitor` entity 群）は
/// **独立に**差し替えられる。
///
/// 2 源を独立に置けることが要件 5.3 の観測（接地点と作業領域下端の差）の前提である——
/// 同じ源から両方を引いたら差は定義上つねに 0 になり、何も観測しない。
#[test]
fn the_work_area_source_and_the_monitor_table_are_swapped_independently() {
    let mut harness = FrameHarness::new();
    harness.set_work_area_source_for_dpi(96);
    harness.set_monitor_table_for_dpi(192);

    let source_bottom = harness
        .world
        .resource::<MonitorSnapshot>()
        .work_areas
        .first()
        .expect("作業領域源が 1 台以上ある")
        .bottom;
    assert_eq!(
        source_bottom,
        s2_work_area_for_dpi(96).bottom,
        "作業領域源が 96 水準になっていない"
    );

    // ゴースト窓が居るモニタ（index 0）の内側に収まる探針矩形。
    let probe = RectPx {
        left: 1483,
        top: 700,
        right: 1917,
        bottom: 1387,
    };
    let live = placement_diag::live_work_area_bottom(&mut harness.world, probe)
        .expect("実行時のモニタ表が 1 台以上ある");
    assert_eq!(
        live,
        s2_work_area_for_dpi(192).bottom,
        "実行時のモニタ表が 192 水準になっていない"
    );
    assert_ne!(
        source_bottom, live,
        "探針の退化: 2 源が同値では独立に差し替わったことを観測できない"
    );
}

// ---------------------------------------------------------------------------
// 二体ぶんのスコープを同時に駆動する
// ---------------------------------------------------------------------------

/// ハーネスは隣接ペア（2 スコープ）を持ち、**両方同時**にも**片方だけ**にも駆動できる。
///
/// 連鎖の検証は隣接ペアを要するため二体が要り、経路 (a)／整合待ちの検証は「片方だけ
/// 拡大率が変わった」状態を要するため per-scope の差し替え口が要る。
#[test]
fn the_harness_drives_both_scopes_and_each_scope_alone() {
    let mut harness = FrameHarness::new();
    assert_eq!(harness.scopes(), [0, 1], "隣接ペアの 2 スコープを持つ");

    let mut source = FakeReports::default();
    harness.set_work_area_source_for_dpi(96);
    harness.advance_frame();
    harness.run_dpi_phase(&mut source);
    let _priming = harness.drain_writes();

    // 二体同時（OS 設定の拡大率変更＝全窓の拡大率が動く）。
    harness.set_work_area_source_for_dpi(192);
    harness.set_window_dpi(192);
    harness.advance_frame();
    harness.run_dpi_phase(&mut source);
    let both: BTreeSet<Option<u32>> = harness
        .drain_writes()
        .iter()
        .map(|cmd| cmd.tag.scope)
        .collect();
    assert_eq!(
        both,
        BTreeSet::from([Some(0), Some(1)]),
        "二体ぶんのスコープが同時に駆動できていない"
    );
    let bottom_192 = s2_work_area_for_dpi(192).bottom;
    for scope in [0usize, 1] {
        assert_eq!(
            harness.ground_point(scope).1,
            bottom_192,
            "scope {scope} の接地点が新しい作業領域下端へ載っていない"
        );
    }

    // フェーズ境界で書込 witness を戻し、「以降の書込」だけを見る。
    harness.reset_write_witness();

    // 片方だけ（scope 1 の拡大率は据え置き＝`Changed<DPI>` が立たない）。
    harness.set_work_area_source_for_dpi(120);
    harness.set_scope_dpi(0, 120);
    harness.advance_frame();
    harness.run_dpi_phase(&mut source);
    let only_first: BTreeSet<Option<u32>> = harness
        .drain_writes()
        .iter()
        .map(|cmd| cmd.tag.scope)
        .collect();
    assert_eq!(
        only_first,
        BTreeSet::from([Some(0)]),
        "片方のスコープだけを駆動できていない"
    );
    assert_no_write(
        &harness.world,
        harness.char_window(1),
        "拡大率を据え置いたスコープのキャラ窓",
    );
}

/// 再スナップと連鎖確定も同じハーネスから駆動でき、**隣接ペアが揃って初めて**連鎖が確定する。
#[test]
fn the_harness_drives_resnap_and_chain_finalize_for_the_adjacent_pair() {
    let mut harness = FrameHarness::new();
    harness.advance_frame();

    // 片方が未表示（`None`）のあいだは確定しない＝隣接ペアが揃うことが条件。
    let half = PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, None)]);
    harness.run_chain_finalize(&half);
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_none(),
        "片方が未表示でも連鎖が確定してしまっている"
    );

    // 両スコープの実表示寸が窓と一致 → 同寸ゆえ再スナップは書かず、連鎖が確定する。
    let settled = settled_sizes();
    harness.run_resnap(&settled);
    assert!(
        harness.drain_writes().is_empty(),
        "実表示寸が窓と同寸なら再スナップは窓を書かない"
    );

    harness.run_chain_finalize(&settled);
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "隣接ペアが揃っても連鎖が確定しない"
    );
    let realigned: BTreeSet<Option<u32>> = harness
        .drain_writes()
        .iter()
        .map(|cmd| cmd.tag.scope)
        .collect();
    assert_eq!(
        realigned,
        BTreeSet::from([Some(1)]),
        "連鎖の解き直しが後続スコープ（scope 1）を動かしていない"
    );
    assert_eq!(
        SPAWN_SIZE_1,
        (278, 357),
        "fixture の後続スコープ寸（判定の由来を行内に残す）"
    );
}

// ---------------------------------------------------------------------------
// 実行形態の明示（要件 7.5／7.6）
// ---------------------------------------------------------------------------

/// 単一スレッド実行器の下で刻印を 1 行出す probe。
///
/// 刻印は `Res<FrameCount>`＋`Res<TickStart>` から組む（D1: World を借りられる観測点は
/// スレッド局所の写しを読まない）。
fn emit_snapshot_probe(frame: Res<FrameCount>, start: Res<TickStart>) {
    let stamp = transition_diag::stamp_from_world(&frame, &start);
    transition_diag::emit_line(&placement_diag::snapshot_line(
        &placement_diag::SnapshotRecord {
            stamp,
            monitors: &[],
        },
    ));
}

/// ログ捕捉に依る検証は**単一スレッド実行器**で走らせる（要件 7.6）。
///
/// 既定の多スレッド実行器では probe がワーカースレッドへ載り、スレッドローカルの捕捉が
/// 1 行も拾えないまま緑になる。本檻は「捕捉できる実行形態」を名指しし、その形で実際に
/// 行が拾えること・刻印がハーネスの進めたフレーム番号と一致することを見る。
#[test]
fn a_log_capture_verification_names_the_single_threaded_executor() {
    let mut harness = FrameHarness::new();
    harness.advance_frame();
    harness.advance_frame();
    harness.advance_frame();

    let mut schedule = single_threaded_schedule();
    // bevy 0.19 で `Schedule::get_executor_kind` が撤去された（実行器は私有）。「捕捉できる
    // 実行形態を名指しする」という要件 7.6 の要請は、供給元の字面で見張る——
    // `single_threaded_schedule()` が単スレッド実行器を据えていることをそのまま固定する。
    assert!(
        FRAME_TEST_SUPPORT_SRC.contains("schedule.set_executor(SingleThreadedExecutor::new());"),
        "捕捉できる実行形態の明示が供給元から消えている（要件 7.6）"
    );
    schedule.add_systems(emit_snapshot_probe);

    let ((), events) = capture_diag_logs(|| schedule.run(&mut harness.world));
    let lines: Vec<&str> = events
        .iter()
        .map(crate::placement::test_support::LogEvent::message)
        .filter(|m| m.contains("kind=snapshot"))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "単一スレッド実行器なら probe の 1 行が捕捉できる: {events:?}"
    );
    assert!(
        lines[0].contains("frame=3"),
        "捕捉行の刻印がハーネスの進めたフレーム番号と違う: {}",
        lines[0]
    );
}

/// 常時テストは純 x64 で走る（要件 7.5）——**接続宣言の形**を本文走査で固定する。
///
/// 実行時に `cfg!(target_pointer_width = "64")` を assert しても、x64 で走っている限り
/// 恒真であって何も守らない（守るべきは「i686 でも走ってしまう形へ書き換わらないこと」）。
/// ゆえに担い手は `frame.rs` の接続宣言そのものであり、本件はその宣言が**接続の直前に**
/// 残っているかを見る。属性を外す・`#[cfg(test)]` へ緩めるといった書き換えで赤になる。
#[test]
fn the_harness_tests_are_connected_under_an_x64_only_gate() {
    let frame_rs = include_str!("frame.rs");
    let connection = "#[path = \"frame_harness_tests.rs\"]";
    let at = frame_rs
        .find(connection)
        .expect("frame.rs に多フレーム駆動ハーネス檻の接続宣言がある");
    let gate = "#[cfg(all(test, target_pointer_width = \"64\"))]";
    let preceding = &frame_rs[..at];
    assert!(
        preceding.trim_end().ends_with(gate),
        "接続宣言の直前が x64 限定の門になっていない（要件 7.5）: 直前 200 文字は {:?}",
        &preceding[preceding.len().saturating_sub(200)..]
    );
}

/// 再スナップの駆動口が**書く側**でも効いていること。
///
/// 姉妹の `the_harness_drives_resnap_and_chain_finalize_for_the_adjacent_pair` が持つ
/// 「同寸なら書かない」は零件の主張なので、`run_resnap` を丸ごと無操作にしても恒真で通る
/// （task 3.3 のレビューで実測——本体を `let _ = source;` に置き換えてもワークスペース全体が
/// 緑のままだった）。陽性の駆動をここで別に固定する。
#[test]
fn resnap_writes_only_the_scope_whose_reported_size_differs() {
    let mut harness = FrameHarness::new();
    harness.advance_frame();

    // スコープ 0 だけ実表示寸が窓と食い違う（高さ +13px）。1 は同寸。
    let mismatched = PerTargetSizes::new([
        (0, Some((SPAWN_SIZE_0.0, SPAWN_SIZE_0.1 + 13))),
        (1, Some(SPAWN_SIZE_1)),
    ]);
    harness.run_resnap(&mismatched);
    let written: BTreeSet<Option<u32>> = harness
        .drain_writes()
        .iter()
        .map(|cmd| cmd.tag.scope)
        .collect();
    assert!(
        written.contains(&Some(0)),
        "実表示寸が食い違うスコープを再スナップが書き直していない: {written:?}"
    );
    assert!(
        !written.contains(&Some(1)),
        "同寸のスコープまで書き直している: {written:?}"
    );

    // 書き直した後は窓寸が実表示寸に揃うので、同じ源をもう一度流しても書かない（べき等）。
    harness.reset_write_witness();
    harness.run_resnap(&mismatched);
    assert!(
        harness.drain_writes().is_empty(),
        "再スナップがべき等でない（同じ源で 2 度書いている）"
    );
}
