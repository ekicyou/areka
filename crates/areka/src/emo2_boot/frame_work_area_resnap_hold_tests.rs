//! 整合ゲートの **4 つ目**の窓書込口（task 6.5・設計 C5／C6・要件 5.8／7.2）の決定論テスト。
//!
//! # ここが押さえる是正
//!
//! 設計 D15 と設計討議 議題 1 は「待ち札のある窓へのすべての窓書込を見送る」と定めており、
//! 例外は随伴バルーンの追従ただ 1 つである。ところが C5 が列挙した見送り点は 3 つ——拡大率の
//! 相・報告寸の突合・再スナップ——しか無く、task 5.2 が新設した
//! [`resnap_for_work_area_change`](super::work_area_sync::resnap_for_work_area_change) が
//! 抜けていた。**「再スナップ」という日本語が別々の 2 関数を指していた**ためである
//! （C5 が指していたのは `resnap_shell_targets` の側）。
//!
//! 抜けた結果、次の 2 条件が揃うと待ち札の付いた窓へ窓書込が届く:
//!
//! 1. 窓の拡大率と帰属モニタの表が食い違ったまま札が残る（設計 Residual Risks が「上限
//!    30 フレームまで待つ」構成として記録している縁の配置と同じ状態）。
//! 2. その待ちのあいだに作業領域が動き、同期段が `Some` を返す。
//!
//! このとき書込の経路語は `WorkAreaResnap` ゆえ随伴バルーンの例外（`BalloonFollow`）にも
//! 当たらず、`follow/window_move.rs` の不変条件監視が `warn!` の直後に `debug_assert!(false, ..)`
//! を撃つ＝**debug ビルドでは panic する**。
//! [`a_work_area_change_during_the_wait_defers_the_held_window_at_120`] ／
//! [`..._at_192`](a_work_area_change_during_the_wait_defers_the_held_window_at_192) が
//! その形そのものであり、是正前はこの panic で赤くなる。
//!
//! # 零件の主張には陽性の対を同じ本体へ置く
//!
//! 本ファイルの主張は零件（「札のある窓への書込 0」）である。再スナップを丸ごと無操作に
//! しても同じ緑になるので、**同じテスト本体の内側**に 2 つの陽性の対を置く:
//!
//! - **札の無いキャラ窓には従来どおり書く**（scope 1・経路語 `WorkAreaResnap`）。これが
//!   無いと「4 点目を足したら再スナップが全部止まった」という退行を検出できない。
//! - **札のあるバルーンへ随伴の追従は届く**（scope 1 のバルーン・経路語 `BalloonFollow`）。
//!   設計 C5 の「不変条件の唯一の例外」が生きていることを、4 点目を足した後の同じ駆動で示す。
//!
//! # 不変条件の監視は生きたまま・ただし `WorkAreaResnap` では鳴らない
//!
//! 4 点目を足しても `enqueue_window_set_pos` 入口の監視は残る（すり抜け経路が将来増えたら
//! そこで鳴るべきである）。[`the_invariant_watch_no_longer_fires_for_work_area_resnap`] が
//! 「`WorkAreaResnap` では 1 行も鳴らない」を、
//! [`the_invariant_watch_still_fires_for_a_write_that_bypasses_the_sites`] が「見送り点を
//! 通らない経路なら今も落ちる」を、**同じ土台**（同じハーネス・同じ整地・同じ札）で対にする。

use bevy_ecs::entity::Entity;
use wintf::ecs::window::SetWindowPosCommand;
use wintf::ecs::{DPI, WindowPos};

use crate::placement::diag::{PlacementRoute, WindowKind};
use crate::placement::dpi_sync::{
    DPI_SYNC_HOLD_MAX_FRAMES, DpiSyncDecision, DpiSyncHold, evaluate,
};
use crate::placement::follow::move_window_with_route;
use crate::placement::test_support::{LogEvent, capture_logs};
use crate::placement::transition_diag::HOLD_SITE_WORK_AREA_RESNAP;

use super::test_support::{
    FakeReports, FrameHarness, s2_monitors_with_work_area, s2_taskbar_hidden_work_area,
    s2_work_area_for_dpi,
};

