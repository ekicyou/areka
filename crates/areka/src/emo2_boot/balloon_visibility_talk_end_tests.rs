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

/// 時間切れの計測が立つまで、表示だけを済ませた状態を作る（scope 0 に 3 文字・既定の待ち時間）。
/// トークの終わりは占有終端 `end` と同じ時刻で届く。
fn shown_and_ended(end: f64) -> BalloonVisibilityState {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(3, false))])
        .at(0.0)
        .talk_started()
        .ended_at(end)
        .run(&mut state);
    let started = Frame::new(&[(0, seen(3, true))]).at(end).run(&mut state);
    assert_eq!(state.deadline, Some(end + DEFAULT_BALLOON_TIMEOUT_SECS));
    assert_eq!(started.timeout_notice, None);
    state
}

// ---------------------------------------------------------------------------
// 起点の採り方（要件 5.1・5.3・5.6・7.3・7.4）
// ---------------------------------------------------------------------------

/// 止まった時刻が終端より前なら止まった時刻、終端とちょうど同じか後なら終端を起点に採る。
/// 起点の記録は計測を始めたフレームに 1 件だけ出て、満了は直前で起きずちょうどで起きる。
#[test]
fn origin_is_the_stop_time_only_when_it_precedes_the_display_end() {
    const END: f64 = 10.0;
    let cases = [
        (
            END - 0.001,
            MeasurementOrigin::StoppedAt(END - 0.001),
            END - 0.001,
        ),
        (END, MeasurementOrigin::DisplayEnd, END),
        (END + 0.001, MeasurementOrigin::DisplayEnd, END),
    ];
    for (stopped, origin, effective) in cases {
        let mut state = BalloonVisibilityState::default();
        Frame::new(&[(0, seen(3, false))])
            .at(0.0)
            .talk_started()
            .display_end(END)
            .run(&mut state);
        let started = Frame::new(&[(0, seen(3, true))])
            .at(10.5)
            .signal(TalkLifecycleSignal::TalkEnded { at: Some(stopped) })
            .run(&mut state);
        let deadline = effective + DEFAULT_BALLOON_TIMEOUT_SECS;
        assert_eq!(
            started.logs,
            vec![VisibilityLogEvent::MeasurementStarted {
                origin,
                display_end: END,
                deadline,
            }],
            "止まった時刻 {stopped}"
        );

        let before = Frame::new(&[(0, seen(3, true))])
            .at(deadline - 0.001)
            .run(&mut state);
        assert_eq!(
            (before.actions, before.logs, before.timeout_notice),
            (vec![], vec![], None),
            "毎フレームの判定は記録せず、満了の直前では隠さない（止まった時刻 {stopped}）"
        );
        let expired = Frame::new(&[(0, seen(3, true))])
            .at(deadline)
            .run(&mut state);
        assert_eq!(
            expired.actions,
            vec![hide_timed_out(&[0])],
            "止まった時刻 {stopped}"
        );
        assert_eq!(expired.timeout_notice, Some(TALK_ID));
    }
}

/// トークの終わりに時刻が無ければ終端を起点にし、そのことを起点の記録に残す（要件 5.6・7.3）。
#[test]
fn missing_stop_time_falls_back_to_the_display_end_and_is_recorded() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(3, false))])
        .at(0.0)
        .talk_started()
        .display_end(10.0)
        .run(&mut state);
    let started = Frame::new(&[(0, seen(3, true))])
        .at(10.0)
        .signal(TalkLifecycleSignal::TalkEnded { at: None })
        .run(&mut state);
    let deadline = 10.0 + DEFAULT_BALLOON_TIMEOUT_SECS;
    assert_eq!(
        started.logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::StopTimeMissing,
            display_end: 10.0,
            deadline,
        }]
    );

    let before = Frame::new(&[(0, seen(3, true))])
        .at(deadline - 0.001)
        .run(&mut state);
    assert_eq!((before.actions, before.logs), (vec![], vec![]));
    let expired = Frame::new(&[(0, seen(3, true))])
        .at(deadline)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.timeout_notice, Some(TALK_ID));
}

