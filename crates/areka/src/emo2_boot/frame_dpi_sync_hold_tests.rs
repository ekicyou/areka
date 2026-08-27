//! 拡大率と表の整合待ち（task 5.4・設計 C5・要件 5.8／4.1／4.6／10.7）の決定論テスト。
//!
//! # ここが押さえる是正
//!
//! Windows は `WM_DPICHANGED`（窓の拡大率）と `WM_DISPLAYCHANGE`（モニタ表）の順序を保証
//! しない。実機の 12 遷移はすべて表更新が先だったが（確定台帳 L2）、**拡大率通知が先に
//! 届く順序**では窓の拡大率だけが新しくなり、作業領域源はまだ旧水準のままになる。この
//! 順序で拡大率の相をそのまま走らせると、窓は**新しい寸のまま旧作業領域下端へ**接地した
//! 中間矩形で 1 度書かれ、表が追いついた次のフレームでもう 1 度書き直される——要件 5.8 が
//! 名指しで禁じる 2 段書込である。
//!
//! [`a_scale_notice_ahead_of_the_table_lands_in_one_write_without_the_old_bottom`] がその形
//! そのものである。是正前は旧下端 1444 の中間矩形が出て赤くなり、整合待ちが入ると待ち
//! フレームの書込が 0 になって、表が追いついたフレームで新寸・新下端へ 1 回で移る。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 本ファイルの主張は零件（「待ち中は書込 0」）に偏る。待ち札を作らずに窓書込口を丸ごと
//! 塞いでも同じ緑になるので、**同じ駆動口が陽性側でも効くこと**を
//! [`the_same_drive_ports_write_when_nothing_is_waiting`] が固定する。加えて各テストは
//! 零件を主張する前に「駆動は生きている」（待ち札が実際に付いた・報告が持ち越された）を
//! 先に主張する——駆動が死んでいたから 0 だった、という読み方を塞ぐためである。

use bevy_ecs::entity::Entity;
use wintf::ecs::window::SetWindowPosCommand;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{Point, WindowPos};

use crate::placement::dpi_sync::{DPI_SYNC_HOLD_MAX_FRAMES, DpiSyncHold, evaluate};
use crate::placement::follow::BalloonFollow;
use crate::placement::test_support::capture_logs;

use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, s2_monitors,
    s2_work_area_for_dpi,
};
use super::{balloon_target, shell_target};

/// 遷移前の拡大率水準。
const LOW_DPI: u16 = 96;

/// 遷移後の拡大率水準（等倍の 2 倍＝寸も下端も動く）。
const HIGH_DPI: u16 = 192;

/// scope 0 のキャラ窓が [`HIGH_DPI`] で報告する物理寸（等倍寸の 2 倍）。
const HIGH_SIZE_0: (u32, u32) = (SPAWN_SIZE_0.0 * 2, SPAWN_SIZE_0.1 * 2);

/// 待ちが解けるまで回す定常フレーム数（上限には遠い値）。
const WAIT_FRAMES: u32 = 3;

/// キャラ窓を隣接モニタへ置くときの位置（随伴バルーンも隣接モニタ内へ収まる値）。
const CHAR_ON_NEIGHBOR: Point = Point { x: 3200, y: 800 };

/// scope 0 のシェル target 番号（報告源の鍵）。
fn shell0() -> u32 {
    shell_target(0).0
}

/// 指定スコープ・指定種別の窓書込だけを取り出す。
fn writes_for(writes: &[SetWindowPosCommand], scope: u32, kind: &str) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind)
        .cloned()
        .collect()
}

/// キャラ窓の書込が置いた接地点（下端）の列。
fn char_write_bottoms(writes: &[SetWindowPosCommand]) -> Vec<i32> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.kind == "char")
        .map(|cmd| cmd.y + cmd.height)
        .collect()
}

/// 起動直後の整地——3 つの源と窓の拡大率をすべて [`LOW_DPI`] へ揃え、キャラ窓を当該水準の
/// 作業領域下端へ接地させる。
///
/// 拡大率の相の初回 run は永続 `SystemState` の仕様で全窓へマッチするので、ここで 1 度
/// 空回しして消費する。
fn settle(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(LOW_DPI);
    harness.set_monitor_table_for_dpi(LOW_DPI);
    harness.set_window_dpi(LOW_DPI);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            harness.ground_point(scope).1,
            s2_work_area_for_dpi(LOW_DPI).bottom,
            "前提が崩れている: scope={scope} のキャラ窓が定常水準で作業領域下端へ接地していない"
        );
    }
}