/// 遷移前の拡大率水準（等倍）。実行時のモニタ表は本ファイルを通じてこの水準に据え置く
/// ——据え置くからこそ札が外れず、到達条件 ⑴ が成立し続ける。
const BASE_DPI: u16 = 96;

/// 要件 7.2 が名指しする 2 水準のうち低い側。
const SCALE_120: u16 = 120;

/// 同上・高い側（等倍の 2 倍）。
const SCALE_192: u16 = 192;

/// 監視が鳴ったことを本文から拾うための語（`window_move.rs` の `warn!` 本文の一部）。
const INVARIANT_WARNING: &str = "整合待ちの札がある窓へ窓書込が到達した";

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

/// 窓 1 枚だけの拡大率を差し替える（ハーネスの口はスコープ単位＝2 窓まとめてなので、
/// 「キャラ窓は表と揃い、バルーンだけ食い違う」形はここでしか作れない）。
fn set_window_dpi_of(harness: &mut FrameHarness, window: Entity, dpi: u16) {
    harness
        .world
        .entity_mut(window)
        .insert(DPI::from_dpi(dpi, dpi));
}

/// 起動直後の整地——3 つの源と窓の拡大率をすべて [`BASE_DPI`] へ揃え、キャラ窓を当該水準の
/// 作業領域下端へ接地させる。
///
/// 拡大率の相の初回 run は永続 `SystemState` の仕様で全窓へマッチするので、ここで 1 度
/// 空回しして消費する（以後のフレームでは真に変化した窓だけが対象になる）。
fn settle(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(BASE_DPI);
    harness.set_monitor_table_for_dpi(BASE_DPI);
    harness.set_window_dpi(BASE_DPI);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            harness.ground_point(scope).1,
            s2_work_area_for_dpi(BASE_DPI).bottom,
            "前提が崩れている: scope={scope} のキャラ窓が定常水準で作業領域下端へ接地していない"
        );
    }
}

/// タスクバーを隠す構成変更を実行時のモニタ表へ流し込む（**拡大率は据え置く**）。
///
/// 拡大率を据え置くのが要点である——ここで表の拡大率まで動かすと札が外れてしまい、
/// 到達条件 ⑴（札が残っていること）が壊れる。
fn hide_the_taskbar(harness: &mut FrameHarness) {
    harness.set_monitor_table(s2_monitors_with_work_area(
        BASE_DPI,
        s2_taskbar_hidden_work_area(BASE_DPI),
    ));
}

/// 到達条件 ⑴ を作る——scope 0 の 2 窓と scope 1 の**バルーンだけ**へ新しい拡大率を届け、
/// 実行時のモニタ表は [`BASE_DPI`] のまま据え置く。
///
/// scope 1 のキャラ窓を揃えたまま残すのが 2 つの陽性の対の土台である: そのキャラ窓は
/// 再スナップに書かれ（4 点目が全部を止めていないことの証拠）、その書込の随伴が札の付いた
/// scope 1 のバルーンへ届く（例外が生きていることの証拠）。
fn leave_holds_in_place(harness: &mut FrameHarness, source: &mut FakeReports, dpi: u16) {
    for window in [
        harness.char_window(0),
        harness.balloon_window(0),
        harness.balloon_window(1),
    ] {
        set_window_dpi_of(harness, window, dpi);
    }
    harness.advance_frame();
    let change = harness.run_placement_phases(source);
    assert!(
        change.is_none(),
        "dpi={dpi}: 表を据え置いたのに同期段が源を作り直した（探針が退化している）"
    );
    let waiting = harness.drain_writes();
    assert!(
        waiting.is_empty(),
        "dpi={dpi}: 待ちフレームで窓書込が出ている: {waiting:?}"
    );
}

/// 当該窓に整合待ちの札が付いているか。
fn is_held(harness: &FrameHarness, window: Entity) -> bool {
    harness.world.get::<DpiSyncHold>(window).is_some()
}

