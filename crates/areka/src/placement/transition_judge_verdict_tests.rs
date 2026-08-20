//! `transition_judge` の**上限と合否**の決定論テスト（task 3.2）。
//!
//! 押さえるのは 3 点である:
//!
//! 1. 上限が **2 系統に分かれている**こと——決定論の上限（[`Bounds::deterministic`]）は実機専用の
//!    量を 1 つも見ず、実機専用の上限（[`Bounds::signoff`]）は決定論の量を 1 つも見ない。両者は
//!    別々の呼び出しで評価され、片方の結果がもう片方を動かさない（C7・設計討議 A-2）。
//! 2. 違反が**列**で返ること——1 件目で打ち切らない。
//! 3. **沈黙を合格にしない**こと——量が無いことは合格の根拠にならない。この主題の負例は
//!    姉妹モジュール `transition_judge_negative_tests` が持つ（本モジュールは上限そのものの
//!    分岐を持つ）。
//!
//! # 実機専用の上限の**値**は固定しない
//!
//! C7 は `visualize_to_write_us`／`flush_total_us` を「非決定なので回帰テストでは固定しない」と
//! 定める。task 4.3 が値を確定させた後も同じで、根拠は確定台帳 L9 と定数の doc が持つ。よって
//! 本モジュールは上限の数字を 1 度も書かず、[`Bounds`] が持つ値を読んでその ±1 で分岐を作る。

use areka_emo_present::presenter::{
    SURFACE_REASON_INVISIBLE, SURFACE_STAGE_SKIPPED, SURFACE_STAGE_VISUALIZE,
};
use wintf::ecs::window::transition_diag::{
    FIELD_SCOPE, FIELD_T_US, FIELD_WIN_KIND, MISSING, STAGE_BEGIN, STAGE_END, STAGE_FLUSH,
};

use super::super::diag::{PlacementRoute, WindowKind};
use super::test_support::{
    COMPLIANT_FRAME, compliant_transition_lines, flush, ground, monitor, replace_once,
    summarize_lines, surface, write,
};
use super::transition_judge_reobservation_tests::REOBSERVATION_TRANSITION_1;
use super::{
    Bounds, FLUSH_TOTAL_US_MAX, Quantity, Report, TransitionVerdict, VISUALIZE_TO_WRITE_US_MAX,
    Violation, WindowKey, judge, judge_transition_log,
};

/// 対照（上限を 1 つも破らない遷移）の判定量。
fn compliant() -> super::TransitionSummary {
    summarize_lines(&compliant_transition_lines())
}

/// 違反の列（`Ok` なら空でなく落とす＝「合格だった」を見逃さない）。
fn violations(summary: &super::TransitionSummary, bounds: &Bounds) -> Vec<Violation> {
    judge(summary, bounds).expect_err("違反があるはず")
}

// ---------------------------------------------------------------------------
// 2 系統の分離
// ---------------------------------------------------------------------------

#[test]
fn the_deterministic_bounds_arm_only_the_deterministic_quantities() {
    let bounds = Bounds::deterministic();
    assert_eq!(bounds.frame_bound, Some(super::TRANSITION_FRAME_BOUND));
    assert_eq!(bounds.hold_frame_allowance, super::HOLD_FRAME_ALLOWANCE);
    assert_eq!(
        bounds.writes_per_window_max,
        Some(super::WRITES_PER_WINDOW_MAX)
    );
    assert_eq!(bounds.path_a_writes_max, Some(super::PATH_A_WRITES_MAX));
    assert_eq!(bounds.ground_diff_abs_max, Some(super::GROUND_DIFF_MAX));
    assert_eq!(
        bounds.chain_realigned_max,
        Some(super::CHAIN_REALIGN_PER_TRANSITION)
    );
    // 非決定量は 1 つも armed でない（回帰テストが実時間で赤くなる形を作らない）。
    assert_eq!(bounds.visualize_to_write_us_max, None);
    assert_eq!(bounds.flush_total_us_max, None);
}

#[test]
fn the_deterministic_constants_hold_the_values_c7_fixes() {
    // 決定論の上限は「回帰テストが固定する」量である（C7）。他の檻は定数を**記号のまま**
    // 使って ±1 の分岐を作るので、値そのものを動かす退行はここでしか赤くならない。
    assert_eq!(super::TRANSITION_FRAME_BOUND, 0);
    assert_eq!(super::WRITES_PER_WINDOW_MAX, 1);
    assert_eq!(super::PATH_A_WRITES_MAX, 0);
    assert_eq!(super::GROUND_DIFF_MAX, 0);
    assert_eq!(super::CHAIN_REALIGN_PER_TRANSITION, 1);
    // 待ちの許容だけは**値を固定しない**——正本は本番の整合ゲートの上限であり、ここで値を
    // 二度書くと task 5.4 が解消した二重定義が檻の側で復活する。固定するのは結線である。
    assert_eq!(
        super::HOLD_FRAME_ALLOWANCE,
        crate::placement::dpi_sync::DPI_SYNC_HOLD_MAX_FRAMES
    );
}

