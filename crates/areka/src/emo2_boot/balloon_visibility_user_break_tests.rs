// =============================================================================
// 判断中核 `decide` の「利用者の中断」の分岐の決定論テスト（areka-P0-balloon-break）
// =============================================================================
//
// 中断で出ているバルーンを全部隠すこと・次のトークが始まるまで内容では出し直さないことを
// ここで固定する。既存の理由（内容・消去・時間切れ・明示の指示）の分岐は
// `balloon_visibility_tests.rs` 群が持つので繰り返さない。

use super::test_support::*;
use super::*;

/// タイムアウト時間（秒）。既定値の唯一の定義箇所から引く（同じ数値を複数箇所へ散らさない）。
const TIMEOUT: f64 = DEFAULT_BALLOON_TIMEOUT_SECS;

/// 表示の合図の線へ任意の信号を混ぜて 1 巡進める。
///
/// [`Frame`] は会話の開始と占有終端しか組み立てられないため、利用者の中断を含む巡はここで組む。
/// 並びは線の上の到着順そのものである（畳み込みが到着順に依存するため）。
fn run_with_signals(
    state: &mut BalloonVisibilityState,
    entries: &[(u32, ScopeObservation)],
    signals: &[TalkLifecycleSignal],
    now: Option<f64>,
) -> VisibilityDecision {
    let mut obs = observations(entries);
    obs.lifecycle.extend_from_slice(signals);
    decide(state, &obs, now, TIMEOUT)
}

/// 利用者の中断だけが届いた 1 巡。
fn break_round(
    state: &mut BalloonVisibilityState,
    entries: &[(u32, ScopeObservation)],
    now: Option<f64>,
) -> VisibilityDecision {
    run_with_signals(state, entries, &[TalkLifecycleSignal::UserBreak], now)
}

/// 期待する表示行動。
fn show(scope: u32) -> VisibilityAction {
    VisibilityAction::Show { scope }
}

/// 期待する表示ログ。
fn shown_log(scope: u32) -> VisibilityLogEvent {
    VisibilityLogEvent::Transition {
        scope,
        trigger: VisibilityTrigger::Content,
        visible: true,
    }
}

/// 期待する非表示行動（利用者の中断が契機）。
fn hide_broken(scopes: &[u32]) -> VisibilityAction {
    VisibilityAction::HideScopes {
        scopes: scopes.to_vec(),
        trigger: VisibilityTrigger::UserBreak,
    }
}

/// 期待する非表示ログ（利用者の中断が契機）。
fn broken_log(scope: u32) -> VisibilityLogEvent {
    VisibilityLogEvent::Transition {
        scope,
        trigger: VisibilityTrigger::UserBreak,
        visible: false,
    }
}

// ---------------------------------------------------------------------------
// 判断分岐 ⑵ の隠す側（要件 4.1・7.4）
// ---------------------------------------------------------------------------

/// 中断は**そのとき出ているバルーンをすべて** 1 件の指示へ畳む。既に隠れている scope は
/// 載せない（隠す指示を重ねて出さない・要件 4.1）。合図が起きた scope だけを隠すのでもない。
#[test]
fn a_user_break_hides_every_visible_scope_in_one_action() {
    let mut state = BalloonVisibilityState::default();
    let shown = step(
        &mut state,
        &[
            (0, seen(3, false)),
            (1, seen(2, false)),
            (2, seen(0, false)),
        ],
    );
    assert_eq!(
        shown.actions,
        vec![show(0), show(1)],
        "前提: 2 つ出ていない"
    );

    let broken = break_round(
        &mut state,
        &[(0, seen(3, true)), (1, seen(2, true)), (2, seen(0, false))],
        None,
    );
    assert_eq!(broken.actions, vec![hide_broken(&[0, 1])]);
    assert_eq!(broken.logs, vec![broken_log(0), broken_log(1)]);
}

/// 中断で隠した scope は、以後の判断でも不可視として扱う（次の巡で「外から表示された」と
/// 読まれない）。
#[test]
fn a_broken_scope_is_carried_over_as_invisible() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);
    assert!(
        state.per_scope[&0].prev_visible,
        "前提: 表示が持ち越されていない"
    );

    break_round(&mut state, &[(0, seen(3, true))], None);
    assert!(
        !state.per_scope[&0].prev_visible,
        "中断の非表示が持ち越されていない"
    );
}

// ---------------------------------------------------------------------------
// 判断分岐 ⑹ 隠した後の出し直し（要件 4.7・4.8）
// ---------------------------------------------------------------------------