// ---------------------------------------------------------------------------
// 完了条件そのもの（是正前の赤＝`debug_assert!` の panic）
// ---------------------------------------------------------------------------

/// 待ち札が残っているあいだに作業領域が動いても、**作業領域変化を契機とする再スナップは
/// 当該窓へ書かない**（設計 D15「待ち札のある窓へのすべての窓書込を見送る」・要件 5.8）。
///
/// 是正前は当該窓へ `WorkAreaResnap` の書込が届き、`enqueue_window_set_pos` 入口の
/// `debug_assert!` が panic する。
fn a_work_area_change_during_the_wait_defers_the_held_window_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let old_bottom = s2_work_area_for_dpi(BASE_DPI).bottom;
    let new_bottom = s2_taskbar_hidden_work_area(BASE_DPI).bottom;
    assert_ne!(
        old_bottom, new_bottom,
        "探針が退化している: 作業領域下端が動かない（到達条件 ⑵ を観測できない）"
    );

    leave_holds_in_place(&mut harness, &mut source, dpi);

    let char0 = harness.char_window(0);
    let balloon0 = harness.balloon_window(0);
    let char1 = harness.char_window(1);
    let balloon1 = harness.balloon_window(1);

    // 到達条件 ⑴: 札が残っている（かつ scope 1 のキャラ窓だけは揃っている）。
    for (window, what) in [(char0, "scope 0 キャラ窓"), (balloon0, "scope 0 バルーン")] {
        assert!(
            is_held(&harness, window),
            "dpi={dpi}: {what} に待ち札が付いていない（到達条件 ⑴ が成立していない）"
        );
    }
    assert!(
        is_held(&harness, balloon1),
        "dpi={dpi}: scope 1 バルーンに待ち札が付いていない（随伴の例外を問えない）"
    );
    assert!(
        !is_held(&harness, char1),
        "dpi={dpi}: scope 1 キャラ窓に待ち札が付いている（陽性の対が成立しない）"
    );

    // 到達条件 ⑵: 札が残っているあいだに作業領域が動き、同期段が `Some` を返す。
    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    let change = harness.run_placement_phases(&mut source);
    assert!(
        change.is_some(),
        "dpi={dpi}: 作業領域を動かしたのに同期段が差し替えを報告しない（到達条件 ⑵ が成立していない）"
    );
    // 待ちはまだ上限に達していない（上限超過で進んだのを「見送った」と読み違えない）。
    let outcome = evaluate(&harness.world, char0, harness.frame());
    assert_eq!(
        outcome.decision,
        DpiSyncDecision::Hold,
        "dpi={dpi}: 作業領域が動いたフレームで判定が Hold ではない（上限超過で進んでいる）"
    );
    assert!(
        harness.frame() - outcome.since_frame < DPI_SYNC_HOLD_MAX_FRAMES,
        "dpi={dpi}: 待ちが上限に達している（到達条件 ⑴ の「30 フレーム内」を外れている）"
    );

    let writes = harness.drain_writes();

    // 主張（零件）: 札のある 2 窓へは 1 件も書かない。
    for (scope, kind, what) in [
        (0, WindowKind::Char, "scope 0 キャラ窓"),
        (0, WindowKind::Balloon, "scope 0 バルーン"),
    ] {
        assert!(
            writes_for(&writes, scope, kind).is_empty(),
            "dpi={dpi}: {what}（待ち札あり）へ作業領域再スナップの窓書込が届いている: {writes:?}"
        );
    }
    assert_eq!(
        harness.ground_point(0).1,
        old_bottom,
        "dpi={dpi}: 待ち札のある scope 0 の接地点が動いている（見送っていない）"
    );

    // 陽性の対 ⑴: 札の無いキャラ窓には従来どおり書く（4 点目が再スナップを丸ごと止めていない）。
    let char1_writes = writes_for(&writes, 1, WindowKind::Char);
    assert_eq!(
        char1_writes.len(),
        1,
        "dpi={dpi}: 札の無い scope 1 キャラ窓が再スナップで書かれていない（4 点目が全部を止めた）: {writes:?}"
    );
    assert_eq!(
        char1_writes[0].tag.origin,
        PlacementRoute::WorkAreaResnap.as_str(),
        "dpi={dpi}: scope 1 キャラ窓の書込が作業領域再スナップの経路語で出ていない: {writes:?}"
    );
    assert_eq!(
        char1_writes[0].y + char1_writes[0].height,
        new_bottom,
        "dpi={dpi}: scope 1 キャラ窓が新しい作業領域下端へ接地していない: {writes:?}"
    );
    assert_eq!(
        harness.ground_point(1).1,
        new_bottom,
        "dpi={dpi}: scope 1 の接地点が新しい作業領域下端に載っていない"
    );

    // 陽性の対 ⑵: 札のあるバルーンへ随伴の追従は届く（設計 C5 の唯一の例外）。
    let balloon1_writes = writes_for(&writes, 1, WindowKind::Balloon);
    assert_eq!(
        balloon1_writes.len(),
        1,
        "dpi={dpi}: 待ち札のある scope 1 バルーンへ随伴が届いていない（引き剥がされている）: {writes:?}"
    );
    assert_eq!(
        balloon1_writes[0].tag.origin,
        PlacementRoute::BalloonFollow.as_str(),
        "dpi={dpi}: scope 1 バルーンの書込が随伴の経路語で出ていない: {writes:?}"
    );
    assert!(
        is_held(&harness, balloon1),
        "dpi={dpi}: 随伴が届いた側の待ち札が外れている（外すのは拡大率の相だけ）"
    );
}