/// 隣接モニタ（ゴーストが決して居ない側）だけを高 DPI にした実行時のモニタ表。
fn neighbor_at_high_dpi() -> Vec<Monitor> {
    let mut monitors = s2_monitors(LOW_DPI);
    monitors[1].dpi = u32::from(HIGH_DPI);
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

/// 当該スコープの二体をまとめて隣接モニタへ移す（随伴の窓相対を保ったまま）。
fn move_scope_to_neighbor(harness: &mut FrameHarness, scope: usize) {
    let char_window = harness.char_window(scope);
    let balloon = harness.balloon_window(scope);
    let offset = harness
        .world
        .get::<BalloonFollow>(char_window)
        .expect("char 窓は BalloonFollow を持つ")
        .offset();
    set_position(harness, char_window, CHAR_ON_NEIGHBOR);
    set_position(
        harness,
        balloon,
        Point {
            x: CHAR_ON_NEIGHBOR.x + offset.x,
            y: CHAR_ON_NEIGHBOR.y + offset.y,
        },
    );
}

/// 拡大率通知だけが先に届いた状態を作る（モニタ表は旧水準のまま据え置く）。
fn deliver_the_scale_notice_first(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_window_dpi(HIGH_DPI);
    source.refresh.insert(shell0(), HIGH_SIZE_0);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

// ---------------------------------------------------------------------------
// 完了条件そのもの（是正前の赤）
// ---------------------------------------------------------------------------

/// 拡大率通知が表更新より先に届く順序でも、旧下端の中間矩形を出さずに **1 回の書込**で
/// 新寸・新下端へ移る（要件 5.8・完了条件）。
#[test]
fn a_scale_notice_ahead_of_the_table_lands_in_one_write_without_the_old_bottom() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let old_bottom = s2_work_area_for_dpi(LOW_DPI).bottom;
    let new_bottom = s2_work_area_for_dpi(HIGH_DPI).bottom;
    assert_ne!(
        old_bottom, new_bottom,
        "探針が退化している: 2 つの拡大率水準で作業領域下端が動かない（中間矩形を観測できない）"
    );

    // 待ちフレーム: 拡大率だけが新しく、表はまだ旧水準。
    deliver_the_scale_notice_first(&mut harness, &mut source);
    // 零件を主張する**前に**、このフレームで本当に食い違いが起きていることを固定する
    // ——条件が成立していなければ「書込 0」は何も意味しない。
    for scope in harness.scopes().to_vec() {
        let outcome = evaluate(&harness.world, harness.char_window(scope), harness.frame());
        assert_eq!(
            (outcome.window_dpi, outcome.table_dpi),
            (u32::from(HIGH_DPI), Some(u32::from(LOW_DPI))),
            "scope={scope}: 拡大率通知が先に届いた状態になっていない（探針が退化している）"
        );
    }
    let waiting = harness.drain_writes();
    assert!(
        waiting.is_empty(),
        "表が追いつく前に窓書込が出ている（旧下端 {old_bottom} の中間矩形）: {waiting:?}"
    );
    // ゲートが実際に走った証拠（見送りの札が付いている）。
    for scope in harness.scopes().to_vec() {
        assert!(
            harness
                .world
                .get::<DpiSyncHold>(harness.char_window(scope))
                .is_some(),
            "scope={scope}: 拡大率と表が食い違うのに待ち札が付いていない（ゲートが走っていない）"
        );
    }

    // 表が追いつくフレーム。
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let released = harness.drain_writes();

    assert_eq!(
        writes_for(&released, 0, "char").len(),
        1,
        "解除フレームで scope 0 のキャラ窓が 1 回で移っていない: {released:?}"
    );
    for bottom in char_write_bottoms(&released) {
        assert_eq!(
            bottom, new_bottom,
            "キャラ窓の書込が新しい作業領域下端に載っていない（旧下端 {old_bottom} の中間矩形）: {released:?}"
        );
    }
    assert_eq!(
        harness.ground_point(0).1,
        new_bottom,
        "遷移後の接地点が新しい作業領域下端に載っていない"
    );
    assert!(
        harness
            .world
            .get::<DpiSyncHold>(harness.char_window(0))
            .is_none(),
        "表が追いついたのに待ち札が外れていない"
    );
}

/// 待ちのあいだに表情差替（`ShowSurface` が積む窓寸要求）と再スナップが来ても、当該窓への
/// 窓書込は 0 のままであり、報告は消費されずに次フレームへ持ち越される（設計 C5 議題 1）。
#[test]
fn a_surface_swap_during_the_wait_writes_nothing_and_keeps_the_report() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    deliver_the_scale_notice_first(&mut harness, &mut source);
    let _waiting = harness.drain_writes();

    // 待ち中の表情差替: drain 相が積んだ未消費の窓寸要求と、実表示寸の食い違い。
    source.pending.insert(shell0(), HIGH_SIZE_0);
    harness.run_reconcile(&mut source);
    harness.run_resnap(&PerTargetSizes::new([
        (0, Some(HIGH_SIZE_0)),
        (1, Some(SPAWN_SIZE_1)),
    ]));

    let writes = harness.drain_writes();
    assert!(
        writes.is_empty(),
        "待ち札のある窓へ報告寸の突合・再スナップから窓書込が届いている: {writes:?}"
    );
    assert!(
        source.pending.contains_key(&shell0()),
        "報告を消費してしまっている（待ち中は消費せず次フレームへ持ち越す）"
    );
}

