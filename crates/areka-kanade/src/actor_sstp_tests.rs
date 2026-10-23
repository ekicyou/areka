//! SSTP の 2 つの入口の返事（[`crate::actor_sstp`]）の檻（areka-P0-mcp-kanade-tools 要件 1.3・2.3・3.6）。
//!
//! 本物の殻と偽の shiori（`actor_raise_reply_tests` の組）で、返信端へ値がちょうど 1 回返ることを見る。

use super::raise_reply_tests::{LIMIT, RAISED, boot_to_steady, close, spawn_rig};
use super::*;
use crate::sstp::{SstpNotifyOutcome, SstpSendOutcome};

/// 普段の状態の外からの台本は `Accepted`。
#[test]
fn send_in_steady_is_accepted() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::NoContent));
    boot_to_steady(&rig);
    let (reply, rx) = areka_actor::reply_channel::<SstpSendOutcome>();
    rig.kanade
        .send(KanadeMsg::SstpSend {
            script: r"\h\s[0]やあ\e".into(),
            strict: None,
            reply,
        })
        .expect("kanade に届く");
    assert_eq!(
        rx.recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::Accepted
    );
    close(rig);
}

/// 普段の状態で台本が返る外からのイベントは、SHIORI が返したままの `Script`。
#[test]
fn notify_in_steady_with_script_is_script_as_returned() {
    let rig = spawn_rig(Box::new(|| {
        ShioriOutcome::Value(r"\h\s[0]はい\e".into(), areka_talk::SecurityLevel::Local)
    }));
    boot_to_steady(&rig);
    let (reply, rx) = areka_actor::reply_channel::<SstpNotifyOutcome>();
    rig.kanade
        .send(KanadeMsg::SstpNotify {
            event: RAISED.into(),
            references: vec![],
            strict: None,
            reply,
        })
        .expect("kanade に届く");
    assert_eq!(
        rx.recv_timeout(LIMIT).expect("返事が届く"),
        SstpNotifyOutcome::Script(r"\h\s[0]はい\e".into())
    );
    close(rig);
}

/// 起動の前（普段の状態でない）の外からの台本は `NotAccepted`。
#[test]
fn send_before_steady_is_not_accepted() {
    let rig = spawn_rig(Box::new(|| ShioriOutcome::NoContent));
    let (reply, rx) = areka_actor::reply_channel::<SstpSendOutcome>();
    rig.kanade
        .send(KanadeMsg::SstpSend {
            script: r"\h\s[0]やあ\e".into(),
            strict: None,
            reply,
        })
        .expect("kanade に届く");
    assert_eq!(
        rx.recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::NotAccepted
    );
    close(rig);
}

// ---------------------------------------------------------------------------
// 4.2: 本物の殻と偽の shiori で 2 つの入口を固定する（要件 1.3・1.7・2.1・2.3・2.4・2.5・2.7・2.8・
// 3.6・3.7・3.8・6.1・6.2・6.5・7.1）。
//
// 待ちはすべて受信端の受け取り（返事・運行の通知・偽の shiori の記録・再生の起動）で揃える。返事は
// その依頼の一括の処理を実行し終えた後に送られるので、返事を受けた時点で偽の shiori の記録と再生の
// 起動は出そろっている（待たずに読める）。期限はハングを失敗に変えるための上限にすぎない。
// ---------------------------------------------------------------------------

use super::raise_reply_tests::Rig;
use crate::change::ShioriMethod;
use crate::msg::KanadeStopped;
use crate::schedule::Phase;
use crate::schedule::log_capture::{capture, logged_once};
use crate::status::ExecutionSnapshot;
use crate::talk::{TalkDone, TalkEndReason};
use areka_actor::ReplyReceiver;
use std::sync::mpsc::{self, Receiver};
use tracing::Level;

/// 外から頼む名前（許可の表に無い・`On` で始まらない）。
const OUTSIDE: &str = "UserDefinedX";
/// 外から頼んだイベントへ偽の shiori が返す台本（翻訳の前）。
const RETURNED: &str = r"\h\s[0]頼まれた台本\e";
/// 偽の shiori の `OnTranslate` の答え（翻訳の前の字と違う字で見分ける）。
const TRANSLATED: &str = r"\h\s[0]翻訳した台本\e";
/// 起動の挨拶の台本（`OnBoot` の答え）。
const GREETING: &str = r"\h\s[0]おはよう\e";

