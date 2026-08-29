//! **収束の保証**（areka-P0-balloon-offset-dpi・task 6.2・design D16・要件 3.1／3.4）の
//! 決定論テスト。
//!
//! # ここが押さえる欠陥
//!
//! 拡大率遷移で追従オフセットが新しい値へ追随しても、続く窓書込が
//! [`resize_window_to`](crate::placement::follow::resize_window_to) の**手順 4 のべき等
//! skip**（位置と寸がともに同一）で抜けると、手順 6 の随伴追従に到達しない。すると
//! 「オフセットは直ったのにバルーンは次に何かが動くまで古い位置に居る」——本仕様が
//! 消しに来た欠陥そのものが残る。`dpi_phase_with` は追随の戻り値と反映の戻り値を
//! 突き合わせ、その腕でだけ随伴追従を 1 度呼ぶ（D16）。
//!
//! # べき等 skip の状況をどう作るか（探針の作り方）
//!
//! **モニタ矩形は物理量ゆえ拡大率で動かないが、作業領域はタスクバーの物理高ぶん動く**
//! ——これが [`s2_work_area_for_dpi`] の機構である。ゆえに素朴に拡大率を上げると作業
//! 領域下端が動き、射影 T が新しい Y を出して**窓書込が起きてしまう**（skip 腕に入らない）。
//!
//! そこで [`s2_monitors_with_work_area`] で「拡大率だけ [`HIGH_DPI`]・作業領域は
//! [`LOW_DPI`] のまま」のモニタ表を組む。実機で言えば、タスクバーの表示設定などの都合で
//! 作業領域が偶然変わらないまま拡大率だけが変わった遷移である。このとき
//!
//! - 整合ゲートは通る（窓の拡大率＝表の拡大率＝[`HIGH_DPI`]）、
//! - 追随は基準 DPI [`LOW_DPI`] → 現在 [`HIGH_DPI`] で**オフセットを動かす**、
//! - 射影 T は同じ作業領域・同じ寸から**同じ位置**を出す＝べき等 skip、
//!
//! が同時に成立する。[`the_transition_really_takes_the_idempotent_skip_path`] が
//! 「本当に skip 腕へ入っているか」を反映口の戻り値そのもので固定する——探針が別の腕を
//! 通っていたら、以降の主張はすべて空虚になる。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 「キャラ書込 0」を主張するので、同じ駆動口が陽性側でも効くことを
//! [`the_normal_arm_still_writes_the_character_once_and_the_balloon_once`] が固定する。
//! また [`no_offset_change_means_no_convergence_write`] は「skip 腕でも**オフセットが
//! 動いていなければ**書かない」＝ D16 の 2 条件のうち `Changed` 側が効いていることを
//! 固定する（この 1 本が無いと、skip 腕で無条件に追従を呼ぶ実装が緑になる）。

use wintf::ecs::WindowPos;
use wintf::ecs::window::SetWindowPosCommand;

use crate::placement::diag::PlacementRoute;
use crate::placement::follow::BalloonFollow;
use crate::placement::resolver::PointPx;

use super::test_support::{
    FakeReports, FrameHarness, s2_monitors_with_work_area, s2_work_area_for_dpi,
};
use super::*;

/// 遷移前の拡大率水準（作者基準と等倍）。
const LOW_DPI: u16 = 96;

/// 遷移後の拡大率水準（2 倍＝オフセットも 2 倍になる）。
const HIGH_DPI: u16 = 192;

/// 指定スコープ・指定種別の窓書込だけを取り出す（`frame_dpi_sync_hold_tests` と同型）。
fn writes_for(writes: &[SetWindowPosCommand], scope: u32, kind: &str) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind)
        .cloned()
        .collect()
}

/// 当該スコープの追従オフセット（現在値）。
fn offset_of(harness: &FrameHarness, scope: usize) -> PointPx {
    harness
        .world
        .get::<BalloonFollow>(harness.char_window(scope))
        .expect("キャラ窓に BalloonFollow がある")
        .offset()
}

