//! 殻が通信中の数を毎メッセージ読んで写しへ渡すことの檻（要件 2.5・2.6・5.1・5.2）。
//!
//! 本物の殻（[`spawn_kanade_with_stop_sink`]）を、受けたリクエストの名前と `Status` を記録する
//! 偽の shiori につなぐ。数は関数内の `static` を [`KanadeConfig::online`] で渡し、本番の
//! [`crate::online::PROCESS`] には触れない（同じ実行ファイルの他のテストを揺らさない）。
//!
//! 待ちはすべて受信端の受け取りで揃える。期限はハングを失敗に変えるための上限であり、
//! 実時間の経過に結果は依らない。

use super::*;
use crate::change::{RaiseOutcome, ShioriMethod};
use crate::online::OnlineCounter;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

const LIMIT: Duration = Duration::from_secs(10);

/// 偽の shiori を立てて、`online` を差し替えた kanade を起こす。記録は `(名前, Status)`。
fn spawn_with(
    online: &'static OnlineCounter,
) -> (
    Sender<KanadeMsg>,
    ActorHandle,
    Receiver<(String, Option<String>)>,
    Receiver<KanadeNotice>,
) {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (calls_tx, calls) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(msg) = shiori_rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    let (id, status, outcome) = match &call {
                        ShioriCall::Get { id, status, .. } => {
                            (id, status, ShioriOutcome::NoContent)
                        }
                        ShioriCall::Notify { id, status, .. } => {
                            (id, status, ShioriOutcome::Notified)
                        }
                    };
                    let _ = calls_tx.send((id.as_str().to_string(), status.render()));
                    let _ = reply.send(outcome);
                }
                ShioriMsg::Unload { reply } => {
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                ShioriMsg::Close => break,
            }
        }
    });
    let (talk_tx, _talks) = mpsc::channel::<TalkCommand>();
    let (notice_tx, notices) = mpsc::channel::<KanadeNotice>();
    let mut config = KanadeConfig::new("master", "1.0.0");
    config.online = online;
    let (kanade, handle) = spawn_kanade_with_stop_sink(
        config,
        shiori_tx,
        talk_tx,
        Box::new(|_, _| {}),
        Some(notice_tx),
    );
    // 再生系の受信端は捨てる（送出失敗は error! の上で運行が続く）。本テストはトークを起こさない。
    (kanade, handle, calls, notices)
}

/// 数を立てたまま起こすと最初のリクエスト（OnInitialize）から `online` が載り、
/// 守り手を落とした後の次のメッセージからは載らない。
#[test]
fn shell_copies_the_online_counter_before_every_message() {
    static ONLINE: OnlineCounter = OnlineCounter::new();
    let guard = ONLINE.begin("test");
    let (kanade, handle, calls, notices) = spawn_with(&ONLINE);

    kanade.send(KanadeMsg::Boot).expect("kanade に届く");
    let (first, status) = calls.recv_timeout(LIMIT).expect("最初のリクエストが届く");
    assert_eq!(first, "OnInitialize");
    assert_eq!(
        status.as_deref(),
        Some("online"),
        "起動の最初から online が載る（要件 2.6）"
    );
    loop {
        if let KanadeNotice::Steady = notices.recv_timeout(LIMIT).expect("定常の通知が届く")
        {
            break;
        }
    }
    let boot: Vec<_> = calls.try_iter().collect();
    assert!(
        boot.iter().all(|(_, s)| s.as_deref() == Some("online")),
        "起動系列のすべてに online が載る（要件 5.1）: {boot:?}"
    );

    drop(guard);
    let (reply, rx) = areka_actor::reply_channel::<RaiseOutcome>();
    kanade
        .send(KanadeMsg::RaiseEvent {
            id: "OnInstallCompleteEx".to_string(),
            references: Vec::new(),
            method: ShioriMethod::Notify,
            reply: Some(reply),
        })
        .expect("kanade に届く");
    rx.recv_timeout(LIMIT).expect("返事が届く");
    let (id, status) = calls.recv_timeout(LIMIT).expect("通知のリクエストが届く");
    assert_eq!(id, "OnInstallCompleteEx");
    assert_eq!(
        status, None,
        "通信が終われば次のメッセージから online は載らない（要件 2.5）"
    );

    kanade.send(KanadeMsg::Close).expect("kanade に届く");
    drop(kanade);
    handle.join().expect("kanade が止まる");
}
