//! トークの完了の後の判断（表 B）の檻（areka-P0-balloon-lifecycle-events タスク 2.3）。
//!
//! 表 B の全行と、表 A の各腕（完了の振り分け）から表 B への到達を、最上位の `step` を直接駆動して
//! 固定する。時刻は `Tick` で注入する。B2 の「終了の要求を保留している」は完了の振り分けからは
//! 届かない（保留の終了は定常の完了が必ず消化する）ので、判断の入口を直に呼んで固定する。

use super::super::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use super::super::{ActiveTalk, Input, Phase};
use super::tests::{FINAL, ORIGINAL, feed, reply_value, start_translated, started, steady_none};
use super::{ShownTalk, after_talk_done};
use crate::change::{
    CancelReason, ChangeOrigin, ChangeRequest, ChangeTarget, GapRaise, KanadeNotice, ShioriMethod,
};
use crate::msg::{CloseReason, MonotonicMs, ShioriCall, ShioriOutcome};
use crate::schedule::{Action, State};
use crate::talk::{TalkDone, TalkEndReason, TalkId};
use tracing::Level;

/// 完了の知らせを組む。
pub(super) fn done(talk_id: TalkId, reason: TalkEndReason, quit_reserved: bool) -> Input {
    Input::TalkDone(TalkDone {
        talk_id,
        reason,
        quit_reserved,
    })
}

/// 入力を捕捉つきで入れる。
pub(super) fn feed_capturing(s: &mut State, input: Input) -> (Vec<Action>, Vec<CapturedEvent>) {
    let mut out = Vec::new();
    let ev = capture(|| out = feed(s, input));
    (out, ev)
}

/// 指示の列の中の GET（ID と Reference）。
pub(super) fn gets(actions: &[Action]) -> Vec<(&str, &[String])> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => {
                Some((id.as_str(), references.as_slice()))
            }
            _ => None,
        })
        .collect()
}

/// 3 つのイベントの GET が 1 本も無い。
pub(super) fn assert_no_balloon_event(actions: &[Action], at: &str) {
    assert!(
        gets(actions)
            .iter()
            .all(|(id, _)| !id.starts_with("OnBalloon")),
        "{at}: バルーンの 3 つのイベントは送らない"
    );
}

/// 記録のフィールドの値。
pub(super) fn field<'a>(line: &'a CapturedEvent, name: &str) -> Option<&'a str> {
    line.fields.get(name).map(String::as_str)
}

/// 送らなかった記録がちょうど 1 行あり、イベント名と理由と相が期待どおり。
pub(super) fn assert_not_sent(ev: &[CapturedEvent], id: &str, reason: &str, phase: &str) {
    let line = logged_once(ev, Level::INFO, "balloon_event_not_sent");
    assert_eq!(
        (
            field(line, "id"),
            field(line, "reason"),
            field(line, "phase")
        ),
        (Some(id), Some(reason), Some(phase)),
        "送らなかったイベント名・理由・完了の後の相を記録する（要件 7.2）。\n捕捉={ev:#?}"
    );
}

/// 定常でトークを再生し、中断を受け入れた状態（scope 付き）。
fn steady_broken(scope: u32) -> (State, TalkId) {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    let actions = feed(&mut s, Input::UserBreak { scope });
    assert!(
        matches!(actions.as_slice(), [Action::CancelChoice { talk_id }] if *talk_id == talk),
        "前提: 中断は受け入れられている"
    );
    (s, talk)
}

// --- B3: 中断で止まって定常へ戻ったら OnBalloonBreak ---

