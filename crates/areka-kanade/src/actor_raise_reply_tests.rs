//! 汎用の通知の入口の返事（[`RaiseOutcome`]）の檻（areka-P0-ghost-install 要件 2.6・2.7・11.5・12.6）。
//!
//! 本物の殻（[`spawn_kanade_with_stop_sink`]）を偽の shiori のスレッドにつなぎ、`KanadeMsg::RaiseEvent`
//! に付けた返信端へ 5 値が 1 つずつ返ることを見る: 台本・空の台本・空白だけ・204・NOTIFY の完了・
//! 失敗・起動の途中（定常の前）・許可表に無い名前。返信端が無い依頼は今日どおり送られ、受け手が
//! 居なくなった返信端でも運行が続くことも見る。
//!
//! 待ちはすべて受信端の受け取りで揃える（返事・運行の通知・偽の shiori の記録）。期限は
//! ハングを失敗に変えるための上限であり、実時間の経過に結果は依らない。

use super::*;
use crate::change::{RaiseOutcome, ShioriMethod};
use crate::schedule::log_capture::{capture, logged_once};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tracing::Level;

/// ハングを失敗に変えるための受け取りの上限。
pub(super) const LIMIT: Duration = Duration::from_secs(10);

/// テストで送るイベント（許可表のインストール系の 1 語）。
pub(super) const RAISED: &str = "OnInstallCompleteEx";

/// 偽の shiori が `RAISED` へ返す応答を作る（`ShioriOutcome` は複製できないので毎回作る）。
pub(super) type Answer = Box<dyn Fn() -> ShioriOutcome + Send>;

/// 本物の殻と偽の shiori・再生系の受信端・運行の通知の受信端の組。
pub(super) struct Rig {
    pub(super) kanade: Sender<KanadeMsg>,
    pub(super) handle: ActorHandle,
    /// 偽の shiori が受けた呼び出しの要約（`GET id`・`NOTIFY id`・`Unload`・`Close`）。
    pub(super) calls: Receiver<String>,
    pub(super) talks: Receiver<TalkCommand>,
    pub(super) notices: Receiver<KanadeNotice>,
}

/// 偽の shiori を立てて kanade を起こす。`RAISED` 以外は良性の既定応答（NOTIFY→完了・GET→204・
/// Unload→完了）を返す。
pub(super) fn spawn_rig(answer: Answer) -> Rig {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (calls_tx, calls) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        while let Ok(msg) = shiori_rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    let (label, id, outcome) = match &call {
                        ShioriCall::Get { id, .. } => {
                            ("GET", id.as_str(), ShioriOutcome::NoContent)
                        }
                        ShioriCall::Notify { id, .. } => {
                            ("NOTIFY", id.as_str(), ShioriOutcome::Notified)
                        }
                    };
                    let _ = calls_tx.send(format!("{label} {id}"));
                    let outcome = match id {
                        RAISED => answer(),
                        // 終了の相で再生を待たせるため、`OnClose` には台本を返す。
                        "OnClose" => ShioriOutcome::Value(r"\h\s[0]またね\e".into()),
                        _ => outcome,
                    };
                    let _ = reply.send(outcome);
                }
                ShioriMsg::Unload { reply } => {
                    let _ = calls_tx.send("Unload".to_string());
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                ShioriMsg::Close => {
                    let _ = calls_tx.send("Close".to_string());
                    break;
                }
            }
        }
    });
    let (talk_tx, talks) = mpsc::channel::<TalkCommand>();
    let (notice_tx, notices) = mpsc::channel::<KanadeNotice>();
    let (kanade, handle) = spawn_kanade_with_stop_sink(
        KanadeConfig::new("master", "1.0.0"),
        shiori_tx,
        talk_tx,
        Box::new(|_, _| {}),
        Some(notice_tx),
    );
    Rig {
        kanade,
        handle,
        calls,
        talks,
        notices,
    }
}

/// 起動して定常の通知を受けるまで待ち、起動系列の呼び出しの記録を読み捨てる。
pub(super) fn boot_to_steady(rig: &Rig) {
    rig.kanade.send(KanadeMsg::Boot).expect("kanade に届く");
    loop {
        match rig.notices.recv_timeout(LIMIT).expect("定常の通知が届く") {
            KanadeNotice::Steady => break,
            other => panic!("定常の前に想定外の通知: {other:?}"),
        }
    }
    // 定常の通知は起動系列の往復がすべて済んだ後に出るので、ここで残りを読み捨てられる。
    while rig.calls.try_recv().is_ok() {}
}

