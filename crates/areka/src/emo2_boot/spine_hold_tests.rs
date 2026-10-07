//! 偽の SHIORI のテスト（「解かれるまで固まる」台本＝要件 7.1・呼び出しの数）。
//!
//! 固まった呼び出しは別スレッドで走らせ、`holding()` が立ったのを見てから解く。固まっている間は
//! 呼び出しが戻れない（条件変数で待っている）ので、「解く前に戻っていない」の判定は時刻に依らない。

use std::thread;

use areka_kanade::ShioriBackend;
use shiori_host32_host::{ExitKind, RequestError, ShutdownError};

use super::super::{RecordedCall, ScriptedShioriBackend, ScriptedShioriHandle, spin_wait_until};
use super::HoldAt;

/// どの呼び出しも台本に成功の応答を持つ偽物に、`at` で固まる台本を足す（固まらなければ成功が返る）。
fn scripted(at: Option<HoldAt>) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
    let builder = ScriptedShioriBackend::builder()
        .get("OnBoot", Ok(Some("hello".to_string())))
        .notify("OnClose", Ok(()))
        .unload(Ok(ExitKind::Clean));
    match at {
        Some(at) => builder.hold_at(at).build(),
        None => builder.build(),
    }
}

/// `at` で固まる偽物の `call` を別スレッドで走らせ、固まったのを見て、戻っていないことを
/// 確かめてから解く。戻り値と観測の口を返す。
fn hold_then_unblock<T: Send + 'static>(
    at: HoldAt,
    call: fn(&mut ScriptedShioriBackend) -> T,
) -> (T, ScriptedShioriHandle) {
    let (mut backend, handle) = scripted(Some(at));
    let unblock = backend
        .unblock_handle()
        .expect("the scripted backend should hand out an unblock handle");
    let worker = thread::spawn(move || call(&mut backend));

    assert!(
        spin_wait_until(|| handle.holding()),
        "{at:?}: the call should be holding until unblocked"
    );
    assert!(
        !worker.is_finished(),
        "{at:?}: the call must not return before it is unblocked"
    );
    assert_eq!(handle.unblock_calls(), 0, "{at:?}: nobody unblocked yet");

    unblock().expect("the unblock handle should succeed");
    let result = worker.join().expect("the held call should not panic");
    assert!(!handle.holding(), "{at:?}: no longer holding after release");
    assert_eq!(handle.unblock_calls(), 1, "{at:?}: unblocked exactly once");
    (result, handle)
}

#[test]
fn held_get_waits_until_unblocked_then_times_out() {
    let (result, handle) = hold_then_unblock(HoldAt::Get("OnBoot"), |b| b.get("OnBoot", &[], None));
    assert!(
        matches!(result, Err(RequestError::Timeout)),
        "a released GET should fail as a wire timeout, got {result:?}"
    );
    assert_eq!(
        handle.non_status_calls(),
        vec![RecordedCall::Get {
            id: "OnBoot".to_string(),
            references: vec![],
        }]
    );
}

#[test]
fn held_notify_waits_until_unblocked_then_times_out() {
    let (result, handle) = hold_then_unblock(HoldAt::Notify("OnClose"), |b| {
        b.notify("OnClose", &[], None)
    });
    assert!(
        matches!(result, Err(RequestError::Timeout)),
        "a released NOTIFY should fail as a wire timeout, got {result:?}"
    );
    assert_eq!(
        handle.non_status_calls(),
        vec![RecordedCall::Notify {
            id: "OnClose".to_string(),
            references: vec![],
        }]
    );
}

#[test]
fn held_unload_waits_until_unblocked_then_times_out() {
    let (result, handle) = hold_then_unblock(HoldAt::Unload, |b| b.unload());
    assert!(
        matches!(result, Err(ShutdownError::ExitTimeout)),
        "a released UNLOAD should fail as an exit timeout, got {result:?}"
    );
    assert_eq!(handle.non_status_calls(), vec![RecordedCall::Unload]);
}

/// 解く手が先に呼ばれていれば、固まる台本の呼び出しは待たずに期限切れで通る（順序に依らない）。
#[test]
fn unblocking_before_the_call_lets_it_pass_without_waiting() {
    let (mut backend, handle) = scripted(Some(HoldAt::Notify("OnClose")));
    let unblock = backend.unblock_handle().expect("unblock handle");
    unblock().expect("unblock");

    let result = backend.notify("OnClose", &[], None);
    assert!(
        matches!(result, Err(RequestError::Timeout)),
        "got {result:?}"
    );
    assert!(!handle.holding());
    assert_eq!(handle.unblock_calls(), 1);
}

/// 固まるのは指定した種類・id の最初の 1 回だけ。ほかの呼び出しと 2 回目は台本どおりに返る。
#[test]
fn only_the_first_matching_call_holds_and_others_follow_the_script() {
    let (mut backend, handle) = ScriptedShioriBackend::builder()
        .get("OnClose", Ok(None))
        .notify("OnBoot", Ok(()))
        .notify("OnClose", Ok(()))
        .hold_at(HoldAt::Notify("OnClose"))
        .build();
    // 同じ id でも GET は固まらない・別 id の NOTIFY も固まらない（解く手はまだ呼ばれていない）。
    assert!(matches!(backend.get("OnClose", &[], None), Ok(None)));
    assert!(matches!(backend.notify("OnBoot", &[], None), Ok(())));
    assert_eq!(handle.unblock_calls(), 0);

    backend.unblock_handle().expect("unblock handle")().expect("unblock");
    assert!(matches!(
        backend.notify("OnClose", &[], None),
        Err(RequestError::Timeout)
    ));
    // 固まった 1 回は台本を消費していないので、2 回目は台本の成功が返る。
    assert!(matches!(backend.notify("OnClose", &[], None), Ok(())));
}

/// 固まる台本を足さなければ何も待たず、台本どおりに返る（既存の使い方は変わらない）。
#[test]
fn without_hold_at_nothing_holds() {
    let (mut backend, handle) = scripted(None);
    assert!(matches!(backend.get("OnBoot", &[], None), Ok(Some(s)) if s == "hello"));
    assert!(matches!(backend.notify("OnClose", &[], None), Ok(())));
    assert!(matches!(backend.unload(), Ok(ExitKind::Clean)));
    assert!(!handle.holding());
    assert_eq!(handle.unblock_calls(), 0);
}

/// 呼び出しの数（待ちの進みの目印・areka-P0-ghost-session-test-load-flake 3.1）は、状態の問い合わせを
/// 何回しても増えず、`Get`・`Notify`・`Unload` の 1 回ごとに 1 つ増える。
#[test]
fn call_count_skips_status_queries_and_counts_every_other_call() {
    let (mut backend, handle) = scripted(None);
    let status_then_count = |backend: &mut ScriptedShioriBackend| {
        for _ in 0..5 {
            backend.status();
        }
        handle.call_count()
    };
    assert_eq!(status_then_count(&mut backend), 0);
    backend.get("OnBoot", &[], None).expect("scripted GET");
    assert_eq!(status_then_count(&mut backend), 1);
    backend
        .notify("OnClose", &[], None)
        .expect("scripted NOTIFY");
    assert_eq!(status_then_count(&mut backend), 2);
    backend.unload().expect("scripted UNLOAD");
    assert_eq!(status_then_count(&mut backend), 3);
    assert_eq!(handle.non_status_calls().len(), 3);
}