#[test]
fn the_signoff_bounds_arm_only_the_real_machine_quantities() {
    let bounds = Bounds::signoff();
    // 固定するのは**結線**（当該の定数から引いていること）であって値ではない。task 4.3 が
    // 定数を確定値へ差し替えても、この檻は書き換えずに緑のまま追随した。
    assert_eq!(
        bounds.visualize_to_write_us_max,
        Some(VISUALIZE_TO_WRITE_US_MAX)
    );
    assert_eq!(bounds.flush_total_us_max, Some(FLUSH_TOTAL_US_MAX));
    assert_eq!(bounds.frame_bound, None);
    assert_eq!(bounds.writes_per_window_max, None);
    assert_eq!(bounds.path_a_writes_max, None);
    assert_eq!(bounds.ground_diff_abs_max, None);
    assert_eq!(bounds.chain_realigned_max, None);
}

#[test]
fn the_signoff_bounds_are_positive_but_their_values_are_not_fixed_here() {
    // 値そのものは檻に書かない（C7: 非決定量は回帰テストで固定しない）。task 4.3 が
    // 確定させた後も同じで、上限の根拠は確定台帳 L9 と定数の doc が持つ——ここへ値を
    // 写すと、根拠を持たない 2 つ目の正本ができる。
    // 固定してよいのは「0 でないこと」だけ——0 の上限はあらゆる遷移を違反にし、判定を
    // 「常に赤」へ倒して合否の情報を消す。
    let bounds = Bounds::signoff();
    assert!(bounds.visualize_to_write_us_max.is_some_and(|max| max > 0));
    assert!(bounds.flush_total_us_max.is_some_and(|max| max > 0));
}

#[test]
fn a_bounds_that_arms_nothing_is_never_a_pass() {
    // 「何も当てなかったから合格」を作らせない（沈黙を PASS にしない）。
    let outcome = judge(&compliant(), &Bounds::nothing());
    assert_eq!(outcome, Err(vec![Violation::NoBoundsArmed]));
}

#[test]
fn a_slice_that_does_not_start_at_an_origin_is_never_a_pass() {
    // 起点の無い区間は「何も起きなかった遷移」ではなく**遷移として読めていない**入力である。
    // 上限は 1 つも破られないので、起点の欠落そのものを違反にしないと合格に化ける。
    let summary = super::summarize(&[]);
    assert!(summary.origin.is_none());
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![
            Violation::MissingOrigin,
            Violation::Unmeasured(Quantity::FramesToLastWrite),
            Violation::Unmeasured(Quantity::GroundDiff),
        ]
    );
}

// ---------------------------------------------------------------------------
// 対照（上限を 1 つも破らない遷移）
// ---------------------------------------------------------------------------

#[test]
fn the_compliant_transition_passes_both_families() {
    // これが赤くなると、以下の負例は「壊したから赤い」のか「元から赤い」のか区別できない。
    let summary = compliant();
    assert_eq!(judge(&summary, &Bounds::deterministic()), Ok(()));
    assert_eq!(judge(&summary, &Bounds::signoff()), Ok(()));
}

// ---------------------------------------------------------------------------
// 決定論の上限の各分岐
// ---------------------------------------------------------------------------

#[test]
fn a_second_write_to_one_window_violates_the_per_window_bound() {
    let mut summary = compliant();
    let window = WindowKey::of(0, WindowKind::Balloon);
    summary.writes += 1;
    *summary
        .writes_per_window
        .get_mut(&window)
        .expect("居るはず") += 1;
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::WritesPerWindow {
            window,
            writes: 2,
            max: super::WRITES_PER_WINDOW_MAX,
        }]
    );
}

#[test]
fn a_path_a_write_violates_both_the_origin_count_and_the_sync_stage_backstop() {
    // 札（`origin`）と段（`stage=sync`）は別々に数え、別々に上限を当てる——片方だけに
    // 当てると、どちらかが壊れた退行が判定から静かに消える。
    let mut summary = compliant();
    summary.path_a_writes = 1;
    summary.sync_stage_writes = 2;
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![
            Violation::PathAWrites {
                writes: 1,
                max: super::PATH_A_WRITES_MAX,
            },
            Violation::SyncStageWrites {
                writes: 2,
                max: super::PATH_A_WRITES_MAX,
            },
        ]
    );
}

#[test]
fn a_nonzero_ground_difference_violates_the_bound_in_either_sign() {
    for diff in [-1, 1, -48] {
        let mut summary = compliant();
        summary.ground_diff_max = Some(diff);
        assert_eq!(
            violations(&summary, &Bounds::deterministic()),
            vec![Violation::GroundDiff {
                diff,
                max: super::GROUND_DIFF_MAX,
            }],
            "接地点差 {diff} は符号によらず違反"
        );
    }
}

#[test]
fn a_second_chain_realignment_in_one_transition_violates_the_bound() {
    let mut summary = compliant();
    summary.chain_realigned = super::CHAIN_REALIGN_PER_TRANSITION;
    assert_eq!(
        judge(&summary, &Bounds::deterministic()),
        Ok(()),
        "一度だけの解き直しは上限ちょうどで合格"
    );
    summary.chain_realigned += 1;
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::ChainRealigned {
            realigned: super::CHAIN_REALIGN_PER_TRANSITION + 1,
            max: super::CHAIN_REALIGN_PER_TRANSITION,
        }]
    );
}

