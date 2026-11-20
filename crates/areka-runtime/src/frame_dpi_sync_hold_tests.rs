//! 拡大率と表の整合待ち（設計 C5・要件 5.8／4.1／4.6／10.7）の決定論テスト。
//!
//! Windows は `WM_DPICHANGED`（窓の拡大率）と `WM_DISPLAYCHANGE`（モニタ表）の順序を保証
//! しない。拡大率通知が先に届く順序の 2 段書込を是正前の赤で示す対テストと、待ちの上限・
//! 別モニタへ移した窓は、`frame_transition_branch_wait_tests.rs` と
//! `frame_transition_branch_tests.rs` が拡大率 120 と 192 で持つ。ここが持つのは、そこに無い
//! 駆動口の陽性の対・待ち札のあるバルーンへの随伴・`kind=hold` の発行点・シェルの切り替えを
//! 跨いだ札である。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 本ファイルの主張は零件（「待ち中は書込 0」）に偏る。待ち札を作らずに窓書込口を丸ごと
//! 塞いでも同じ緑になるので、**同じ駆動口が陽性側でも効くこと**を
//! [`the_same_drive_ports_write_when_nothing_is_waiting`] が固定する。加えて各テストは
//! 零件を主張する前に「駆動は生きている」（待ち札が実際に付いた・報告が持ち越された）を
//! 先に主張する——駆動が死んでいたから 0 だった、という読み方を塞ぐためである。

use areka_placement::diag::WindowKind;
use areka_placement::dpi_sync::DpiSyncHold;
use log_capture_kit::{ExpectField, capture};

use super::shared_test_support::{HIGH_DPI, LOW_DPI};
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, s2_work_area_for_dpi,
};
use super::transition_test_support::{
    CHAR_ON_NEIGHBOR, neighbor_at, set_position, settle_at, writes_for,
};
use super::{balloon_target, shell_target};

/// scope 0 のキャラ窓が [`HIGH_DPI`] で報告する物理寸（等倍寸の 2 倍）。
const HIGH_SIZE_0: (u32, u32) = (SPAWN_SIZE_0.0 * 2, SPAWN_SIZE_0.1 * 2);

/// scope 0 のシェル target 番号（報告源の鍵）。
fn shell0() -> u32 {
    shell_target(0).0
}

// ---------------------------------------------------------------------------
// 零件の主張の陽性の対（同じ駆動口）
// ---------------------------------------------------------------------------

/// 待ち札が 1 つも無ければ、本ファイルが駆動する **3 つの口**（拡大率の相・報告寸の突合・
/// 実表示寸の再スナップ）はすべて窓を書く。
///
/// 見送り点は 5 つ（＋シェルの切り替えの見送り）。作業領域変化を契機とする再スナップの点の
/// 零件と陽性の対は `frame_work_area_resnap_hold_tests.rs` が持つ——ここへ足さないのは、その点の
/// 駆動には「札が残ったまま作業領域が動く」という別の配置が要るからである。
#[test]
fn the_same_drive_ports_write_when_nothing_is_waiting() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);
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
        writes_for(&phase_writes, 0, WindowKind::Char).len(),
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

/// 随伴バルーンの追従は、待ち札のあるバルーンへも**届く**（設計 C5 の適用範囲の例外）。
///
/// 2 窓の中心が別々のモニタに乗る配置では、キャラ窓だけが表と揃ってバルーンが食い違う。
/// ここで随伴まで見送ると、バルーンがキャラから引き剥がされて宙に残る——1 フレーム古い矩形
/// より悪い。バルーン**自身**の書込は引き続き見送られることも同じ本体で固定する。
#[test]
fn a_companion_follow_still_reaches_a_waiting_balloon() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);

    harness.set_monitor_table(neighbor_at(HIGH_DPI));
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
        writes_for(&writes, 0, WindowKind::Char).len(),
        1,
        "キャラ窓が書かれていない（探針が退化している）: {writes:?}"
    );
    assert_eq!(
        writes_for(&writes, 0, WindowKind::Balloon).len(),
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
/// `crates/areka-placement/src/follow_transition_diag_tests.rs` が directive 単位で所有する）。
fn transition_lines(drive: impl FnOnce()) -> Vec<String> {
    let (_, events) = capture(drive);
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with("[transition]"))
        .collect()
}

