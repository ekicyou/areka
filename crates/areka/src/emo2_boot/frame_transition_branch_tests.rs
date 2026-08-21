//! 整合待ちと作業領域追随の**判断分岐**（task 6.2・設計 Testing Strategy「Integration Tests」
//! 項目 2／3／6・要件 5.8／4.6／4.7／5.4／10.7／7.2）を多フレーム駆動で固定する。
//!
//! # このファイルが持つもの／兄弟が持つもの
//!
//! 是正そのものの対テスト（是正前は赤・是正後は緑）は、各是正の隣に置いてある:
//!
//! | 是正 | 対テストの持ち主 |
//! |---|---|
//! | 整合待ち（経路 (a) の 2 段書込） | `frame_dpi_sync_hold_tests.rs`（task 5.4・拡大率 192 の 1 水準） |
//! | 作業領域源の実行時同期 | `frame_work_area_sync_tests.rs`（task 5.1） |
//! | 作業領域変化を契機とする再スナップ | `frame_work_area_resnap_tests.rs`（task 5.2・主に拡大率 96・合流を見る 1 件だけ 192） |
//! | 再導出結果なしの窓（`None` 経路） | `frame_dpi_reproject_none_tests.rs`（`dpi-window-vanish` S2・単一フレーム） |
//! | 判定器の `reason=invisible`／`k-unchanged` の選り分け | `placement/transition_judge_tests.rs`（純関数） |
//!
//! 本ファイルが持つのは、そこに**無い**側——同じ分岐を**要件 7.2 が名指しする 2 水準
//! （120 と 192）の両方**で、かつ複数モニタの作業領域を注入した状態で走らせることと、
//! 兄弟が 1 水準でしか通していない腕（上限超過の警告・別モニタでの寸法追従・作業領域だけの
//! 変化・定常フレームの窓書込 0・見送られた窓の隣で他窓の遷移が続くこと）である。
//!
//! # 本番入口（`emo2_frame_system`）を通す檻を 1 本持つ
//!
//! 上の 19 本はいずれも [`FrameHarness::run_placement_phases`] を駆動する。これは本番の相順を
//! **ハーネス側へ写した実装**であって `emo2_frame_system` を通らないので、本番の呼出が残った
//! まま**到達不能**になっても（`if false` で包む等）1 本も赤にならない——task 5.2 のレビューが
//! 実演で確定させた穴であり、`frame_work_area_sync_tests.rs`／`frame_work_area_resnap_tests.rs`
//! の本文走査は呼出の**存在と前後関係**しか押さえない。
//!
//! [`the_production_frame_system_reaches_all_three_placement_call_sites`] がその引受先である。
//! 本番の相順所有者 `emo2_frame_system` をそのまま 2 フレーム回し、同期段・拡大率の相・作業
//! 領域再スナップの 3 呼出が**到達可能**であることを、それぞれ固有の観測（源の作り直し／
//! `origin=DpiReproject` の書込／`origin=WorkAreaResnap` の書込）で示す。
//!
//! # 零件の主張には陽性の対を同じ本体へ置く
//!
//! 本ファイルの主張は零件（「書込 0」「待ち札 0」「警告 0」）に多く偏る。駆動が死んでいれば
//! どれも空虚に緑になるので、**同じテスト本体の内側**で同じ駆動口が陽性側でも効くことを
//! 続けて主張する（待ち → 解除で 4 本／同一表 → 変化ある表で書込／0 台 → 台が戻れば書込）。
//!
//! # 要件 4.6 の「現状維持」が指すのは**寸**である（本文の字句を狭めて読んでいる）
//!
//! **要件 4.6 の本文は「当該窓の位置と寸を変更せずに」と書く**が、本ファイルはそれを**寸だけ**
//! と読む。括弧書きの束縛「（`dpi-window-vanish` R4.5 の挙動を変えない）」が上位に効くため
//! である——先行仕様の R4.5 は `completed/areka-P0-dpi-window-vanish/design.md` の D7（位置権威と
//! 寸権威の分離）と対になっており、同 Traceability が「4.5 再導出不能なら現状維持＝
//! `refresh_scale` が `None`＝**寸不変**、**位置は D7 で独立に判断**」と明記している。実装も
//! そのとおりで、`frame/dpi.rs` の `None` 腕は窓寸を 1 bit も触らず
//! `reproject_char_window_at_current_size` で現寸のまま射影 T を一度通す。
//!
//! ゆえに本ファイルは「寸が動かないこと」と「他窓の遷移が続くこと」を問い、位置の側は
//! 接地点規約が保たれることとして問う。**本文の字句だけを読むと位置も不動に見える**ので、
//! この段落を根拠として残す（黙って狭めない）。

use std::sync::mpsc;

use bevy_ecs::entity::Entity;

use areka_emo_present::PresentCommand;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::window::{SetWindowPosCommand, drain_window_pos_commands};
use wintf::ecs::{DPI, Point, SizeI, WindowPos};

use crate::placement::WORK_AREA_SYNC_CONTEXT;
use crate::placement::diag::{PlacementRoute, WindowKind};
use crate::placement::dpi_sync::{DPI_SYNC_HOLD_MAX_FRAMES, DpiSyncHold, evaluate};
use crate::placement::follow::{BalloonFollow, MonitorDpiTable, MonitorSnapshot};
use crate::placement::test_support::{LogEvent, capture_logs};
use crate::placement::transition_judge::WRITES_PER_WINDOW_MAX;

use super::emo2_frame_system;
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, dpi_world,
    headless_wiring_with, pos_of, s2_assert_work_area_bottom_moves, s2_monitors,
    s2_monitors_with_work_area, s2_sources, s2_taskbar_hidden_work_area, s2_work_area_for_dpi,
    size_of, zero_clock,
};
use super::{balloon_target, shell_target};

