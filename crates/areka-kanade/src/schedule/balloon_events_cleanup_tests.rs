//! 完了を通らずに相が変わる経路での控えの後始末の檻（areka-P0-balloon-lifecycle-events タスク 2.6）。
//!
//! 中断の控え（`State.user_break_talk`）・預かった時間切れの知らせ（`ShownTalk.timeout_pending`）を
//! 持ったまま、完了の知らせより先に強制の終了・SHIORI の故障・切替の送り出しの台詞の期限切れ・翻訳の
//! 輸送路の失敗・別の応答による置き換えが来たら、相は表 B を通らずに進む。どの経路でも `step` の
//! 出口の掃除（`settle` の ⑴）が控えを捨て、捨てた控えごとに理由 `talk_gone` を 1 行記録し、3 つの
//! イベントはその後の相の終わりまで 1 本も送られないことを固定する（要件 1.5・1.6・2.6・4.4・7.2）。
//! 経路ごとに新しい分岐は無く、掃除の条件「控えの相手が現行トークでない」1 つで効く。
//!
//! 預かりを持てない経路（切替の送り出しの台詞の間・翻訳へ預けている間）は、その入口で時間切れの
//! 知らせが預からずに断られることを同じ檻の中で確かめ、中断の控えだけを持たせて駆動する。

use super::super::log_capture::{CapturedEvent, capture};
use super::super::{Input, Phase, TermCause};
use super::BreakNote;
use super::done_tests::{done, field};
use super::tests::{
    FINAL, ORIGINAL, cfg, feed, reply_value, start_translated, started, steady_none,
};
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget};
use crate::msg::{
    CloseReason, MonotonicMs, MouseButton, MouseEventKind, MouseInput, ShioriCall, ShioriDownKind,
    ShioriFailure, ShioriOutcome,
};
use crate::schedule::{Action, State};
use crate::talk::{TalkEndReason, TalkId};

/// 完了の前に持たせる控えの組。
#[derive(Debug, Clone, Copy)]
enum Held {
    /// 中断の控えだけ。
    Break,
    /// 預かった時間切れの知らせだけ。
    Timeout,
    /// 両方（時間切れを預かった後に中断した）。
    Both,
}

impl Held {
    const ALL: [Held; 3] = [Held::Break, Held::Timeout, Held::Both];

    /// 捨てた記録に出るイベント名（掃除が記録する順）。
    fn ids(self) -> &'static [&'static str] {
        match self {
            Held::Break => &["OnBalloonBreak"],
            Held::Timeout => &["OnBalloonTimeout"],
            Held::Both => &["OnBalloonBreak", "OnBalloonTimeout"],
        }
    }
}

/// 定常でトークを 1 つ再生し、`held` の控えを持たせた状態（相手はそのトーク）。
fn holding(held: Held) -> (State, TalkId) {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    if matches!(held, Held::Timeout | Held::Both) {
        feed(&mut s, Input::BalloonTimeout { talk_id: talk });
        assert!(
            s.shown.as_ref().is_some_and(|shown| shown.timeout_pending),
            "前提: 完了の前の時間切れの知らせを預かっている（表 D の D5）"
        );
    }
    if matches!(held, Held::Break | Held::Both) {
        feed(&mut s, Input::UserBreak { scope: 1 });
        assert_eq!(
            s.user_break_talk,
            Some(BreakNote {
                talk_id: talk,
                scope: 1
            }),
            "前提: 中断の控えを持っている"
        );
    }
    (s, talk)
}

/// 入力を順に入れ、捕捉と指示の列（すべての入力の分をつないだもの）を返す。
fn feed_all(s: &mut State, inputs: Vec<Input>) -> (Vec<Action>, Vec<CapturedEvent>) {
    let mut out = Vec::new();
    let ev = capture(|| {
        for input in inputs {
            out.extend(feed(s, input));
        }
    });
    (out, ev)
}

