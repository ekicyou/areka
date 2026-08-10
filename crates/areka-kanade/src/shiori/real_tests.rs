use super::*;
use crate::msg::EventId;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use areka_actor::reply_channel;
use shiori_host32_host::{HandshakeError, ShioriError};
use shiori_host32_ipc::IpcError;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

const BOUND: Duration = Duration::from_secs(5);

/// 全 `ShioriCall` 構築点が明示する共通ヘッダ status。INACTIVE 由来ゆえ `render()==None`
/// （空ヘッダ）——`handle_call` はこの `None` を backend の `status` 引数へそのまま届ける
/// （転送檻は `handle_call_forwards_rendered_status_to_backend`）。
fn inactive_status() -> ExecutionStatus {
    ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE)
}

/// `ShioriOutcome` の variant 名を返す（`ShioriOutcome` は Debug 非実装かつ msg.rs は本タスクで
/// 不変ゆえ、assert 失敗メッセージ用に variant を局所的に説明する）。
fn describe(outcome: &ShioriOutcome) -> &'static str {
    match outcome {
        ShioriOutcome::Value(_) => "Value",
        ShioriOutcome::NoContent => "NoContent",
        ShioriOutcome::Notified => "Notified",
        ShioriOutcome::Unloaded => "Unloaded",
        ShioriOutcome::Failed(ShioriFailure::Handshake(_)) => "Failed(Handshake)",
        ShioriOutcome::Failed(ShioriFailure::Timeout(_)) => "Failed(Timeout)",
        ShioriOutcome::Failed(ShioriFailure::Ipc(_)) => "Failed(Ipc)",
        ShioriOutcome::Failed(ShioriFailure::Shiori(_)) => "Failed(Shiori)",
        ShioriOutcome::Failed(ShioriFailure::Internal(_)) => "Failed(Internal)",
    }
}

/// fake の GET 応答クロージャ（スクリプト化した戻り値・`Send`・第 3 引数＝render 済み wire status）。
type GetFn =
    Box<dyn Fn(&str, &[String], Option<&str>) -> Result<Option<String>, RequestError> + Send>;
/// fake の NOTIFY 応答クロージャ（スクリプト化した戻り値・`Send`・第 3 引数＝render 済み wire status）。
type NotifyFn = Box<dyn Fn(&str, &[String], Option<&str>) -> Result<(), RequestError> + Send>;
/// fake の unload 応答クロージャ（スクリプト化した戻り値・`Send`・状態変化を許すため `FnMut`）。
type UnloadFn = Box<dyn FnMut() -> Result<ExitKind, ShutdownError> + Send>;
/// fake の status 応答クロージャ（スクリプト化した戻り値・`Send`・状態変化を許すため `FnMut`）。
type StatusFn = Box<dyn FnMut() -> HelperStatus + Send>;

/// スクリプト化した fake backend: GET／NOTIFY／unload／status の戻り値を差し替え可能にする。
///
/// runner を別スレッドで走らせて往復を観測するため、boxed 呼出には `Send` を課す
/// （本番 `ShioriConnection` は `!Send` だが `spawn_actor` の closure 内で move される・
/// テスト fake は `Send` で構わない）。
struct FakeBackend {
    get_result: GetFn,
    notify_result: NotifyFn,
    unload_result: UnloadFn,
    status_result: StatusFn,
}

impl ShioriBackend for FakeBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        (self.get_result)(id, references, status)
    }
    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        (self.notify_result)(id, references, status)
    }
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        (self.unload_result)()
    }
    fn status(&mut self) -> HelperStatus {
        (self.status_result)()
    }
}

/// GET だけを差し替えた fake（NOTIFY／unload は使われない前提で unreachable・status は常に
/// `Running` を返し死活監視ノイズを起こさない）。
fn fake_get(
    f: impl Fn(&str, &[String], Option<&str>) -> Result<Option<String>, RequestError>
    + Send
    + 'static,
) -> FakeBackend {
    FakeBackend {
        get_result: Box::new(f),
        notify_result: Box::new(|_, _, _| unreachable!("notify not expected in this test")),
        unload_result: Box::new(|| unreachable!("unload not expected in this test")),
        status_result: Box::new(|| HelperStatus::Running),
    }
}

