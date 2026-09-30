//! 台詞の切れ目の口の返信端の檻（areka-P0-shell-balloon-switch 要件 1.14・2.3・5.7・8.11）。
//!
//! 本物の殻（[`spawn_kanade_with_stop_sink`]）を偽の shiori につなぎ（汎用の入口の檻の組み立てを
//! 借りる）、`KanadeMsg::AwaitTalkGap` の返信端へ結果がちょうど 1 回届くこと・見張りの最中に
//! kanade を止めると受け手に「送り手が落ちた」が見えること・2 つ目の依頼で古い返信端が捨てられる
//! ことを見る。
//!
//! 「まだ届いていない」は、後から送った依頼（空のリソース照会）の返事を受けた後に覗いて確かめる
//! （受信は順に 1 件ずつなので、その時点で前の依頼の処理は済んでいる）。期限はハングを失敗に
//! 変えるための上限であり、実時間の経過に結果は依らない。

use super::raise_reply_tests::{LIMIT, RAISED, Rig, boot_to_steady, close, spawn_rig};
use super::*;
use crate::change::{ShioriMethod, TalkGap};
use crate::schedule::log_capture::{assert_not_logged, capture, logged_once};
use crate::talk::{TalkDone, TalkEndReason, TalkId};
use areka_actor::ReplyReceiver;
use tracing::Level;

/// 印の無い依頼を送り、結果の受信端を返す。
fn await_gap(rig: &Rig) -> ReplyReceiver<TalkGap> {
    let (reply, rx) = reply_channel::<TalkGap>();
    rig.kanade
        .send(KanadeMsg::AwaitTalkGap { raise: None, reply })
        .expect("kanade に届く");
    rx
}

/// 前に送った依頼の処理が済むまで待つ（殻がその場で答える空のリソース照会の返事で揃える）。
fn settle(rig: &Rig) {
    let (reply, rx) = reply_channel();
    rig.kanade
        .send(KanadeMsg::ResourceQuery {
            ids: Vec::new(),
            reply,
        })
        .expect("kanade に届く");
    rx.recv_timeout(LIMIT).expect("照会の返事が届く");
}

/// `RAISED` に台本を返させて再生を始め、再生中のトークの番号を返す。
fn start_talk(rig: &Rig) -> TalkId {
    let (reply, rx) = reply_channel();
    rig.kanade
        .send(KanadeMsg::RaiseEvent {
            id: RAISED.to_string(),
            references: Vec::new(),
            method: ShioriMethod::Get,
            reply: Some(reply),
        })
        .expect("kanade に届く");
    rx.recv_timeout(LIMIT).expect("返事が届く");
    match rig.talks.try_recv() {
        Ok(TalkCommand::Start(start)) => start.talk_id,
        _ => panic!("返事の前に再生の起動が済んでいる"),
    }
}

fn talking_rig() -> (Rig, TalkId) {
    let rig = spawn_rig(Box::new(|| {
        ShioriOutcome::Value("\\h\\s[0]しゃべっている\\e".into())
    }));
    boot_to_steady(&rig);
    let talk = start_talk(&rig);
    (rig, talk)
}

fn talk_ended(rig: &Rig, talk_id: TalkId) {
    rig.kanade
        .send(KanadeMsg::TalkDone(TalkDone {
            talk_id,
            reason: TalkEndReason::Ended,
            quit_reserved: false,
        }))
        .expect("kanade に届く");
}

#[test]
fn outcome_arrives_exactly_once_when_idle() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::NoContent));
    boot_to_steady(&rig);
    let rx = await_gap(&rig);
    settle(&rig);
    assert_eq!(
        rx.try_recv().expect("送り手は結果を送ってから消える"),
        Some(TalkGap::Reached { marked: None }),
        "依頼の処理の後に結果が届いている"
    );
    settle(&rig);
    assert!(
        matches!(rx.try_recv(), Err(ReplyError::Dropped)),
        "2 回目は無い（送り手は 1 回送って消えた）"
    );
    close(rig);
}

#[test]
fn outcome_arrives_at_the_end_of_the_playing_talk() {
    let (rig, talk) = talking_rig();
    let rx = await_gap(&rig);
    settle(&rig);
    assert!(
        matches!(rx.try_recv(), Ok(None)),
        "再生中はまだ届かない（送り手は生きている）"
    );
    talk_ended(&rig, talk);
    assert_eq!(
        rx.recv_timeout(LIMIT).expect("台詞の終わりで届く"),
        TalkGap::Reached { marked: None }
    );
    close(rig);
}

#[test]
fn stopping_kanade_while_watching_drops_the_sender() {
    let (rig, _talk) = talking_rig();
    let rx = await_gap(&rig);
    settle(&rig);
    close(rig);
    assert!(
        matches!(rx.recv_timeout(LIMIT), Err(ReplyError::Dropped)),
        "止まれば受け手には「送り手が落ちた」が見える"
    );
}

#[test]
fn second_request_drops_the_first_sender_and_watches_the_new_one() {
    let (rig, talk) = talking_rig();
    let first = await_gap(&rig);
    let second = await_gap(&rig);
    settle(&rig);
    assert!(
        matches!(first.try_recv(), Err(ReplyError::Dropped)),
        "古い返信端は捨てられた"
    );
    assert!(matches!(second.try_recv(), Ok(None)), "新しい依頼を見張る");
    talk_ended(&rig, talk);
    assert_eq!(
        second.recv_timeout(LIMIT).expect("新しい依頼に届く"),
        TalkGap::Reached { marked: None }
    );
    close(rig);
}

/// 2 つ目の依頼で `warn!` を 1 件残す（呼出スレッドで同期に走る控えの口を直接見る）。
#[test]
fn replacing_the_held_sender_leaves_one_warn() {
    let (first, _rx1) = reply_channel::<TalkGap>();
    let (second, _rx2) = reply_channel::<TalkGap>();
    let mut slot = None;

    let ev = capture(|| hold_gap_reply(&mut slot, first));
    assert_not_logged(&ev, "talk_gap_replaced");

    let ev = capture(|| hold_gap_reply(&mut slot, second));
    logged_once(&ev, Level::WARN, "talk_gap_replaced");
    assert_eq!(
        ev.iter().filter(|e| e.level == Level::WARN).count(),
        1,
        "warn! はちょうど 1 件"
    );
}