/// 定常のトークを中断して止まり終えたら、定常へ戻った同じ `step` で `OnBalloonBreak` を 1 本送る
/// （Reference0＝翻訳の後の最終の台本・Reference1＝scope・Reference2＝空。要件 1.1〜1.5・7.1）。
#[test]
fn b3_break_then_steady_sends_balloon_break() {
    let (mut s, talk) = steady_broken(1);
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, false));
    assert!(
        matches!(s.phase, Phase::Steady { talk: None }),
        "定常へ戻る"
    );
    assert_eq!(
        gets(&actions),
        vec![(
            "OnBalloonBreak",
            &[FINAL.to_string(), "1".to_string(), String::new()][..]
        )],
        "指示は OnBalloonBreak の GET 1 本だけ"
    );
    let line = logged_once(&ev, Level::INFO, "balloon_event_sent");
    assert_eq!(
        (
            field(line, "id"),
            field(line, "cause"),
            field(line, "scope"),
            field(line, "talk_id")
        ),
        (
            Some("OnBalloonBreak"),
            Some("break"),
            Some("1"),
            Some(talk.0.to_string().as_str())
        ),
        "イベント名・きっかけ・scope・トークの番号を 1 行で記録する（要件 7.1）。\n捕捉={ev:#?}"
    );
    assert_not_logged(&ev, "balloon_event_not_sent");
}

/// 応答が台本なら翻訳を通って新しいトークが始まり、204 なら定常のまま何もしない（要件 4.2・4.3）。
#[test]
fn b3_reply_script_is_translated_and_204_stays_steady() {
    let (mut s, talk) = steady_broken(0);
    feed(&mut s, done(talk, TalkEndReason::Interrupted, false));
    let actions = reply_value(&mut s, ORIGINAL, "OnBalloonBreak");
    assert!(
        matches!(actions.as_slice(), [Action::Translate(req)] if req.source.id.as_str() == "OnBalloonBreak"),
        "台本の応答は元のイベント OnBalloonBreak として翻訳へ預ける"
    );
    let actions = feed(&mut s, Input::TranslateDone(Ok(FINAL.to_string())));
    let (next, script) = started(&actions);
    assert_ne!(next, talk, "新しい番号のトークが始まる");
    assert_eq!(script, FINAL);
    assert!(matches!(s.phase, Phase::Steady { talk: Some(_) }));

    let (mut s, talk) = steady_broken(0);
    feed(&mut s, done(talk, TalkEndReason::Interrupted, false));
    let actions = feed(
        &mut s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnBalloonBreak",
        },
    );
    assert!(actions.is_empty(), "204 は何も再生しない");
    assert!(
        matches!(s.phase, Phase::Steady { talk: None }),
        "定常のまま"
    );
}

// --- B4: 中断を出したがトークが自分で最後まで流れていた（設計 D7） ---

/// 中断の控えがあって `Ended` で終わったら `OnBalloonClose`（Reference0＝控えの台本）を 1 本送り、
/// 送り済みの印を立てる。印が既に立っていれば送らず理由を記録する（要件 3.1 の補足・4.5）。
#[test]
fn b4_break_but_ended_sends_balloon_close_once() {
    let (mut s, talk) = steady_broken(0);
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Ended, false));
    assert_eq!(
        gets(&actions),
        vec![("OnBalloonClose", &[FINAL.to_string()][..])]
    );
    assert!(s.shown.as_ref().is_some_and(|shown| shown.close_sent));
    let line = logged_once(&ev, Level::INFO, "balloon_event_sent");
    assert_eq!(field(line, "cause"), Some("close"), "捕捉={ev:#?}");
    assert_eq!(field(line, "scope"), None, "scope は中断のときだけ載せる");

    let (mut s, talk) = steady_broken(0);
    s.shown.as_mut().expect("控えがある").close_sent = true;
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Ended, false));
    assert_no_balloon_event(&actions, "送り済み");
    assert_not_sent(&ev, "OnBalloonClose", "already_sent", "Steady");
}

// --- B5: 預かった時間切れの知らせ ---