/// NOTIFY だけを差し替えた fake（GET／unload は使われない前提で unreachable・status は常に
/// `Running`）。
fn fake_notify(
    f: impl Fn(&str, &[String], Option<&str>) -> Result<(), RequestError> + Send + 'static,
) -> FakeBackend {
    FakeBackend {
        get_result: Box::new(|_, _, _| unreachable!("get not expected in this test")),
        notify_result: Box::new(f),
        unload_result: Box::new(|| unreachable!("unload not expected in this test")),
        status_result: Box::new(|| HelperStatus::Running),
    }
}

/// unload だけを差し替えた fake（GET／NOTIFY は使われない前提で unreachable・status は常に
/// `Running`——死活検出は unload とは独立にテストする）。
fn fake_unload(
    f: impl FnMut() -> Result<ExitKind, ShutdownError> + Send + 'static,
) -> FakeBackend {
    FakeBackend {
        get_result: Box::new(|_, _, _| unreachable!("get not expected in this test")),
        notify_result: Box::new(|_, _, _| unreachable!("notify not expected in this test")),
        unload_result: Box::new(f),
        status_result: Box::new(|| HelperStatus::Running),
    }
}

/// backend を `Box<dyn ShioriBackend>` へ格上げする（`spawn_shiori_actor` の connect closure
/// と同じ形——純 x64 の偽装注入シームをテストでも同一の型で通す）。
fn boxed(backend: FakeBackend) -> Box<dyn ShioriBackend> {
    Box::new(backend)
}

/// runner を spawn し、Request を 1 往復させて outcome を返す（fake backend・helper 不要）。
fn round_trip_via_runner(backend: FakeBackend, call: ShioriCall) -> ShioriOutcome {
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, _on_down_rx) = mpsc::channel::<KanadeMsg>();
    let handle =
        std::thread::spawn(move || run_shiori_loop(rx, boxed(backend), on_down_tx));
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Request { call, reply }).expect("send Request");
    let outcome = receiver.recv().expect("reply received");
    // 後片付け: Close で runner を止めて join（ハングしない）。
    tx.send(ShioriMsg::Close).expect("send Close");
    drop(tx);
    handle.join().expect("runner joins");
    outcome
}

// --- map_error: 4 語彙の機械的写像（Req 6.1）------------------------------

#[test]
fn map_error_handshake_maps_to_handshake_with_display() {
    let mapped = map_error(RequestError::Handshake(HandshakeError::Incomplete));
    match mapped {
        ShioriFailure::Handshake(detail) => {
            assert_eq!(detail, HandshakeError::Incomplete.to_string());
        }
        other => panic!("expected Handshake, got {other:?}"),
    }
}