/// 返信端つきで `id` を送り、返事を受け取る。
fn raise(rig: &Rig, id: &str, method: ShioriMethod) -> RaiseOutcome {
    let (reply, rx) = areka_actor::reply_channel::<RaiseOutcome>();
    rig.kanade
        .send(KanadeMsg::RaiseEvent {
            id: id.to_string(),
            references: vec!["ghost".to_string(), "name".to_string()],
            method,
            reply: Some(reply),
        })
        .expect("kanade に届く");
    rx.recv_timeout(LIMIT).expect("返事がちょうど 1 回届く")
}

/// 今までに偽の shiori が受けた呼び出し（待たない）。
fn drain(calls: &Receiver<String>) -> Vec<String> {
    calls.try_iter().collect()
}

/// 終わらせる（Close は運行表を経ず即時に止まる）。
pub(super) fn close(rig: Rig) {
    rig.kanade.send(KanadeMsg::Close).expect("kanade に届く");
    rig.handle.join().expect("kanade が止まる");
}

/// 空でない台本は `Script`。返事は動作（再生の起動）を実行し終えた後に届く。
#[test]
fn script_reply_is_script_after_the_talk_is_started() {
    let rig = spawn_rig(Box::new(|| {
        ShioriOutcome::Value("\\h\\s[0]入れてくれてありがとう\\e".into())
    }));
    boot_to_steady(&rig);

    assert_eq!(raise(&rig, RAISED, ShioriMethod::Get), RaiseOutcome::Script);
    // 台詞の再生の前に OnTranslate を 1 回送る（偽の shiori は既定の 204）。
    assert_eq!(
        drain(&rig.calls),
        vec![format!("GET {RAISED}"), "GET OnTranslate".to_string()]
    );
    assert!(
        matches!(rig.talks.try_recv(), Ok(TalkCommand::Start(_))),
        "返事の前に再生の起動が済んでいる"
    );
    close(rig);
}

/// 空の台本・空白だけの台本・204 はどれも `NoReply`。依頼のイベントの往復は 1 回で、1 文字以上の
/// 空白だけの台本にだけ OnTranslate が続く（翻訳は 1 文字以上の台詞にかける）。
#[test]
fn empty_blank_and_no_content_are_no_reply() {
    let answers: [(&str, Answer, &[&str]); 3] = [
        (
            "空の台本",
            Box::new(|| ShioriOutcome::Value(String::new())),
            &[],
        ),
        (
            "空白だけの台本",
            Box::new(|| ShioriOutcome::Value(" \t\r\n\u{3000}".into())),
            &["GET OnTranslate"],
        ),
        ("204", Box::new(|| ShioriOutcome::NoContent), &[]),
    ];
    for (label, answer, translated) in answers {
        let rig = spawn_rig(answer);
        boot_to_steady(&rig);
        assert_eq!(
            raise(&rig, RAISED, ShioriMethod::Get),
            RaiseOutcome::NoReply,
            "{label} は返事なし"
        );
        let mut expected = vec![format!("GET {RAISED}")];
        expected.extend(translated.iter().map(|call| call.to_string()));
        assert_eq!(drain(&rig.calls), expected, "{label}");
        close(rig);
    }
}

/// NOTIFY の完了も `NoReply`。
#[test]
fn notify_completion_is_no_reply() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::Notified));
    boot_to_steady(&rig);
    assert_eq!(
        raise(&rig, RAISED, ShioriMethod::Notify),
        RaiseOutcome::NoReply
    );
    assert_eq!(drain(&rig.calls), vec![format!("NOTIFY {RAISED}")]);
    close(rig);
}

/// 往復の失敗は `Failed`。kanade は今日どおり終了系列へ進み、返事はその後（降ろして止まった後）に届く。
#[test]
fn failed_round_trip_is_failed_after_the_fault_sequence() {
    let rig = spawn_rig(Box::new(|| {
        ShioriOutcome::Failed(ShioriFailure::Ipc("helper gone".into()))
    }));
    boot_to_steady(&rig);

    assert_eq!(raise(&rig, RAISED, ShioriMethod::Get), RaiseOutcome::Failed);
    // 同期の往復（送った GET・降ろす）は返事の前に済んでいる（偽の shiori は記録してから応える）。
    // 閉じるは届く時刻の保証が無いので、ここでは先頭の 2 件だけを待たずに読む。
    let before_reply: Vec<String> = (0..2)
        .map(|_| rig.calls.try_recv().expect("返事の前に記録されている"))
        .collect();
    assert_eq!(
        before_reply,
        vec![format!("GET {RAISED}"), "Unload".to_string()],
        "返事の前に降ろす往復まで済んでいる"
    );
    rig.handle.join().expect("kanade は自分で止まる");
    // 閉じる（`ShioriMsg::Close`）は投げるだけで応えを待たないので、届いたことだけを受信端で見る。
    assert_eq!(
        rig.calls.recv_timeout(LIMIT).expect("閉じるが送られた"),
        "Close"
    );
}

