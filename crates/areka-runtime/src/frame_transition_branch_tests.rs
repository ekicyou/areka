//! 整合待ちと作業領域追随の**判断分岐**を多フレーム駆動で固定する。
//!
//! # このファイルが持つもの／兄弟が持つもの
//!
//! 是正そのものの対テスト（是正前は赤・是正後は緑）の持ち主:
//!
//! | 是正 | 対テストの持ち主 |
//! |---|---|
//! | 整合待ち（経路 (a) の 2 段書込） | `frame_transition_branch_wait_tests.rs`（拡大率 120 と 192） |
//! | 作業領域変化を契機とする再スナップ | `frame_transition_branch_tests.rs` の [`a_work_area_only_change_writes_once`]（拡大率 96・120・192） |
//! | 作業領域源の実行時同期 | `frame_work_area_sync_tests.rs` |
//! | 再導出結果なしの窓（`None` 経路） | `frame_dpi_reproject_none_tests.rs`（単一フレーム） |
//! | 判定器の `reason=invisible`／`k-unchanged` の選り分け | `crates/areka-runtime/src/placement_tests/transition_judge_tests.rs`（純関数） |
//!
//! 本ファイルが持つのは、別モニタでの寸法追従・作業領域だけの変化・定常フレームの窓書込 0・
//! モニタ 0 台・見送られた窓の隣で他窓の遷移が続くことを、複数モニタの作業領域を注入した
//! 状態で走らせることである。複数モニタの観測条件そのものは `frame_transition_test_support.rs`
//! が 1 本で固定する。
//!
//! # 本番入口（`frame_system`）を通す檻を 1 本持つ
//!
//! ほかのテストはいずれも [`FrameHarness::run_placement_phases`] を駆動する。これは本番の相順を
//! **ハーネス側へ写した実装**であって `frame_system` を通らないので、本番の呼出が残った
//! まま**到達不能**になっても（`if false` で包む等）1 本も赤にならない。
//! `frame_work_area_resnap_tests.rs` の本文走査は呼出の**存在と前後関係**しか押さえない。
//!
//! [`the_production_frame_system_reaches_all_three_placement_call_sites`] がその引受先である。
//! 本番の相順所有者 `frame_system` をそのまま 2 フレーム回し、同期段・拡大率の相・作業
//! 領域再スナップの 3 呼出が**到達可能**であることを、それぞれ固有の観測（源の作り直し／
//! `origin=DpiReproject` の書込／`origin=WorkAreaResnap` の書込）で示す。
//!
//! # 零件の主張には陽性の対を同じ本体へ置く
//!
//! 本ファイルの主張は零件（「書込 0」「待ち札 0」「警告 0」）に多く偏る。駆動が死んでいれば
//! どれも空虚に緑になるので、**同じテスト本体の内側**で同じ駆動口が陽性側でも効くことを
//! 続けて主張する（同一表 → 変化ある表で書込／0 台 → 台が戻れば書込）。
//!
//! # 「現状維持」が指すのは**寸**である（本文の字句を狭めて読んでいる）
//!
//! **約束の本文は「当該窓の位置と寸を変更せずに」と書く**が、本ファイルはそれを**寸だけ**
//! と読む。窓が消える件を直したときの「再導出不能なら現状維持」は位置の権威と寸の権威の分離と
//! 対になっており、`refresh_scale` が `None`＝**寸不変**、**位置は独立に判断**、を指すからである。実装も
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
use wintf::ecs::{DPI, Point};

use areka_placement::WORK_AREA_SYNC_CONTEXT;
use areka_placement::diag::{PlacementRoute, WindowKind};
use areka_placement::follow::{BalloonFollow, MonitorDpiTable, MonitorSnapshot};
use log_capture_kit::capture;

use super::frame_system;
use super::shared_test_support::{BALLOON_SPAWN_SIZE, HIGH_DPI, LOW_DPI, SCALE_120};
use super::test_support::{
    FakeReports, FrameHarness, SPAWN_SIZE_0, SPAWN_SIZE_1, dpi_world, headless_wiring_with, pos_of,
    s2_assert_work_area_bottom_moves, s2_monitors, s2_monitors_with_work_area, s2_sources,
    s2_taskbar_hidden_work_area, s2_work_area_for_dpi, size_of, zero_clock,
};
use super::transition_test_support::{
    CHAR_ON_NEIGHBOR, as_window_size, assert_writes_per_window_within_bound, holds_of,
    move_scope_to, neighbor_at, scaled, settle_at, warnings_containing, writes_for,
};
use super::{balloon_target, shell_target};