/// 遷移前の拡大率水準（等倍）。
const BASE_DPI: u16 = 96;

/// 要件 7.2 が名指しする 2 水準のうち低い側。
const SCALE_120: u16 = 120;

/// 同上・高い側（等倍の 2 倍）。
const SCALE_192: u16 = 192;

/// 定常フレームを何コマ回して「窓書込 0」を主張するか（要件 4.7）。
const STEADY_FRAMES: u32 = 5;

/// 別モニタ検査で札の不在を確かめ続けるフレーム数。
const WAIT_FRAMES: u32 = 3;

/// 檻の両スコープが持つバルーンの等倍寸（`resnap_placements` と一致）。
const BALLOON_SPAWN_SIZE: (u32, u32) = (223, 158);

/// キャラ窓を隣接モニタへ置くときの位置（随伴バルーンも隣接モニタ内へ収まる値）。
const CHAR_ON_NEIGHBOR: Point = Point { x: 3200, y: 800 };

/// どのモニタにも属さない位置（両モニタの**左**の外・y は両モニタの域内）。
///
/// 帰属は**中心**で決まる（`monitor_containing`／`work_area_for_window_with_origin`）。最近傍は
/// clamp 点との自乗距離で決まるので、ゴーストが居たモニタ（index 0）が最近傍になる側へ置く
/// ——隣接モニタ（右側・x が 2574 以上）より確実に近い左外へ出す。
const CHAR_OFF_ALL_MONITORS: Point = Point { x: -3000, y: 700 };

// ---------------------------------------------------------------------------
// 合成寸と選り分け
// ---------------------------------------------------------------------------

/// 当該拡大率水準における物理寸（等倍寸を `dpi/96` 倍する）。
fn scaled(size: (u32, u32), dpi: u16) -> (u32, u32) {
    let k = u32::from(dpi);
    (size.0 * k / 96, size.1 * k / 96)
}

/// `(u32, u32)` の報告寸を窓寸の通貨（`SizeI`）へ移す。
fn as_window_size(size: (u32, u32)) -> SizeI {
    SizeI {
        width: size.0 as i32,
        height: size.1 as i32,
    }
}

/// 指定スコープ・指定種別の窓書込だけを取り出す。
fn writes_for(
    writes: &[SetWindowPosCommand],
    scope: u32,
    kind: WindowKind,
) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind.as_str())
        .cloned()
        .collect()
}

/// 捕捉行のうち WARN で、`needle` を本文に含むものの件数。
fn warnings_containing(events: &[LogEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .filter(|e| e.message().contains(needle))
        .count()
}

/// 隣接モニタ（ゴーストが決して居ない側）だけを当該水準にした実行時のモニタ表。
fn neighbor_at(dpi: u16) -> Vec<Monitor> {
    let mut monitors = s2_monitors(BASE_DPI);
    monitors[1].dpi = u32::from(dpi);
    monitors
}

/// 窓の位置を直接置く（ドラッグの結果だけを再現する・書込は経ない）。
fn set_position(harness: &mut FrameHarness, window: Entity, position: Point) {
    let mut window_pos = harness
        .world
        .get_mut::<WindowPos>(window)
        .expect("WindowPos がある");
    window_pos.position = Some(position);
}

/// 当該スコープの二体をまとめて指定位置へ移す（随伴の窓相対を保ったまま）。
fn move_scope_to(harness: &mut FrameHarness, scope: usize, position: Point) {
    let char_window = harness.char_window(scope);
    let balloon = harness.balloon_window(scope);
    let offset = harness
        .world
        .get::<BalloonFollow>(char_window)
        .expect("char 窓は BalloonFollow を持つ")
        .offset;
    set_position(harness, char_window, position);
    set_position(
        harness,
        balloon,
        Point {
            x: position.x + offset.x,
            y: position.y + offset.y,
        },
    );
}

/// 当該水準の再表示報告を 4 窓すべてへ与える（遷移で全窓の寸が動く形）。
fn report_all_targets(source: &mut FakeReports, dpi: u16) {
    source
        .refresh
        .insert(shell_target(0).0, scaled(SPAWN_SIZE_0, dpi));
    source
        .refresh
        .insert(shell_target(1).0, scaled(SPAWN_SIZE_1, dpi));
    source
        .refresh
        .insert(balloon_target(0).0, scaled(BALLOON_SPAWN_SIZE, dpi));
    source
        .refresh
        .insert(balloon_target(1).0, scaled(BALLOON_SPAWN_SIZE, dpi));
}

/// 起動直後の整地——3 つの源と窓の拡大率をすべて当該水準へ揃え、キャラ窓を当該水準の
/// 作業領域下端へ接地させる。
///
/// 拡大率の相の初回 run は永続 `SystemState` の仕様で全窓へマッチするので、ここで 1 度
/// 空回しして消費する（以後のフレームでは真に変化した窓だけが対象になる）。
fn settle_at(harness: &mut FrameHarness, source: &mut FakeReports, dpi: u16) {
    harness.set_monitor_sources_for_dpi(dpi);
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            harness.ground_point(scope).1,
            s2_work_area_for_dpi(dpi).bottom,
            "前提が崩れている: dpi={dpi} scope={scope} のキャラ窓が作業領域下端へ接地していない"
        );
    }
}