/// 起動の途中（定常の前）は `NotSteady` で、SHIORI へは何も送らない。
#[test]
fn before_steady_is_not_steady_and_nothing_is_sent() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::NoContent));
    assert_eq!(
        raise(&rig, RAISED, ShioriMethod::Get),
        RaiseOutcome::NotSteady
    );
    assert!(drain(&rig.calls).is_empty(), "送っていない");
    close(rig);
}

/// 終了の挨拶の再生を待つ相（定常でない）でも `NotSteady` で、SHIORI へは何も送らない。
#[test]
fn close_talk_wait_is_not_steady_and_nothing_is_sent() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::NoContent));
    boot_to_steady(&rig);
    rig.kanade
        .send(KanadeMsg::CloseRequest {
            reason: crate::msg::CloseReason::System,
        })
        .expect("kanade に届く");
    assert_eq!(
        raise(&rig, RAISED, ShioriMethod::Get),
        RaiseOutcome::NotSteady
    );
    // 終了の挨拶は送られて再生が始まり、依頼のイベントは送られていない。
    assert_eq!(
        drain(&rig.calls),
        vec!["GET OnClose".to_string(), "GET OnTranslate".to_string()]
    );
    assert!(
        matches!(rig.talks.try_recv(), Ok(TalkCommand::Start(_))),
        "終了の挨拶の再生を待っている"
    );
    close(rig);
}

/// 許可表に無い名前（`OnInstallReroute` を含む）は `NotAllowed` で、SHIORI へは何も送らない。
#[test]
fn name_outside_the_table_is_not_allowed_and_nothing_is_sent() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::NoContent));
    boot_to_steady(&rig);
    for id in ["OnInstallReroute", "OnTalk", "oninstallbegin"] {
        assert_eq!(
            raise(&rig, id, ShioriMethod::Get),
            RaiseOutcome::NotAllowed,
            "{id}"
        );
    }
    assert!(drain(&rig.calls).is_empty(), "送っていない");
    close(rig);
}

/// 返信端が無い依頼は今日どおり送られ、次の依頼も受ける。
#[test]
fn without_reply_the_event_is_sent_as_today() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::Value("\\h\\s[0]はい\\e".into())));
    boot_to_steady(&rig);

    rig.kanade
        .send(KanadeMsg::RaiseEvent {
            id: RAISED.to_string(),
            references: Vec::new(),
            method: ShioriMethod::Get,
            reply: None,
        })
        .expect("kanade に届く");
    // 次の依頼の返事が届けば、前の依頼の処理は済んでいる（受信は順に 1 件ずつ）。
    assert_eq!(raise(&rig, RAISED, ShioriMethod::Get), RaiseOutcome::Script);
    assert_eq!(
        drain(&rig.calls),
        vec![
            format!("GET {RAISED}"),
            "GET OnTranslate".to_string(),
            format!("GET {RAISED}"),
            "GET OnTranslate".to_string(),
        ]
    );
    close(rig);
}

/// 受け手が居なくなった返信端でも運行は続く（次の依頼の返事が届く）。
#[test]
fn dropped_receiver_does_not_stop_the_run() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::NoContent));
    boot_to_steady(&rig);

    let (reply, rx) = areka_actor::reply_channel::<RaiseOutcome>();
    drop(rx);
    rig.kanade
        .send(KanadeMsg::RaiseEvent {
            id: RAISED.to_string(),
            references: Vec::new(),
            method: ShioriMethod::Get,
            reply: Some(reply),
        })
        .expect("kanade に届く");
    assert_eq!(
        raise(&rig, RAISED, ShioriMethod::Get),
        RaiseOutcome::NoReply
    );
    close(rig);
}

/// 受け手が居ない返信端へ送ると `debug!` を 1 件残す（呼出スレッドで同期に走る送り口を直接見る）。
#[test]
fn dropped_receiver_leaves_one_debug_record() {
    let (reply, rx) = areka_actor::reply_channel::<RaiseOutcome>();
    drop(rx);
    let events = capture(|| send_raise_reply(reply, RAISED, RaiseOutcome::Script));
    let debug = logged_once(&events, Level::DEBUG, "raise_reply_dropped");
    assert_eq!(debug.fields.get("id").map(String::as_str), Some(RAISED));
}