/// 定常フレームを何コマ回して「窓書込 0」を主張するか。
const STEADY_FRAMES: u32 = 5;

/// 別モニタ検査で札の不在を確かめ続けるフレーム数。
const WAIT_FRAMES: u32 = 3;

/// どのモニタにも属さない位置（両モニタの**左**の外・y は両モニタの域内）。
///
/// 帰属は**中心**で決まる（`monitor_containing`／`work_area_for_window_with_origin`）。最近傍は
/// clamp 点との自乗距離で決まるので、ゴーストが居たモニタ（index 0）が最近傍になる側へ置く
/// ——隣接モニタ（右側・x が 2574 以上）より確実に近い左外へ出す。
const CHAR_OFF_ALL_MONITORS: Point = Point { x: -3000, y: 700 };

// ---------------------------------------------------------------------------
// 群 B: 別モニタへ移した窓
// ---------------------------------------------------------------------------

/// 別モニタへ移した窓（移動先の拡大率が表に既在）は**待たずに通り**、寸法が追従する。
///
/// 「待ち札 0」の陽性の対は同じ本体の内側——待たずに通ったからこそ書込が出て寸が変わる。
///
/// **⚠ 区分＝
/// 拡大率遷移での追従オフセットの追随に対して本檻は空振りであり、追随の証拠にならない。**
/// 本檻は待ち札・書込回数・**寸**の追従を主張するのみで、`BalloonFollow.offset` の値について
/// は何も主張しない（[`move_scope_to`] が読む `offset` は遷移を起こす**前**に二体を隣接モニタ
/// へ揃えて置くための配置用であり、主張の対象ではない）。遷移そのものは起きるが、追随が
/// 入っても出なくても本檻の主張は同じ値を取る。
///
/// 実測: 追随の適用（`frame::balloon_offset_follow::rescale_balloon_follow_offset`
/// の呼出）を外した走行で、本件は 120／192 とも**緑のまま**であった（同走行では追随の檻 13 件
/// が赤になる）。ゆえに**本件が緑であることを「追随を壊していない」の根拠に使ってはならない**。
///
/// なお本檻を「書込**後**に読んだ `offset` で恒等式を突き合わせる」群として数えていた
/// `research.md` §2.8 (b) の記述は、構造としては本檻に当たらない（恒等式すら主張しない）。
/// 結論（追随に対して空振り）は変わらないが、**行番号ではなく構造で見分けよ**という
/// 2026-08-28 の訂正の教訓がここにも当てはまる。
fn a_window_moved_to_another_monitor_never_waits_at(dpi: u16) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);

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
        "dpi={dpi}: 移動先の拡大率が表に既在なのに待ち札が付いた"
    );
    // 陽性の対: 待たずに通ったので書込が出ており、寸が追従している。
    let writes = harness.drain_writes();
    assert_writes_per_window_within_bound(
        &writes,
        0,
        WindowKind::Char,
        &format!("dpi={dpi}: 別モニタのキャラ窓"),
    );
    assert_eq!(
        size_of(&harness.world, harness.char_window(0)),
        Some(as_window_size(followed)),
        "dpi={dpi}: 別モニタへ移した窓の寸法が追従していない"
    );

    for _ in 0..WAIT_FRAMES {
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        assert_eq!(
            holds_of(&harness, 0),
            (false, false),
            "dpi={dpi}: 後続フレームで待ち札が付いた"
        );
    }
}

#[test]
fn a_window_moved_to_another_monitor_never_waits_at_120() {
    a_window_moved_to_another_monitor_never_waits_at(SCALE_120);
}

#[test]
fn a_window_moved_to_another_monitor_never_waits_at_192() {
    a_window_moved_to_another_monitor_never_waits_at(HIGH_DPI);
}

