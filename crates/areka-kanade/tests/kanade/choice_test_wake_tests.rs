//! 選択待ちの期限の知らせを殻を通して確かめる檻（audit-kanade #7・要件 9.5）。
//!
//! `KanadeConfig.choice_wake` に送り端を入れた構成で選択待ちを作ると、殻が期限の時刻を 1 つ
//! 受け端へ送る（刻み役への起こしの頼み）。檻は刻み役の代わりに `KanadeMsg::ChoiceTimeoutDue`
//! をその時刻で返し、毎秒の `Tick` を送らずに `OnChoiceTimeout` が出ることを見る。時刻は注入値だけで、
//! 待ちは期限付きの `recv_timeout` だけ（実時刻は読まない）。

use std::sync::mpsc::{self, RecvTimeoutError, TryRecvError};
use std::time::Duration;

use super::test_support::{
    CANONICAL_CHOICE_ID, CHOICE_DEADLINE_MS, choice_get_ids, establish_choice_wait,
};
use super::{
    CloseReason, Driven, Fixture, KanadeConfig, KanadeMsg, MonotonicMs, QuitPolicy, TalkDone,
    TalkEndReason, TalkId, spawn_harness_gated,
};

/// 起こしの頼みが届くまでの待ちの上限（殻が止まったときに檻が詰まらないための期限）。
const WAKE_RECV_BOUND: Duration = Duration::from_secs(10);

/// **期限の知らせで時間切れが出る（#7）**: 選択待ちを作ると受け端に期限（既定 30 秒の写し）が
/// 1 つ届き、その時刻の `ChoiceTimeoutDue` で `OnChoiceTimeout` の GET が 1 件出る。
///
/// # 弁別
/// - 殻が頼みを送らない（`ArmChoiceWake` を捨てる）と受け端が期限内に何も受けず落ちる。
/// - 殻が `ChoiceTimeoutDue` を段取りへ写さないと、毎秒の `Tick` を送っていないので
///   `OnChoiceTimeout` が出ず落ちる。
/// - 頼みが 2 つ以上届くと、殻が止まった後の受け端に残りが見えて落ちる。
#[test]
fn kanade_asks_the_ticker_and_fires_on_the_deadline_notice() {
    let (wake_tx, wake_rx) = mpsc::channel::<MonotonicMs>();
    let config = KanadeConfig {
        choice_wake: Some(wake_tx),
        ..KanadeConfig::new("master", "1.0.0")
    };
    // OnChoiceTimeout は未注入＝204（CancelChoice が出る）。
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);
    // index0（steady talk・保留）=false・index1（close talk）=true。
    let (harness, _gate) = spawn_harness_gated(
        config,
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    let tx = &harness.sender;
    establish_choice_wait(tx, &[CANONICAL_CHOICE_ID], None);
    let at = match wake_rx.recv_timeout(WAKE_RECV_BOUND) {
        Ok(at) => at,
        Err(RecvTimeoutError::Timeout) => panic!("選択待ちを作ったのに起こしの頼みが届かない"),
        Err(RecvTimeoutError::Disconnected) => panic!("起こしの線が頼みの前に切れた"),
    };
    assert_eq!(
        at,
        MonotonicMs(CHOICE_DEADLINE_MS),
        "頼みの時刻は選択待ちの期限のはず"
    );

    tx.send(KanadeMsg::ChoiceTimeoutDue { now: at })
        .expect("send ChoiceTimeoutDue");
    tx.send(KanadeMsg::TalkDone(TalkDone {
        talk_id: TalkId(1),
        reason: TalkEndReason::Interrupted,
        quit_reserved: false,
    }))
    .expect("send TalkDone{Interrupted}");
    tx.send(KanadeMsg::CloseRequest {
        reason: CloseReason::User { scope: 0 },
    })
    .expect("send CloseRequest");

    let Driven { recorded, .. } = harness.finish("kanade choice-wake join");
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceTimeout"],
        "期限の知らせ 1 つで OnChoiceTimeout の GET が 1 件だけ出るはず: {recorded:?}"
    );
    // 殻が止まると送り端も落ちる——残りの頼みが無いこと（頼みは 1 つだけ）。
    assert_eq!(
        wake_rx.try_recv(),
        Err(TryRecvError::Disconnected),
        "起こしの頼みは 1 つだけのはず"
    );
}