// ---------------------------------------------------------------------------
// トークごとの待ち時間（要件 9.1・9.2・9.5・9.7）
// ---------------------------------------------------------------------------

/// 正の待ち時間では、表示の終わりからその時間で隠れる——直前では隠れず、ちょうどで隠れる。
/// 隠れたスコープの数によらず知らせは 1 つで、満了のフレームの後には立たない（要件 2.1・9.1）。
#[test]
fn positive_talk_timeout_hides_exactly_at_its_own_deadline() {
    let both = |a: bool| [(0, seen(3, a)), (1, seen(2, a))];
    let mut state = BalloonVisibilityState::default();
    Frame::new(&both(false))
        .at(0.0)
        .talk_started()
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Millis(
            3000,
        )))
        .ended_at(5.0)
        .run(&mut state);
    let started = Frame::new(&both(true)).at(5.0).run(&mut state);
    assert_eq!(state.deadline, Some(8.0));
    assert_eq!(started.timeout_notice, None);

    let before = Frame::new(&both(true)).at(7.999).run(&mut state);
    assert_eq!((before.actions, before.timeout_notice), (vec![], None));
    let expired = Frame::new(&both(true)).at(8.0).run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0, 1])]);
    assert_eq!(expired.timeout_notice, Some(TALK_ID));

    let after = Frame::new(&both(false)).at(8.001).run(&mut state);
    assert_eq!(
        (after.actions, after.logs, after.timeout_notice),
        (vec![], vec![], None),
        "知らせは時間切れで隠したフレームにだけ立つ"
    );
}

/// 同じトークで最後に届いた「なし」が勝ち、そのトークは既定の満了のちょうどを過ぎても
/// 隠れず、計測の記録も知らせも出ない（要件 9.2・9.5）。
#[test]
fn never_given_last_keeps_the_balloon_and_raises_no_notice() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(3, false))])
        .at(0.0)
        .talk_started()
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Millis(
            3000,
        )))
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Never))
        .ended_at(2.0)
        .run(&mut state);
    let default_deadline = 2.0 + DEFAULT_BALLOON_TIMEOUT_SECS;
    let instants = [
        2.0,
        2.0 + 3.0,
        default_deadline - 0.001,
        default_deadline,
        default_deadline + 0.001,
    ];
    for now in instants {
        let frame = Frame::new(&[(0, seen(3, true))]).at(now).run(&mut state);
        assert_eq!(
            (frame.actions, frame.logs, frame.timeout_notice),
            (vec![], vec![], None),
            "時刻 {now}"
        );
        assert_eq!(state.deadline, None, "時刻 {now}");
    }
}

/// 抑止が解けた後の計り直しは、そのトークで差し替えた待ち時間を使う（既定の 30 秒ではない）。
/// 抑止の間は知らせが立たず、解けて隠れたフレームに立つ（要件 2.4・5.5・9.7）。
#[test]
fn restart_after_suppression_uses_the_talk_timeout() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(3, false))])
        .at(0.0)
        .talk_started()
        .signal(TalkLifecycleSignal::BalloonTimeout(TalkTimeout::Millis(
            2000,
        )))
        .ended_at(1.0)
        .run(&mut state);
    Frame::new(&[(0, seen(3, true))])
        .at(1.0)
        .dragging()
        .run(&mut state);
    assert_eq!(state.deadline, Some(3.0));

    let held = Frame::new(&[(0, seen(3, true))])
        .at(3.0)
        .dragging()
        .run(&mut state);
    assert_eq!((held.actions, held.timeout_notice), (vec![], None));

    let restarted = Frame::new(&[(0, seen(3, true))]).at(10.0).run(&mut state);
    assert_eq!(
        restarted.logs,
        vec![VisibilityLogEvent::MeasurementRestarted {
            now: 10.0,
            deadline: 12.0,
        }]
    );
    assert_eq!(restarted.timeout_notice, None);

    let before = Frame::new(&[(0, seen(3, true))]).at(11.999).run(&mut state);
    assert_eq!((before.actions, before.timeout_notice), (vec![], None));
    let expired = Frame::new(&[(0, seen(3, true))]).at(12.0).run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.timeout_notice, Some(TALK_ID));
}