/// 起動直後の整地——3 つの源と窓の拡大率を [`LOW_DPI`] へ揃え、拡大率の相の初回全窓
/// マッチ（永続 `SystemState` の仕様）を空回しして消費する。
///
/// この 1 巡で未係留の基準（`OffsetBase::unpinned`）が [`LOW_DPI`] へ**係留**される
/// （要件 5.2）——係留が済んでいないと次の遷移は係留の腕へ落ち、追随が起きない。
fn settle(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(LOW_DPI);
    harness.set_monitor_table_for_dpi(LOW_DPI);
    harness.set_window_dpi(LOW_DPI);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert!(
            harness
                .world
                .get::<BalloonFollow>(harness.char_window(scope))
                .expect("キャラ窓に BalloonFollow がある")
                .base()
                .dpi
                .is_some(),
            "scope={scope}: 整地で基準が係留されていない（探針が退化している）"
        );
    }
}

/// **拡大率だけ**を [`HIGH_DPI`] へ上げる（作業領域は [`LOW_DPI`] のまま据え置く）。
fn raise_scale_without_moving_the_work_area(harness: &mut FrameHarness) {
    harness.set_monitor_table(s2_monitors_with_work_area(
        HIGH_DPI,
        s2_work_area_for_dpi(LOW_DPI),
    ));
    harness.set_window_dpi(HIGH_DPI);
}

/// **探針の非退化検査**: この構成では反映口が本当にべき等 skip（`false`）で抜ける。
///
/// 反映口の戻り値そのものを問う——`dpi_phase_with` が D16 の条件に使うのはまさにこの
/// 値であり、ここが `true` なら以降のテストは「skip 腕」を一度も通っていない。
#[test]
fn the_transition_really_takes_the_idempotent_skip_path() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    raise_scale_without_moving_the_work_area(&mut harness);
    harness.advance_frame();
    // 作業領域源だけ先に更新する（拡大率の相と同じ順＝相の中で見えるのと同一の源）。
    harness.run_work_area_sync();

    for scope in harness.scopes().to_vec() {
        let char_window = harness.char_window(scope);
        let wrote = reproject_char_window_at_current_size(
            &mut harness.world,
            char_window,
            PlacementRoute::DpiReproject,
        );
        assert!(
            !wrote,
            "scope={scope}: 位置と寸が同一なのに窓書込が起きた（べき等 skip 腕へ入っていない＝探針の退化）"
        );
    }
    let writes = harness.drain_writes();
    assert!(
        writes.is_empty(),
        "べき等 skip のはずが窓書込指令が積まれている: {writes:?}"
    );
}

/// **完了条件**（要件 3.1／3.4・D16）: 早期に抜ける状況で、バルーンが同一フレームで
/// 新しいオフセットの位置へ **1 度だけ**書かれ、書込回数がキャラ 0・バルーン 1 になる。
#[test]
fn an_idempotent_skip_converges_the_balloon_in_exactly_one_write() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let before: Vec<(usize, PointPx)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| (scope, offset_of(&harness, scope)))
        .collect();

    raise_scale_without_moving_the_work_area(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();

    for (scope, old_offset) in before {
        let scope_u32 = u32::try_from(scope).expect("scope は u32 域");
        let new_offset = offset_of(&harness, scope);
        assert_ne!(
            new_offset, old_offset,
            "scope={scope}: 追随が起きていない（探針の退化——収束の主張が空虚になる）"
        );

        let char_writes = writes_for(&writes, scope_u32, "char");
        assert!(
            char_writes.is_empty(),
            "scope={scope}: べき等 skip の腕でキャラ窓が書かれている（予算＝キャラ 0）: {char_writes:?}"
        );

        let balloon_writes = writes_for(&writes, scope_u32, "balloon");
        assert_eq!(
            balloon_writes.len(),
            1,
            "scope={scope}: バルーンの収束が 1 度きりでない（予算＝バルーン 1・中間位置の禁止）: {balloon_writes:?}"
        );

        // 収束先は「確定済みキャラ窓位置 ＋ **新しい** offset」——1 度の書込で最終位置へ
        // 行っている（古い offset で書いてから直す＝中間位置の提示、が起きていない）。
        let char_pos = harness
            .world
            .get::<WindowPos>(harness.char_window(scope))
            .and_then(|wp| wp.position)
            .expect("キャラ窓に位置がある");
        let cmd = &balloon_writes[0];
        assert_eq!(
            (cmd.x, cmd.y),
            (char_pos.x + new_offset.x, char_pos.y + new_offset.y),
            "scope={scope}: バルーンが新しい offset の位置へ収束していない（旧 offset {old_offset:?} の中間位置か）"
        );
        assert_eq!(
            cmd.tag.origin,
            PlacementRoute::BalloonFollow.as_str(),
            "scope={scope}: 収束の書込が随伴追従の経路語で記録されていない"
        );
    }

    // 別経路の書込は 0（予算の 3 つ目）。
    let other: Vec<&SetWindowPosCommand> = writes
        .iter()
        .filter(|cmd| cmd.tag.kind != "char" && cmd.tag.kind != "balloon")
        .collect();
    assert!(
        other.is_empty(),
        "ゴースト窓以外への書込が出ている（予算＝別経路 0）: {other:?}"
    );
}