/// 指示の列に 3 つのイベントの要求が無い（GET・NOTIFY のどちらでも）。
fn assert_no_balloon_request(actions: &[Action], at: &str) {
    let ids: Vec<&str> = actions
        .iter()
        .filter_map(|a| match a {
            Action::ShioriRequest(ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. }) => {
                Some(id.as_str())
            }
            _ => None,
        })
        .filter(|id| id.starts_with("OnBalloon"))
        .collect();
    assert!(
        ids.is_empty(),
        "{at}: 3 つのイベントは送らない。送った={ids:?}"
    );
}

/// 送らなかった記録の全行（イベント名・理由・トークの番号・相）。
fn not_sent_lines(ev: &[CapturedEvent]) -> Vec<[Option<&str>; 4]> {
    ev.iter()
        .filter(|e| e.target == "kanade" && e.event.as_deref() == Some("balloon_event_not_sent"))
        .map(|e| {
            [
                field(e, "id"),
                field(e, "reason"),
                field(e, "talk_id"),
                field(e, "phase"),
            ]
        })
        .collect()
}

/// 捨てた控えごとに `talk_gone` がちょうど 1 行ずつ出て、他の送らなかった記録・送った記録は無い。
/// 控えはどちらも下りている。
fn assert_dropped(s: &State, ev: &[CapturedEvent], held: Held, talk: TalkId, phase: &str) {
    let talk = talk.0.to_string();
    let expected: Vec<[Option<&str>; 4]> = held
        .ids()
        .iter()
        .map(|id| {
            [
                Some(*id),
                Some("talk_gone"),
                Some(talk.as_str()),
                Some(phase),
            ]
        })
        .collect();
    assert_eq!(
        not_sent_lines(ev),
        expected,
        "{held:?}: 捨てた控えごとに理由 talk_gone を 1 行（要件 7.2）。\n捕捉={ev:#?}"
    );
    assert!(
        !ev.iter()
            .any(|e| e.event.as_deref() == Some("balloon_event_sent")),
        "{held:?}: 送った記録は出ない。\n捕捉={ev:#?}"
    );
    assert!(s.user_break_talk.is_none(), "{held:?}: 中断の控えは下りる");
    assert!(
        s.shown.as_ref().is_none_or(|shown| !shown.timeout_pending),
        "{held:?}: 預かりは下りる"
    );
}

/// その後の入力では 3 つのイベントを送らず、送った・送らなかったの記録も出ない（捨て直さない）。
fn assert_quiet_after(s: &mut State, inputs: Vec<Input>, at: &str) {
    let (actions, ev) = feed_all(s, inputs);
    assert_no_balloon_request(&actions, at);
    assert!(
        ev.iter().all(|e| !matches!(
            e.event.as_deref(),
            Some("balloon_event_sent" | "balloon_event_not_sent")
        )),
        "{at}: 捨てた後は何も記録しない。\n捕捉={ev:#?}"
    );
}

/// 降ろし終えた応答。
fn unloaded() -> Input {
    Input::ShioriReply {
        outcome: ShioriOutcome::Unloaded,
        origin: "Unload",
    }
}

/// 終了系列へ進んだ後に、止めたトークの完了（遅れて届く）と降ろし終えた応答を入れても送らず、止まる。
fn finish_unloading(s: &mut State, talk: TalkId, at: &str) {
    assert_quiet_after(
        s,
        vec![done(talk, TalkEndReason::Interrupted, false), unloaded()],
        at,
    );
    assert!(matches!(s.phase, Phase::Stopped), "{at}: 止まる");
}

/// 強制の終了・SHIORI の故障: 控えを持ったまま終了系列へ直行すると、その `step` の出口で捨てて
/// 記録し、遅れて届く完了・降ろし終えた応答まで 3 つのイベントを送らない（要件 1.6・4.4・7.2）。
#[test]
fn force_quit_and_shiori_down_drop_held_notes_with_one_talk_gone_line_each() {
    let cases: [(&str, fn() -> Input); 2] = [
        ("ForceQuit", || Input::ForceQuit {
            reason: CloseReason::System,
        }),
        ("ShioriDown", || Input::ShioriDown {
            kind: ShioriDownKind::HelperExited,
            reason: "helper crashed".to_string(),
        }),
    ];
    for (label, input) in cases {
        for held in Held::ALL {
            let at = format!("{label}・{held:?}");
            let (mut s, talk) = holding(held);
            let (actions, ev) = feed_all(&mut s, vec![input()]);
            assert!(
                matches!(
                    s.phase,
                    Phase::Unloading {
                        cause: TermCause::Forced | TermCause::Fault(_)
                    }
                ),
                "{at}: 終了系列へ進む"
            );
            assert_no_balloon_request(&actions, &at);
            assert_dropped(&s, &ev, held, talk, "Unloading");
            finish_unloading(&mut s, talk, &at);
        }
    }
}