#[test]
fn a_later_frame_for_the_last_write_violates_the_frame_bound() {
    let mut summary = compliant();
    summary.frames_to_last_write = Some(1);
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::FramesToLastWrite {
            frames: 1,
            max: super::TRANSITION_FRAME_BOUND,
        }]
    );
}

#[test]
fn a_transition_with_holds_may_take_the_hold_allowance_but_not_one_frame_more() {
    let allowance = super::TRANSITION_FRAME_BOUND + super::HOLD_FRAME_ALLOWANCE;
    let mut summary = compliant();
    summary.holds = 1;
    summary.frames_to_last_write = Some(allowance);
    assert_eq!(
        judge(&summary, &Bounds::deterministic()),
        Ok(()),
        "整合待ちを含む遷移は許容ぶんだけ遅れてよい（C7）"
    );
    summary.frames_to_last_write = Some(allowance + 1);
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::FramesToLastWrite {
            frames: allowance + 1,
            max: allowance,
        }]
    );
}

#[test]
fn the_hold_allowance_does_not_apply_to_a_transition_without_holds() {
    let mut summary = compliant();
    summary.holds = 0;
    summary.frames_to_last_write = Some(super::TRANSITION_FRAME_BOUND + 1);
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::FramesToLastWrite {
            frames: super::TRANSITION_FRAME_BOUND + 1,
            max: super::TRANSITION_FRAME_BOUND,
        }],
        "待ち札が 1 件も無い遷移に待ちの許容を足さない"
    );
}

#[test]
fn a_window_whose_content_and_rectangle_land_in_different_frames_violates_the_bound() {
    let mut summary = compliant();
    let window = WindowKey::of(0, WindowKind::Char);
    summary.mismatch_frames_per_window.insert(window.clone(), 3);
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::MismatchFrames {
            window,
            frames: 3,
            max: super::TRANSITION_FRAME_BOUND,
        }]
    );
}

#[test]
fn a_balloon_written_in_another_frame_than_its_character_violates_the_bound() {
    let mut summary = compliant();
    summary.balloon_same_frame = false;
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::BalloonWrittenInAnotherFrame]
    );
}

#[test]
fn an_unchecked_balloon_pair_is_reported_instead_of_passing_vacuously() {
    // `balloon_same_frame` の既定は `true` なので、検査できたはずの対が 1 組も数えられて
    // いないときにそのまま合格にすると「随伴を 1 度も見ていない」が合格に化ける。判定側は
    // 書込のあった窓と見送り窓から**独立に**検査できるはずの対を数え直して突き合わせる。
    let mut summary = compliant();
    summary.balloon_pairs_checked = 0;
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::Unmeasured(Quantity::BalloonSameFrame(
            WindowKey::of(0, WindowKind::Char)
        ))]
    );
}

#[test]
fn violations_come_back_as_a_list_not_one_at_a_time() {
    let mut summary = compliant();
    summary.path_a_writes = 1;
    summary.ground_diff_max = Some(-48);
    summary.chain_realigned = 9;
    let violations = violations(&summary, &Bounds::deterministic());
    assert_eq!(violations.len(), 3, "3 件とも列に載る: {violations:?}");
}

/// scope を 1 つだけ持ち、**その唯一のバルーンの再表示が見送られた**遷移。
///
/// 実機サインオフ（C10 の手順＝クリックもドラッグもせず拡大率だけ変える）では、発話していない
/// バルーン窓は不可視であり `areka-emo-present` の再表示は `stage=skipped reason=invisible` を
/// 出す（`crates/areka-emo-present/src/presenter/refresh.rs`）。つまりこれが**定常の形**である。
/// それでもバルーン窓は随伴の窓書込（`BalloonFollow`）を受ける——再観測 §3.1 の実ログと同じ。
fn one_scope_with_a_skipped_balloon(balloon_write_frame: u32) -> Vec<String> {
    vec![
        monitor(COMPLIANT_FRAME, 96, 192, 1752, 1704),
        surface(
            COMPLIANT_FRAME,
            1_000,
            SURFACE_STAGE_VISUALIZE,
            0,
            "764",
            "1094",
            MISSING,
            MISSING,
        ),
        surface(
            COMPLIANT_FRAME,
            1_050,
            SURFACE_STAGE_SKIPPED,
            1,
            MISSING,
            MISSING,
            MISSING,
            SURFACE_REASON_INVISIBLE,
        ),
        ground(
            COMPLIANT_FRAME,
            0,
            1704,
            1704,
            PlacementRoute::DpiReproject.as_str(),
        ),
        flush(COMPLIANT_FRAME, 1_200, STAGE_BEGIN, 2, MISSING),
        write(
            COMPLIANT_FRAME,
            1_300,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            WindowKind::Char.as_str(),
            500,
        ),
        write(
            balloon_write_frame,
            1_400,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            WindowKind::Balloon.as_str(),
            400,
        ),
        flush(COMPLIANT_FRAME, 1_500, STAGE_END, 2, "300"),
    ]
}

