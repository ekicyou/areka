// =============================================================================
// バルーンの差し替えで scope の可視の記憶を消す口 `forget_scope` の決定論テスト
// =============================================================================
//
// 差し替えで古いバルーンの装着は消え、新しいバルーンは外部所有のまま隠れている。そこで
// ⑴ 消えた古いバルーンを「外から隠された」と読まないよう可視の記憶を偽へ倒し、⑵ 計測には
// 触れず（見えるものが無ければ次のフレームの判断が破棄し、箱に文字が出ていれば箱が時間切れを
// 待ち続ける）、⑶ 文字の数の記憶は保つ（保たないと、表示の済んだ文字が次のフレームで増加の縁に
// 化けて新しいバルーンが台詞の外で出てしまう）。

use super::test_support::*;
use super::*;

/// 2 つの scope を出し、占有終端を過ぎて計測が立った状態を作る。
fn shown_and_measuring() -> BalloonVisibilityState {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(5, false)), (1, seen(3, false))])
        .ended_at(1.0)
        .at(0.5)
        .run(&mut state);
    Frame::new(&[(0, seen(5, true)), (1, seen(3, true))])
        .at(2.0)
        .run(&mut state);
    state
}

/// scope 0 だけを忘れる: scope 0 は可視の記憶が偽・文字の数は保つ、scope 1 は不変、満了予定は
/// そのまま。次のフレームに新しいバルーンが隠れたまま同じ文字の数を見せても表示は 0 で、見える
/// バルーンが無くなったので計測の破棄の記録が 1 件だけ出る。
#[test]
fn forget_scope_drops_visibility_but_keeps_glyphs_and_measurement() {
    let mut state = shown_and_measuring();
    let before = (state.per_scope.clone(), state.deadline.is_some());

    state.forget_scope(0);
    let after = (state.per_scope.clone(), state.deadline);
    let next = Frame::new(&[(0, seen(5, false)), (1, seen(3, false))])
        .at(2.5)
        .run(&mut state);

    let kept = |last_glyphs, prev_visible| ScopeVisibility {
        last_glyphs,
        last_clear_count: 0,
        prev_visible,
    };
    assert_eq!(
        (before, after, next),
        (
            ([(0, kept(5, true)), (1, kept(3, true))].into(), true),
            ([(0, kept(5, false)), (1, kept(3, true))].into(), Some(31.0)),
            VisibilityDecision {
                logs: vec![VisibilityLogEvent::MeasurementDiscarded {
                    reason: MeasurementDiscardReason::NoVisibleScope,
                    deadline: 31.0,
                }],
                ..VisibilityDecision::default()
            },
        ),
        "(忘れる前 (scope の記憶, 計測中), 忘れた後 (scope の記憶, 満了予定), 次のフレームの判断)"
    );
}

/// 箱に文字が出たまま、抑止の解けた縁で測り直した後にバルーンを差し替えても、満了予定は
/// 早まらない。差し替えは窓だけを替え、箱は見えたまま時間切れを待つ。
#[test]
fn forget_scope_keeps_the_restarted_deadline_while_a_box_is_showing() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, boxed(seen(5, false)))])
        .ended_at(0.0)
        .at(0.0)
        .run(&mut state);
    Frame::new(&[(0, hovered(boxed(seen(5, true))))])
        .at(40.0)
        .run(&mut state);
    Frame::new(&[(0, boxed(seen(5, true)))])
        .at(100.0)
        .run(&mut state);

    state.forget_scope(0);
    let next = Frame::new(&[(0, boxed(seen(5, false)))])
        .at(110.0)
        .run(&mut state);

    assert_eq!(
        (next, state.deadline),
        (VisibilityDecision::default(), Some(130.0)),
        "(差し替えの次のフレームの判断, 満了予定)"
    );
}