// ---------------------------------------------------------------------------
// 群 C: 作業領域の分岐
// ---------------------------------------------------------------------------

/// 作業領域だけが変わったフレーム（拡大率は据え置き）で、下端吸着のキャラ窓が
/// `WorkAreaResnap` の 1 書込で新しい下端へ移り、随伴バルーンが**同一フレーム**で追従する。
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
                .offset();
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
            assert_writes_per_window_within_bound(
                &writes,
                scope32,
                kind,
                &format!("dpi={dpi} scope={scope} {kind}"),
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

    // 随伴は同一フレームで窓相対へ移り、追従 offset は 1 bit も変わらない。
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
            .offset();
        assert_eq!(
            (after.x, after.y),
            (*offset_x, *offset_y),
            "dpi={dpi} scope={scope}: 追従オフセットを補正している（補正しない）"
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

#[test]
fn a_work_area_only_change_writes_once() {
    for dpi in [LOW_DPI, SCALE_120, HIGH_DPI] {
        a_work_area_only_change_writes_once_at(dpi);
    }
}

/// 同一表のフレームでは源を作り直さず、遷移を伴わない定常フレームでは窓書込が
/// **1 件も**出ない。
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
            "dpi={dpi}: 同一表のフレームで源を作り直している（frame={}）",
            harness.frame()
        );
        let writes = harness.drain_writes();
        assert!(
            writes.is_empty(),
            "dpi={dpi}: 定常フレームで窓書込が出ている（frame={}）: {writes:?}",
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

#[test]
fn steady_frames_write_nothing() {
    for dpi in [LOW_DPI, SCALE_120, HIGH_DPI] {
        steady_frames_write_nothing_at(dpi);
    }
}

/// モニタ 0 台（列挙異常）では源を差し替えず、窓を 1 枚も動かさず、警告を残す。
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
    let (change, events) = capture(|| harness.run_placement_phases(&mut source));

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
    let (change, events) = capture(|| harness.run_placement_phases(&mut source));
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

#[test]
fn an_empty_monitor_table_keeps_everything_and_warns() {
    for dpi in [LOW_DPI, SCALE_120, HIGH_DPI] {
        an_empty_monitor_table_keeps_everything_and_warns_at(dpi);
    }
}

/// **「帰属不能」は「解決できない」ではない**（開発者裁定 2026-08-20）。
///
/// どのモニタにも中心が乗らない窓は最近傍で**解決される**——現状維持で画面外に取り残さない。
/// 判断の軸は「ゴーストが触れなくなる事態を避けること」であり、モニタ 0 台のときだけが 5.5 の
/// 現状維持である。両者を**同じ本体**で対にして、混同していないことを実行で示す。
#[test]
fn a_window_off_all_monitors_is_resolved_by_the_nearest_fallback_not_left_in_place() {
    let dpi = HIGH_DPI;
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
    assert_writes_per_window_within_bound(
        &writes,
        0,
        WindowKind::Char,
        "帰属不能な窓が現状維持で放置されている（現状維持の適用対象と取り違えている）",
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
    let (change, _events) = capture(|| harness.run_placement_phases(&mut source));
    assert!(change.is_none(), "モニタ 0 台で源を差し替えている");
    assert!(
        harness.drain_writes().is_empty(),
        "モニタ 0 台で窓書込が出ている（現状維持でない）"
    );
    assert_eq!(
        harness.ground_point(0),
        held_ground,
        "モニタ 0 台で接地点が動いた（現状維持でない）"
    );
}

// ---------------------------------------------------------------------------
// 群 D: 遷移時点で再導出結果が得られない窓
// ---------------------------------------------------------------------------

/// 遷移時点で再導出結果が得られない窓（不可視・未表示）は**窓寸**を変えず、その隣で
/// **他窓の遷移は継続する**（窓が消える件を直したときの挙動を変えない）。
///
/// 「寸が動かない」の陽性の対は同じ本体の内側——報告を与えた scope 1 は寸が動き、接地点が
/// 新しい下端へ載り、1 回で書かれる。片方の欠落が他方を巻き込まないことがここの主題である。
///
/// 位置の側は現寸のまま射影 T を一度通る。その単一フレームの形は
/// `frame_dpi_reproject_none_tests.rs` が持つので、ここでは遷移の文脈で接地点規約が保たれる
/// ことだけを問う。
fn a_window_without_a_re_derivation_result_keeps_its_size_at(dpi: u16) {
    s2_assert_work_area_bottom_moves(LOW_DPI, dpi);

    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle_at(&mut harness, &mut source, LOW_DPI);

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

    // 見送られた窓の**寸**は 1 bit も動かない。
    assert_eq!(
        size_of(&harness.world, harness.char_window(0)),
        Some(kept_char),
        "dpi={dpi}: 再導出結果の無いキャラ窓の寸が動いた（現状維持でない）"
    );
    assert_eq!(
        size_of(&harness.world, harness.balloon_window(0)),
        Some(kept_balloon),
        "dpi={dpi}: 再導出結果の無いバルーン窓の寸が動いた（現状維持でない）"
    );

    // 陽性の対: 他窓の遷移は継続する。
    assert_eq!(
        size_of(&harness.world, harness.char_window(1)),
        Some(as_window_size(followed)),
        "dpi={dpi}: 報告のある scope 1 の寸が追従していない（見送りが他窓を巻き込んでいる）"
    );
    assert_writes_per_window_within_bound(
        &writes,
        1,
        WindowKind::Char,
        &format!("dpi={dpi}: scope 1 のキャラ窓"),
    );

    // 接地点規約は両スコープで保たれる——見送られた窓も浮かない。
    let new_bottom = s2_work_area_for_dpi(dpi).bottom;
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            harness.ground_point(scope).1,
            new_bottom,
            "dpi={dpi} scope={scope}: 遷移後の接地点が新しい作業領域下端から外れている"
        );
    }
}

#[test]
fn a_window_without_a_re_derivation_result_keeps_its_size_at_120() {
    a_window_without_a_re_derivation_result_keeps_its_size_at(SCALE_120);
}

#[test]
fn a_window_without_a_re_derivation_result_keeps_its_size_at_192() {
    a_window_without_a_re_derivation_result_keeps_its_size_at(HIGH_DPI);
}

// ---------------------------------------------------------------------------
// 本番入口（`frame_system`）の到達可能性（レビューが名指しした申し送り）
// ---------------------------------------------------------------------------

/// 本番の相順所有者 [`frame_system`] をそのまま回し、配置に関わる 3 呼出——同期段
/// （`work_area_sync::sync_monitor_snapshot`）・拡大率の相（`run_dpi_phase`）・作業領域再スナップ
/// （`work_area_sync::resnap_for_work_area_change`）——が**到達可能**であることを固定する。
///
/// # なぜこの 1 本が要るのか
///
/// 本ファイルのほかのテストと、兄弟の `frame_transition_branch_wait_tests.rs`／
/// `frame_work_area_sync_tests.rs`／`frame_work_area_resnap_tests.rs`／
/// `frame_dpi_sync_hold_tests.rs` は、本文の走査のテストを除き
/// [`FrameHarness::run_placement_phases`] を駆動する。あれは本番の相順を**ハーネス側へ写した
/// 実装**であって [`frame_system`] を通らない。ゆえに本番の呼出を残したまま `if false` で
/// 包んで**到達不能**にしても、挙動テストは 1 本も赤にならない（レビューが実演で
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
/// べき等 skip で抜ける。分けると、どちらの経路が書いたのかが経路語で一意に読める。
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
/// **位置だけ**を現寸で射影し直すので、窓書込は出る。GPU も実 fixture も要らない。
#[test]
fn the_production_frame_system_reaches_all_three_placement_call_sites() {
    let dpi = HIGH_DPI;
    // 前のテストが残した窓書込指令を捨てる（**実行はしない**）。
    let _residue = drain_window_pos_commands();

    let stale = s2_sources(LOW_DPI);
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
    world.insert_non_send(headless_wiring_with(rx, zero_clock()));

    // ── フレーム A: 本番入口をそのまま 1 度回す ──────────────────────────
    frame_system(&mut world);
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
    frame_system(&mut world);
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

    // 次に走るのがハーネスでない場合に備えて末尾でも閉じる。
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