/// 上限フレームを超えたら、警告の上で**現在の源のまま**進む（設計 C5・要件 4.4）。
#[test]
fn the_wait_gives_up_after_the_bounded_number_of_frames_and_proceeds() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    deliver_the_scale_notice_first(&mut harness, &mut source);
    let _waiting = harness.drain_writes();

    // 表は永遠に追いつかない。上限に達するまでは 1 件も書かない。
    for _ in 0..DPI_SYNC_HOLD_MAX_FRAMES - 1 {
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        let writes = harness.drain_writes();
        assert!(
            writes.is_empty(),
            "上限に達する前に書いている（frame={}）: {writes:?}",
            harness.frame()
        );
    }

    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();
    assert!(
        !writes.is_empty(),
        "上限を超えても待ち続けている（有界でない）"
    );
    assert!(
        harness
            .world
            .get::<DpiSyncHold>(harness.char_window(0))
            .is_none(),
        "上限を超えたのに待ち札が残っている"
    );
}

// ---------------------------------------------------------------------------
// 零件の主張の陽性の対（同じ駆動口）
// ---------------------------------------------------------------------------

/// 待ち札が 1 つも無ければ、本ファイルが駆動する **3 つの口**（拡大率の相・報告寸の突合・
/// 実表示寸の再スナップ）はすべて窓を書く。
///
/// 見送り点は task 6.5 で 4 つになった（4 点目＝作業領域変化を契機とする再スナップ）。その点の
/// 零件と陽性の対は `frame_work_area_resnap_hold_tests.rs` が持つ——ここへ足さないのは、4 点目の
/// 駆動には「札が残ったまま作業領域が動く」という別の配置が要るからである。
#[test]
fn the_same_drive_ports_write_when_nothing_is_waiting() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    assert!(
        harness
            .world
            .get::<DpiSyncHold>(harness.char_window(0))
            .is_none(),
        "整地の時点で待ち札が付いている（陽性の対が成立しない）"
    );

    // ⑴ 拡大率の相: 拡大率と表を**同時に**動かす（経路 (b)＝待ちは起きない）。
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    source.refresh.insert(shell0(), HIGH_SIZE_0);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let phase_writes = harness.drain_writes();
    assert_eq!(
        writes_for(&phase_writes, 0, "char").len(),
        1,
        "経路 (b) で拡大率の相が書いていない（駆動口が死んでいる）: {phase_writes:?}"
    );

    // ⑵ 報告寸の突合: 未消費の報告を積めば書く。
    source
        .pending
        .insert(shell0(), (HIGH_SIZE_0.0 + 8, HIGH_SIZE_0.1));
    harness.run_reconcile(&mut source);
    let reconcile_writes = harness.drain_writes();
    assert!(
        !reconcile_writes.is_empty(),
        "報告寸の突合が書いていない（駆動口が死んでいる）"
    );

    // ⑶ 再スナップ: 実表示寸が窓と食い違えば書く。
    harness.run_resnap(&PerTargetSizes::new([
        (0, Some((HIGH_SIZE_0.0 + 16, HIGH_SIZE_0.1))),
        (1, None),
    ]));
    let resnap_writes = harness.drain_writes();
    assert!(
        !resnap_writes.is_empty(),
        "再スナップが書いていない（駆動口が死んでいる）"
    );
}