#[test]
fn map_error_timeout_maps_to_timeout_with_display() {
    let mapped = map_error(RequestError::Timeout);
    match mapped {
        ShioriFailure::Timeout(detail) => {
            assert_eq!(detail, RequestError::Timeout.to_string());
        }
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[test]
fn map_error_ipc_maps_to_ipc_with_display() {
    let mapped = map_error(RequestError::Ipc(IpcError::SendFailed));
    match mapped {
        ShioriFailure::Ipc(detail) => {
            assert_eq!(detail, RequestError::Ipc(IpcError::SendFailed).to_string());
        }
        other => panic!("expected Ipc, got {other:?}"),
    }
}

#[test]
fn map_error_shiori_maps_to_shiori_with_display() {
    let mapped = map_error(RequestError::Shiori(ShioriError::Status {
        status: 500,
        error_level: Some("critical".to_string()),
        error_description: Some("boom".to_string()),
    }));
    match mapped {
        ShioriFailure::Shiori(detail) => {
            let expected = RequestError::Shiori(ShioriError::Status {
                status: 500,
                error_level: Some("critical".to_string()),
                error_description: Some("boom".to_string()),
            })
            .to_string();
            assert_eq!(detail, expected);
        }
        other => panic!("expected Shiori, got {other:?}"),
    }
}

// --- GET 往復（fake backend・helper 不要）---------------------------------

#[test]
fn get_ok_some_maps_to_value() {
    let backend = fake_get(|id, refs, _status| {
        assert_eq!(id, "OnBoot");
        assert_eq!(refs, &["master".to_string()]);
        Ok(Some(r"\0hi\e".to_string()))
    });
    let outcome = round_trip_via_runner(
        backend,
        ShioriCall::Get {
            id: EventId::Static("OnBoot"),
            references: vec!["master".to_string()],
            status: inactive_status(),
        },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Value(ref s) if s == r"\0hi\e"),
        "Ok(Some) は Value へ: got {}", describe(&outcome)
    );
}

#[test]
fn get_ok_none_maps_to_no_content() {
    let backend = fake_get(|_, _, _| Ok(None));
    let outcome = round_trip_via_runner(
        backend,
        ShioriCall::Get {
            id: EventId::Static("OnFirstBoot"),
            references: Vec::new(),
            status: inactive_status(),
        },
    );
    assert!(
        matches!(outcome, ShioriOutcome::NoContent),
        "Ok(None) は NoContent へ: got {}", describe(&outcome)
    );
}

#[test]
fn get_err_each_vocabulary_maps_to_failed() {
    // Timeout
    let outcome = round_trip_via_runner(
        fake_get(|_, _, _| Err(RequestError::Timeout)),
        ShioriCall::Get { id: EventId::Static("OnBoot"), references: Vec::new(), status: inactive_status() },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Timeout(_))),
        "Timeout: got {}", describe(&outcome)
    );

    // Handshake
    let outcome = round_trip_via_runner(
        fake_get(|_, _, _| Err(RequestError::Handshake(HandshakeError::Timeout))),
        ShioriCall::Get { id: EventId::Static("OnBoot"), references: Vec::new(), status: inactive_status() },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Handshake(_))),
        "Handshake: got {}", describe(&outcome)
    );

    // Ipc
    let outcome = round_trip_via_runner(
        fake_get(|_, _, _| Err(RequestError::Ipc(IpcError::SendFailed))),
        ShioriCall::Get { id: EventId::Static("OnBoot"), references: Vec::new(), status: inactive_status() },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Ipc(_))),
        "Ipc: got {}", describe(&outcome)
    );

    // Shiori
    let outcome = round_trip_via_runner(
        fake_get(|_, _, _| {
            Err(RequestError::Shiori(ShioriError::Status {
                status: 400,
                error_level: None,
                error_description: None,
            }))
        }),
        ShioriCall::Get { id: EventId::Static("OnBoot"), references: Vec::new(), status: inactive_status() },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Shiori(_))),
        "Shiori: got {}", describe(&outcome)
    );
}

// --- NOTIFY 往復 ---------------------------------------------------------

#[test]
fn notify_ok_maps_to_notified() {
    let backend = fake_notify(|id, refs, _status| {
        assert_eq!(id, "OnInitialize");
        assert!(refs.is_empty());
        Ok(())
    });
    let outcome = round_trip_via_runner(
        backend,
        ShioriCall::Notify {
            id: EventId::Static("OnInitialize"),
            references: Vec::new(),
            status: inactive_status(),
        },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Notified),
        "Ok(()) は Notified へ: got {}", describe(&outcome)
    );
}

#[test]
fn notify_err_maps_to_failed() {
    let backend = fake_notify(|_, _, _| Err(RequestError::Ipc(IpcError::SendFailed)));
    let outcome = round_trip_via_runner(
        backend,
        ShioriCall::Notify { id: EventId::Static("OnClose"), references: Vec::new(), status: inactive_status() },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Ipc(_))),
        "NOTIFY Err は Failed へ: got {}", describe(&outcome)
    );
}

// --- status 転送檻: render 済み wire 値が backend の status 引数へ届く（Req 2.2/2.3・Testing #9）---