/// **陽性の対**: 作業領域も動く通常の遷移では、従来どおりキャラ 1・バルーン 1 で
/// 落ち着く——予算（要件 3.4）が通常腕でも保たれることを固定する。
///
/// ⚠ 本テストは「収束が二重書込を足していないこと」の檻**ではない**。D16 の
/// `!wrote` 条件を落としても本テストは緑のまま通る——通常腕ではバルーンが既に
/// 目的座標に居るため、余分な `follow_balloon` が `enqueue_window_set_pos` へ
/// 指令を積まず件数が動かないからである。二重書込を実際に捕まえるのは
/// `frame_dpi_reproject_none_tests.rs` の
/// `s2_none_report_path_reprojects_position_without_touching_size`（同座標 2 行で赤）。
/// 帰属をここへ書き違えると「本テストが緑だから二重書込は無い」という誤った
/// 安心の根拠に使われる。
#[test]
fn the_normal_arm_still_writes_the_character_once_and_the_balloon_once() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    assert_ne!(
        s2_work_area_for_dpi(LOW_DPI).bottom,
        s2_work_area_for_dpi(HIGH_DPI).bottom,
        "探針が退化している: 2 水準で作業領域下端が動かない（通常腕を作れない）"
    );
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();

    for scope in harness.scopes().to_vec() {
        let scope_u32 = u32::try_from(scope).expect("scope は u32 域");
        assert_eq!(
            writes_for(&writes, scope_u32, "char").len(),
            1,
            "scope={scope}: 通常腕のキャラ窓書込が 1 回でない（予算＝キャラ ≤1）: {writes:?}"
        );
        assert_eq!(
            writes_for(&writes, scope_u32, "balloon").len(),
            1,
            "scope={scope}: 通常腕のバルーン書込が 1 回でない（予算＝バルーン ≤1）: {writes:?}"
        );
    }
}

/// **D16 の 2 条件のうち追随側**: べき等 skip でも、オフセットが動いていなければ
/// 収束は走らない（冗長な書込を出さない・要件 3.2 の据置きと整合）。
#[test]
fn no_offset_change_means_no_convergence_write() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let before: Vec<PointPx> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| offset_of(&harness, scope))
        .collect();

    // 同じ拡大率を**もう一度**書き込む＝変化は立つが基準 DPI と現在 DPI は同一。
    harness.set_window_dpi(LOW_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();

    for (scope, old_offset) in harness.scopes().to_vec().into_iter().zip(before) {
        assert_eq!(
            offset_of(&harness, scope),
            old_offset,
            "scope={scope}: 拡大率が変わらないのに offset が動いた（要件 3.2）"
        );
    }
    assert!(
        writes.is_empty(),
        "オフセットが動いていないのに収束の書込が出ている（冗長な書込）: {writes:?}"
    );
}