/// 要件 7.2 の低い側（120）。
#[test]
fn a_work_area_change_during_the_wait_defers_the_held_window_at_120() {
    a_work_area_change_during_the_wait_defers_the_held_window_at(SCALE_120);
}

/// 要件 7.2 の高い側（192）。
#[test]
fn a_work_area_change_during_the_wait_defers_the_held_window_at_192() {
    a_work_area_change_during_the_wait_defers_the_held_window_at(SCALE_192);
}

// ---------------------------------------------------------------------------
// 見送りの記録（4 つ目の観測点語）
// ---------------------------------------------------------------------------

/// 4 点目の見送りは **`site=work-area-resnap`** として記録される（3 つ目の `site=resnap` とは
/// 別語＝どちらの「再スナップ」が見送ったのかがログから判る）。
///
/// 対（零件）は同じ本体の前半——札の無いフレームでは 1 行も出ない。
#[test]
fn the_fourth_site_records_its_own_deferral_word() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    // 前半（零件）: 札が 1 つも無いフレームで作業領域が動いても、見送りの行は出ない。
    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    let (_, events) = capture_logs(|| {
        harness.run_placement_phases(&mut source);
    });
    let quiet: Vec<String> = events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.contains(&format!("site={HOLD_SITE_WORK_AREA_RESNAP}")))
        .collect();
    assert!(
        quiet.is_empty(),
        "札が 1 つも無いのに 4 点目の見送りが記録されている: {quiet:?}"
    );
    // 上の零件は、そもそも 4 点目が走っていなければ空虚に緑になる。同じフレームの書込で
    // 「作業領域が実際に動き、再スナップが札の無い窓へ書いた」ことを固定しておく。
    let ran = harness.drain_writes();
    assert!(
        !writes_for(&ran, 0, WindowKind::Char).is_empty(),
        "札の無いフレームで 4 点目が 1 本も書いていない（駆動が死んでいる）: {ran:?}"
    );

    // 後半（陽性）: 札を残したまま作業領域をもう一度動かすと、4 点目の見送りが記録される。
    leave_holds_in_place(&mut harness, &mut source, SCALE_192);
    harness.set_monitor_table(s2_monitors_with_work_area(
        BASE_DPI,
        s2_work_area_for_dpi(BASE_DPI),
    ));
    harness.advance_frame();
    let frame = harness.frame();
    let (_, events) = capture_logs(|| {
        harness.run_placement_phases(&mut source);
    });
    let lines: Vec<String> = events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with("[transition]") && m.contains("kind=hold"))
        .collect();
    let record = lines
        .iter()
        .find(|m| {
            m.contains(&format!("site={HOLD_SITE_WORK_AREA_RESNAP}"))
                && m.contains("win_kind=char")
                && m.contains("scope=0")
        })
        .unwrap_or_else(|| panic!("4 点目の見送りが記録されていない: {lines:?}"));
    for token in [
        format!("frame={frame} "),
        "decision=hold".to_string(),
        format!("window_dpi={}", u32::from(SCALE_192)),
        format!("table_dpi={}", u32::from(BASE_DPI)),
    ] {
        assert!(
            record.contains(&token),
            "記録に `{token}` が載っていない: {record}"
        );
    }
}