/// 止めた台本の文字は、再生を止めた後も時刻の進行だけで見える数が増えうる。掛け金が立って
/// いる間は出し直さず、記録も 1 行も出さない（要件 4.8）。次のトークの始まりで掛け金が解け、
/// 以後の増加は通常どおり表示になる（要件 4.7）。
#[test]
fn glyphs_growing_after_a_user_break_show_again_only_once_the_next_talk_starts() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);
    assert_eq!(
        break_round(&mut state, &[(0, seen(3, true))], None).actions,
        vec![hide_broken(&[0])],
        "前提: 中断で隠れていない"
    );

    for glyphs in [4usize, 5, 9] {
        let d = step(&mut state, &[(0, seen(glyphs, false))]);
        assert!(
            d.actions.is_empty(),
            "中断で隠した後に出し直した（glyphs={glyphs}）"
        );
        assert!(d.logs.is_empty(), "見送りで記録が出た（glyphs={glyphs}）");
    }

    // 次のトークが始まる。冒頭の全消去は既に不可視なので何も起きない。
    let started = Frame::new(&[(0, seen(0, false))])
        .talk_started()
        .run(&mut state);
    assert!(started.actions.is_empty(), "会話開始そのもので発行が起きた");

    let d = step(&mut state, &[(0, seen(2, false))]);
    assert_eq!(d.actions, vec![show(0)], "次のトークのバルーンが出ない");
    assert_eq!(d.logs, vec![shown_log(0)]);
}

/// 同じ巡に中断と次のトークの始まりが揃った場合は、隠してから出す順になる
/// （古い表示が消えて新しいトークが出る）。畳み込みは線の上の到着順に従う。
#[test]
fn a_user_break_and_the_next_talk_in_the_same_round_hide_then_show() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);

    let d = run_with_signals(
        &mut state,
        &[(0, seen(5, true))],
        &[
            TalkLifecycleSignal::UserBreak,
            TalkLifecycleSignal::TalkStarted,
        ],
        None,
    );
    assert_eq!(d.actions, vec![hide_broken(&[0]), show(0)]);
    assert_eq!(d.logs, vec![broken_log(0), shown_log(0)]);
}

// ---------------------------------------------------------------------------
// 時間切れとの関係（要件 4.6）
// ---------------------------------------------------------------------------

/// 中断で隠した後に満了予定の時刻が来ても行動は 0 件で、本仕様が新しく足す記録も 0 行。
/// 中断の巡に出る「出ているバルーンが無くなったので計測を捨てた」の 1 行は既存の記録で、
/// 本仕様は足しも消しもしない（要件 4.6）。
#[test]
fn a_deadline_reached_after_a_user_break_produces_no_action() {
    let mut state = BalloonVisibilityState::default();
    let armed = Frame::new(&[(0, seen(2, false))])
        .at(0.0)
        .talk_started()
        .display_end(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(armed.actions, vec![show(0)], "前提: 表示が起きていない");
    assert_eq!(
        state.deadline,
        Some(TIMEOUT),
        "前提: 満了予定が立っていない"
    );

    let broken = break_round(&mut state, &[(0, seen(2, true))], Some(1.0));
    assert_eq!(broken.actions, vec![hide_broken(&[0])]);
    assert_eq!(
        broken.logs,
        vec![
            broken_log(0),
            VisibilityLogEvent::MeasurementDiscarded {
                reason: MeasurementDiscardReason::NoVisibleScope,
                deadline: TIMEOUT,
            }
        ]
    );
    assert_eq!(state.deadline, None, "隠した後も計測が残っている");

    let expired = Frame::new(&[(0, seen(2, false))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(
        expired.actions.is_empty(),
        "隠れているバルーンへ満了の非表示が出た"
    );
    assert!(expired.logs.is_empty(), "満了の時刻に記録が出た");
}

/// 抑止（ドラッグ・滞在・選択肢の表示中）は時間切れだけの規則であり、中断は見ない
/// ——ダブルクリックした利用者は必ずバルーンの上に居る。
#[test]
fn suppression_does_not_hold_back_a_user_break() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);

    let mut obs = observations(&[(0, choosing(hovered(seen(3, true))))]);
    obs.dragging = true;
    obs.lifecycle.push(TalkLifecycleSignal::UserBreak);
    let broken = decide(&mut state, &obs, Some(1.0), TIMEOUT);
    assert_eq!(broken.actions, vec![hide_broken(&[0])]);
    assert_eq!(broken.logs, vec![broken_log(0)]);
}