/// 拡大率通知だけが先に届いた状態を作る（実行時のモニタ表は旧水準のまま据え置く）。
fn deliver_the_scale_notice_first(harness: &mut FrameHarness, source: &mut FakeReports, dpi: u16) {
    harness.set_window_dpi(dpi);
    report_all_targets(source, dpi);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// 当該スコープの 2 窓に整合待ちの札が付いているか（キャラ, バルーン）。
fn holds_of(harness: &FrameHarness, scope: usize) -> (bool, bool) {
    (
        harness
            .world
            .get::<DpiSyncHold>(harness.char_window(scope))
            .is_some(),
        harness
            .world
            .get::<DpiSyncHold>(harness.balloon_window(scope))
            .is_some(),
    )
}

// ---------------------------------------------------------------------------
// 群 A: 拡大率通知が表更新より先（経路 (a)）——設計 Integration Tests 項目 2
// ---------------------------------------------------------------------------

/// 待ち → 解除 → 当該窓への書込 1 回。旧作業領域下端の中間矩形は 1 度も可視化しない
/// （要件 5.8・設計 D15）。
///
/// 零件（待ちフレームの書込 0）の**陽性の対**は同じ本体の後半——解除フレームで 4 本の指令が
/// 出ることである。待ちが起きていなければ前半で書かれ、駆動が死んでいれば後半で 0 本になる。
fn scale_notice_first_lands_in_one_write_at(dpi: u16) {
    // 探針の非退化: この 2 水準で作業領域下端が実際に動く（動かなければ中間矩形は観測できない）。
    s2_assert_work_area_bottom_moves(BASE_DPI, dpi);

    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, BASE_DPI);

    let old_bottom = s2_work_area_for_dpi(BASE_DPI).bottom;
    let new_bottom = s2_work_area_for_dpi(dpi).bottom;

    // ── 待ちフレーム: 拡大率だけが新しく、表はまだ旧水準 ──────────────────
    deliver_the_scale_notice_first(&mut harness, &mut source, dpi);
    // 零件を主張する**前に**、このフレームで本当に食い違いが起きていることを固定する。
    for scope in harness.scopes().to_vec() {
        let outcome = evaluate(&harness.world, harness.char_window(scope), harness.frame());
        assert_eq!(
            (outcome.window_dpi, outcome.table_dpi),
            (u32::from(dpi), Some(u32::from(BASE_DPI))),
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
    scale_notice_first_lands_in_one_write_at(SCALE_192);
}

/// 待ちのあいだに表情差替（drain 相の `ShowSurface` が積む新 k・寸変化の窓寸要求）が来ても、
/// 報告寸の突合と再スナップの**どちらからも**当該窓への書込は 0 のままである
/// （設計 C5 議題 1・D15 の「4 点すべて」のうち、この 2 点）。
///
/// 陽性の対は同じ本体の後半——待ちが解けた後は、**同じ 2 つの駆動口**が書く。
fn a_surface_swap_during_the_wait_writes_nothing_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, BASE_DPI);
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
    a_surface_swap_during_the_wait_writes_nothing_at(SCALE_192);
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
    settle_at(&mut harness, &mut source, BASE_DPI);
    deliver_the_scale_notice_first(&mut harness, &mut source, dpi);
    let _waiting = harness.drain_writes();

    // 表は永遠に追いつかない。上限に達するまでは 1 件も書かず、警告も出さない。
    for _ in 0..DPI_SYNC_HOLD_MAX_FRAMES - 1 {
        harness.advance_frame();
        let (_, events) = capture_logs(|| {
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
    let (_, events) = capture_logs(|| {
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
    the_wait_gives_up_after_the_bounded_number_of_frames_at(SCALE_192);
}

/// 待ち札が見送るのは**窓書込の 4 点だけ**であり、描画の相（`run_drain_phase`）は素通しである
/// （設計 D15「描画そのものは止めない」）。
///
/// 決定論テストが買える射程はここまでである——本ハーネスは GPU を持たないので `apply_show` が
/// 実際に画を出したことは検証できない。ゆえに問うのは**構造**——描画の相の本文に整合ゲートの
/// 参照が 1 つも無く、逆に 4 つの見送り点にはあること（陽性の対）である。発話・アニメが実機で
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

    // 陽性の対: 4 つの見送り点にはゲートがある（無ければ上の「無い」は空虚）。
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
        "作業領域変化を契機とする再スナップにゲートが無い（見送り点の 4 つ目・task 6.5）"
    );
}

// ---------------------------------------------------------------------------
// 群 B: 別モニタへ移した窓（要件 10.7）——設計 Integration Tests 項目 6
// ---------------------------------------------------------------------------

/// 別モニタへ移した窓（移動先の拡大率が表に既在）は**待たずに通り**、寸法が追従する
/// （要件 10.7・設計 Integration Tests 項目 6）。
///
/// 「待ち札 0」の陽性の対は同じ本体の内側——待たずに通ったからこそ書込が出て寸が変わる。
fn a_window_moved_to_another_monitor_never_waits_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, BASE_DPI);

    // 隣接モニタだけが当該水準の表を作る（実行時のモニタ表 → 同期段が 2 源を作り直す）。
    harness.set_monitor_table(neighbor_at(dpi));
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let _sync_writes = harness.drain_writes();

    // scope 0 の**二体とも**隣接モニタ（当該水準の側）へ移し、窓の拡大率もその値にする
    // ——ドラッグでゴーストを移せばバルーンも一緒に移る（随伴は窓相対）。
    move_scope_to(&mut harness, 0, CHAR_ON_NEIGHBOR);
    harness.set_scope_dpi(0, dpi);
    let followed = scaled(SPAWN_SIZE_0, dpi);
    assert_ne!(
        followed, SPAWN_SIZE_0,
        "探針が退化している: dpi={dpi} で寸が動かない（追従を観測できない）"
    );
    source.refresh.insert(shell_target(0).0, followed);
    source
        .refresh
        .insert(balloon_target(0).0, scaled(BALLOON_SPAWN_SIZE, dpi));
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    assert_eq!(
        holds_of(&harness, 0),
        (false, false),
        "dpi={dpi}: 移動先の拡大率が表に既在なのに待ち札が付いた（要件 10.7）"
    );
    // 陽性の対: 待たずに通ったので書込が出ており、寸が追従している。
    let writes = harness.drain_writes();
    assert_eq!(
        writes_for(&writes, 0, WindowKind::Char).len() as u32,
        WRITES_PER_WINDOW_MAX,
        "dpi={dpi}: 別モニタのキャラ窓が 1 回で書かれていない: {writes:?}"
    );
    assert_eq!(
        size_of(&harness.world, harness.char_window(0)),
        Some(as_window_size(followed)),
        "dpi={dpi}: 別モニタへ移した窓の寸法が追従していない（要件 10.7）"
    );

    for _ in 0..WAIT_FRAMES {
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        assert_eq!(
            holds_of(&harness, 0),
            (false, false),
            "dpi={dpi}: 後続フレームで待ち札が付いた（要件 10.7）"
        );
    }
}

/// 要件 7.2 の低い側（120）。
#[test]
fn a_window_moved_to_another_monitor_never_waits_at_120() {
    a_window_moved_to_another_monitor_never_waits_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn a_window_moved_to_another_monitor_never_waits_at_192() {
    a_window_moved_to_another_monitor_never_waits_at(SCALE_192);
}

// ---------------------------------------------------------------------------
// 群 C: 作業領域の分岐——設計 Integration Tests 項目 3
// ---------------------------------------------------------------------------

/// 作業領域だけが変わったフレーム（拡大率は据え置き）で、下端吸着のキャラ窓が
/// `WorkAreaResnap` の 1 書込で新しい下端へ移り、随伴バルーンが**同一フレーム**で追従する
/// （要件 5.1／5.2・設計 Integration Tests 項目 3）。
fn a_work_area_only_change_writes_once_at(dpi: u16) {
    let visible = s2_work_area_for_dpi(dpi).bottom;
    let hidden = s2_taskbar_hidden_work_area(dpi).bottom;
    assert_ne!(
        visible, hidden,
        "探針が退化している: dpi={dpi} でタスクバーの表示切替が作業領域下端を動かさない"
    );

    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, dpi);

    let offsets_before: Vec<(usize, i32, i32)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| {
            let offset = harness
                .world
                .get::<BalloonFollow>(harness.char_window(scope))
                .expect("char 窓は BalloonFollow を持つ")
                .offset;
            (scope, offset.x, offset.y)
        })
        .collect();
    let balloon_y_before: Vec<(usize, i32)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| {
            (
                scope,
                pos_of(&harness.world, harness.balloon_window(scope))
                    .expect("balloon 位置がある")
                    .y,
            )
        })
        .collect();

    // タスクバーを隠す＝**拡大率を 1 つも動かさずに**作業領域だけが動く構成変更。
    harness.set_monitor_table(s2_monitors_with_work_area(
        dpi,
        s2_taskbar_hidden_work_area(dpi),
    ));
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();

    assert_eq!(
        writes.len(),
        harness.scopes().len() * 2,
        "dpi={dpi}: 作業領域変化のフレームの窓書込が 4 本ではない: {writes:?}"
    );
    for scope in harness.scopes().to_vec() {
        let scope32 = scope as u32;
        for kind in [WindowKind::Char, WindowKind::Balloon] {
            assert_eq!(
                writes_for(&writes, scope32, kind).len() as u32,
                WRITES_PER_WINDOW_MAX,
                "dpi={dpi} scope={scope} {kind}: 窓あたりの書込が {WRITES_PER_WINDOW_MAX} 本ではない: {writes:?}"
            );
        }
        let char_write = writes_for(&writes, scope32, WindowKind::Char)
            .pop()
            .expect("キャラ窓の書込がある");
        assert_eq!(
            char_write.tag.origin,
            PlacementRoute::WorkAreaResnap.as_str(),
            "dpi={dpi} scope={scope}: 経路語が作業領域再スナップの語になっていない: {writes:?}"
        );
        assert_eq!(
            harness.ground_point(scope).1,
            hidden,
            "dpi={dpi} scope={scope}: 接地点が新しい作業領域下端に載っていない（旧下端 {visible} に留まっている）"
        );
    }

    // 随伴は同一フレームで窓相対へ移り、追従 offset は 1 bit も変わらない（要件 5.2／10.1）。
    for (scope, offset_x, offset_y) in &offsets_before {
        let char_pos =
            pos_of(&harness.world, harness.char_window(*scope)).expect("char 位置がある");
        let balloon_pos =
            pos_of(&harness.world, harness.balloon_window(*scope)).expect("balloon 位置がある");
        assert_eq!(
            (balloon_pos.x - char_pos.x, balloon_pos.y - char_pos.y),
            (*offset_x, *offset_y),
            "dpi={dpi} scope={scope}: 随伴恒等式 balloon − char ≡ BalloonFollow.offset が崩れている"
        );
        let after = harness
            .world
            .get::<BalloonFollow>(harness.char_window(*scope))
            .expect("char 窓は BalloonFollow を持つ")
            .offset;
        assert_eq!(
            (after.x, after.y),
            (*offset_x, *offset_y),
            "dpi={dpi} scope={scope}: 追従オフセットを補正している（要件 10.1: 補正しない）"
        );
    }
    // 「相対不変」が「何も動かなかった」の言い換えに退化していないこと。
    for (scope, before_y) in &balloon_y_before {
        assert_ne!(
            pos_of(&harness.world, harness.balloon_window(*scope))
                .expect("balloon 位置がある")
                .y,
            *before_y,
            "dpi={dpi} scope={scope}: バルーンの絶対位置が動いていない（恒等式が空虚に成立している）"
        );
    }
}