/// `handle_call` が `ExecutionStatus::render()` の結果を backend の `status` 引数へ Some/None
/// 双方で届けることを固定する（語彙は kanade 所有・backend は転記のみ・DD-IT-1）。
#[test]
fn handle_call_forwards_rendered_status_to_backend() {
    // talk_active=true → render() == Some("talking") が NOTIFY backend の status へ届く（Req 2.2）。
    let (talking_tx, talking_rx) = mpsc::channel::<Option<String>>();
    let notify_backend = fake_notify(move |_id, _refs, status: Option<&str>| {
        let _ = talking_tx.send(status.map(|s| s.to_string()));
        Ok(())
    });
    let outcome = round_trip_via_runner(
        notify_backend,
        ShioriCall::Notify {
            id: EventId::Static("OnSecondChange"),
            references: Vec::new(),
            status: ExecutionStatus::derive(&ExecutionSnapshot { talk_active: true, choice_active: false }),
        },
    );
    assert!(
        matches!(outcome, ShioriOutcome::Notified),
        "NOTIFY 往復は Notified: got {}", describe(&outcome)
    );
    assert_eq!(
        talking_rx.recv_timeout(BOUND).expect("status captured"),
        Some("talking".to_string()),
        "talk_active=true の render 結果 Some(\"talking\") が backend の status へ届く"
    );

    // INACTIVE → render() == None（ヘッダ行なし）が GET backend の status へ届く（Req 2.3）。
    let (idle_tx, idle_rx) = mpsc::channel::<Option<String>>();
    let get_backend = fake_get(move |_id, _refs, status: Option<&str>| {
        let _ = idle_tx.send(status.map(|s| s.to_string()));
        Ok(None)
    });
    let outcome = round_trip_via_runner(
        get_backend,
        ShioriCall::Get {
            id: EventId::Static("OnSecondChange"),
            references: Vec::new(),
            status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
        },
    );
    assert!(
        matches!(outcome, ShioriOutcome::NoContent),
        "GET 往復（Ok(None)）は NoContent: got {}", describe(&outcome)
    );
    assert_eq!(
        idle_rx.recv_timeout(BOUND).expect("status captured"),
        None,
        "INACTIVE の render 結果 None（ヘッダ行なし）が backend の status へ届く"
    );
}

// --- Unload 正規化: Ok(Clean)／Ok(非 Clean)／Err の 3 分岐（要件 6.2/6.3）--

#[test]
fn unload_ok_clean_returns_unloaded() {
    let backend = fake_unload(|| Ok(ExitKind::Clean));
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, on_down_rx) = mpsc::channel::<KanadeMsg>();
    let handle =
        std::thread::spawn(move || run_shiori_loop(rx, boxed(backend), on_down_tx));
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Unload { reply }).expect("send Unload");
    let outcome = receiver.recv().expect("reply received");
    assert!(
        matches!(outcome, ShioriOutcome::Unloaded),
        "Ok(Clean) は Unloaded を返す: got {}", describe(&outcome)
    );
    tx.send(ShioriMsg::Close).expect("send Close");
    drop(tx);
    handle.join().expect("runner joins");
    // join 後は on_down（Sender）が drop 済みゆえ try_recv は Empty ではなく Disconnected を
    // 返し得る——いずれであれ ShioriDown が届いていないことのみを問う。
    assert!(
        !matches!(on_down_rx.try_recv(), Ok(KanadeMsg::ShioriDown { .. })),
        "正規 clean shutdown は死活報告を発火しない"
    );
}

#[test]
fn unload_ok_non_clean_logs_warn_and_returns_unloaded() {
    let backend = fake_unload(|| Ok(ExitKind::Abnormal(3)));
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, _on_down_rx) = mpsc::channel::<KanadeMsg>();
    let handle =
        std::thread::spawn(move || run_shiori_loop(rx, boxed(backend), on_down_tx));
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Unload { reply }).expect("send Unload");
    let outcome = receiver.recv().expect("reply received");
    assert!(
        matches!(outcome, ShioriOutcome::Unloaded),
        "Ok(非 Clean) でも unload 完了として Unloaded を返す: got {}", describe(&outcome)
    );
    tx.send(ShioriMsg::Close).expect("send Close");
    drop(tx);
    handle.join().expect("runner joins");
}