/// 預かりがあれば完了で `OnBalloonTimeout`（Reference1＝`0`）を 1 本送り、送り済みの印を立てて
/// 預かりを下ろす。送り済みなら送らず理由を記録する（要件 2.1〜2.3・4.5）。
#[test]
fn b5_pending_timeout_is_sent_on_completion_once() {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    s.shown.as_mut().expect("控えがある").timeout_pending = true;
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Ended, false));
    assert_eq!(
        gets(&actions),
        vec![(
            "OnBalloonTimeout",
            &[FINAL.to_string(), "0".to_string()][..]
        )]
    );
    let shown = s.shown.as_ref().expect("控えは残る");
    assert!(shown.timeout_sent && !shown.timeout_pending);
    let line = logged_once(&ev, Level::INFO, "balloon_event_sent");
    assert_eq!(field(line, "cause"), Some("timeout"), "捕捉={ev:#?}");

    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    let shown = s.shown.as_mut().expect("控えがある");
    shown.timeout_pending = true;
    shown.timeout_sent = true;
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Ended, false));
    assert_no_balloon_event(&actions, "送り済み");
    assert_not_sent(&ev, "OnBalloonTimeout", "already_sent", "Steady");
    assert!(
        !s.shown.as_ref().expect("控えは残る").timeout_pending,
        "預かりは下ろす"
    );
}

/// B3 と B5 が同時なら B3 を採り、預かりを捨てたことを記録する（`talk_gone` とは取り違えない）。
#[test]
fn b3_wins_over_b5_and_drops_the_pending_timeout() {
    let (mut s, talk) = steady_broken(1);
    s.shown.as_mut().expect("控えがある").timeout_pending = true;
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, false));
    assert_eq!(
        gets(&actions).iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec!["OnBalloonBreak"]
    );
    assert_not_sent(&ev, "OnBalloonTimeout", "superseded_by_break", "Steady");
    let shown = s.shown.as_ref().expect("控えは残る");
    assert!(!shown.timeout_pending && !shown.timeout_sent);
}

// --- B1: 控えも預かりも無い ---

/// 普通の完了は何もせず、3 つのイベントの記録も出さない。
#[test]
fn b1_plain_completion_does_nothing() {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Ended, false));
    assert!(actions.is_empty());
    assert_not_logged(&ev, "balloon_event_sent");
    assert_not_logged(&ev, "balloon_event_not_sent");
}

/// 選択肢の時間切れの解除で終わったトークは中断の控えが無いので B1 に落ち、`OnBalloonBreak` は
/// 出ない（要件 2.7・設計の行き違いの表）。
#[test]
fn b1_choice_timeout_release_sends_no_balloon_break() {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    feed(
        &mut s,
        Input::ChoiceWaiting {
            talk_id: talk,
            choice_ids: vec!["OnMenu".to_string()],
            display_end: MonotonicMs(3_000),
            timeout_directive_secs: Some(1.0),
        },
    );
    let actions = feed(
        &mut s,
        Input::Tick {
            now: MonotonicMs(10_000),
        },
    );
    assert_eq!(
        gets(&actions).iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec!["OnChoiceTimeout"],
        "前提: 選択肢の時間切れを送る"
    );
    let actions = feed(
        &mut s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceTimeout",
        },
    );
    assert!(
        matches!(actions.as_slice(), [Action::CancelChoice { talk_id }] if *talk_id == talk),
        "前提: 204 で選択待ちを解除してトークを止める"
    );
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, false));
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
    assert_no_balloon_event(&actions, "選択肢の時間切れの解除");
    assert_not_logged(&ev, "balloon_event_sent");
}

// --- 表 A の各腕から: 送る（切替の中止の知らせが GET より先） ---