/// 要件 7.2 の低い側（120）。
#[test]
fn a_work_area_only_change_writes_once_at_120() {
    a_work_area_only_change_writes_once_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn a_work_area_only_change_writes_once_at_192() {
    a_work_area_only_change_writes_once_at(SCALE_192);
}

/// 同一表のフレームでは源を作り直さず（要件 5.4）、遷移を伴わない定常フレームでは窓書込が
/// **1 件も**出ない（要件 4.7＝本タスクの観察可能な完了条件）。
///
/// 陽性の対は同じ本体の前後にある——先に「動かせば書く」を通してから定常へ入り、最後にもう
/// 一度動かして書込が戻ることを見る。これが無いと、再スナップを丸ごと無操作にしても緑になる。
fn steady_frames_write_nothing_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, dpi);

    // 陽性の対（前）: 作業領域を動かせば書く。
    harness.set_monitor_table(s2_monitors_with_work_area(
        dpi,
        s2_taskbar_hidden_work_area(dpi),
    ));
    harness.advance_frame();
    let change = harness.run_placement_phases(&mut source);
    assert!(
        change.is_some(),
        "dpi={dpi}: 表が変わったのに同期段が差し替えを報告していない（駆動が死んでいる）"
    );
    assert!(
        !harness.drain_writes().is_empty(),
        "dpi={dpi}: 作業領域が動いたのに窓書込が出ていない（駆動が死んでいる）"
    );

    // 零件: 表を触らないフレームでは作り直しも窓書込も起きない。
    for _ in 0..STEADY_FRAMES {
        harness.advance_frame();
        let change = harness.run_placement_phases(&mut source);
        assert!(
            change.is_none(),
            "dpi={dpi}: 同一表のフレームで源を作り直している（frame={}・要件 5.4）",
            harness.frame()
        );
        let writes = harness.drain_writes();
        assert!(
            writes.is_empty(),
            "dpi={dpi}: 定常フレームで窓書込が出ている（frame={}・要件 4.7）: {writes:?}",
            harness.frame()
        );
    }

    // 陽性の対（後）: 同じ定常ループの続きでもう一度動かせば、また書く。
    harness.set_monitor_table_for_dpi(dpi);
    harness.advance_frame();
    let change = harness.run_placement_phases(&mut source);
    assert!(
        change.is_some(),
        "dpi={dpi}: 定常を抜けても同期段が差し替えを報告しない（比較が恒真に潰れている）"
    );
    assert!(
        !harness.drain_writes().is_empty(),
        "dpi={dpi}: 定常を抜けて作業領域が戻ったのに窓書込が出ていない"
    );
}