#[test]
fn an_invisible_balloon_that_still_follows_in_the_same_frame_is_a_pass() {
    // 見送りは**サーフェスの再表示**についての事実であって、窓書込の有無ではない。随伴が
    // 同一フレームで書かれている以上、要件 4.3 は満たされている——ここを違反にすると、
    // 発話していない定常状態（＝サインオフ手順のほぼ全域）で非欠陥が不合格になる。
    let summary = summarize_lines(&one_scope_with_a_skipped_balloon(COMPLIANT_FRAME));
    assert!(
        summary
            .skipped_windows
            .contains(&WindowKey::of(0, WindowKind::Balloon)),
        "バルーンは見送り窓として集計されている"
    );
    assert_eq!(judge(&summary, &Bounds::deterministic()), Ok(()));
    assert_eq!(judge(&summary, &Bounds::signoff()), Ok(()));
}

#[test]
fn an_invisible_balloon_that_follows_in_another_frame_is_still_caught() {
    // 逆向きの檻。見送りを理由に対の検査ごと降りると、**随伴が実際に遅れて動いた**という
    // 要件 4.3 の欠陥が観測されないまま合格になる。
    let summary = summarize_lines(&one_scope_with_a_skipped_balloon(COMPLIANT_FRAME + 1));
    let violations = violations(&summary, &Bounds::deterministic());
    assert!(
        violations.contains(&Violation::BalloonWrittenInAnotherFrame),
        "遅れた随伴を捕まえていない: {violations:?}"
    );
}

/// 再観測 §3.2 の**遷移 2** と同じ形（レポートが「欠陥ではない」と明記した唯一の外れ値）。
///
/// バルーン 1 は遷移時点で不可視ゆえ再表示を見送られたが、**随伴の位置書込（`BalloonFollow`）は
/// 他の窓と同じフレームで届き**、遅れたのは表示された時点の寸書込（`KeepPositionResize`）だけ
/// である（レポート §3.2 の注「+649ms に表示された時点で新寸へ（Requirement 4.6 の現状維持
/// 挙動＝欠陥ではない）」）。2 本の書込は**別の義務**を果たす——位置は要件 4.3 が同一フレームで
/// 求め、寸は要件 4.6 が先送りを許す。経路語で選り分けなければ、遅れた寸が随伴の遅れに化ける。
fn one_scope_with_a_deferred_resize(follow_frame: u32, resize_frame: u32) -> Vec<String> {
    vec![
        monitor(COMPLIANT_FRAME, 96, 192, 1752, 1704),
        surface(
            COMPLIANT_FRAME,
            1_000,
            SURFACE_STAGE_VISUALIZE,
            0,
            "764",
            "1094",
            MISSING,
            MISSING,
        ),
        surface(
            COMPLIANT_FRAME,
            1_050,
            SURFACE_STAGE_SKIPPED,
            1,
            MISSING,
            MISSING,
            MISSING,
            SURFACE_REASON_INVISIBLE,
        ),
        ground(
            COMPLIANT_FRAME,
            0,
            1704,
            1704,
            PlacementRoute::DpiReproject.as_str(),
        ),
        write(
            COMPLIANT_FRAME,
            1_300,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            WindowKind::Char.as_str(),
            500,
        ),
        // 随伴の位置（要件 4.3 が同一フレームで求める側）。
        write(
            follow_frame,
            1_400,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            WindowKind::Balloon.as_str(),
            400,
        ),
        // 見送られた寸（要件 4.6 が先送りを許す側）。
        write(
            resize_frame,
            1_500,
            STAGE_FLUSH,
            2,
            "0x2",
            PlacementRoute::KeepPositionResize.as_str(),
            "0",
            WindowKind::Balloon.as_str(),
            300,
        ),
        flush(COMPLIANT_FRAME, 1_600, STAGE_END, 3, "300"),
    ]
}

/// 見送られた寸書込が届くまでのフレーム差（+649ms ≒ 60Hz で 40 フレーム）。
const DEFERRED_RESIZE_FRAMES: u32 = 40;

#[test]
fn a_deferred_resize_after_a_punctual_follow_is_not_a_companion_defect() {
    // 再観測 §3.2 が「欠陥ではない」と書いた形を、判定器が欠陥と呼ばないこと。残るのは
    // バルーン窓の書込 2 回だけ——これは 6 遷移すべてに出る是正前の量（task 5.3 が 1 回へ
    // 落とす）であって、この遷移に固有の判定ではない。
    let summary = summarize_lines(&one_scope_with_a_deferred_resize(
        COMPLIANT_FRAME,
        COMPLIANT_FRAME + DEFERRED_RESIZE_FRAMES,
    ));
    assert_eq!(
        violations(&summary, &Bounds::deterministic()),
        vec![Violation::WritesPerWindow {
            window: WindowKey::of(0, WindowKind::Balloon),
            writes: 2,
            max: super::WRITES_PER_WINDOW_MAX,
        }]
    );
}

#[test]
fn a_late_follow_is_caught_even_when_the_resize_was_punctual() {
    // 2 本の経路を取り違えたら赤くなる側の檻。位置が遅れた（＝要件 4.3 の欠陥）ときは、
    // 寸が同一フレームで届いていても捕まえなければならない。
    let summary = summarize_lines(&one_scope_with_a_deferred_resize(
        COMPLIANT_FRAME + DEFERRED_RESIZE_FRAMES,
        COMPLIANT_FRAME,
    ));
    let violations = violations(&summary, &Bounds::deterministic());
    assert!(
        violations.contains(&Violation::BalloonWrittenInAnotherFrame),
        "遅れた随伴の位置を捕まえていない: {violations:?}"
    );
}