/// 別モニタへ移した窓は**待たずに通る**（要件 10.7）。移動先の拡大率は表に既在なので、
/// 窓の拡大率と表は最初から一致している。
#[test]
fn a_window_on_another_monitor_with_a_known_dpi_never_waits() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    // 隣接モニタだけが高 DPI の表を作る（実行時のモニタ表 → 同期段が 2 源を作り直す）。
    harness.set_monitor_table(neighbor_at_high_dpi());
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let _sync_writes = harness.drain_writes();

    // scope 0 の**二体とも**隣接モニタ（高 DPI 側）へ移し、窓の拡大率もその値にする
    // ——ドラッグでゴーストを移せばバルーンも一緒に移る（随伴は窓相対）。
    move_scope_to_neighbor(&mut harness, 0);
    harness.set_scope_dpi(0, HIGH_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    let char0 = harness.char_window(0);
    let balloon0 = harness.balloon_window(0);
    for (window, what) in [(char0, "キャラ窓"), (balloon0, "バルーン窓")] {
        assert!(
            harness.world.get::<DpiSyncHold>(window).is_none(),
            "{what}: 移動先の拡大率が表に既在なのに待ち札が付いた（要件 10.7）"
        );
    }
    for _ in 0..WAIT_FRAMES {
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        assert!(
            harness.world.get::<DpiSyncHold>(char0).is_none(),
            "後続フレームで待ち札が付いた（要件 10.7）"
        );
    }
}

/// 随伴バルーンの追従は、待ち札のあるバルーンへも**届く**（設計 C5 の適用範囲の例外）。
///
/// 2 窓の中心が別々のモニタに乗る配置では、キャラ窓だけが表と揃ってバルーンが食い違う。
/// ここで随伴まで見送ると、バルーンがキャラから引き剥がされて宙に残る——1 フレーム古い矩形
/// より悪い。バルーン**自身**の書込は引き続き見送られることも同じ本体で固定する。
#[test]
fn a_companion_follow_still_reaches_a_waiting_balloon() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    harness.set_monitor_table(neighbor_at_high_dpi());
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let _sync_writes = harness.drain_writes();

    // キャラ窓**だけ**を隣接モニタ（高 DPI 側）へ移す。バルーンは低 DPI 側に残る。
    let char0 = harness.char_window(0);
    let balloon0 = harness.balloon_window(0);
    set_position(&mut harness, char0, CHAR_ON_NEIGHBOR);
    harness.set_scope_dpi(0, HIGH_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    // 前提: キャラ窓は揃っていて、バルーンだけが待っている。
    assert!(
        harness.world.get::<DpiSyncHold>(char0).is_none(),
        "キャラ窓が待っている（この探針ではバルーンだけが待つ形を作れていない）"
    );
    assert!(
        harness.world.get::<DpiSyncHold>(balloon0).is_some(),
        "バルーンが待っていない（この探針ではバルーンだけが待つ形を作れていない）"
    );

    // 随伴は届く（キャラ窓の書込と同一フレーム）。
    let writes = harness.drain_writes();
    assert_eq!(
        writes_for(&writes, 0, "char").len(),
        1,
        "キャラ窓が書かれていない（探針が退化している）: {writes:?}"
    );
    assert_eq!(
        writes_for(&writes, 0, "balloon").len(),
        1,
        "待ち札のあるバルーンへ随伴が届いていない（引き剥がされている）: {writes:?}"
    );

    // バルーン**自身**の経路は見送られたまま（報告は消費されない）。
    source.pending.insert(balloon_target(0).0, (300, 200));
    harness.run_reconcile(&mut source);
    let own_writes = harness.drain_writes();
    assert!(
        own_writes.is_empty(),
        "待ち札のあるバルーン自身の書込が出ている: {own_writes:?}"
    );
    assert!(
        source.pending.contains_key(&balloon_target(0).0),
        "バルーン自身の報告を消費してしまっている"
    );
}

// ---------------------------------------------------------------------------
// 観測レコード（`kind=hold` の発行点）
// ---------------------------------------------------------------------------

/// 与えられた駆動を回し、遷移観測の行だけを拾う。
///
/// 捕捉窓の内側は観測 target が点いた状態である（既定 OFF そのものは
/// `placement/follow_transition_diag_tests.rs` が directive 単位で所有する）。
fn transition_lines(drive: impl FnOnce()) -> Vec<String> {
    let (_, events) = capture_logs(drive);
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with("[transition]"))
        .collect()
}