/// 要件 7.2 の低い側（120）。
#[test]
fn steady_frames_write_nothing_at_120() {
    steady_frames_write_nothing_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn steady_frames_write_nothing_at_192() {
    steady_frames_write_nothing_at(SCALE_192);
}

/// モニタ 0 台（列挙異常）では源を差し替えず、窓を 1 枚も動かさず、警告を残す（要件 5.5）。
///
/// 陽性の対は同じ本体の後半——台が戻れば警告は止み、変化に応じて書込が出る。
fn an_empty_monitor_table_keeps_everything_and_warns_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, dpi);
    let ground_before: Vec<(usize, (i32, i32))> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| (scope, harness.ground_point(scope)))
        .collect();

    harness.set_monitor_table(Vec::new());
    harness.advance_frame();
    let (change, events) = capture_logs(|| harness.run_placement_phases(&mut source));

    assert!(change.is_none(), "dpi={dpi}: 空の表で源を差し替えている");
    assert_eq!(
        harness.work_area_source(),
        Some(&s2_sources(dpi).snapshot),
        "dpi={dpi}: 空の表で作業領域源が潰れている（現状維持でない）"
    );
    let writes = harness.drain_writes();
    assert!(
        writes.is_empty(),
        "dpi={dpi}: モニタ 0 台のフレームで窓書込が出ている（現状維持でない）: {writes:?}"
    );
    for (scope, before) in &ground_before {
        assert_eq!(
            harness.ground_point(*scope),
            *before,
            "dpi={dpi} scope={scope}: モニタ 0 台で接地点が動いた（現状維持でない）"
        );
    }
    assert_eq!(
        warnings_containing(&events, WORK_AREA_SYNC_CONTEXT),
        1,
        "dpi={dpi}: モニタ 0 台が無言で素通りしている（ログ無し失敗経路）: {events:?}"
    );

    // ── 陽性の対（同じ本体）: 台が戻れば警告は止み、変化に応じて書く ──────
    harness.set_monitor_table(s2_monitors_with_work_area(
        dpi,
        s2_taskbar_hidden_work_area(dpi),
    ));
    harness.advance_frame();
    let (change, events) = capture_logs(|| harness.run_placement_phases(&mut source));
    assert!(
        change.is_some(),
        "dpi={dpi}: 台が戻っても差し替えが起きない（0 台の腕から抜けていない）"
    );
    assert_eq!(
        warnings_containing(&events, WORK_AREA_SYNC_CONTEXT),
        0,
        "dpi={dpi}: 正常な表更新で 0 台の警告が出ている（警告が常時鳴っている）: {events:?}"
    );
    assert!(
        !harness.drain_writes().is_empty(),
        "dpi={dpi}: 台が戻って作業領域が動いたのに窓書込が出ていない"
    );
}

