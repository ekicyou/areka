// =============================================================================
// 判断中核 `decide` の「利用者の中断」の分岐の決定論テスト
// =============================================================================
//
// 中断で出ているバルーンを全部隠すこと・次のトークが始まるまで内容では出し直さないことを
// ここで固定する。

use super::test_support::*;
use super::*;

/// タイムアウト時間（秒）。[`Frame::new`] の既定と同じ値。
const TIMEOUT: f64 = DEFAULT_BALLOON_TIMEOUT_SECS;

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
// 判断分岐 ⑵ の隠す側
// ---------------------------------------------------------------------------

/// 中断は**そのとき出ているバルーンをすべて** 1 件の指示へ畳む。既に隠れている scope は
/// 載せない（隠す指示を重ねて出さない）。合図が起きた scope だけを隠すのでもない。
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

    let broken = Frame::new(&[(0, seen(3, true)), (1, seen(2, true)), (2, seen(0, false))])
        .signal(TalkLifecycleSignal::UserBreak)
        .run(&mut state);
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

    Frame::new(&[(0, seen(3, true))])
        .signal(TalkLifecycleSignal::UserBreak)
        .run(&mut state);
    assert!(
        !state.per_scope[&0].prev_visible,
        "中断の非表示が持ち越されていない"
    );
}

// ---------------------------------------------------------------------------
// 判断分岐 ⑹ 隠した後の出し直し
// ---------------------------------------------------------------------------

/// 止めた台本の文字は、再生を止めた後も時刻の進行だけで見える数が増えうる。掛け金が立って
/// いる間は出し直さず、記録も 1 行も出さない。次のトークの始まりで掛け金が解け、
/// 以後の増加は通常どおり表示になる。
#[test]
fn glyphs_growing_after_a_user_break_show_again_only_once_the_next_talk_starts() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);
    assert_eq!(
        Frame::new(&[(0, seen(3, true))])
            .signal(TalkLifecycleSignal::UserBreak)
            .run(&mut state)
            .actions,
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

    let d = Frame::new(&[(0, seen(5, true))])
        .signal(TalkLifecycleSignal::UserBreak)
        .signal(TalkLifecycleSignal::TalkStarted { talk_id: None })
        .run(&mut state);
    assert_eq!(d.actions, vec![hide_broken(&[0]), show(0)]);
    assert_eq!(d.logs, vec![broken_log(0), shown_log(0)]);
}

// ---------------------------------------------------------------------------
// 時間切れとの関係
// ---------------------------------------------------------------------------

/// 中断で隠した後に満了予定の時刻が来ても行動は 0 件で、記録も 0 行。計測は中断の巡で
/// 「出ているバルーンが無くなった」として捨てる。
#[test]
fn a_deadline_reached_after_a_user_break_produces_no_action() {
    let mut state = BalloonVisibilityState::default();
    let armed = Frame::new(&[(0, seen(2, false))])
        .at(0.0)
        .talk_started()
        .ended_at(0.0)
        .run(&mut state);
    assert_eq!(armed.actions, vec![show(0)], "前提: 表示が起きていない");
    assert_eq!(
        state.deadline,
        Some(TIMEOUT),
        "前提: 満了予定が立っていない"
    );

    let broken = Frame::new(&[(0, seen(2, true))])
        .signal(TalkLifecycleSignal::UserBreak)
        .at(1.0)
        .run(&mut state);
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
        .run(&mut state);
    assert!(
        expired.actions.is_empty(),
        "隠れているバルーンへ満了の非表示が出た"
    );
    assert!(expired.logs.is_empty(), "満了の時刻に記録が出た");
}

/// 抑止（ドラッグ・滞在）は時間切れだけの規則であり、中断は見ない
/// ——ダブルクリックした利用者は必ずバルーンの上に居る。
#[test]
fn suppression_does_not_hold_back_a_user_break() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);

    let broken = Frame::new(&[(0, hovered(seen(3, true)))])
        .signal(TalkLifecycleSignal::UserBreak)
        .at(1.0)
        .dragging()
        .run(&mut state);
    assert_eq!(broken.actions, vec![hide_broken(&[0])]);
    assert_eq!(broken.logs, vec![broken_log(0)]);
}
