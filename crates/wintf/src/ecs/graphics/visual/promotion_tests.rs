//! 上げ下げの判断の純粋な規則の真理値表と時系列。

use super::promotion::*;
use super::promotion_shape::Ineligible;
use dola::TimePoint;
use dola::time::testing::at_ms;

const OK: fn() -> Result<(), Ineligible> = || Ok(());
const PENDING: SurfaceReadiness = SurfaceReadiness::Pending;

/// `times` の各時刻に更新を 1 回ずつ入れ、最後の答えを返す（形はよい・描画面は待ち）。
fn feed(s: &mut RuleState, times: &[TimePoint]) -> Verdict {
    let mut v = Verdict::Stay;
    for &t in times {
        v = s.step(t, true, OK, PENDING);
    }
    v
}

/// 8 Hz（0.125 秒おき）で `n` 回の時刻。
fn hz8(start_ms: i64, n: usize) -> Vec<TimePoint> {
    (0..n).map(|k| at_ms(start_ms + k as i64 * 125)).collect()
}

/// 上げて（Prepare）描画面ができた（Activate）状態を作る。戻り値は Activate の時刻。
fn active_at(start_ms: i64) -> (RuleState, TimePoint) {
    let mut s = RuleState::default();
    assert_eq!(
        feed(&mut s, &hz8(start_ms, PROMOTE_COUNT)),
        Verdict::Prepare
    );
    let t = at_ms(start_ms + PROMOTE_COUNT as i64 * 125);
    assert_eq!(
        s.step(t, false, OK, SurfaceReadiness::Ready),
        Verdict::Activate
    );
    assert_eq!(s.phase(), Phase::Active);
    (s, t)
}

/// 戻した時刻 `d` の後、履歴の内（3 秒未満）は頻繁でも形を聞かずに据え置き、
/// 履歴を過ぎた最初の回に上げることを確かめる。
fn assert_promote_hold(s: &mut RuleState, d: TimePoint) {
    for k in 1..=23u32 {
        let v = s.step(
            d + dola::time::testing::ms(k * 125),
            true,
            || panic!("履歴の内は形を見ない"),
            PENDING,
        );
        assert_eq!(v, Verdict::Stay);
    }
    assert_eq!(
        s.step(d + HOLD_AFTER_DEMOTE, true, OK, PENDING),
        Verdict::Prepare
    );
}

// ---- 真理値表 ----

#[test]
fn node_frequent_and_good_shape_prepares() {
    let mut s = RuleState::default();
    assert_eq!(feed(&mut s, &hz8(0, PROMOTE_COUNT - 1)), Verdict::Stay);
    assert_eq!(s.phase(), Phase::Node);
    assert_eq!(s.step(at_ms(875), true, OK, PENDING), Verdict::Prepare);
    assert_eq!(s.phase(), Phase::Preparing);
}

#[test]
fn node_frequent_but_bad_shape_skips_and_stays_node() {
    let mut s = RuleState::default();
    feed(&mut s, &hz8(0, PROMOTE_COUNT - 1));
    let v = s.step(at_ms(875), true, || Err(Ineligible::OutOfBounds), PENDING);
    assert_eq!(v, Verdict::Skip(Ineligible::OutOfBounds));
    assert_eq!(s.phase(), Phase::Node);
}

#[test]
fn shape_is_not_asked_when_not_frequent() {
    let mut s = RuleState::default();
    let v = s.step(at_ms(0), true, || panic!("形を見ないはず"), PENDING);
    assert_eq!(v, Verdict::Stay);
}

#[test]
fn preparing_pending_waits_ready_activates() {
    let mut s = RuleState::default();
    feed(&mut s, &hz8(0, PROMOTE_COUNT));
    assert_eq!(s.step(at_ms(1000), false, OK, PENDING), Verdict::Stay);
    assert_eq!(s.phase(), Phase::Preparing);
    assert_eq!(
        s.step(at_ms(1100), false, OK, SurfaceReadiness::Ready),
        Verdict::Activate
    );
    assert_eq!(s.phase(), Phase::Active);
}

#[test]
fn preparing_surface_failed_demotes() {
    let mut s = RuleState::default();
    feed(&mut s, &hz8(0, PROMOTE_COUNT));
    let v = s.step(at_ms(1000), true, OK, SurfaceReadiness::Failed);
    assert_eq!(v, Verdict::Demote(DemoteReason::SurfaceFailed));
    assert_eq!(s.phase(), Phase::Node);
}

#[test]
fn preparing_bad_shape_demotes_even_if_ready() {
    let mut s = RuleState::default();
    feed(&mut s, &hz8(0, PROMOTE_COUNT));
    let v = s.step(
        at_ms(1000),
        true,
        || Err(Ineligible::LaterContentOverlap),
        SurfaceReadiness::Ready,
    );
    assert_eq!(
        v,
        Verdict::Demote(DemoteReason::Shape(Ineligible::LaterContentOverlap))
    );
    assert_eq!(s.phase(), Phase::Node);
}