/// 要件 7.2 の低い側（120）。
#[test]
fn an_empty_monitor_table_keeps_everything_and_warns_at_120() {
    an_empty_monitor_table_keeps_everything_and_warns_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn an_empty_monitor_table_keeps_everything_and_warns_at_192() {
    an_empty_monitor_table_keeps_everything_and_warns_at(SCALE_192);
}

/// **「帰属不能」は「解決できない」ではない**（開発者裁定 2026-08-20・要件 5.5 の適用範囲）。
///
/// どのモニタにも中心が乗らない窓は最近傍で**解決される**——現状維持で画面外に取り残さない。
/// 判断の軸は「ゴーストが触れなくなる事態を避けること」であり、モニタ 0 台のときだけが 5.5 の
/// 現状維持である。両者を**同じ本体**で対にして、混同していないことを実行で示す。
#[test]
fn a_window_off_all_monitors_is_resolved_by_the_nearest_fallback_not_left_in_place() {
    let dpi = SCALE_192;
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, dpi);

    // どのモニタにも中心が乗らない位置へ二体を移す（ドラッグ相当・書込は経ない）。
    move_scope_to(&mut harness, 0, CHAR_OFF_ALL_MONITORS);
    assert_ne!(
        harness.ground_point(0).1,
        s2_taskbar_hidden_work_area(dpi).bottom,
        "探針が退化している: 画面外へ置いたつもりの接地点が既に目標下端に載っている"
    );

    // 作業領域を動かして再スナップを走らせる。
    harness.set_monitor_table(s2_monitors_with_work_area(
        dpi,
        s2_taskbar_hidden_work_area(dpi),
    ));
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();

    // 帰属不能でも**解決された**——最近傍（ゴーストが居たモニタ）の下端へ引き寄せられる。
    assert_eq!(
        writes_for(&writes, 0, WindowKind::Char).len() as u32,
        WRITES_PER_WINDOW_MAX,
        "帰属不能な窓が現状維持で放置されている（要件 5.5 の適用対象と取り違えている）: {writes:?}"
    );
    assert_eq!(
        harness.ground_point(0).1,
        s2_taskbar_hidden_work_area(dpi).bottom,
        "帰属不能な窓が最近傍の作業領域下端へ解決されていない"
    );

    // 対照（同じ本体）: **モニタ 0 台**なら同じ窓が 1px も動かない＝そちらだけが 5.5 である。
    let held_ground = harness.ground_point(0);
    harness.set_monitor_table(Vec::new());
    harness.advance_frame();
    let (change, _events) = capture_logs(|| harness.run_placement_phases(&mut source));
    assert!(change.is_none(), "モニタ 0 台で源を差し替えている");
    assert!(
        harness.drain_writes().is_empty(),
        "モニタ 0 台で窓書込が出ている（要件 5.5 の現状維持でない）"
    );
    assert_eq!(
        harness.ground_point(0),
        held_ground,
        "モニタ 0 台で接地点が動いた（要件 5.5 の現状維持でない）"
    );
}

// ---------------------------------------------------------------------------
// 群 D: 遷移時点で再導出結果が得られない窓（要件 4.6）
// ---------------------------------------------------------------------------

/// 遷移時点で再導出結果が得られない窓（不可視・未表示）は**窓寸**を変えず、その隣で
/// **他窓の遷移は継続する**（要件 4.6・`dpi-window-vanish` R4.5 の挙動不変）。
///
/// 「寸が動かない」の陽性の対は同じ本体の内側——報告を与えた scope 1 は寸が動き、接地点が
/// 新しい下端へ載り、1 回で書かれる。片方の欠落が他方を巻き込まないことがここの主題である。
///
/// 位置の側は現寸のまま射影 T を一度通る（設計 D7）。その単一フレームの形は
/// `frame_dpi_reproject_none_tests.rs` が持つので、ここでは遷移の文脈で接地点規約が保たれる
/// ことだけを問う。
fn a_window_without_a_re_derivation_result_keeps_its_size_at(dpi: u16) {
    s2_assert_work_area_bottom_moves(BASE_DPI, dpi);

    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, BASE_DPI);

    let kept_char = size_of(&harness.world, harness.char_window(0)).expect("窓寸がある");
    let kept_balloon = size_of(&harness.world, harness.balloon_window(0)).expect("窓寸がある");
    let followed = scaled(SPAWN_SIZE_1, dpi);
    assert_ne!(
        Some(as_window_size(followed)),
        size_of(&harness.world, harness.char_window(1)),
        "探針が退化している: dpi={dpi} で scope 1 の寸が動かない（陽性の対が空虚）"
    );

    // 経路 (b)（表更新が先）の遷移。報告は scope 1 にだけ与える——scope 0 は遷移時点で
    // 再導出結果が得られない窓（不可視・未表示）である。
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    source.refresh.insert(shell_target(1).0, followed);
    source
        .refresh
        .insert(balloon_target(1).0, scaled(BALLOON_SPAWN_SIZE, dpi));
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();

    // 要件 4.6: 見送られた窓の**寸**は 1 bit も動かない。
    assert_eq!(
        size_of(&harness.world, harness.char_window(0)),
        Some(kept_char),
        "dpi={dpi}: 再導出結果の無いキャラ窓の寸が動いた（要件 4.6 の現状維持）"
    );
    assert_eq!(
        size_of(&harness.world, harness.balloon_window(0)),
        Some(kept_balloon),
        "dpi={dpi}: 再導出結果の無いバルーン窓の寸が動いた（要件 4.6 の現状維持）"
    );

    // 陽性の対: 他窓の遷移は継続する。
    assert_eq!(
        size_of(&harness.world, harness.char_window(1)),
        Some(as_window_size(followed)),
        "dpi={dpi}: 報告のある scope 1 の寸が追従していない（見送りが他窓を巻き込んでいる）"
    );
    assert_eq!(
        writes_for(&writes, 1, WindowKind::Char).len() as u32,
        WRITES_PER_WINDOW_MAX,
        "dpi={dpi}: scope 1 のキャラ窓が 1 回で書かれていない: {writes:?}"
    );

    // 接地点規約（要件 4.1）は両スコープで保たれる——見送られた窓も浮かない。
    let new_bottom = s2_work_area_for_dpi(dpi).bottom;
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            harness.ground_point(scope).1,
            new_bottom,
            "dpi={dpi} scope={scope}: 遷移後の接地点が新しい作業領域下端から外れている"
        );
    }
}