/// 本ファイルが駆動する 3 つの点それぞれが `kind=hold` を出し、**どの点で見送ったか**が
/// `site=` で判る（5 つのうち `site=work-area-resnap` は `frame_work_area_resnap_hold_tests.rs` が持つ）。
#[test]
fn each_of_the_three_sites_emits_its_own_hold_record() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);

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
    settle_at(&mut harness, &mut source, LOW_DPI);

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

// ---------------------------------------------------------------------------
// シェルの切り替えを跨いだ札
// ---------------------------------------------------------------------------

/// 札の付いた窓はシェルの切り替えの置き直しで見送られ、札が外れるフレームに拡大率の相が
/// 入れ直した揃え方で 1 回だけ書く。札の無い窓は切り替えの段で揃え方の変化として動く。
#[test]
fn a_window_held_through_a_shell_switch_is_written_on_the_frame_its_hold_lifts() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);
    let top = s2_work_area_for_dpi(HIGH_DPI).top;
    let char0 = harness.char_window(0);
    let char1 = harness.char_window(1);

    harness.set_scope_dpi(0, HIGH_DPI);
    source.refresh.insert(shell0(), HIGH_SIZE_0);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    assert!(
        harness.world.get::<DpiSyncHold>(char0).is_some(),
        "scope 0 に札が付いていない（探針が退化している）"
    );
    assert!(
        harness.world.get::<DpiSyncHold>(char1).is_none(),
        "scope 1 に札が付いた（陽性の対が成立しない）"
    );
    let _held = harness.drain_writes();

    let windows = harness
        .world
        .resource::<areka_placement::spawn::GhostWindows>()
        .clone();
    let mut basis = super::test_support::placement_basis(
        [("seriko.alignmenttodesktop".to_string(), "top".to_string())].into(),
        harness.scopes().len(),
    );
    basis
        .source
        .ghost_kv
        .insert("kero.name".to_string(), "k".to_string());
    let ((), events) = capture(|| {
        areka_placement::reseed::apply_shell_descript(
            &mut harness.world,
            &windows,
            &basis.source,
            &basis.balloon,
            &[],
        );
        areka_placement::reseed::reanchor_char_windows(&mut harness.world, &windows);
    });
    let held: Vec<&str> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reseed_held"))
        .map(|e| e.expect_field("scope"))
        .collect();
    assert_eq!(held, vec!["0"], "見送りの記録が scope 0 の 1 行でない");
    let switched = harness.drain_writes();
    assert!(
        writes_for(&switched, 0, WindowKind::Char).is_empty(),
        "札のある窓へ切り替えの段で書いている: {switched:?}"
    );
    let moved = writes_for(&switched, 1, WindowKind::Char);
    assert_eq!(
        moved
            .iter()
            .map(|cmd| (cmd.tag.origin, cmd.y))
            .collect::<Vec<_>>(),
        vec![("AnchorChange", top)],
        "札の無い窓が切り替えの段で新しい揃え方へ動いていない: {switched:?}"
    );

    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let released = harness.drain_writes();
    assert_eq!(
        writes_for(&released, 0, WindowKind::Char)
            .iter()
            .map(|cmd| (cmd.tag.origin, cmd.y))
            .collect::<Vec<_>>(),
        vec![("DpiReproject", top)],
        "札が外れるフレームに入れ直した揃え方で 1 回だけ書かれていない: {released:?}"
    );
    assert!(
        harness.world.get::<DpiSyncHold>(char0).is_none(),
        "表が追いついたのに札が外れていない"
    );
}