#[test]
fn active_quiet_demotes_only_after_quiet_and_hold() {
    let (mut s, t) = active_at(0);
    let last = at_ms(875); // 最後の更新
    // 落ち着いた（最後の更新から 3 秒）が上げた後の履歴（Activate から 3 秒）の内なら据え置く。
    assert!(last + DEMOTE_QUIET < t + HOLD_AFTER_PROMOTE);
    assert_eq!(
        s.step(last + DEMOTE_QUIET, false, OK, PENDING),
        Verdict::Stay
    );
    // 履歴のちょうど前は据え置き、ちょうどで戻す。
    let hold_end = t + HOLD_AFTER_PROMOTE;
    assert_eq!(
        s.step(
            TimePoint::from_ticks(hold_end.as_ticks() - 1),
            false,
            OK,
            PENDING
        ),
        Verdict::Stay
    );
    assert_eq!(
        s.step(hold_end, false, OK, PENDING),
        Verdict::Demote(DemoteReason::Quiet)
    );
    assert_eq!(s.phase(), Phase::Node);
}

#[test]
fn active_not_quiet_stays() {
    let (mut s, t) = active_at(0);
    // 更新が続く（3 秒より短い間隔）なら履歴を過ぎても戻さない。
    let mut now = t;
    for _ in 0..20 {
        now = now + dola::time::testing::ms(2900);
        assert_eq!(s.step(now, true, OK, PENDING), Verdict::Stay);
    }
    // 最後の更新から 3 秒ちょうど前は据え置き、ちょうどで戻す。
    assert_eq!(
        s.step(
            TimePoint::from_ticks((now + DEMOTE_QUIET).as_ticks() - 1),
            false,
            OK,
            PENDING
        ),
        Verdict::Stay
    );
    assert_eq!(
        s.step(now + DEMOTE_QUIET, false, OK, PENDING),
        Verdict::Demote(DemoteReason::Quiet)
    );
}

#[test]
fn active_bad_shape_demotes_at_once_ignoring_hold() {
    let (mut s, t) = active_at(0);
    let v = s.step(
        t + dola::time::testing::ms(10),
        true,
        || Err(Ineligible::Hidden),
        PENDING,
    );
    assert_eq!(v, Verdict::Demote(DemoteReason::Shape(Ineligible::Hidden)));
    assert_eq!(s.phase(), Phase::Node);
}

#[test]
fn immediate_demote_then_promote_hold_applies() {
    let (mut s, t) = active_at(0);
    let d = t + dola::TimeSpan::try_from_ticks(625_000).unwrap();
    s.step(d, true, || Err(Ineligible::AncestorLayer), PENDING);
    assert_promote_hold(&mut s, d);
}

#[test]
fn quiet_demote_then_promote_hold_applies() {
    let (mut s, t) = active_at(0);
    let d = t + HOLD_AFTER_PROMOTE;
    assert_eq!(
        s.step(d, false, OK, PENDING),
        Verdict::Demote(DemoteReason::Quiet)
    );
    assert_promote_hold(&mut s, d);
}

#[test]
fn absorb_demotes_owner_and_applies_promote_hold() {
    let (mut s, t) = active_at(0);
    assert_eq!(
        s.absorb(t + dola::time::testing::ms(125)),
        Verdict::Demote(DemoteReason::Absorbed)
    );
    assert_eq!(s.phase(), Phase::Node);
    assert_promote_hold(&mut s, t + dola::time::testing::ms(125));

    // 用意中も吸い込む。節は何もしない。
    let mut p = RuleState::default();
    feed(&mut p, &hz8(0, PROMOTE_COUNT));
    assert_eq!(
        p.absorb(at_ms(1000)),
        Verdict::Demote(DemoteReason::Absorbed)
    );
    let mut n = RuleState::default();
    assert_eq!(n.absorb(at_ms(1000)), Verdict::Stay);
    assert_eq!(n, RuleState::default());
}

#[test]
fn reset_clears_history_without_hold() {
    let (mut s, t) = active_at(0);
    s.reset();
    assert_eq!(s, RuleState::default());
    // 上げの履歴を掛けない: すぐの頻繁な更新で上げる。
    assert_eq!(feed(&mut s, &hz8_from(t, PROMOTE_COUNT)), Verdict::Prepare);
}

#[test]
fn auto_promotion_defaults_to_node() {
    let a = AutoPromotion::default();
    assert_eq!(a.rule.phase(), Phase::Node);
    assert_eq!(a.skip_memo, None);
    assert!(!a.built);
}

#[test]
fn ineligible_words() {
    use Ineligible::*;
    let words: Vec<_> = [
        LaterContentOverlap,
        ContainsOwner,
        AncestorLayer,
        NonSourceOver,
        OutOfBounds,
        FractionalOrigin,
        Hidden,
        NoDevice,
    ]
    .iter()
    .map(|e| e.as_str())
    .collect();
    assert_eq!(
        words,
        [
            "later_content_overlap",
            "contains_owner",
            "ancestor_layer",
            "non_source_over",
            "out_of_bounds",
            "fractional_origin",
            "hidden",
            "no_device",
        ]
    );
}