/// 本ファイルが駆動する 3 つの点それぞれが `kind=hold` を出し、**どの点で見送ったか**が
/// `site=` で判る（4 点目 `site=work-area-resnap` は `frame_work_area_resnap_hold_tests.rs` が持つ）。
#[test]
fn each_of_the_three_sites_emits_its_own_hold_record() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    // 拡大率の相（`site=dpi`）。
    harness.set_window_dpi(HIGH_DPI);
    source.refresh.insert(shell0(), HIGH_SIZE_0);
    harness.advance_frame();
    let frame = harness.frame();
    let lines = transition_lines(|| {
        harness.run_placement_phases(&mut source);
    });
    let holds: Vec<&String> = lines.iter().filter(|m| m.contains("kind=hold")).collect();
    assert!(
        !holds.is_empty(),
        "拡大率の相が整合待ちの記録を出していない: {lines:?}"
    );
    let char_hold = holds
        .iter()
        .find(|m| m.contains("win_kind=char") && m.contains("scope=0"))
        .unwrap_or_else(|| panic!("scope 0 のキャラ窓の記録が無い: {holds:?}"));
    for token in [
        format!("frame={frame} "),
        "decision=hold".to_string(),
        "site=dpi".to_string(),
        format!("window_dpi={}", u32::from(HIGH_DPI)),
        format!("table_dpi={}", u32::from(LOW_DPI)),
        format!("since_frame={frame}"),
    ] {
        assert!(
            char_hold.contains(&token),
            "記録に `{token}` が載っていない: {char_hold}"
        );
    }
    let _waiting = harness.drain_writes();

    // 報告寸の突合（`site=reconcile`）と再スナップ（`site=resnap`）。
    source.pending.insert(shell0(), HIGH_SIZE_0);
    let lines = transition_lines(|| {
        harness.run_reconcile(&mut source);
        harness.run_resnap(&PerTargetSizes::new([
            (0, Some(HIGH_SIZE_0)),
            (1, Some(SPAWN_SIZE_1)),
        ]));
    });
    for site in ["site=reconcile", "site=resnap"] {
        assert!(
            lines
                .iter()
                .any(|m| m.contains("kind=hold") && m.contains(site)),
            "{site} の整合待ちの記録が出ていない: {lines:?}"
        );
    }
}

/// 対（零件）: 判定の下らない定常フレームでは `kind=hold` が 1 行も出ない。**陽性の対**は
/// 同じ本体の後半——揃った順序（経路 (b)）で遷移すれば `decision=proceed` の記録が出る。
///
/// 毎フレーム出す形にすると、判定側が遷移を切り出すときの雑音になるうえ、定常状態で確保が
/// 走る（要件 10.4）。逆に、遷移フレームで 1 行も出なければゲートが走っていない——後半が
/// 無いと「発行点を丸ごと消しても緑」になる。
#[test]
fn a_steady_frame_emits_no_hold_record_while_a_transition_emits_a_proceed() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    // 前半（零件）: 拡大率も表も動かない定常フレーム。
    harness.advance_frame();
    let lines = transition_lines(|| {
        harness.run_placement_phases(&mut source);
        harness.run_reconcile(&mut source);
        harness.run_resnap(&PerTargetSizes::new([(0, None), (1, None)]));
    });
    assert!(
        lines.iter().all(|m| !m.contains("kind=hold")),
        "判定の下っていない定常フレームで整合待ちの記録が出ている: {lines:?}"
    );

    // 後半（陽性の対）: 拡大率と表が**同時に**動く順序＝待ちは起きないが判定は下る。
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    harness.advance_frame();
    let lines = transition_lines(|| {
        harness.run_placement_phases(&mut source);
    });
    let proceeds: Vec<&String> = lines
        .iter()
        .filter(|m| m.contains("kind=hold") && m.contains("decision=proceed"))
        .collect();
    assert!(
        !proceeds.is_empty(),
        "遷移フレームでゲートの判定が 1 行も記録されていない（発行点が死んでいる）: {lines:?}"
    );
    assert!(
        proceeds.iter().all(|m| m.contains("site=dpi")),
        "見送っていないのに拡大率の相以外の点が記録を出している: {proceeds:?}"
    );
}