/// 定常からゴーストの切替を受理する（`OnGhostChanging` の応答待ちへ）。
fn accept_change(s: &mut State) {
    feed(
        s,
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
}

/// 台本の応答を翻訳して再生を始める。
fn reply_and_start(s: &mut State, origin: &'static str) -> TalkId {
    let actions = reply_value(s, ORIGINAL, origin);
    assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    let actions = feed(s, Input::TranslateDone(Ok(FINAL.to_string())));
    started(&actions).0
}

/// 中断して止まり終えた結果が「切替の中止の知らせ → OnBalloonBreak」の順であることを確かめる。
fn assert_cancelled_then_break(s: &mut State, talk: TalkId, scope: u32) {
    feed(s, Input::UserBreak { scope });
    let actions = feed(s, done(talk, TalkEndReason::Interrupted, true));
    assert!(
        matches!(s.phase, Phase::Steady { talk: None }),
        "定常へ戻る"
    );
    match actions.as_slice() {
        [
            Action::Notice(KanadeNotice::ChangeCancelled {
                reason: CancelReason::UserBreak,
            }),
            Action::ShioriRequest(ShioriCall::Get { id, references, .. }),
        ] => {
            assert_eq!(id.as_str(), "OnBalloonBreak");
            assert_eq!(
                references,
                &vec![FINAL.to_string(), scope.to_string(), String::new()]
            );
        }
        _ => panic!("切替の中止の知らせが GET より先に並ぶはず（要件 1.1）"),
    }
}

/// ゴーストの切替の送り出しの台詞（`OnGhostChanging`）を中断したら、切替を中止して定常へ戻り
/// `OnBalloonBreak` を送る（終了の予約は切替の相では効かない。要件 1.1・設計 E）。
#[test]
fn a4_ghost_changing_talk_break_sends_after_cancel_notice() {
    let mut s = steady_none();
    accept_change(&mut s);
    let talk = reply_and_start(&mut s, "OnGhostChanging");
    assert!(matches!(s.phase, Phase::ChangeTalkWait { .. }));
    assert_cancelled_then_break(&mut s, talk, 0);
}

/// 切替の `OnClose` の別れの台詞を中断しても、切替の中止で定常へ戻るので送る（要件 1.6 の後段）。
#[test]
fn a4_change_farewell_talk_break_sends_after_cancel_notice() {
    let mut s = steady_none();
    accept_change(&mut s);
    feed(
        &mut s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnGhostChanging",
        },
    );
    let talk = reply_and_start(&mut s, "OnClose");
    assert!(matches!(s.phase, Phase::ChangeCloseTalkWait { .. }));
    assert_cancelled_then_break(&mut s, talk, 1);
}

/// シェルの切替の台詞（`OnShellChanging`）の中断は、終了の予約があっても定常へ戻るので送る
/// （A2 は印の中断で外れる。要件 1.1）。
#[test]
fn a4_shell_changing_talk_break_sends() {
    let mut s = steady_none();
    feed(
        &mut s,
        Input::AwaitTalkGap {
            raise: Some(GapRaise {
                id: "OnShellChanging".to_string(),
                references: vec!["次のシェル".to_string()],
                method: ShioriMethod::Get,
            }),
        },
    );
    let talk = reply_and_start(&mut s, "OnShellChanging");
    feed(&mut s, Input::UserBreak { scope: 0 });
    let actions = feed(&mut s, done(talk, TalkEndReason::Interrupted, true));
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
    assert_eq!(
        gets(&actions),
        vec![(
            "OnBalloonBreak",
            &[FINAL.to_string(), "0".to_string(), String::new()][..]
        )]
    );
}

// --- 表 A の各腕から: 送らない（理由が残る） ---

/// 止めた台本が終了を予約していたら終了へ進み、送らない（A2・要件 1.6）。
#[test]
fn a2_break_quit_sends_nothing_and_records_why() {
    let (mut s, talk) = steady_broken(0);
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, true));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert_not_sent(&ev, "OnBalloonBreak", "not_steady", "Unloading");
}

/// 終了の握手の別れの台詞を中断したら終了へ進み、送らない（要件 1.6）。
#[test]
fn a4_close_farewell_break_sends_nothing() {
    let mut s = steady_none();
    feed(
        &mut s,
        Input::CloseRequest {
            reason: CloseReason::System,
        },
    );
    let talk = reply_and_start(&mut s, "OnClose");
    assert!(matches!(s.phase, Phase::CloseTalkWait { .. }));
    feed(&mut s, Input::UserBreak { scope: 0 });
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, false));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert_not_sent(&ev, "OnBalloonBreak", "not_steady", "Unloading");
}