/// 切替を始め、送り出しの台詞（`closing` が偽なら `OnGhostChanging`・真なら切替の `OnClose`）を
/// 再生している状態。
fn change_talking(closing: bool) -> (State, TalkId) {
    let mut s = steady_none();
    feed(
        &mut s,
        Input::ChangeGhost(ChangeRequest {
            target: ChangeTarget {
                sakura_name: "ポスト".to_string(),
                name: "R_POST_and_KOMAINU".to_string(),
                dir: r"C:\areka\ghost\r_post".to_string(),
            },
            origin: ChangeOrigin::Manual,
            raise_event: true,
        }),
    );
    let origin = if closing {
        feed(
            &mut s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnGhostChanging",
            },
        );
        "OnClose"
    } else {
        "OnGhostChanging"
    };
    let actions = reply_value(&mut s, ORIGINAL, origin);
    assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    let actions = feed(&mut s, Input::TranslateDone(Ok(FINAL.to_string())));
    let (talk, _) = started(&actions);
    assert!(
        matches!(
            s.phase,
            Phase::ChangeTalkWait { talk_id, .. } | Phase::ChangeCloseTalkWait { talk_id, .. }
                if talk_id == talk
        ),
        "前提: 送り出しの台詞の完了待ち"
    );
    (s, talk)
}

/// 切替の送り出しの台詞の期限切れ: 中断の控えを持ったまま期限を過ぎて降ろすと、その `step` の出口
/// で捨てて記録し、遅れて届く中断の完了・降ろし終えた応答まで送らない（要件 1.5・1.6・7.2）。
/// この相では時間切れの知らせは定常でないとして断られ（表 D の D1）、預かりは持てない。
#[test]
fn change_talk_deadline_drops_the_break_note_with_one_talk_gone_line() {
    for closing in [false, true] {
        let at = if closing {
            "切替の OnClose の台詞"
        } else {
            "OnGhostChanging の台詞"
        };
        let (mut s, talk) = change_talking(closing);
        feed(&mut s, Input::BalloonTimeout { talk_id: talk });
        assert!(
            s.shown.as_ref().is_some_and(|shown| !shown.timeout_pending),
            "{at}: 送り出しの台詞の間は預からない（D1）"
        );
        feed(&mut s, Input::UserBreak { scope: 0 });
        assert!(s.user_break_talk.is_some(), "{at}: 前提: 中断の控え");

        let now = 1_000_000;
        let (mut actions, mut ev) = feed_all(
            &mut s,
            vec![Input::Tick {
                now: MonotonicMs(now),
            }],
        );
        if !matches!(s.phase, Phase::Unloading { .. }) {
            // 期限が未設定だった（最初の Tick で決まる）なら、もう 1 度だけ期限ちょうどの Tick を入れる。
            let (more, more_ev) = feed_all(
                &mut s,
                vec![Input::Tick {
                    now: MonotonicMs(now + cfg().close_talk_deadline_ms),
                }],
            );
            actions.extend(more);
            ev.extend(more_ev);
        }
        assert!(
            matches!(
                s.phase,
                Phase::Unloading {
                    cause: TermCause::DeadlineExceeded
                }
            ),
            "{at}: 期限切れで降ろす"
        );
        assert_no_balloon_request(&actions, at);
        assert_dropped(&s, &ev, Held::Break, talk, "Unloading");
        finish_unloading(&mut s, talk, at);
    }
}