// ---- 時系列 ----

#[test]
fn exactly_one_second_window_promotes_just_over_does_not() {
    // 8 回の最古から今までがちょうど 1 秒なら上げる。
    let mut times = hz8(0, PROMOTE_COUNT - 1);
    times.push(at_ms(0) + PROMOTE_WINDOW);
    let mut s = RuleState::default();
    assert_eq!(feed(&mut s, &times), Verdict::Prepare);

    // ちょうど 1 秒を少し越えると上げない。
    let mut times = hz8(0, PROMOTE_COUNT - 1);
    times.push(TimePoint::from_ticks(
        (at_ms(0) + PROMOTE_WINDOW).as_ticks() + 1,
    ));
    let mut s = RuleState::default();
    assert_eq!(feed(&mut s, &times), Verdict::Stay);
    assert_eq!(s.phase(), Phase::Node);
}

#[test]
fn just_under_8hz_never_promotes() {
    // 1 秒に 7 回ほど（間隔 1/7 秒より少し長い）の更新が続いても上げない。
    let mut s = RuleState::default();
    let step_ticks = 10_000_000 / 7 + 10;
    for k in 0..200i64 {
        assert_eq!(
            s.step(TimePoint::from_ticks(k * step_ticks), true, OK, PENDING),
            Verdict::Stay
        );
    }
}

#[test]
fn interrupted_updates_restart_the_count() {
    let mut s = RuleState::default();
    // 7 回の後に 2 秒の途切れ、その後の 7 回では古い分が窓の外。
    assert_eq!(feed(&mut s, &hz8(0, 7)), Verdict::Stay);
    assert_eq!(feed(&mut s, &hz8(2875, 7)), Verdict::Stay);
    // 途切れの後に 8 回そろえば上げる。
    assert_eq!(
        s.step(at_ms(2875 + 7 * 125), true, OK, PENDING),
        Verdict::Prepare
    );
}

#[test]
fn frames_without_time_progress_count_once() {
    // 同じ時刻の複数の更新（同じフレームの中）は 1 回に数える。
    let mut s = RuleState::default();
    for _ in 0..50 {
        assert_eq!(s.step(at_ms(0), true, OK, PENDING), Verdict::Stay);
    }
    assert_eq!(feed(&mut s, &hz8(125, PROMOTE_COUNT - 2)), Verdict::Stay);
    assert_eq!(s.step(at_ms(875), true, OK, PENDING), Verdict::Prepare);
}

#[test]
fn time_going_backwards_is_treated_as_previous() {
    // 戻った時刻の更新は前の回の時刻の更新として数え、同じ時刻なので重ねない。
    let mut s = RuleState::default();
    feed(&mut s, &hz8(10_000, PROMOTE_COUNT - 1)); // 最後は 10.75
    assert_eq!(s.step(at_ms(0), true, OK, PENDING), Verdict::Stay);
    assert_eq!(s.step(at_ms(5000), true, OK, PENDING), Verdict::Stay);
    assert_eq!(s.step(at_ms(10_875), true, OK, PENDING), Verdict::Prepare);

    // 戻った時刻を理由に戻さない（経過 0）。
    let (mut s, t) = active_at(100_000);
    let quiet_at = t + HOLD_AFTER_PROMOTE;
    assert_eq!(
        s.step(t + dola::time::testing::ms(1000), false, OK, PENDING),
        Verdict::Stay
    );
    assert_eq!(s.step(at_ms(0), false, OK, PENDING), Verdict::Stay);
    assert_eq!(
        s.step(at_ms(-1_000_000_000_000), false, OK, PENDING),
        Verdict::Stay
    );
    assert_eq!(
        s.step(quiet_at, false, OK, PENDING),
        Verdict::Demote(DemoteReason::Quiet)
    );
}

#[test]
fn same_inputs_give_same_answers() {
    let inputs: Vec<(TimePoint, bool)> = (0..400i64)
        .map(|k| (at_ms(k * 50), (k / 80) % 2 == 0))
        .collect();
    let run = || {
        let mut s = RuleState::default();
        inputs
            .iter()
            .map(|&(t, u)| s.step(t, u, OK, SurfaceReadiness::Ready))
            .collect::<Vec<_>>()
    };
    let a = run();
    assert_eq!(a, run());
    assert!(a.contains(&Verdict::Prepare));
    assert!(a.contains(&Verdict::Activate));
    assert!(a.contains(&Verdict::Demote(DemoteReason::Quiet)));
}

/// `t` から 100 ミリ秒後を起点に 8 Hz で `n` 回の時刻。
fn hz8_from(t: TimePoint, n: usize) -> Vec<TimePoint> {
    (0..n)
        .map(|k| t + dola::time::testing::ms(100 + k as u32 * 125))
        .collect()
}