/// 保留の終了があれば終了の握手へ進み、送らない（要件 1.6）。
#[test]
fn a4_pending_close_sends_nothing() {
    let (mut s, talk) = steady_broken(0);
    feed(
        &mut s,
        Input::CloseRequest {
            reason: CloseReason::System,
        },
    );
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, false));
    assert_no_balloon_event(&actions, "保留の終了");
    assert_not_sent(&ev, "OnBalloonBreak", "not_steady", "ClosePending");
}

/// 保留の切替があれば切替を始め、送らない（A3・要件 1.6）。
#[test]
fn a3_pending_change_sends_nothing() {
    let (mut s, talk) = steady_broken(0);
    accept_change(&mut s);
    assert!(s.pending_change.is_some(), "前提: 再生中の切替は保留される");
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, false));
    assert_no_balloon_event(&actions, "保留の切替");
    assert_not_sent(&ev, "OnBalloonBreak", "not_steady", "ChangePending");
}

/// 起動の途中の相のまま挨拶の完了が届いたら起動の途中のままで、送らない（守りの腕・要件 4.4）。
#[test]
fn a4_boot_greeting_break_sends_nothing() {
    let mut s = State {
        phase: Phase::BootVersion {
            talk: Some(ActiveTalk {
                talk_id: TalkId(3),
                origin: "OnBoot",
                script: FINAL.to_string(),
            }),
        },
        next_talk_id: 4,
        ..steady_none()
    };
    s.shown = Some(ShownTalk {
        talk_id: TalkId(3),
        script: FINAL.to_string(),
        close_sent: false,
        timeout_sent: false,
        timeout_pending: false,
    });
    feed(&mut s, Input::UserBreak { scope: 0 });
    let (actions, ev) = feed_capturing(&mut s, done(TalkId(3), TalkEndReason::Interrupted, false));
    assert!(matches!(s.phase, Phase::BootVersion { talk: None }));
    assert_no_balloon_event(&actions, "起動の途中");
    assert_not_sent(&ev, "OnBalloonBreak", "not_steady", "BootVersion");
}

/// B2 の後段: 完了の後が再生中のトークの無い定常でも、終了の要求を保留していれば送らない（設計 D8）。
/// 完了の振り分けからは届かない組なので、判断の入口を直に呼ぶ。きっかけごとに理由を残す。
#[test]
fn b2_close_pending_sends_nothing_for_each_cause() {
    let mut s = steady_none();
    s.pending_close = Some(CloseReason::System);
    s.shown = Some(ShownTalk {
        talk_id: TalkId(3),
        script: FINAL.to_string(),
        close_sent: false,
        timeout_sent: false,
        timeout_pending: true,
    });
    let note = super::BreakNote {
        talk_id: TalkId(3),
        scope: 0,
    };
    let completed = TalkDone {
        talk_id: TalkId(3),
        reason: TalkEndReason::Interrupted,
        quit_reserved: false,
    };
    let mut out = None;
    let ev = capture(|| out = Some(after_talk_done(s, Vec::new(), &completed, Some(note))));
    let (s, actions) = out.expect("結果を返す");
    assert!(actions.is_empty());
    let lines: Vec<_> = ev
        .iter()
        .filter(|e| e.event.as_deref() == Some("balloon_event_not_sent"))
        .map(|e| (field(e, "id"), field(e, "reason")))
        .collect();
    assert_eq!(
        lines,
        vec![
            (Some("OnBalloonBreak"), Some("close_pending")),
            (Some("OnBalloonTimeout"), Some("close_pending")),
        ],
        "きっかけごとに理由を 1 行ずつ。\n捕捉={ev:#?}"
    );
    assert!(
        !s.shown.expect("控えは残る").timeout_pending,
        "預かりは下ろす"
    );
}

// --- 控えが無い・番号が食い違う（構造上は起きない・設計 D9） ---