/// 要件 7.2 の低い側（120）。
#[test]
fn a_window_without_a_re_derivation_result_keeps_its_size_at_120() {
    a_window_without_a_re_derivation_result_keeps_its_size_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn a_window_without_a_re_derivation_result_keeps_its_size_at_192() {
    a_window_without_a_re_derivation_result_keeps_its_size_at(SCALE_192);
}

// ---------------------------------------------------------------------------
// 本番入口（`emo2_frame_system`）の到達可能性（task 5.2 のレビューが名指しした申し送り）
// ---------------------------------------------------------------------------

/// 本番の相順所有者 [`emo2_frame_system`] をそのまま回し、配置に関わる 3 呼出——同期段
/// （`work_area_sync::sync_monitor_snapshot`）・拡大率の相（`run_dpi_phase`）・作業領域再スナップ
/// （`work_area_sync::resnap_for_work_area_change`）——が**到達可能**であることを固定する。
///
/// # なぜこの 1 本が要るのか
///
/// 本ファイルの他の 19 本と、兄弟の `frame_work_area_sync_tests.rs`／
/// `frame_work_area_resnap_tests.rs`／`frame_dpi_sync_hold_tests.rs` はすべて
/// [`FrameHarness::run_placement_phases`] を駆動する。あれは本番の相順を**ハーネス側へ写した
/// 実装**であって [`emo2_frame_system`] を通らない。ゆえに本番の呼出を残したまま `if false` で
/// 包んで**到達不能**にしても、挙動テストは 1 本も赤にならない（task 5.2 のレビューが実演で
/// 確定させた）。本文走査（`the_resnap_is_called_after_the_scale_phase_in_the_frame_system` ほか）は
/// 呼出の**存在と前後関係**しか押さえないので、到達可能性はどこにも掛かっていなかった。
///
/// # 観測の組み立て——3 呼出を別々の観測へ分離する
///
/// | 呼出 | 固有の観測 | どのフレームで |
/// |---|---|---|
/// | 同期段 | 作業領域源とモニタ別拡大率表が実行時のモニタ表から作り直される | フレーム A |
/// | 拡大率の相 | `origin=DpiReproject` の窓書込 | フレーム A（`WorkAreaResnap` は 0 件） |
/// | 作業領域再スナップ | `origin=WorkAreaResnap` の窓書込 | フレーム B（`DpiReproject` は 0 件） |
///
/// フレームを 2 つに分けるのは、両者が同一フレームでは**合流して 1 本になる**からである
/// ——拡大率の相が先に新しい下端へ書き終えた窓は、再スナップの導出値が現在値と一致して
/// べき等 skip で抜ける（設計 C6）。分けると、どちらの経路が書いたのかが経路語で一意に読める。
///
/// フレーム A は「源だけが古い」状態から始める（実行時のモニタ表と窓の拡大率は当該水準・
/// 作業領域源だけが等倍水準）。整合ゲートは窓の拡大率と**表**だけを見るので待ちは起きず、
/// 拡大率の相の初回 run が全窓へマッチして現寸のまま新しい下端へ射影する。フレーム B は
/// 拡大率を 1 つも動かさずタスクバーを隠す——`Changed<DPI>` が 1 件も立たないので拡大率の相は
/// 何もせず、書込が出るなら再スナップ以外にあり得ない。
///
/// # presenter は未装着でよい
///
/// 本檻が問うのは**到達可能性**であって表示ではない。未装着の実 [`EmoPresenter`] は
/// `refresh_scale_report` が全 target で `None` を返すが、拡大率の相の `None` 腕は
/// **位置だけ**を現寸で射影し直す（設計 D7）ので、窓書込は出る。GPU も実 fixture も要らない。
#[test]
fn the_production_frame_system_reaches_all_three_placement_call_sites() {
    let dpi = SCALE_192;
    // 前のテストが残した窓書込指令を捨てる（**実行はしない**・要件 7.7）。
    let _residue = drain_window_pos_commands();

    let stale = s2_sources(BASE_DPI);
    let fresh = s2_sources(dpi);
    assert_ne!(
        stale.snapshot.work_areas, fresh.snapshot.work_areas,
        "探針が退化している: 古い源と新しい源の作業領域が同一（同期段の到達を観測できない）"
    );

    // 2 スコープのゴースト窓・偽 `WindowHandle`・書込 witness を持つ World。
    let (mut world, gw) = dpi_world();
    // 窓の拡大率は当該水準へ揃える——整合待ちを起こさないため（待ちは群 A の主題であり、
    // ここで混ざると「書込 0」が到達不能と区別できなくなる）。
    for scope in [0usize, 1] {
        for window in [
            gw.char_window(scope).expect("char 窓がある"),
            gw.balloon_window(scope).expect("balloon 窓がある"),
        ] {
            world.entity_mut(window).insert(DPI::from_dpi(dpi, dpi));
        }
    }
    // 実行時のモニタ表は当該水準（同期段が読む側）。
    for monitor in s2_monitors(dpi) {
        world.spawn(monitor);
    }
    // 作業領域源とモニタ別拡大率表だけを**等倍水準のまま**据える＝同期段が作り直す条件。
    world.insert_resource(stale.snapshot.clone());
    world.insert_resource(stale.dpi_table.clone());
    // 本番の相順所有者が要る結線資源（実 `EmoPresenter`・未装着・GPU 不要）。
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    world.insert_non_send_resource(headless_wiring_with(rx, zero_clock()));

    // ── フレーム A: 本番入口をそのまま 1 度回す ──────────────────────────
    emo2_frame_system(&mut world);
    let writes_a = drain_window_pos_commands();

    // ⑴ 同期段が到達している——2 源とも実行時のモニタ表から作り直された。
    assert_eq!(
        world
            .get_resource::<MonitorSnapshot>()
            .expect("作業領域源がある")
            .work_areas,
        fresh.snapshot.work_areas,
        "同期段が到達していない（作業領域源が等倍水準のまま＝`sync_monitor_snapshot` の呼出が到達不能）"
    );
    assert_eq!(
        world
            .get_resource::<MonitorDpiTable>()
            .expect("モニタ別拡大率表がある")
            .entries,
        fresh.dpi_table.entries,
        "同期段が到達していない（モニタ別拡大率表が等倍水準のまま）"
    );

    // ⑵ 拡大率の相が到達している——`origin=DpiReproject` の書込が出ている。
    let new_bottom = s2_work_area_for_dpi(dpi).bottom;
    assert!(
        origins_of(&writes_a).contains(&PlacementRoute::DpiReproject.as_str().to_string()),
        "拡大率の相が到達していない（`run_dpi_phase` の呼出が到達不能）: {writes_a:?}"
    );
    // 経路語の帰属が一意であること——このフレームの書込は再スナップ由来ではない
    // （同一フレームなら合流でべき等 skip になるので 0 件が正しい）。
    assert!(
        !origins_of(&writes_a).contains(&PlacementRoute::WorkAreaResnap.as_str().to_string()),
        "フレーム A に再スナップ由来の書込が混ざっている（⑵ の観測が拡大率の相のものだと言えない）: {writes_a:?}"
    );
    for scope in [0usize, 1] {
        assert_eq!(
            s2_ground_of(&world, gw.char_window(scope).expect("char 窓がある")),
            new_bottom,
            "scope={scope}: 拡大率の相が新しい作業領域下端へ接地させていない"
        );
    }

    // ── フレーム B: 拡大率を動かさず作業領域だけを動かす ─────────────────
    let hidden_bottom = s2_taskbar_hidden_work_area(dpi).bottom;
    assert_ne!(
        new_bottom, hidden_bottom,
        "探針が退化している: タスクバーの表示切替で作業領域下端が動かない"
    );
    for entity in world
        .query_filtered::<Entity, bevy_ecs::prelude::With<Monitor>>()
        .iter(&world)
        .collect::<Vec<_>>()
    {
        world.despawn(entity);
    }
    for monitor in s2_monitors_with_work_area(dpi, s2_taskbar_hidden_work_area(dpi)) {
        world.spawn(monitor);
    }
    emo2_frame_system(&mut world);
    let writes_b = drain_window_pos_commands();

    // ⑶ 作業領域再スナップが到達している——`origin=WorkAreaResnap` の書込が出ている。
    assert!(
        origins_of(&writes_b).contains(&PlacementRoute::WorkAreaResnap.as_str().to_string()),
        "作業領域再スナップが到達していない（`resnap_for_work_area_change` の呼出が到達不能）: {writes_b:?}"
    );
    // 経路語の帰属が一意であること——拡大率は 1 つも動いていないので相は何も書かない。
    assert!(
        !origins_of(&writes_b).contains(&PlacementRoute::DpiReproject.as_str().to_string()),
        "拡大率を動かしていないフレームに拡大率の相由来の書込が出ている（⑶ の観測が再スナップのものだと言えない）: {writes_b:?}"
    );
    for scope in [0usize, 1] {
        assert_eq!(
            s2_ground_of(&world, gw.char_window(scope).expect("char 窓がある")),
            hidden_bottom,
            "scope={scope}: 再スナップが新しい作業領域下端へ接地させていない"
        );
    }

    // 次に走るのがハーネスでない場合に備えて末尾でも閉じる（要件 7.7）。
    let _residue = drain_window_pos_commands();
}

/// 窓書込指令の経路語（`tag.origin`）を列として取り出す。
fn origins_of(writes: &[SetWindowPosCommand]) -> Vec<String> {
    writes
        .iter()
        .map(|cmd| cmd.tag.origin.to_string())
        .collect()
}

/// 指定窓の接地点 Y（下端）。[`FrameHarness`] を使わない檻のための直読み。
fn s2_ground_of(world: &bevy_ecs::world::World, window: Entity) -> i32 {
    let pos = pos_of(world, window).expect("WindowPos.position がある");
    let size = size_of(world, window).expect("WindowPos.size がある");
    pos.y + size.height
}

// ---------------------------------------------------------------------------
// 7.2 の観測条件そのもの（複数モニタの作業領域を注入した状態で走っている）
// ---------------------------------------------------------------------------

/// 上の各群が**複数モニタ**の作業領域を注入した状態で走っていることを、源の中身で固定する。
///
/// 単一モニタへ退化すると作業領域の解決は「候補が 1 つしか無いから当たる」になり、帰属を
/// 通っているかどうかが観測できなくなる——それでも上の各本は緑のまま通ってしまうので、
/// 条件の側を別に固定する（要件 7.2）。
#[test]
fn the_branch_cases_run_against_a_multi_monitor_work_area_table() {
    for dpi in [SCALE_120, SCALE_192] {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        settle_at(&mut harness, &mut source, dpi);

        let source_areas = &harness
            .work_area_source()
            .expect("作業領域源がある")
            .work_areas;
        assert!(
            source_areas.len() >= 2,
            "dpi={dpi}: 作業領域源が複数モニタになっていない（帰属を通らない退化した観測条件）: {source_areas:?}"
        );
        assert_ne!(
            source_areas[0], source_areas[1],
            "dpi={dpi}: 2 つの作業領域が同一（隣接モニタが候補として効いていない）"
        );
        let table = harness.monitor_dpi_table().expect("モニタ別拡大率表がある");
        assert!(
            table.entries.len() >= 2,
            "dpi={dpi}: モニタ別拡大率表が複数モニタになっていない: {table:?}"
        );
    }
}