#[test]
fn a_deferred_write_to_a_skipped_window_does_not_extend_the_transition() {
    // 要件 4.6 の裁定（本 spec で確定）: 見送り窓への書込は遷移の所要（要件 4.4 の
    // 「全ゴースト窓の遷移」）に数えない。4.6 は当該窓を「変更せずに現状を維持」させる
    // 規定であり、後から表示された時点の書込は**その遷移の続き**ではないからである。
    // 数えると、再観測 §3.2 が非欠陥と明記した遷移 2 がフレーム上限で不合格になる。
    // 見送り窓への書込でも**随伴の位置**は別の量（`balloon_same_frame`）で見張り続ける。
    let summary = summarize_lines(&one_scope_with_a_deferred_resize(
        COMPLIANT_FRAME,
        COMPLIANT_FRAME + DEFERRED_RESIZE_FRAMES,
    ));
    assert_eq!(
        summary.frames_to_last_write,
        Some(0),
        "遅れた寸書込は見送り窓のものなので所要へ数えない"
    );
    assert_eq!(
        summary
            .writes_per_window
            .get(&WindowKey::of(0, WindowKind::Balloon)),
        Some(&2),
        "書込回数そのものは数える（除外するのは所要と可視化由来の量だけ）"
    );
}

#[test]
fn a_skipped_balloon_with_only_a_deferred_resize_is_left_out_of_the_verdict() {
    // 見送り窓に届いたのが**寸だけ**（`KeepPositionResize`）なら、随伴の位置は 1 度も
    // 書かれていない＝要件 4.6 の現状維持そのものであり、対を求めない。
    //
    // 「書込が在るか」で判定すると、この形が「対を検査できるはず」に化けて未測定の違反へ
    // 落ちる——2 本の書込が別の義務を果たすことを、判定側でも取り違えないための檻である。
    let lines: Vec<String> =
        one_scope_with_a_deferred_resize(COMPLIANT_FRAME, COMPLIANT_FRAME + DEFERRED_RESIZE_FRAMES)
            .into_iter()
            .filter(|line| !line.contains(PlacementRoute::BalloonFollow.as_str()))
            .collect();
    let summary = summarize_lines(&lines);
    let balloon = WindowKey::of(0, WindowKind::Balloon);
    assert!(summary.skipped_windows.contains(&balloon));
    assert_eq!(summary.writes_per_window.get(&balloon), Some(&1));
    assert!(
        !summary.balloon_follow_windows.contains(&balloon),
        "位置の書込は 1 度も無い"
    );
    assert_eq!(judge(&summary, &Bounds::deterministic()), Ok(()));
}

#[test]
fn a_skipped_balloon_that_is_never_written_is_left_out_of_the_verdict() {
    // 見送られたうえ窓書込も 1 度も無い＝現状維持そのもの（要件 4.6）。この対だけは求めない
    // ——求めると「動かさなかったこと」が違反に化ける。
    let lines: Vec<String> = one_scope_with_a_skipped_balloon(COMPLIANT_FRAME)
        .into_iter()
        .filter(|line| {
            !line.contains(&format!(
                "{FIELD_WIN_KIND}={}",
                WindowKind::Balloon.as_str()
            ))
        })
        .collect();
    let summary = summarize_lines(&lines);
    assert!(
        summary
            .skipped_windows
            .contains(&WindowKey::of(0, WindowKind::Balloon))
    );
    assert!(
        !summary
            .writes_per_window
            .contains_key(&WindowKey::of(0, WindowKind::Balloon))
    );
    assert_eq!(judge(&summary, &Bounds::deterministic()), Ok(()));
}

#[test]
fn a_character_written_without_its_companion_balloon_is_not_a_pass() {
    // バルーンが見送り窓でもないのに 1 度も書かれていないなら、随伴の同一フレーム性は
    // 「満たした」のではなく**測れていない**（要件 4.3 の判定対象が欠けている）。
    let without_balloon: Vec<String> = compliant_transition_lines()
        .into_iter()
        .filter(|line| {
            !line.contains(&format!(
                "{FIELD_WIN_KIND}={}",
                WindowKind::Balloon.as_str()
            ))
        })
        .collect();
    let summary = summarize_lines(&without_balloon);
    assert!(
        !summary
            .writes_per_window
            .contains_key(&WindowKey::of(0, WindowKind::Balloon))
    );
    assert!(
        violations(&summary, &Bounds::deterministic()).contains(&Violation::Unmeasured(
            Quantity::BalloonSameFrame(WindowKey::of(0, WindowKind::Char))
        )),
        "随伴を測れていないことを黙らせない"
    );
}