/// 控えが無ければ Reference0 を空で送り、警告を残す。番号が食い違えば同じく空で送り、誤りを残す。
#[test]
fn missing_or_mismatched_shown_sends_empty_script_with_a_warning() {
    for (label, shown, level) in [
        ("控えなし", None, Level::WARN),
        (
            "番号の食い違い",
            Some(ShownTalk {
                talk_id: TalkId(2),
                script: "\\0別のトーク\\e".to_string(),
                close_sent: false,
                timeout_sent: false,
                timeout_pending: false,
            }),
            Level::ERROR,
        ),
    ] {
        let mut s = State {
            phase: Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(3),
                    origin: "OnSecondChange",
                    script: FINAL.to_string(),
                }),
            },
            ..steady_none()
        };
        s.shown = shown;
        feed(&mut s, Input::UserBreak { scope: 0 });
        let (actions, ev) =
            feed_capturing(&mut s, done(TalkId(3), TalkEndReason::Interrupted, false));
        assert_eq!(
            gets(&actions),
            vec![(
                "OnBalloonBreak",
                &[String::new(), "0".to_string(), String::new()][..]
            )],
            "{label}: Reference0 を空で送る"
        );
        let line = logged_once(&ev, level, "balloon_event_script_missing");
        assert_eq!(field(line, "id"), Some("OnBalloonBreak"), "{label}");
    }
}

// --- 中断の側が預かりより勝つ・受け入れられない合図・`\-` への到達 ---

/// B4 と B5 が同時（中断を出したがトークは自分で終わり、時間切れの知らせも預かっていた）なら、
/// 上の行の B4 を採って `OnBalloonClose` を 1 本だけ送り、預かりを捨てた記録を 1 行残す。
#[test]
fn b4_wins_over_b5_and_drops_the_pending_timeout() {
    let (mut s, talk) = steady_broken(0);
    s.shown.as_mut().expect("控えがある").timeout_pending = true;
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Ended, false));
    assert_eq!(
        gets(&actions),
        vec![("OnBalloonClose", &[FINAL.to_string()][..])],
        "GET は OnBalloonClose の 1 本だけ"
    );
    assert_not_sent(&ev, "OnBalloonTimeout", "superseded_by_break", "Steady");
    let shown = s.shown.as_ref().expect("控えは残る");
    assert!(!shown.timeout_pending && !shown.timeout_sent && shown.close_sent);
}

/// 同じトークへの 2 回目の中断の合図は控えを増やさず、完了で `OnBalloonBreak` は 1 本だけ
/// （要件 1.7・1.8）。
#[test]
fn second_break_signal_still_sends_one_balloon_break() {
    let (mut s, talk) = steady_broken(1);
    let actions = feed(&mut s, Input::UserBreak { scope: 0 });
    assert!(actions.is_empty(), "2 回目の合図は何も出さない");
    assert_eq!(
        s.user_break_talk,
        Some(super::BreakNote {
            talk_id: talk,
            scope: 1
        }),
        "控えは 1 回目のまま（scope も書き換えない）"
    );
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Interrupted, false));
    assert_eq!(
        gets(&actions),
        vec![(
            "OnBalloonBreak",
            &[FINAL.to_string(), "1".to_string(), String::new()][..]
        )],
        "OnBalloonBreak は 1 本だけ"
    );
    logged_once(&ev, Level::INFO, "balloon_event_sent");
}

/// 中断を出したがトークが `\-` に辿り着いて終わった（`Quit`）なら、終了へ進むので送らない。
/// 中断で止まったのではないので、記録のイベント名は `OnBalloonClose` 側に載る（要件 1.6・7.2）。
#[test]
fn break_note_with_quit_completion_sends_nothing() {
    let (mut s, talk) = steady_broken(0);
    let (actions, ev) = feed_capturing(&mut s, done(talk, TalkEndReason::Quit, false));
    assert!(matches!(s.phase, Phase::Unloading { .. }), "終了へ進む");
    assert!(gets(&actions).is_empty(), "GET は 1 本も無い");
    assert_not_sent(&ev, "OnBalloonClose", "not_steady", "Unloading");
}