/// 名前ごとの偽の shiori の答え（`None` なら良性の既定: GET→204・NOTIFY→完了）。
type Respond = Box<dyn Fn(&str) -> Option<ShioriOutcome> + Send>;
/// 偽の shiori の答えを毎回作る手（`ShioriOutcome` は複製できない）。
type MakeOutcome = fn() -> ShioriOutcome;

/// 名前ごとに答えを選べる偽の shiori で本物の殻を起こす（記録の形は `raise_reply_tests` の組と同じ）。
fn spawn_with(respond: Respond) -> Rig {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (calls_tx, calls) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        while let Ok(msg) = shiori_rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    let (label, id, fallback) = match &call {
                        ShioriCall::Get { id, .. } => {
                            ("GET", id.as_str(), ShioriOutcome::NoContent)
                        }
                        ShioriCall::Notify { id, .. } => {
                            ("NOTIFY", id.as_str(), ShioriOutcome::Notified)
                        }
                    };
                    let _ = calls_tx.send(format!("{label} {id}"));
                    let _ = reply.send(respond(id).unwrap_or(fallback));
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

/// `OUTSIDE` へ `answer` を、`OnTranslate` へ `TRANSLATED` を返す偽の shiori。
fn outside_answers(answer: MakeOutcome) -> Respond {
    Box::new(move |id| match id {
        OUTSIDE => Some(answer()),
        "OnTranslate" => Some(ShioriOutcome::Value(
            TRANSLATED.into(),
            areka_talk::SecurityLevel::Local,
        )),
        _ => None,
    })
}

fn send_script(rig: &Rig) -> ReplyReceiver<SstpSendOutcome> {
    let (reply, rx) = areka_actor::reply_channel::<SstpSendOutcome>();
    rig.kanade
        .send(KanadeMsg::SstpSend {
            script: r"\h\s[0]外からの台本\e".into(),
            strict: None,
            reply,
        })
        .expect("kanade に届く");
    rx
}

fn send_notify(rig: &Rig, event: &str) -> ReplyReceiver<SstpNotifyOutcome> {
    let (reply, rx) = areka_actor::reply_channel::<SstpNotifyOutcome>();
    rig.kanade
        .send(KanadeMsg::SstpNotify {
            event: event.to_string(),
            references: vec!["r0".into(), "r1".into()],
            strict: None,
            reply,
        })
        .expect("kanade に届く");
    rx
}

/// 今までに偽の shiori が受けた呼び出し（待たない）。
fn drain(calls: &Receiver<String>) -> Vec<String> {
    calls.try_iter().collect()
}

/// 今までに再生の側へ届いた再生の開始の台本（待たない）。
fn started(talks: &Receiver<TalkCommand>) -> Vec<String> {
    talks
        .try_iter()
        .map(|command| match command {
            TalkCommand::Start(start) => start.script,
            other => panic!("再生の開始のほかは届かないはず: {other:?}"),
        })
        .collect()
}

/// 停止の通知（原因は Fault）が届き、kanade が自分で止まる。
fn stops_by_fault(rig: Rig) {
    match rig.notices.recv_timeout(LIMIT).expect("停止の通知が届く") {
        KanadeNotice::Stopped(KanadeStopped {
            cause: KanadeStopCause::Fault(_),
            ..
        }) => {}
        other => panic!("停止の通知（Fault）のはず: {other:?}"),
    }
    rig.handle.join().expect("kanade は自分で止まる");
}

/// 殻が止まらず次の依頼を受け付ける（後続の外からの台本が `Accepted`・停止の通知が無い）。
fn still_running(rig: &Rig) {
    assert_eq!(
        send_script(rig).recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::Accepted,
        "ゴーストは止まらず次の台本を受け付ける"
    );
    assert!(rig.notices.try_recv().is_err(), "停止の通知は出ていない");
}

/// ⑴ 外からの台本 → `Accepted`・再生の開始がちょうど 1 つ（翻訳の後の台本）・偽の shiori に `OnTranslate`。
#[test]
fn send_starts_exactly_one_talk_through_on_translate() {
    let rig = spawn_with(outside_answers(|| ShioriOutcome::NoContent));
    boot_to_steady(&rig);
    while rig.talks.try_recv().is_ok() {}

    assert_eq!(
        send_script(&rig).recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::Accepted
    );
    assert_eq!(drain(&rig.calls), vec!["GET OnTranslate".to_string()]);
    assert_eq!(
        started(&rig.talks),
        vec![TRANSLATED.to_string()],
        "再生の開始は 1 つで、台本は翻訳の後"
    );
    close(rig);
}

/// ⑵ 外から頼まれたイベント → 許可の表に無い名前の GET が偽の shiori に届き、返事は翻訳の前の台本。
/// 再生は翻訳の後の台本で始まる（返事と再生の字が違うことで、返事が翻訳の前だと見分ける）。
#[test]
fn notify_reaches_shiori_and_answers_the_script_before_translation() {
    let rig = spawn_with(outside_answers(|| {
        ShioriOutcome::Value(RETURNED.into(), areka_talk::SecurityLevel::Local)
    }));
    boot_to_steady(&rig);
    while rig.talks.try_recv().is_ok() {}

    assert_eq!(
        send_notify(&rig, OUTSIDE)
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::Script(RETURNED.into()),
        "返事は SHIORI が返したまま（翻訳の前）"
    );
    assert_eq!(
        drain(&rig.calls),
        vec![format!("GET {OUTSIDE}"), "GET OnTranslate".to_string()]
    );
    assert_eq!(started(&rig.talks), vec![TRANSLATED.to_string()]);
    close(rig);
}

/// ⑵ 204・空白だけ・エラー応答は `NoScript` で、何も再生しない（`OnTranslate` も送らない）。
#[test]
fn notify_without_script_is_no_script_and_starts_nothing() {
    let answers: [(&str, MakeOutcome); 3] = [
        ("204", || ShioriOutcome::NoContent),
        ("空白だけ", || {
            ShioriOutcome::Value(" \t\r\n\u{3000}".into(), areka_talk::SecurityLevel::Local)
        }),
        ("エラー応答", || {
            ShioriOutcome::Failed(ShioriFailure::Shiori("500 Internal Server Error".into()))
        }),
    ];
    for (label, answer) in answers {
        let rig = spawn_with(outside_answers(answer));
        boot_to_steady(&rig);
        while rig.talks.try_recv().is_ok() {}

        assert_eq!(
            send_notify(&rig, OUTSIDE)
                .recv_timeout(LIMIT)
                .expect("返事が届く"),
            SstpNotifyOutcome::NoScript,
            "{label}"
        );
        assert_eq!(drain(&rig.calls), vec![format!("GET {OUTSIDE}")], "{label}");
        assert!(started(&rig.talks).is_empty(), "{label} は何も再生しない");
        close(rig);
    }
}

/// ⑶ 起動の前は、どちらの入口も受け付けず偽の shiori へ何も送らない。
#[test]
fn before_boot_neither_entrance_is_accepted() {
    let rig = spawn_with(outside_answers(|| {
        ShioriOutcome::Value(RETURNED.into(), areka_talk::SecurityLevel::Local)
    }));
    assert_eq!(
        send_script(&rig).recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::NotAccepted
    );
    assert_eq!(
        send_notify(&rig, OUTSIDE)
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::NotAccepted
    );
    assert!(drain(&rig.calls).is_empty(), "送っていない");
    assert!(started(&rig.talks).is_empty(), "再生していない");
    close(rig);
}

/// ⑶ 起動の挨拶が流れている間は受け付けず、挨拶の `TalkDone` の後は受け付ける。
#[test]
fn boot_greeting_refuses_until_it_ends() {
    let rig = spawn_with(Box::new(|id| match id {
        "OnBoot" => Some(ShioriOutcome::Value(
            GREETING.into(),
            areka_talk::SecurityLevel::Local,
        )),
        OUTSIDE => Some(ShioriOutcome::Value(
            RETURNED.into(),
            areka_talk::SecurityLevel::Local,
        )),
        _ => None,
    }));
    boot_to_steady(&rig);
    let greeting = match rig.talks.try_recv() {
        Ok(TalkCommand::Start(start)) => {
            assert_eq!(start.script, GREETING);
            start.talk_id
        }
        other => panic!("定常の前に起動の挨拶の再生が始まっているはず: {other:?}"),
    };

    assert_eq!(
        send_script(&rig).recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::NotAccepted,
        "挨拶の最中の外からの台本"
    );
    assert_eq!(
        send_notify(&rig, OUTSIDE)
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::NotAccepted,
        "挨拶の最中の外からのイベント"
    );
    assert!(drain(&rig.calls).is_empty(), "挨拶の最中は送っていない");
    assert!(started(&rig.talks).is_empty(), "挨拶の最中は再生していない");

    // 受信は順に 1 件ずつなので、後から送った依頼は挨拶の終わりの処理の後に処理される。
    rig.kanade
        .send(KanadeMsg::TalkDone(TalkDone {
            talk_id: greeting,
            reason: TalkEndReason::Ended,
            quit_reserved: false,
        }))
        .expect("kanade に届く");
    assert_eq!(
        send_script(&rig).recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::Accepted,
        "挨拶の後の外からの台本"
    );
    assert_eq!(
        send_notify(&rig, OUTSIDE)
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::Script(RETURNED.into()),
        "挨拶の後の外からのイベント"
    );
    close(rig);
}

/// ⑶b 検査を破った外からのイベント（名前に LF）を殻へ直に送る → 偽の shiori に何も届かず
/// `NotAccepted`。ゴーストは止まらない（後続の台本を受け付ける）。
#[test]
fn invalid_notify_sent_directly_is_not_accepted_and_the_ghost_keeps_running() {
    let rig = spawn_with(outside_answers(|| {
        ShioriOutcome::Value(RETURNED.into(), areka_talk::SecurityLevel::Local)
    }));
    boot_to_steady(&rig);

    assert_eq!(
        send_notify(&rig, "On\nX")
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::NotAccepted
    );
    assert!(drain(&rig.calls).is_empty(), "偽の shiori に何も届かない");
    still_running(&rig);
    close(rig);
}

/// 普段の状態（`Steady{talk: None}`）を直に組む（呼んだスレッドで殻の部品を走らせる檻用）。
fn steady() -> State {
    State {
        phase: Phase::Steady { talk: None },
        ..State::initial()
    }
}

/// 呼んだスレッドで殻の一括の処理（`drive`）を 1 回走らせる。
fn drive_here(
    state: &mut State,
    input: Input,
    shiori: &Sender<ShioriMsg>,
    sakura: &Sender<TalkCommand>,
) -> (Drive, Option<RaiseOutcome>) {
    let sink: ResourceSink = Box::new(|_, _| {});
    drive(
        state,
        input,
        &KanadeConfig::new("master", "1.0.0"),
        shiori,
        sakura,
        &sink,
        None,
    )
}

/// ⑶b の warn 1 件。記録は kanade のスレッドで出て呼出スレッドの捕捉窓に入らないので、殻の 2 つの
/// 部品を呼んだスレッドで走らせて足し合わせる: 一括の処理（`drive`）は運行表の腕の
/// `sstp_notify_invalid` をちょうど 1 件だけ残して往復を出さず、返事の組み立て（`answer`）は
/// 受け付けなかった控えに警告を重ねず `NotAccepted` を返す。
#[test]
fn invalid_notify_leaves_exactly_one_warn_across_the_shell_parts() {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (talk_tx, talks) = mpsc::channel::<TalkCommand>();
    let mut state = steady();
    let events = capture(|| {
        let input = Input::SstpNotify {
            event: "On\nX".into(),
            references: Vec::new(),
            strict: None,
        };
        let (flow, first) = drive_here(&mut state, input, &shiori_tx, &talk_tx);
        assert!(matches!(flow, Drive::Continue), "止まらない");
        assert!(first.is_none(), "往復を出さない");
    });
    assert!(shiori_rx.try_recv().is_err(), "SHIORI へ何も送らない");
    assert!(talks.try_recv().is_err(), "再生しない");
    assert_eq!(
        events.iter().filter(|e| e.level <= Level::WARN).count(),
        1,
        "warn 以上はちょうど 1 件: {events:#?}"
    );
    logged_once(&events, Level::WARN, "sstp_notify_invalid");

    let (reply, rx) = areka_actor::reply_channel::<SstpNotifyOutcome>();
    let events = capture(|| {
        crate::actor_sstp::answer(
            SstpReply::Notify {
                reply,
                accepted: false,
            },
            false,
            None,
        );
    });
    assert_eq!(
        rx.recv_timeout(LIMIT).expect("返事が届く"),
        SstpNotifyOutcome::NotAccepted
    );
    assert!(
        !events.iter().any(|e| e.level <= Level::WARN),
        "返事の組み立ては警告を重ねない: {events:#?}"
    );
}

/// ⑷ 往復の失敗（`Failed(Ipc)`）→ `Failed` の返事、続けて停止の通知。
#[test]
fn notify_round_trip_failure_is_failed_then_stops() {
    let rig = spawn_with(outside_answers(|| {
        ShioriOutcome::Failed(ShioriFailure::Ipc("helper gone".into()))
    }));
    boot_to_steady(&rig);
    assert_eq!(
        send_notify(&rig, OUTSIDE)
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::Failed
    );
    stops_by_fault(rig);
}

/// `OUTSIDE` へ台本を返し、`OnTranslate` は輸送路の失敗にする偽の shiori。
fn translate_fails() -> Respond {
    Box::new(|id| match id {
        OUTSIDE => Some(ShioriOutcome::Value(
            RETURNED.into(),
            areka_talk::SecurityLevel::Local,
        )),
        "OnTranslate" => Some(ShioriOutcome::Failed(ShioriFailure::Ipc(
            "helper gone".into(),
        ))),
        _ => None,
    })
}

/// ⑷ 翻訳の輸送路の失敗 → どちらの入口も `Failed`、続けて停止の通知。再生は始まらない。
#[test]
fn translate_transport_failure_is_failed_then_stops() {
    let rig = spawn_with(translate_fails());
    boot_to_steady(&rig);
    while rig.talks.try_recv().is_ok() {}
    assert_eq!(
        send_script(&rig).recv_timeout(LIMIT).expect("返事が届く"),
        SstpSendOutcome::Failed,
        "外からの台本"
    );
    assert!(started(&rig.talks).is_empty(), "再生しない");
    stops_by_fault(rig);

    let rig = spawn_with(translate_fails());
    boot_to_steady(&rig);
    while rig.talks.try_recv().is_ok() {}
    assert_eq!(
        send_notify(&rig, OUTSIDE)
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::Failed,
        "外からのイベントの台本"
    );
    assert!(started(&rig.talks).is_empty(), "再生しない");
    stops_by_fault(rig);
}

/// ⑸ 返事の受け手を落としてから送っても、再生は始まり殻は止まらない（要件 3.8）。
#[test]
fn dropped_reply_receiver_still_plays_and_keeps_running() {
    let rig = spawn_with(outside_answers(|| {
        ShioriOutcome::Value(RETURNED.into(), areka_talk::SecurityLevel::Local)
    }));
    boot_to_steady(&rig);
    while rig.talks.try_recv().is_ok() {}

    drop(send_script(&rig));
    drop(send_notify(&rig, OUTSIDE));
    // 受信は順に 1 件ずつなので、後続の依頼の返事が届けば前の 2 件の処理は済んでいる。
    still_running(&rig);
    assert_eq!(
        started(&rig.talks),
        vec![TRANSLATED.to_string(); 3],
        "落とした 2 件も後続の 1 件も再生が始まる"
    );
    close(rig);
}

/// 返信端つきの汎用の通知で `OUTSIDE` を送り、返事を受け取る。
fn raise_outside(rig: &Rig) -> RaiseOutcome {
    let (reply, rx) = areka_actor::reply_channel::<RaiseOutcome>();
    rig.kanade
        .send(KanadeMsg::RaiseEvent {
            id: OUTSIDE.to_string(),
            references: Vec::new(),
            method: ShioriMethod::Get,
            reply: Some(reply),
        })
        .expect("kanade に届く");
    rx.recv_timeout(LIMIT).expect("返事が届く")
}

/// ⑻ 同じ許可の表に無い名前を汎用の通知で送ると今日どおり `NotAllowed`（送られない）。外から頼むと
/// 偽の shiori へ届き、その後も汎用の通知では断られたまま（外からの依頼が許可の表を開かない）。
#[test]
fn outside_name_is_refused_by_raise_event_but_reaches_shiori_by_notify() {
    let rig = spawn_with(outside_answers(|| ShioriOutcome::NoContent));
    boot_to_steady(&rig);

    assert_eq!(raise_outside(&rig), RaiseOutcome::NotAllowed);
    assert!(drain(&rig.calls).is_empty(), "汎用の通知では送らない");
    assert_eq!(
        send_notify(&rig, OUTSIDE)
            .recv_timeout(LIMIT)
            .expect("返事が届く"),
        SstpNotifyOutcome::NoScript
    );
    assert_eq!(drain(&rig.calls), vec![format!("GET {OUTSIDE}")]);
    assert_eq!(
        raise_outside(&rig),
        RaiseOutcome::NotAllowed,
        "許可の表は開かない"
    );
    assert!(drain(&rig.calls).is_empty(), "汎用の通知では送らない");
    close(rig);
}

/// ⑻ 外から頼まれた名前は、殻の一括の処理を通しても警告なしに渡された Reference のまま SHIORI へ
/// 1 回だけ届く（呼んだスレッドで `drive` を走らせて記録を捕まえる）。
#[test]
fn outside_name_reaches_shiori_verbatim_without_warning() {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let helper = std::thread::spawn(move || {
        let mut seen = Vec::new();
        while let Ok(msg) = shiori_rx.recv() {
            if let ShioriMsg::Request { call, reply } = msg {
                let (ShioriCall::Get { id, references, .. }
                | ShioriCall::Notify { id, references, .. }) = call;
                seen.push((id.as_str().to_string(), references));
                let _ = reply.send(ShioriOutcome::NoContent);
            }
        }
        seen
    });
    let (talk_tx, _talks) = mpsc::channel::<TalkCommand>();
    let references = vec!["a\u{1}b".to_string(), String::new()];
    let mut state = steady();
    let events = capture(|| {
        let input = Input::SstpNotify {
            event: OUTSIDE.into(),
            references: references.clone(),
            strict: None,
        };
        let (flow, _) = drive_here(&mut state, input, &shiori_tx, &talk_tx);
        assert!(matches!(flow, Drive::Continue), "止まらない");
    });
    drop(shiori_tx);
    assert_eq!(
        helper.join().expect("偽の shiori のスレッド"),
        vec![(OUTSIDE.to_string(), references)],
        "名前と Reference は逐語で 1 回だけ送られる"
    );
    assert!(
        !events.iter().any(|e| e.level <= Level::WARN),
        "警告もエラーも出さない（要件 2.1）: {events:#?}"
    );
}

/// ⑻ 送出の檻の既存の規則は今日どおり: 同じ名前でも、スケジューラ起源（許可の表）と選択起源
/// （`On` 接頭）では送らず `event_id_not_allowed` の error、外から頼まれた起源だけが送られる。
#[test]
fn egress_rules_for_static_and_choice_origins_are_unchanged() {
    for (id, sent) in [
        (EventId::Static(OUTSIDE), false),
        (EventId::Choice(OUTSIDE.to_string()), false),
        (EventId::Sstp(OUTSIDE.to_string()), true),
    ] {
        let label = format!("{id:?}");
        let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
        let helper = std::thread::spawn(move || match shiori_rx.recv() {
            Ok(ShioriMsg::Request { reply, .. }) => {
                let _ = reply.send(ShioriOutcome::NoContent);
                true
            }
            _ => false,
        });
        let call = ShioriCall::Get {
            id,
            references: Vec::new(),
            status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
        };
        let mut outcome = None;
        let events = capture(|| outcome = Some(round_trip_request(&shiori_tx, call)));
        drop(shiori_tx);
        assert_eq!(helper.join().expect("偽の shiori"), sent, "{label}");
        if sent {
            assert!(matches!(outcome, Some(ShioriOutcome::NoContent)), "{label}");
            assert!(!events.iter().any(|e| e.level <= Level::WARN), "{label}");
        } else {
            assert!(
                matches!(
                    outcome,
                    Some(ShioriOutcome::Failed(ShioriFailure::Internal(_)))
                ),
                "{label}"
            );
            logged_once(&events, Level::ERROR, "event_id_not_allowed");
        }
    }
}

/// 殻の「受け付けた」の控え（[`crate::actor_sstp::notify_accepted`]）: 普段の状態で検査を満たすときだけ真。
/// 検査を破った知らせを受け付けたと控えると、殻が「往復が無い」の warn を重ねて 2 件になる（⑶b）。
#[test]
fn notify_accepted_requires_steady_and_valid_arguments() {
    use crate::actor_sstp::notify_accepted;
    let refs = |items: &[&str]| items.iter().map(|r| r.to_string()).collect::<Vec<_>>();
    assert!(
        notify_accepted(&steady(), OUTSIDE, &refs(&["r0", ""])),
        "普段の状態で正しい引数"
    );
    assert!(!notify_accepted(&steady(), "On\nX", &[]), "名前に LF");
    assert!(
        !notify_accepted(&steady(), OUTSIDE, &refs(&["a\0b"])),
        "Reference に NUL"
    );
    assert!(
        !notify_accepted(&State::initial(), OUTSIDE, &[]),
        "普段の状態でない（起動の前）"
    );
}
