//! 終了系列の完了を UI へ知らせる停止通知の統合檻（areka-P0-emo2-conformance-e2e R15.3）。
//!
//! `Action::StopSelf`（終了系列完了）で `KanadeStopped` がちょうど 1 件届くこと、原因が
//! 終了の起因どおりであること、受信端が既に落ちていても停止が完走することを、実アクターを
//! 起動して決定的に観測する（sleep なし・全 join は期限付き）。
//!
//! 3 本の構成:
//!
//! 1. **強制終了**（`ForceQuit`）→ `KanadeStopped{Forced}` が 1 件。
//! 2. **無言終了**（`OnClose` が 204）→ `KanadeStopped{CloseSilent}` が 1 件（原因が
//!    固定値でないことの対照）。
//! 3. **受信端 drop** → 送出は失敗するが停止は完走する（join 成功・記録の末尾は `Unload`）。
//!    このとき `warn!`（`event="stop_notify_failed"`）が 1 件出ることは発行点の単体檻
//!    （`src/actor_stop_notify_tests.rs`）が固定する——アクタースレッドで発火するログを本層で
//!    数えるには全スレッド捕捉の窓口が要り、その利用は別表への登録を伴うため、観測点を
//!    「同じ関数を呼出スレッドで直に叩く」側へ寄せてある。

use std::sync::mpsc;

use areka_kanade::{KanadeNotice, KanadeStopCause};

use super::{
    CallMethod, CloseReason, Fixture, Harness, KanadeConfig, KanadeMsg, QuitPolicy,
    expected_unload, send, single_stop_cause, spawn_harness_with_stop_sink, tick,
};

/// 通知端つきハーネスを組み、`ForceQuit`／`CloseRequest` のいずれかで終了系列へ入れる。
///
/// 戻り値は（ハーネス, 停止通知の受信端）。呼び手は kanade を join してから受信端を読む
/// ——join 成功は `StopSelf` の実行完了を意味するので、通知は既に投函済みである（race なし）。
fn harness_with_stop_channel(fixture: Fixture) -> (Harness, mpsc::Receiver<KanadeNotice>) {
    let (stop_tx, stop_rx) = mpsc::channel::<KanadeNotice>();
    let harness = spawn_harness_with_stop_sink(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        // boot talk（index 0）は quit:false。終了は下の指示が駆動する。
        QuitPolicy::PerTalk(vec![false]),
        Some(stop_tx),
    );
    (harness, stop_rx)
}

/// 強制終了（`ForceQuit`）の終了系列完了で `KanadeStopped{Forced}` が 1 件届く（R15.3）。
///
/// # 非空虚性
/// 通知が出ていなければ `try_recv` が空で落ちる。原因を取り違えていれば `Forced` と一致しない。
#[test]
fn force_quit_delivers_exactly_one_stop_notification_with_the_forced_cause() {
    let (harness, stop_rx) = harness_with_stop_channel(Fixture::default());

    send(&harness, KanadeMsg::Boot);
    tick(&harness, 1_000);
    send(
        &harness,
        KanadeMsg::ForceQuit {
            reason: CloseReason::User { scope: 0 },
        },
    );

    let recorded = harness
        .finish("kanade stop-notify force-quit join")
        .recorded;

    assert_eq!(
        single_stop_cause(&stop_rx),
        KanadeStopCause::Forced,
        "強制終了の停止通知は原因 Forced を運ぶ"
    );
    assert_eq!(
        *recorded.last().expect("記録列は空でない"),
        expected_unload(),
        "終了系列は Unload で閉じている（通知は完了後に出ている）"
    );
}

/// 無言終了（`OnClose` が 204）では原因が `CloseSilent` になる（原因が固定値でないことの対照）。
///
/// # 非空虚性
/// 実装が原因を控えずに単一の値を送っていれば、本檻と上の `Forced` の檻のどちらかが必ず落ちる。
#[test]
fn silent_close_delivers_the_close_silent_cause() {
    // `Fixture::default()`: OnClose→204（無言終了）。close talk は起動しない。
    let (harness, stop_rx) = harness_with_stop_channel(Fixture::default());

    send(&harness, KanadeMsg::Boot);
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );

    let recorded = harness
        .finish("kanade stop-notify silent-close join")
        .recorded;

    assert_eq!(
        single_stop_cause(&stop_rx),
        KanadeStopCause::CloseSilent,
        "無言終了の停止通知は原因 CloseSilent を運ぶ"
    );
    assert_eq!(
        recorded.last().expect("記録列は空でない").method,
        CallMethod::Unload,
        "無言終了も Unload まで完走する"
    );
}

/// 受信端（UI）が既に落ちていても停止は完走する——送出失敗で止まらない（R15.3・log-first）。
///
/// # 非空虚性
/// 送出失敗を `expect`／`unwrap` で扱えばアクタースレッドが panic し、join が
/// `Err` になって本檻が落ちる。
#[test]
fn a_dropped_receiver_does_not_stop_the_termination_sequence() {
    let (stop_tx, stop_rx) = mpsc::channel::<KanadeNotice>();
    // 通知の受信端を先に落とす（UI が既に消えている状況）。
    drop(stop_rx);

    let harness = spawn_harness_with_stop_sink(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::default(),
        QuitPolicy::PerTalk(vec![false]),
        Some(stop_tx),
    );

    send(&harness, KanadeMsg::Boot);
    send(
        &harness,
        KanadeMsg::ForceQuit {
            reason: CloseReason::User { scope: 0 },
        },
    );

    let recorded = harness.finish("kanade stop-notify orphan join").recorded;

    assert_eq!(
        *recorded.last().expect("記録列は空でない"),
        expected_unload(),
        "通知の送出失敗は終了系列を止めない（末尾は Unload）"
    );
}
