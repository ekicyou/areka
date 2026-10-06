//! 切替の中止と終了要求への譲りのテスト（areka-P0-ghost-shell-balloon-switch 要件 2.9・5.1〜5.4・5.6・5.7・10.4）。
//!
//! 送り出しの台詞を利用者が中断したら（`\-` の予約が在っても）切替を中止して定常へ戻ること、
//! 中止のあとの終了要求が今日どおり `OnClose` を出すこと、切替の 4 相に届いた終了要求が切替を
//! 取りやめて今日の終了の握手へ合流し `OnClose` を二度送らないことを `step` 経由で固定する。

use super::*;

const USER: CloseReason = CloseReason::User { scope: 0 };

/// 利用者の中断を受けてから、そのトークの `TalkDone{Interrupted}` を返す。
/// 止める指示は今日の中断と同じ単一の閉じ口（`CancelChoice`）であることも確かめる。
fn break_talk(s: State, talk_id: TalkId, quit_reserved: bool) -> (State, Vec<Action>) {
    let (s, actions) = step(s, Input::UserBreak { scope: 0 }, &cfg());
    assert!(
        matches!(actions.as_slice(), [Action::CancelChoice { talk_id: t }] if *t == talk_id),
        "中断の指示は今日と同じ（バルーンを隠す規則に触れない）"
    );
    let done = TalkDone {
        talk_id,
        reason: TalkEndReason::Interrupted,
        quit_reserved,
    };
    step(s, Input::TalkDone(done), &cfg())
}

fn close_request(s: State) -> (State, Vec<Action>) {
    step(s, Input::CloseRequest { reason: USER }, &cfg())
}

fn cancel_notices(actions: &[Action], reason: CancelReason) -> usize {
    actions
        .iter()
        .filter(|a| matches!(a, Action::Notice(KanadeNotice::ChangeCancelled { reason: r }) if *r == reason))
        .count()
}

fn on_close_sends(actions: &[Action]) -> usize {
    sent_events(actions)
        .iter()
        .filter(|id| id.as_str() == "OnClose")
        .count()
}

fn is_unloading(s: &State) -> bool {
    matches!(s.phase, Phase::Unloading { .. } | Phase::Stopped)
}

// ---- 利用者の中断で中止（要件 5.1・5.2・5.4・5.6・5.7） ----

#[test]
fn user_break_in_either_farewell_cancels_back_to_steady_even_with_quit_reserved() {
    type Enter = fn() -> (State, TalkId);
    let enters: [(&str, Enter); 2] = [
        ("changing", || talking(r"\0じゃあね\-")),
        ("close", close_talking),
    ];
    for (label, enter) in enters {
        for quit_reserved in [true, false] {
            let (s, talk_id) = enter();
            let mut out = None;
            let ev = capture(|| out = Some(break_talk(s, talk_id, quit_reserved)));
            let (s, actions) = out.expect("step は必ず結果を返す");
            logged_once(&ev, Level::INFO, "change_cancelled");
            assert!(
                ev.iter()
                    .all(|e| e.event.as_deref() != Some("talk_done_break_quit")),
                "{label}: 切替の相では `\\-` の予約を終了へ結ばない"
            );
            // 中止の通知が先、その後に定常へ戻った中断の `OnBalloonBreak` の GET が 1 本
            // （降ろす要求は出ない・areka-P0-balloon-lifecycle-events 要件 1.1・設計 E）。
            assert!(
                matches!(
                    actions.first(),
                    Some(Action::Notice(KanadeNotice::ChangeCancelled {
                        reason: CancelReason::UserBreak
                    }))
                ) && sent_events(&actions[1..]) == ["OnBalloonBreak"]
                    && actions.len() == 2,
                "{label}/{quit_reserved}: 中止の通知の後に OnBalloonBreak の GET 1 本だけ"
            );
            assert!(
                matches!(s.phase, Phase::Steady { talk: None }),
                "{label}: 元の定常へ戻る"
            );
            assert!(!is_unloading(&s), "{label}: 降ろす相へ進まない");
            assert!(
                s.change.is_none() && s.pending_change.is_none(),
                "{label}: 切替の帳簿は消える"
            );
            assert!(s.user_break_talk.is_none(), "{label}: 中断の帳簿も空");

            // 中止のあとの終了要求は今日どおり OnClose を出す（要件 5.6）。
            let (after, actions) = close_request(s);
            assert_eq!(only_get(&actions).0, "OnClose", "{label}");
            assert!(matches!(after.phase, Phase::ClosePending { .. }), "{label}");
        }
    }
}

#[test]
fn new_change_request_is_accepted_right_after_cancel() {
    let (s, talk_id) = talking("x");
    let (s, _) = break_talk(s, talk_id, false);
    let (s, actions) = step(
        s,
        Input::ChangeGhost(req(ChangeOrigin::Automatic, true)),
        &cfg(),
    );
    assert!(
        matches!(s.phase, Phase::ChangePending),
        "直後の新しい切替を受け付ける"
    );
    assert_eq!(only_get(&actions).0, "OnGhostChanging");
}

