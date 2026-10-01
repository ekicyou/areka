// =============================================================================
// バルーンの差し替えで scope の可視の記憶を消す口 `forget_scope` の決定論テスト
// （areka-P0-shell-balloon-switch 要件 3.3・design「SwitchPhase」の完了の後始末）
// =============================================================================
//
// 差し替えで古いバルーンの装着は消え、新しいバルーンは外部所有のまま隠れている。そこで
// ⑴ 消えた古いバルーンを「外から隠された」と読まないよう可視の記憶を偽へ倒し、⑵ 見えている
// バルーンが無くなったので計測を止め、⑶ 文字の数の記憶は保つ（保たないと、表示の済んだ文字が
// 次のフレームで増加の縁に化けて新しいバルーンが台詞の外で出てしまう）。

use super::test_support::*;
use super::*;

/// 2 つの scope を出し、占有終端を過ぎて計測が立った状態を作る。
fn shown_and_measuring() -> BalloonVisibilityState {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(5, false)), (1, seen(3, false))])
        .display_end(1.0)
        .at(0.5)
        .run(&mut state);
    Frame::new(&[(0, seen(5, true)), (1, seen(3, true))])
        .at(2.0)
        .run(&mut state);
    state
}

/// scope 0 だけを忘れる: scope 0 は可視の記憶が偽・文字の数は保つ、scope 1 は不変、計測は止まる。
/// 次のフレームに新しいバルーンが隠れたまま同じ文字の数を見せても、表示も記録も 0（計測を
/// 止めていなければ、見えるバルーンが無くなったことで計測の破棄の記録が 1 件出る）。
#[test]
fn forget_scope_drops_visibility_and_measurement_but_keeps_glyphs() {
    let mut state = shown_and_measuring();
    let before = (state.per_scope.clone(), state.deadline.is_some());

    state.forget_scope(0);
    let after = (state.per_scope.clone(), state.deadline);
    let next = Frame::new(&[(0, seen(5, false)), (1, seen(3, false))])
        .at(2.5)
        .run(&mut state);

    let kept = |last_glyphs, prev_visible| ScopeVisibility {
        last_glyphs,
        prev_visible,
    };
    assert_eq!(
        (before, after, next),
        (
            ([(0, kept(5, true)), (1, kept(3, true))].into(), true),
            ([(0, kept(5, false)), (1, kept(3, true))].into(), None),
            VisibilityDecision::default(),
        ),
        "(忘れる前 (scope の記憶, 計測中), 忘れた後 (scope の記憶, 満了予定), 次のフレームの判断)"
    );
}