// ---------------------------------------------------------------------------
// 知らせ（要件 2.1・2.5）
// ---------------------------------------------------------------------------

/// 満了のちょうどのフレームでも、利用者の中断・内容の消去・次のトークの開始で隠れた（あるいは
/// 計測を捨てた）なら知らせは立たない（要件 2.5）。
#[test]
fn notice_is_not_raised_when_the_balloon_goes_for_another_reason() {
    let deadline = 2.0 + DEFAULT_BALLOON_TIMEOUT_SECS;

    let mut state = shown_and_ended(2.0);
    let broke = Frame::new(&[(0, seen(3, true))])
        .at(deadline)
        .signal(TalkLifecycleSignal::UserBreak)
        .run(&mut state);
    assert_eq!(
        broke.actions,
        vec![VisibilityAction::HideScopes {
            scopes: vec![0],
            trigger: VisibilityTrigger::UserBreak,
        }]
    );
    assert_eq!(broke.timeout_notice, None, "利用者の中断");

    let mut state = shown_and_ended(2.0);
    let cleared = Frame::new(&[(0, seen(0, true))])
        .at(deadline)
        .run(&mut state);
    assert_eq!(
        cleared.actions,
        vec![VisibilityAction::HideScopes {
            scopes: vec![0],
            trigger: VisibilityTrigger::Clear,
        }]
    );
    assert_eq!(cleared.timeout_notice, None, "内容の消去");

    let mut state = shown_and_ended(2.0);
    let next = Frame::new(&[(0, seen(3, true))])
        .at(deadline)
        .talk_started()
        .run(&mut state);
    assert_eq!(next.actions, vec![]);
    assert_eq!(
        next.logs,
        vec![VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::TalkStarted,
            deadline,
        }]
    );
    assert_eq!(next.timeout_notice, None, "次のトークの開始");
}

/// 番号の無いトークが時間切れで隠れると、隠す行動はそのままで知らせは立たず、警告の事象が
/// 出る。次のトークの番号が届けば、知らせはその番号で立つ（要件 2.1）。
#[test]
fn notice_carries_the_current_talk_id_and_is_withheld_without_one() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(3, false))])
        .at(0.0)
        .signal(TalkLifecycleSignal::TalkStarted { talk_id: None })
        .ended_at(2.0)
        .run(&mut state);
    Frame::new(&[(0, seen(3, true))]).at(2.0).run(&mut state);
    let deadline = 2.0 + DEFAULT_BALLOON_TIMEOUT_SECS;
    let expired = Frame::new(&[(0, seen(3, true))])
        .at(deadline)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.timeout_notice, None);
    assert_eq!(
        expired.logs,
        vec![
            VisibilityLogEvent::Transition {
                scope: 0,
                trigger: VisibilityTrigger::Timeout,
                visible: false,
            },
            VisibilityLogEvent::TimeoutNoticeWithoutTalkId,
        ]
    );

    // 次のトークは番号 8 で始まり、時間切れの知らせはその番号で立つ。
    let next_id = TalkId(8);
    Frame::new(&[(0, seen(0, false))])
        .at(0.0)
        .signal(TalkLifecycleSignal::TalkStarted {
            talk_id: Some(next_id),
        })
        .run(&mut state);
    Frame::new(&[(0, seen(4, false))])
        .at(1.0)
        .ended_at(1.0)
        .run(&mut state);
    let next_deadline = 1.0 + DEFAULT_BALLOON_TIMEOUT_SECS;
    let next = Frame::new(&[(0, seen(4, true))])
        .at(next_deadline)
        .run(&mut state);
    assert_eq!(next.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(next.timeout_notice, Some(next_id));
    assert!(
        !next
            .logs
            .contains(&VisibilityLogEvent::TimeoutNoticeWithoutTalkId)
    );
}