// ---------------------------------------------------------------------------
// 不変条件の監視（生かしたまま・`WorkAreaResnap` では鳴らない）
// ---------------------------------------------------------------------------

/// 4 点目を足した後、作業領域変化を契機とする再スナップでは不変条件の監視が**もう鳴らない**
/// （設計 C5 の監視・偽の警報を出さない）。
///
/// 零件（「鳴らない」）の陽性の対は
/// [`the_invariant_watch_still_fires_for_a_write_that_bypasses_the_sites`]——**同じハーネスの
/// World の同じ窓**へ、見送り点を通らない経路で書けば監視は今も落ちる。同じ土台で対にして
/// あるので、「ハーネスが監視を黙らせているから鳴らなかった」という読み方が塞がれる。
#[test]
fn the_invariant_watch_no_longer_fires_for_work_area_resnap() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    leave_holds_in_place(&mut harness, &mut source, SCALE_192);

    assert!(
        is_held(&harness, harness.char_window(0)),
        "scope 0 キャラ窓に待ち札が付いていない（監視の的が無い）"
    );

    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    let (_, events) = capture_logs(|| {
        harness.run_placement_phases(&mut source);
    });
    assert_eq!(
        warnings_containing(&events, INVARIANT_WARNING),
        0,
        "作業領域再スナップで不変条件の監視が鳴っている（4 点目の見送りが効いていない）"
    );
    // 駆動が生きていたことの対（監視が鳴らなかったのは「何も走らなかったから」ではない）。
    let writes = harness.drain_writes();
    assert!(
        !writes_for(&writes, 1, WindowKind::Char).is_empty(),
        "作業領域再スナップが 1 窓も書いていない（駆動が死んでいる）: {writes:?}"
    );
}

/// 陽性の対——監視そのものは生きている。**見送りが覆うべき経路**（`ChainRealign`＝遷移後の
/// 連鎖の解き直し）で札のある窓へ書けば、`enqueue_window_set_pos` 入口の `debug_assert!` が
/// その場で落ちる。反映の手続きを直接呼ぶことで、本番側の見送り（`realign_chain_once_with`
/// の解決条件 2＝札を持つゴースト窓が 1 つも無いこと）を迂回した形である。
///
/// 上の零件と**同じ土台**（同じハーネス・同じ整地・同じ札）で組んであるのが要点である。
/// 監視を丸ごと削れば本テストが赤くなり、上の零件だけが空虚に緑になることはない。
///
/// 経路語は当初 `move_window_to`（＝`MoveCue`）だったが、task 7.5 で「明示操作は見送らない
/// ことが正しい＝鳴らすのは偽の警報」と裁定したため差し替えた（分類の全体は
/// `placement/follow_window_move_hold_watch_tests.rs`）。
#[test]
#[should_panic(expected = "DpiSyncHold")]
fn the_invariant_watch_still_fires_for_a_write_that_bypasses_the_sites() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    leave_holds_in_place(&mut harness, &mut source, SCALE_192);

    let char0 = harness.char_window(0);
    assert!(
        is_held(&harness, char0),
        "scope 0 キャラ窓に待ち札が付いていない（監視の的が無い）"
    );
    let position = harness
        .world
        .get::<WindowPos>(char0)
        .and_then(|wp| wp.position)
        .expect("WindowPos.position がある");
    move_window_with_route(
        &mut harness.world,
        char0,
        position.x + 17,
        position.y,
        PlacementRoute::ChainRealign,
    );
}