/// 翻訳の輸送路の失敗: 次のトークの台本を翻訳へ預けている間（相は次のトークを再生中）に受け入れた
/// 中断の控えは、失敗で終了系列へ進んだ `step` の出口で捨てて記録する（要件 1.6・7.2）。
/// この間の時間切れの知らせは表 D の D2 で断られ、預かりは持てない。
#[test]
fn translate_transport_failure_drops_the_break_note_with_one_talk_gone_line() {
    let mut s = steady_none();
    feed(
        &mut s,
        Input::Tick {
            now: MonotonicMs(2_000),
        },
    );
    let actions = reply_value(&mut s, ORIGINAL, "OnSecondChange");
    assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    let Phase::Steady {
        talk: Some(ref active),
    } = s.phase
    else {
        panic!("前提: 翻訳を待つ間も相は次のトークを再生中");
    };
    let talk = active.talk_id;
    feed(&mut s, Input::BalloonTimeout { talk_id: talk });
    assert!(
        s.shown.as_ref().is_none_or(|shown| !shown.timeout_pending),
        "翻訳を待つ間は預からない（D2）"
    );
    feed(&mut s, Input::UserBreak { scope: 1 });
    assert_eq!(
        s.user_break_talk.map(|note| note.talk_id),
        Some(talk),
        "前提: 翻訳を待つトークへの中断の控え"
    );

    let (actions, ev) = feed_all(
        &mut s,
        vec![Input::TranslateDone(Err(ShioriFailure::Ipc(
            "通信が切れた".to_string(),
        )))],
    );
    assert!(
        matches!(
            s.phase,
            Phase::Unloading {
                cause: TermCause::Fault(_)
            }
        ),
        "輸送路の失敗は終了系列（Fault）へ"
    );
    assert_no_balloon_request(&actions, "翻訳の失敗");
    assert_dropped(&s, &ev, Held::Break, talk, "Unloading");
    assert_quiet_after(&mut s, vec![unloaded()], "降ろし終えた応答");
    assert!(matches!(s.phase, Phase::Stopped));
}

/// 別の応答による置き換え: 控えを持ったまま再生中のダブルクリックの応答がトークを置き換えると、
/// 置き換えの `step` の出口で捨てて記録する。その後に翻訳が通って次のトークが最後まで流れても、
/// 翻訳の輸送路が失敗して降ろしても、3 つのイベントは送らない（要件 1.5・2.6・7.2）。
#[test]
fn replacement_drops_held_notes_then_neither_translation_outcome_sends() {
    for translated in [true, false] {
        for held in Held::ALL {
            let at = format!(
                "{held:?}・翻訳の{}",
                if translated { "成功" } else { "失敗" }
            );
            let (mut s, old) = holding(held);
            feed(
                &mut s,
                Input::Mouse(MouseInput {
                    scope: 0,
                    x: 10,
                    y: 20,
                    region: Some("Head".to_string()),
                    kind: MouseEventKind::DoubleClick {
                        button: MouseButton::Left,
                    },
                }),
            );
            let mut actions = Vec::new();
            let ev = capture(|| actions = reply_value(&mut s, ORIGINAL, "OnMouseDoubleClick"));
            assert!(
                matches!(actions.as_slice(), [Action::Translate(_)]),
                "{at}: 応答は翻訳へ預けられ、今のトークを置き換える"
            );
            assert_dropped(&s, &ev, held, old, "Steady");

            if translated {
                let actions = feed(&mut s, Input::TranslateDone(Ok(FINAL.to_string())));
                let (next, _) = started(&actions);
                assert_ne!(next, old);
                assert_quiet_after(
                    &mut s,
                    vec![
                        done(old, TalkEndReason::Interrupted, false),
                        done(next, TalkEndReason::Ended, false),
                    ],
                    &at,
                );
                assert!(
                    matches!(s.phase, Phase::Steady { talk: None }),
                    "{at}: 次のトークを終えて定常へ戻る"
                );
            } else {
                assert_quiet_after(
                    &mut s,
                    vec![Input::TranslateDone(Err(ShioriFailure::Ipc(
                        "通信が切れた".to_string(),
                    )))],
                    &at,
                );
                assert!(
                    matches!(
                        s.phase,
                        Phase::Unloading {
                            cause: TermCause::Fault(_)
                        }
                    ),
                    "{at}: 輸送路の失敗は終了系列（Fault）へ"
                );
                finish_unloading(&mut s, old, &at);
            }
        }
    }
}