// ---- 終了要求への譲り（要件 2.9） ----

/// 終了要求を送り、取りやめ（通知 1 件・`info!(change_yield_to_close)`）を確かめて返す。
fn yield_to_close(s: State) -> State {
    let mut out = None;
    let ev = capture(|| out = Some(close_request(s)));
    let (s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::INFO, "change_yield_to_close");
    assert!(
        matches!(
            actions.as_slice(),
            [Action::Notice(KanadeNotice::ChangeCancelled {
                reason: CancelReason::CloseRequest
            })]
        ),
        "取りやめの通知だけ（その場で OnClose は送らない）"
    );
    assert!(
        s.change.is_none() && s.pending_change.is_none(),
        "切替の帳簿は消える"
    );
    s
}

#[test]
fn close_request_while_waiting_changing_reply_waits_for_it_then_closes() {
    // 台本が返った: 台詞を定常のトークとして最後まで流し、その完了で OnClose。
    let (s, _) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let s = yield_to_close(s);
    assert!(
        matches!(s.phase, Phase::ChangePending),
        "応答を見るまで相は維持"
    );
    assert!(matches!(
        s.pending_close,
        Some(CloseReason::User { scope: 0 })
    ));
    // 2 件目の終了要求で通知を重ねない。
    let (s, again) = close_request(s);
    assert_eq!(cancel_notices(&again, CancelReason::CloseRequest), 0);
    assert!(again.is_empty());
    let (s, actions) = reply(s, ShioriOutcome::Value("bye".to_string()));
    assert!(
        matches!(actions.as_slice(), [Action::StartTalk(t)] if t.talk_id == TalkId(5) && t.script == "bye")
    );
    assert!(
        matches!(&s.phase, Phase::Steady { talk: Some(t) } if t.talk_id == TalkId(5) && t.origin == "OnGhostChanging")
    );
    let (s, actions) = talk_done(s, TalkId(5), TalkEndReason::Ended);
    assert_eq!(on_close_sends(&actions), 1, "OnClose はここで 1 件");
    assert!(matches!(
        s.phase,
        Phase::ClosePending {
            reason: CloseReason::User { scope: 0 }
        }
    ));

    // 204 が返った: そのまま今日の OnClose の握手へ。
    let (s, _) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let s = yield_to_close(s);
    let (s, actions) = reply(s, ShioriOutcome::NoContent);
    assert_eq!(only_get(&actions).0, "OnClose");
    assert!(matches!(
        s.phase,
        Phase::ClosePending {
            reason: CloseReason::User { scope: 0 }
        }
    ));
    assert!(s.pending_close.is_none(), "保留の終了は消化される");
}

#[test]
fn close_request_during_changing_talk_lets_it_finish_then_closes() {
    let (s, talk_id) = talking("long");
    let s = yield_to_close(s);
    assert!(
        matches!(&s.phase, Phase::Steady { talk: Some(t) } if t.talk_id == talk_id && t.script == "long")
    );
    assert!(matches!(
        s.pending_close,
        Some(CloseReason::User { scope: 0 })
    ));
    let (s, actions) = talk_done(s, talk_id, TalkEndReason::Ended);
    assert_eq!(on_close_sends(&actions), 1, "台詞の完了で OnClose を 1 件");
    assert!(matches!(s.phase, Phase::ClosePending { .. }));
}

#[test]
fn close_request_after_change_on_close_was_sent_never_sends_it_again() {
    // ChangeClosePending: 応答を待って今日の終了の握手の続きへ。
    let (s, _) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let (s, _) = reply(s, ShioriOutcome::NoContent);
    assert!(matches!(s.phase, Phase::ChangeClosePending));
    let s = yield_to_close(s);
    assert!(matches!(
        s.phase,
        Phase::ClosePending {
            reason: CloseReason::User { scope: 0 }
        }
    ));
    let (s, a1) = reply(s, ShioriOutcome::Value("bye".to_string()));
    let Phase::CloseTalkWait { talk_id, .. } = s.phase else {
        panic!("別れの台詞の待ちへ進むはず");
    };
    let (s, a2) = talk_done(s, talk_id, TalkEndReason::Ended);
    assert_eq!(
        on_close_sends(&a1) + on_close_sends(&a2),
        0,
        "OnClose の追加送出 0"
    );
    assert!(matches!(
        s.phase,
        Phase::Unloading {
            cause: TermCause::Quit
        }
    ));

    // ChangeCloseTalkWait: その別れの台詞の終わりで終了。
    let (s, talk_id) = close_talking();
    let Phase::ChangeCloseTalkWait { deadline, .. } = s.phase else {
        unreachable!()
    };
    let s = yield_to_close(s);
    assert!(
        matches!(s.phase, Phase::CloseTalkWait { talk_id: t, deadline: d } if t == talk_id && d == deadline)
    );
    let (s, actions) = talk_done(s, talk_id, TalkEndReason::Ended);
    assert_eq!(on_close_sends(&actions), 0, "OnClose の追加送出 0");
    assert!(matches!(
        s.phase,
        Phase::Unloading {
            cause: TermCause::Quit
        }
    ));
}
