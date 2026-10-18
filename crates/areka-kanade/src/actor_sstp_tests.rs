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
    let rig = spawn_rig(Box::new(|| ShioriOutcome::Value(r"\h\s[0]はい\e".into())));
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