#[test]
fn a_window_whose_refresh_was_skipped_is_left_out_of_the_verdict() {
    // 要件 4.6: 寸の再導出結果が得られない窓は現状維持であり、合否から外す。可視化が無い
    // ぶん「量が欠けている」形になるので、除外しないと**見送りそのものが違反に化ける**。
    let mut lines = compliant_transition_lines();
    lines.push(surface(
        COMPLIANT_FRAME,
        1_150,
        SURFACE_STAGE_SKIPPED,
        2,
        MISSING,
        MISSING,
        MISSING,
        SURFACE_REASON_INVISIBLE,
    ));
    lines.push(write(
        COMPLIANT_FRAME,
        1_450,
        STAGE_FLUSH,
        2,
        "0x3",
        PlacementRoute::DpiReproject.as_str(),
        "1",
        WindowKind::Char.as_str(),
        300,
    ));
    let summary = summarize_lines(&lines);
    assert!(
        summary
            .skipped_windows
            .contains(&WindowKey::of(1, WindowKind::Char)),
        "見送り窓として集計されているはず"
    );
    assert_eq!(judge(&summary, &Bounds::deterministic()), Ok(()));
    assert_eq!(judge(&summary, &Bounds::signoff()), Ok(()));
}

// ---------------------------------------------------------------------------
// 実機専用の上限（別の呼び出し）
// ---------------------------------------------------------------------------

#[test]
fn the_signoff_bounds_flag_a_window_whose_visualize_to_write_gap_exceeds_the_limit() {
    let bounds = Bounds::signoff();
    let max = bounds.visualize_to_write_us_max.expect("armed のはず");
    let window = WindowKey::of(0, WindowKind::Char);
    let mut summary = compliant();
    summary.visualize_to_write_us.insert(window.clone(), max);
    assert_eq!(judge(&summary, &bounds), Ok(()), "上限ちょうどは合格");
    summary
        .visualize_to_write_us
        .insert(window.clone(), max + 1);
    assert_eq!(
        violations(&summary, &bounds),
        vec![Violation::VisualizeToWriteUs {
            window,
            us: max + 1,
            max,
        }]
    );
}

#[test]
fn the_signoff_bounds_flag_a_flush_interval_that_exceeds_the_limit() {
    let bounds = Bounds::signoff();
    let max = bounds.flush_total_us_max.expect("armed のはず");
    let mut summary = compliant();
    summary.flush_total_us_max = Some(max + 1);
    assert_eq!(
        violations(&summary, &bounds),
        vec![Violation::FlushTotalUs { us: max + 1, max }]
    );
}

#[test]
fn the_deterministic_family_ignores_the_real_machine_quantities_entirely() {
    // 実機量がどれだけ大きくても決定論判定は動かない（逆も同じ）＝2 系統が独立していること。
    let mut summary = compliant();
    summary.flush_total_us_max = Some(u64::MAX);
    for value in summary.visualize_to_write_us.values_mut() {
        *value = u64::MAX;
    }
    assert_eq!(judge(&summary, &Bounds::deterministic()), Ok(()));

    let mut summary = compliant();
    summary.path_a_writes = 99;
    summary.ground_diff_max = Some(-48);
    summary.frames_to_last_write = Some(u32::MAX);
    assert_eq!(judge(&summary, &Bounds::signoff()), Ok(()));
}

// ---------------------------------------------------------------------------
// 現行ログ（是正前）の合否
// ---------------------------------------------------------------------------

#[test]
fn the_current_log_violates_the_per_window_write_bound_before_the_coalescing_correction() {
    // **是正前の赤**（task 3.2 の観察可能な完了条件）。再観測 §3.2 のバルーン窓 2 回は
    // task 5.3（同一 tick・同一窓のジオメトリ指令の合流）が 1 回へ落とす。接地点差 −48px は
    // task 5.1／5.2（作業領域源の同期と再スナップ）が 0 へ落とす。
    let report = judge_transition_log(REOBSERVATION_TRANSITION_1);
    assert_eq!(report.transitions.len(), 1);
    let violations = report.transitions[0]
        .deterministic
        .clone()
        .expect_err("現行ログは決定論の上限を満たさない");
    assert_eq!(
        violations,
        vec![
            Violation::WritesPerWindow {
                window: WindowKey::of(0, WindowKind::Balloon),
                writes: 2,
                max: super::WRITES_PER_WINDOW_MAX,
            },
            Violation::WritesPerWindow {
                window: WindowKey::of(1, WindowKind::Balloon),
                writes: 2,
                max: super::WRITES_PER_WINDOW_MAX,
            },
            Violation::GroundDiff {
                diff: -48,
                max: super::GROUND_DIFF_MAX,
            },
        ]
    );
}