#[test]
fn unload_err_logs_error_and_returns_failed_ipc() {
    let backend = fake_unload(|| Err(ShutdownError::ExitTimeout));
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, _on_down_rx) = mpsc::channel::<KanadeMsg>();
    let handle =
        std::thread::spawn(move || run_shiori_loop(rx, boxed(backend), on_down_tx));
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Unload { reply }).expect("send Unload");
    let outcome = receiver.recv().expect("reply received");
    match outcome {
        ShioriOutcome::Failed(ShioriFailure::Ipc(detail)) => {
            assert_eq!(detail, ShutdownError::ExitTimeout.to_string());
        }
        other => panic!("Err は Failed(Ipc) を返す: got {}", describe(&other)),
    }
    tx.send(ShioriMsg::Close).expect("send Close");
    drop(tx);
    handle.join().expect("runner joins");
}

// --- 死活監視: メッセージ到達ごとの sticky 検出（要件 3.2/3.4）------------

#[test]
fn death_detected_once_reports_shiori_down_and_only_once() {
    let backend = FakeBackend {
        get_result: Box::new(|_, _, _| Ok(Some(r"\0hi\e".to_string()))),
        notify_result: Box::new(|_, _, _| unreachable!("notify not expected in this test")),
        unload_result: Box::new(|| unreachable!("unload not expected in this test")),
        status_result: Box::new(|| HelperStatus::Exited(ExitKind::Abnormal(1))),
    };
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, on_down_rx) = mpsc::channel::<KanadeMsg>();
    let handle =
        std::thread::spawn(move || run_shiori_loop(rx, boxed(backend), on_down_tx));

    // 1 通目のメッセージ到達で死活検出される。
    let (reply1, receiver1) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Request {
        call: ShioriCall::Get { id: EventId::Static("OnBoot"), references: Vec::new(), status: inactive_status() },
        reply: reply1,
    })
    .expect("send Request 1");
    let _ = receiver1.recv().expect("reply 1 received");

    match on_down_rx.recv_timeout(BOUND) {
        Ok(KanadeMsg::ShioriDown { reason }) => {
            assert!(reason.contains("Abnormal"), "reason に終了種別を含む: {reason}");
        }
        Ok(_) => panic!("expected ShioriDown variant"),
        Err(RecvTimeoutError::Timeout) => panic!("ShioriDown not reported within bound"),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("on_down disconnected without ShioriDown")
        }
    }

    // 2 通目のメッセージ到達でも status は Exited のままだが、再報告しない（sticky）。
    let (reply2, receiver2) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Request {
        call: ShioriCall::Get { id: EventId::Static("OnBoot"), references: Vec::new(), status: inactive_status() },
        reply: reply2,
    })
    .expect("send Request 2");
    let _ = receiver2.recv().expect("reply 2 received");

    assert!(
        matches!(on_down_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "死活報告は一度きり——2 度目のメッセージ到達では再送されない"
    );

    tx.send(ShioriMsg::Close).expect("send Close");
    drop(tx);
    handle.join().expect("runner joins");
}

