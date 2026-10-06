// =============================================================================
// トークの終わりから計る時間切れの判断中核の檻（areka-P0-balloon-lifecycle-events）
//
// 計測はトークの終わりの合図が届いてから立ち、起点は占有区間の終端と止まった時刻の早い方
// （決定 D6・要件 5.1）。待ち時間はトークごとの指定で差し替わり、時間切れで隠したフレームに
// だけトークの番号の知らせが立つ（要件 2.1・9.1〜9.3・9.6）。時刻はすべて注入する。
// =============================================================================

use super::test_support::*;
use super::*;

/// 時間切れで隠す行動。
fn hide_timed_out(scopes: &[u32]) -> VisibilityAction {
    VisibilityAction::HideScopes {
        scopes: scopes.to_vec(),
        trigger: VisibilityTrigger::Timeout,
    }
}

/// 占有終端に達してもトークの終わりが届くまでは計測が立たず、届いたフレームに「止まった時刻」を
/// 起点に立つ。満了のフレームにだけ番号の知らせが載る（要件 2.1・5.1・7.3）。
#[test]
fn measurement_waits_for_talk_end_and_anchors_on_the_earlier_stop_time() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(3, false))])
        .at(0.0)
        .talk_started()
        .display_end(10.0)
        .run(&mut state);

    // 占有終端を過ぎても、トークの終わりが無ければ計測は立たない。
    let waiting = Frame::new(&[(0, seen(3, true))]).at(10.0).run(&mut state);
    assert_eq!((waiting.logs, state.deadline), (vec![], None));

    // 止まった時刻 4.0 は終端 10.0 より早いので、起点は止まった時刻。
    let started = Frame::new(&[(0, seen(3, true))])
        .at(10.5)
        .signal(TalkLifecycleSignal::TalkEnded { at: Some(4.0) })
        .run(&mut state);
    let deadline = 4.0 + DEFAULT_BALLOON_TIMEOUT_SECS;
    assert_eq!(
        started.logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::StoppedAt(4.0),
            display_end: 10.0,
            deadline,
        }]
    );
    assert_eq!(
        started.timeout_notice, None,
        "計測を始めただけでは知らせない"
    );

    let before = Frame::new(&[(0, seen(3, true))])
        .at(deadline - 0.001)
        .run(&mut state);
    assert_eq!((before.actions, before.timeout_notice), (vec![], None));
    let expired = Frame::new(&[(0, seen(3, true))])
        .at(deadline)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.timeout_notice, Some(TALK_ID));
}

/// トークの待ち時間の指定はミリ秒を 1,000 で割った秒に写し、「なし」なら計測しない。次の会話の
/// 開始で既定へ戻る（要件 9.1〜9.3・9.6）。
#[test]
fn talk_timeout_replaces_the_wait_until_the_next_talk() {
    let mut state = BalloonVisibilityState::default();
    let started = Frame::new(&[(0, seen(3, false))])
        .at(0.0)
        .talk_started()
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Never))
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Millis(
            1234,
        )))
        .ended_at(2.0)
        .run(&mut state);
    assert_eq!(started.actions, vec![VisibilityAction::Show { scope: 0 }]);
    let measured = Frame::new(&[(0, seen(3, true))]).at(2.0).run(&mut state);
    assert_eq!(
        measured.logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::DisplayEnd,
            display_end: 2.0,
            deadline: 2.0 + 1234.0 / 1000.0,
        }],
        "最後に届いた指定が残り、ミリ秒は丸めずに秒へ写す"
    );

    // 次の会話で「なし」が指定されると、終わっても計測しない。
    Frame::new(&[(0, seen(3, true))])
        .at(0.0)
        .talk_started()
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Never))
        .ended_at(1.0)
        .run(&mut state);
    let never = Frame::new(&[(0, seen(3, true))]).at(500.0).run(&mut state);
    assert_eq!((never.actions, state.deadline), (vec![], None));

    // さらに次の会話は既定の待ち時間へ戻る。
    Frame::new(&[(0, seen(3, true))])
        .at(1.0)
        .talk_started()
        .ended_at(1.0)
        .run(&mut state);
    assert_eq!(state.deadline, Some(1.0 + DEFAULT_BALLOON_TIMEOUT_SECS));
}