#[test]
fn the_current_log_is_judged_separately_by_the_real_machine_bounds() {
    // 同一 tick の内側の食い違いはフレーム単位の量では 0 なので、**別の呼び出し**でしか
    // 見えない（設計討議 A-2）。上限の**値**は固定せず、`Bounds` が持つ値と比べる。
    let bounds = Bounds::signoff();
    let visualize_max = bounds.visualize_to_write_us_max.expect("armed のはず");
    let flush_max = bounds.flush_total_us_max.expect("armed のはず");
    let report = judge_transition_log(REOBSERVATION_TRANSITION_1);
    let verdict: &TransitionVerdict = &report.transitions[0];
    let violations = verdict
        .signoff
        .clone()
        .expect_err("現行ログは実機専用の上限も満たさない");

    let over_windows: Vec<&WindowKey> = violations
        .iter()
        .filter_map(|violation| match violation {
            Violation::VisualizeToWriteUs { window, us, max } => {
                assert!(*us > *max && *max == visualize_max);
                Some(window)
            }
            _ => None,
        })
        .collect();
    assert_eq!(over_windows.len(), 4, "4 窓とも上限超: {violations:?}");
    assert!(violations.contains(&Violation::FlushTotalUs {
        us: 171_000,
        max: flush_max,
    }));

    // フレーム単位の量は是正前でも 0 なので、決定論側には実機量の違反が 1 件も混ざらない。
    let deterministic = verdict.deterministic.clone().expect_err("回数と接地点で赤");
    assert!(
        !deterministic.iter().any(|violation| matches!(
            violation,
            Violation::VisualizeToWriteUs { .. } | Violation::FlushTotalUs { .. }
        )),
        "決定論判定に実機量が混ざっている: {deterministic:?}"
    );
}

#[test]
fn the_report_carries_both_families_and_fails_the_log() {
    let report = judge_transition_log(REOBSERVATION_TRANSITION_1);
    assert_eq!(report.records, 29);
    assert_eq!(report.unassigned_records, 0);
    assert_eq!(report.unassigned_malformed_records, 0);
    assert!(report.log_violations().is_empty(), "ログ水準の違反は無い");
    assert!(report.failed(), "遷移の違反があれば不合格");

    let rendered = report.to_string();
    for needle in ["deterministic", "signoff"] {
        assert!(
            rendered.contains(needle),
            "{needle} が列挙に出ない: {rendered}"
        );
    }
}

#[test]
fn the_report_prints_the_quantities_even_when_both_families_pass() {
    // task 4.3 の裁定: 判定量は**合否によらず**刷る。違反の列挙だけだと `PASS` の系統の量が
    // 1 つも残らず、是正の前後を「量そのもの」で並べる側（基準値 §7・task 7.3）が生ログから
    // 手で起こす羽目になる（task 4.2 で実際に起きた）。
    //
    // 対照の遷移は 2 系統とも合格するので、違反の列挙は 1 行も出ない——ここで量が読めるなら
    // 「刷られているのは違反ではなく量である」ことが確かめられる。
    let report = judge_transition_log(&compliant_transition_lines().join("\n"));
    assert_eq!(report.transitions[0].deterministic, Ok(()));
    assert_eq!(report.transitions[0].signoff, Ok(()));
    assert!(!report.failed(), "対照は合格する");

    let rendered = report.to_string();
    // 手順書 §6.3 の 9 量が、合格の遷移でも字面で読めること。
    for needle in [
        "frames_to_last_write=",
        "path_a_writes=",
        "sync_stage_writes=",
        "balloon_same_frame=",
        "chain_realigned=",
        "ground_diff_max=",
        "flush_total_us_max=",
        "writes=",
        "mismatch_frames=",
        "visualize_to_write_us=",
        "量(見送り窓):",
    ] {
        assert!(
            rendered.contains(needle),
            "{needle} が刷られない:\n{rendered}"
        );
    }
    // 窓ごとの行は窓の鍵つきで出る（どの窓の量かが読めないと比較に使えない）。
    assert!(
        rendered.contains(&format!("{FIELD_SCOPE}=0 {FIELD_WIN_KIND}=char")),
        "窓ごとの量に窓の鍵が無い:\n{rendered}"
    );
}

#[test]
fn a_missing_quantity_is_printed_as_a_sentinel_not_as_a_zero() {
    // 「測っていない」と「測って 0 だった」は別の事実である。空欄や 0 で刷ると、是正後の
    // 比較で「量が消えた」ことが「0 になった」と読めてしまう（本仕様で 2 度出た形）。
    let summary = super::summarize(&[]);
    assert!(summary.frames_to_last_write.is_none());
    assert!(summary.ground_diff_max.is_none());
    let report = Report {
        records: 0,
        unassigned_records: 0,
        unassigned_malformed_records: 0,
        transitions: vec![TransitionVerdict {
            deterministic: judge(&summary, &Bounds::deterministic()),
            signoff: judge(&summary, &Bounds::signoff()),
            summary,
        }],
    };
    let rendered = report.to_string();
    assert!(
        rendered.contains("frames_to_last_write=-") && rendered.contains("ground_diff_max=-"),
        "欠けている量が番兵で刷られていない:\n{rendered}"
    );
    assert!(
        !rendered.contains("frames_to_last_write=0"),
        "欠けている量が 0 に化けている:\n{rendered}"
    );
}

#[test]
fn a_log_that_fails_only_the_deterministic_family_still_fails_the_report() {
    // バルーン窓を 2 回書く（合流前の形）。フレームも µs も動かないので実機専用側は合格する。
    let mut lines = compliant_transition_lines();
    lines.push(write(
        COMPLIANT_FRAME,
        1_450,
        STAGE_FLUSH,
        2,
        "0x2",
        PlacementRoute::KeepPositionResize.as_str(),
        "0",
        WindowKind::Balloon.as_str(),
        200,
    ));
    let report = judge_transition_log(&lines.join("\n"));
    assert!(report.transitions[0].deterministic.is_err());
    assert_eq!(report.transitions[0].signoff, Ok(()));
    assert!(report.failed(), "決定論側だけの違反でも不合格");
}