#[test]
fn death_report_suppressed_after_successful_unload() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // 実装を模して: unload 成功まで Running・成功後は Exited(Clean) を返す共有フラグ。
    let exited = Arc::new(AtomicBool::new(false));
    let status_flag = exited.clone();
    let unload_flag = exited.clone();
    let backend = FakeBackend {
        get_result: Box::new(|_, _, _| unreachable!("get not expected in this test")),
        notify_result: Box::new(|_, _, _| unreachable!("notify not expected in this test")),
        unload_result: Box::new(move || {
            unload_flag.store(true, Ordering::SeqCst);
            Ok(ExitKind::Clean)
        }),
        status_result: Box::new(move || {
            if status_flag.load(Ordering::SeqCst) {
                HelperStatus::Exited(ExitKind::Clean)
            } else {
                HelperStatus::Running
            }
        }),
    };
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, on_down_rx) = mpsc::channel::<KanadeMsg>();
    let handle =
        std::thread::spawn(move || run_shiori_loop(rx, boxed(backend), on_down_tx));

    // 1 通目の Unload: 到達時点では status=Running（死活報告なし）→ unload 成功で unloaded=true。
    let (reply1, receiver1) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Unload { reply: reply1 }).expect("send Unload 1");
    let outcome1 = receiver1.recv().expect("reply 1 received");
    assert!(matches!(outcome1, ShioriOutcome::Unloaded), "1 通目は Unloaded");

    // 2 通目の Unload: 到達時点で status は Exited(Clean) を返すはずだが、unloaded フラグに
    // より死活チェック自体が skip される——報告は発火しない。
    let (reply2, receiver2) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Unload { reply: reply2 }).expect("send Unload 2");
    let outcome2 = receiver2.recv().expect("reply 2 received");
    assert!(matches!(outcome2, ShioriOutcome::Unloaded), "2 通目も Unloaded（冪等）");

    tx.send(ShioriMsg::Close).expect("send Close");
    drop(tx);
    handle.join().expect("runner joins");

    // join 後は on_down（Sender）が drop 済みゆえ try_recv は Empty ではなく Disconnected を
    // 返し得る——いずれであれ ShioriDown が届いていないことのみを問う。
    assert!(
        !matches!(on_down_rx.try_recv(), Ok(KanadeMsg::ShioriDown { .. })),
        "unload 成功後は死活報告を発火しない（正規終了は死ではない）"
    );
}

// --- 停止規約: 全 Sender drop で正常終了（on_down 保持後もループ内で終了する）---

#[test]
fn all_senders_dropped_terminates_runner() {
    let backend = fake_get(|_, _, _| unreachable!("no request"));
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, _on_down_rx) = mpsc::channel::<KanadeMsg>();
    let handle =
        std::thread::spawn(move || run_shiori_loop(rx, boxed(backend), on_down_tx));
    // 何も送らず全 Sender を drop → recv が Err → ループ正常終了。
    drop(tx);
    let (join_tx, join_rx) = mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        handle.join().expect("runner joins on disconnect");
        let _ = join_tx.send(());
    });
    assert_eq!(
        join_rx.recv_timeout(BOUND),
        Ok(()),
        "全 Sender drop で runner が有界時間内に正常終了する"
    );
}

// --- 接続失敗: ShioriDown 死活報告＋受信ループ非突入（helper 不要）--------

#[test]
fn connect_failure_reports_shiori_down_and_does_not_loop() {
    let (on_down_tx, on_down_rx) = mpsc::channel::<KanadeMsg>();
    let (shiori_tx, handle) =
        spawn_shiori_actor(|| Err("boom".to_string()), on_down_tx);

    // 死活報告を有界時間内に受領する（KanadeMsg は Debug 非実装ゆえ variant を明示照合）。
    match on_down_rx.recv_timeout(BOUND) {
        Ok(KanadeMsg::ShioriDown { reason }) => assert_eq!(reason, "boom"),
        Ok(_) => panic!("expected ShioriDown variant"),
        Err(RecvTimeoutError::Timeout) => panic!("ShioriDown not reported within bound"),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("on_down disconnected without ShioriDown")
        }
    }

    // 受信ループに入っていないこと: Request を送っても応答は来ない（reply が drop され Err）。
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    let _ = shiori_tx.send(ShioriMsg::Request {
        call: ShioriCall::Get { id: EventId::Static("OnBoot"), references: Vec::new(), status: inactive_status() },
        reply,
    });
    assert!(
        receiver.recv().is_err(),
        "接続失敗後は受信ループに入らない——Request は処理されず reply は切断される"
    );

    // アクターは有界時間内に join する（ループに入らず終了）。
    drop(shiori_tx);
    let (join_tx, join_rx) = mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        handle.join().expect("shiori actor joins after connect failure");
        let _ = join_tx.send(());
    });
    assert_eq!(
        join_rx.recv_timeout(BOUND),
        Ok(()),
        "接続失敗時アクターは有界時間内に終了する"
    );
}