#[test]
fn a_log_that_fails_only_the_signoff_family_still_fails_the_report() {
    // 可視化から書込までを上限の外へ引き伸ばす。フレームは同一のままなので決定論側は合格する
    // ——「同一 tick の内側の食い違いは実機専用の量でしか見えない」形そのものである。
    let stretched_us = 1_000 + VISUALIZE_TO_WRITE_US_MAX + 1;
    let lines = replace_once(
        &compliant_transition_lines(),
        &format!("{FIELD_T_US}=1300"),
        &format!("{FIELD_T_US}={stretched_us}"),
    );
    let report = judge_transition_log(&lines.join("\n"));
    assert_eq!(report.transitions[0].deterministic, Ok(()));
    assert!(report.transitions[0].signoff.is_err());
    assert!(report.failed(), "実機専用側だけの違反でも不合格");
}

// ---------------------------------------------------------------------------
// ⑼ の被覆検査が自分の番をできること（task 4.2 の実機採取で判明した穴）
// ---------------------------------------------------------------------------

/// 書込のあった窓が**1 つ残らず**除外される遷移（2 窓とも不可視ゆえの見送り）。
///
/// 除外そのものは要件 4.6 に沿っている。問題は、この形で ⑼ の被覆検査が
/// `judged_windows` を 1 度も回らないことである——「窓ごとの量が 1 件も無い」が
/// 合格として通ってしまう（恒真の主張）。
fn every_written_window_excluded() -> Vec<String> {
    vec![
        monitor(COMPLIANT_FRAME, 96, 192, 1752, 1704),
        surface(
            COMPLIANT_FRAME,
            1_000,
            SURFACE_STAGE_SKIPPED,
            0,
            MISSING,
            MISSING,
            MISSING,
            SURFACE_REASON_INVISIBLE,
        ),
        surface(
            COMPLIANT_FRAME,
            1_050,
            SURFACE_STAGE_SKIPPED,
            1,
            MISSING,
            MISSING,
            MISSING,
            SURFACE_REASON_INVISIBLE,
        ),
        ground(
            COMPLIANT_FRAME,
            0,
            1704,
            1704,
            PlacementRoute::DpiReproject.as_str(),
        ),
        flush(COMPLIANT_FRAME, 1_200, STAGE_BEGIN, 2, MISSING),
        write(
            COMPLIANT_FRAME,
            1_300,
            STAGE_FLUSH,
            0,
            "0x1",
            PlacementRoute::DpiReproject.as_str(),
            "0",
            WindowKind::Char.as_str(),
            500,
        ),
        write(
            COMPLIANT_FRAME,
            1_400,
            STAGE_FLUSH,
            1,
            "0x2",
            PlacementRoute::BalloonFollow.as_str(),
            "0",
            WindowKind::Balloon.as_str(),
            400,
        ),
        flush(COMPLIANT_FRAME, 1_500, STAGE_END, 2, "300"),
    ]
}

#[test]
fn excluding_every_written_window_is_a_violation_not_a_pass() {
    // 実機専用系統は窓ごとの量（`visualize_to_write_us`）だけを窓へ当てるので、全窓が
    // 除外されると被覆検査が 1 度も回らず、**何も測っていない遷移が合格になる**。
    let summary = summarize_lines(&every_written_window_excluded());
    assert_eq!(summary.writes_per_window.len(), 2, "書込のあった窓は 2 つ");
    assert!(
        summary.visualize_to_write_us.is_empty(),
        "窓ごとの量は 1 件も無い"
    );
    let violations = violations(&summary, &Bounds::signoff());
    assert!(
        violations.contains(&Violation::AllWrittenWindowsExcluded { windows: 2 }),
        "全窓除外を違反として立てていない: {violations:?}"
    );
}

#[test]
fn a_transition_that_still_judges_one_window_does_not_raise_the_exclusion_violation() {
    // 上の陽性の対。除外が全窓に及んでいなければこの違反は立たない——立つようにすると、
    // 定常の形（発話していないバルーンだけが不可視で見送られる）が毎回不合格になる。
    let summary = summarize_lines(&one_scope_with_a_skipped_balloon(COMPLIANT_FRAME));
    assert_eq!(summary.writes_per_window.len(), 2);
    assert_eq!(summary.skipped_windows.len(), 1, "除外は 1 窓だけ");
    for bounds in [Bounds::deterministic(), Bounds::signoff()] {
        assert_eq!(
            judge(&summary, &bounds),
            Ok(()),
            "1 窓でも判定対象が残っていれば合格である"
        );
    }
}

#[test]
fn the_exclusion_violation_is_silent_when_no_window_quantity_is_armed() {
    // 窓ごとの量を 1 つも当てない組（`writes_per_window_max` だけ）では、全窓除外は
    // 「測れていない」ことにならない——armed な項目に応じてしか鳴らさない。
    let summary = summarize_lines(&every_written_window_excluded());
    let bounds = Bounds {
        writes_per_window_max: Some(super::WRITES_PER_WINDOW_MAX),
        ..Bounds::nothing()
    };
    assert_eq!(judge(&summary, &bounds), Ok(()));
}
