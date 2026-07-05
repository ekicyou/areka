//! real shiori アクター（`src/shiori/real.rs`）: 既存 SHIORI 出口 API
//! （`shiori-host32-host` の [`Shiori3Client`]）を専有スレッドで包む。
//!
//! **本ファイルは areka-kanade 内で host32 型（[`Shiori3Client`] / [`RequestError`] /
//! [`ParentMessageWindow`] / [`HelperHandle`]）を import してよい唯一の場所である**
//! （Boundary Commitment）。呼出結果と区別失敗語彙は既存 API の戻り値をそのまま機械的に
//! 写像して [`ShioriOutcome`] へ載せる（status 判定・区別語彙の再実装をしない・Req 5.3/6.1）。
//!
//! # 構造（backend 抽象）
//!
//! `ShioriMsg` の dispatch／写像ロジックは backend 抽象（[`ShioriBackend`]）越しに書かれ、
//! [`run_shiori_loop`] が唯一の受信ループとして所有する。本番は [`ConnectionBackend`]（実
//! `Shiori3Client` を呼ぶ）を、テストはスクリプト化した fake backend を**同一の runner**へ
//! 結線する——mapping と往復は本番と同一コードパス上で検証される（実 32bit helper を要さない）。
//! backend は窓所有スレッド上でのみ生きるため `Send` を要求しない。
//!
//! アクター境界の受理規約（envelope・停止・on_down の寿命）は親モジュール
//! [`crate::shiori`] の rustdoc に記す。

use std::sync::mpsc::{Receiver, Sender};

use areka_actor::{ActorHandle, spawn_actor};
use shiori_host32_host::{
    HelperHandle, ParentMessageWindow, RequestError, Shiori3Client,
};

use crate::msg::{KanadeMsg, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome};

/// 接続済み SHIORI 一式（`!Send` 資材はスレッド内で connect が生成する）。
///
/// `window`（[`ParentMessageWindow`]・`!Send`）と `helper`（[`HelperHandle`]）を所有し、
/// アクタースレッド終了時（Close／全 Sender drop）の drop で RAII teardown される。
pub struct ShioriConnection {
    /// HELLO ハンドシェイク済みの親メッセージ窓（`Shiori3Client` が借用する送信経路）。
    pub window: ParentMessageWindow,
    /// helper プロセスハンドル（drop で子プロセス資材が解放される）。
    pub helper: HelperHandle,
}

/// `ShioriMsg` dispatch の背後にある呼出面（GET／NOTIFY）。
///
/// 本番実装（[`ConnectionBackend`]）は実 [`Shiori3Client`] を呼び、テストはスクリプト化した
/// fake を差し込む。窓所有スレッド上でのみ生きるため `Send` を要求しない（thread-local）。
trait ShioriBackend {
    /// 応答を要するイベント（GET）。`Ok(Some)`＝Value・`Ok(None)`＝204・`Err`＝失敗。
    fn get(&self, id: &str, references: &[String]) -> Result<Option<String>, RequestError>;
    /// 片道イベント（NOTIFY）。`Ok(())`＝完了・`Err`＝失敗。
    fn notify(&self, id: &str, references: &[String]) -> Result<(), RequestError>;
}

/// 本番 backend: [`ShioriConnection`] の窓ごとに [`Shiori3Client`] を構築して呼ぶ。
///
/// client 構築は安価（窓の `&` 借用のみ）ゆえ呼出ごとに `Shiori3Client::new` する。
struct ConnectionBackend {
    connection: ShioriConnection,
}

impl ShioriBackend for ConnectionBackend {
    fn get(&self, id: &str, references: &[String]) -> Result<Option<String>, RequestError> {
        Shiori3Client::new(&self.connection.window).get(id, references)
    }

    fn notify(&self, id: &str, references: &[String]) -> Result<(), RequestError> {
        Shiori3Client::new(&self.connection.window).notify(id, references)
    }
}

/// [`RequestError`] を区別語彙を保った [`ShioriFailure`] へ**機械的に写像**する純関数（Req 6.1）。
///
/// host32 の status 分類（`map_send_error`／`map_get_result`）は再実装せず、戻り値の variant を
/// そのまま写す（Req 5.3）。詳細文字列は各エラーの [`std::fmt::Display`] を carry する
/// （host32 型は境界を跨がない——`String` へ落とす）。
///
/// - [`RequestError::Handshake`] → [`ShioriFailure::Handshake`]（接続確立失敗）
/// - [`RequestError::Timeout`] → [`ShioriFailure::Timeout`]（wire timeout）
/// - [`RequestError::Ipc`] → [`ShioriFailure::Ipc`]（helper 死活の一態様）
/// - [`RequestError::Shiori`] → [`ShioriFailure::Shiori`]（SHIORI エラー応答）
fn map_error(err: RequestError) -> ShioriFailure {
    match err {
        RequestError::Handshake(h) => ShioriFailure::Handshake(h.to_string()),
        RequestError::Timeout => ShioriFailure::Timeout(RequestError::Timeout.to_string()),
        RequestError::Ipc(e) => ShioriFailure::Ipc(RequestError::Ipc(e).to_string()),
        RequestError::Shiori(e) => ShioriFailure::Shiori(RequestError::Shiori(e).to_string()),
    }
}

/// 1 件の [`ShioriCall`] を backend へ dispatch し [`ShioriOutcome`] へ写す（本番・テスト共通）。
///
/// GET: `Ok(Some)`→`Value`・`Ok(None)`→`NoContent`・`Err`→`Failed(map_error(..))`。
/// NOTIFY: `Ok(())`→`Notified`・`Err`→`Failed(map_error(..))`（NOTIFY は Value を運ばない）。
fn handle_call(backend: &impl ShioriBackend, call: ShioriCall) -> ShioriOutcome {
    match call {
        ShioriCall::Get { id, references } => match backend.get(id, &references) {
            Ok(Some(value)) => ShioriOutcome::Value(value),
            Ok(None) => ShioriOutcome::NoContent,
            Err(e) => ShioriOutcome::Failed(map_error(e)),
        },
        ShioriCall::Notify { id, references } => match backend.notify(id, &references) {
            Ok(()) => ShioriOutcome::Notified,
            Err(e) => ShioriOutcome::Failed(map_error(e)),
        },
    }
}

/// shiori アクターの受信ループ（本番・テスト共通の唯一の dispatch 経路）。
///
/// `ShioriMsg` を blocking `recv` で受け、[`handle_call`] の結果を同梱 `reply` へちょうど 1 回
/// 送る。`Unload` は暫定実装で `Unloaded` を返し（実資材解放は `backend` drop 時の RAII）、
/// `Close` は即時停止する。全 `Sender<ShioriMsg>` drop（`recv` が `Err`）でも正常終了する。
fn run_shiori_loop(rx: Receiver<ShioriMsg>, backend: impl ShioriBackend) {
    while let Ok(msg) = rx.recv() {
        match msg {
            ShioriMsg::Request { call, reply } => {
                let outcome = handle_call(&backend, call);
                // envelope 規約: ちょうど 1 回応答する。要求側の取消／切断による send Err は無視。
                let _ = reply.send(outcome);
            }
            ShioriMsg::Unload { reply } => {
                // 暫定実装（M1）: 正規 unload 経路（helper 側 exit(0)）は host32-lifecycle が増設中。
                // 実資材の解放は backend（接続資材）の drop 時 RAII に委ね、ここでは Unloaded を返す。
                // 境界契約（ShioriMsg::Unload／Unloaded）は不変ゆえ、正規経路確立時にこの 1 アームのみ
                // 差し替えればよい。
                let _ = reply.send(ShioriOutcome::Unloaded);
            }
            ShioriMsg::Close => {
                tracing::info!(
                    target: "shiori-actor",
                    event = "close",
                    "停止指示（Close）を受領——即時停止（接続資材を RAII teardown）"
                );
                // Break: backend（＝接続資材）は関数終了で drop され RAII teardown される。
                return;
            }
        }
    }
}

/// real shiori アクターを起動する（areka-actor 規約: スレッド名 "shiori"）。
///
/// `connect` はアクタースレッド上で**一度だけ**実行される（[`ParentMessageWindow`] が `!Send`
/// のため spawn 前に実行できない）。接続確立に失敗した場合
/// （`connect` が `Err(reason)`）は [`KanadeMsg::ShioriDown`] を `on_down` へ送って死活報告と
/// し、受信ループには入らず終了する（Req 5.3/6.1）。
///
/// `on_down`（kanade inbox の送信端）は接続確立の成否確定後に**直ちに drop** し、受信ループ中は
/// 保持しない——保持すると kanade inbox が生き続け、kanade の「全 Sender drop で正常終了」
/// （Req 4.9）を妨げるためである。
///
/// inbox の送信端（[`Sender<ShioriMsg>`]）と [`ActorHandle`] を返す。
pub fn spawn_shiori_actor(
    connect: impl FnOnce() -> Result<ShioriConnection, String> + Send + 'static,
    on_down: Sender<KanadeMsg>,
) -> (Sender<ShioriMsg>, ActorHandle) {
    spawn_actor("shiori", move |rx| {
        // 接続はアクタースレッド上で一度だけ実行（!Send window）。
        match connect() {
            Ok(connection) => {
                // 成否確定: on_down を直ちに drop（受信ループ中は保持しない・Req 4.9）。
                drop(on_down);
                let backend = ConnectionBackend { connection };
                run_shiori_loop(rx, backend);
            }
            Err(reason) => {
                tracing::error!(
                    target: "shiori-actor",
                    event = "connect_failed",
                    reason = %reason,
                    "SHIORI 接続確立に失敗——死活報告（ShioriDown）し受信ループに入らず終了"
                );
                // 死活報告後、on_down はスコープ終了で drop される（保持しない）。
                let _ = on_down.send(KanadeMsg::ShioriDown { reason });
                // 受信ループには入らず終了（rx はここで drop→残る Sender の送信は Err で観測される）。
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_actor::reply_channel;
    use shiori_host32_host::{HandshakeError, ShioriError};
    use shiori_host32_ipc::IpcError;
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::time::Duration;

    const BOUND: Duration = Duration::from_secs(5);

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
        }
    }

    /// fake の GET 応答クロージャ（スクリプト化した戻り値・`Send`）。
    type GetFn = Box<dyn Fn(&str, &[String]) -> Result<Option<String>, RequestError> + Send>;
    /// fake の NOTIFY 応答クロージャ（スクリプト化した戻り値・`Send`）。
    type NotifyFn = Box<dyn Fn(&str, &[String]) -> Result<(), RequestError> + Send>;

    /// スクリプト化した fake backend: GET／NOTIFY の戻り値を差し替え可能にする。
    ///
    /// runner を別スレッドで走らせて往復を観測するため、boxed 呼出には `Send` を課す
    /// （本番 `ConnectionBackend` は `!Send` だが `spawn_actor` の closure 内で move される・
    /// テスト fake は `Send` で構わない）。
    struct FakeBackend {
        get_result: GetFn,
        notify_result: NotifyFn,
    }

    impl ShioriBackend for FakeBackend {
        fn get(&self, id: &str, references: &[String]) -> Result<Option<String>, RequestError> {
            (self.get_result)(id, references)
        }
        fn notify(&self, id: &str, references: &[String]) -> Result<(), RequestError> {
            (self.notify_result)(id, references)
        }
    }

    /// GET だけを差し替えた fake（NOTIFY は使われない前提で unreachable）。
    fn fake_get(
        f: impl Fn(&str, &[String]) -> Result<Option<String>, RequestError> + Send + 'static,
    ) -> FakeBackend {
        FakeBackend {
            get_result: Box::new(f),
            notify_result: Box::new(|_, _| unreachable!("notify not expected in this test")),
        }
    }

    /// NOTIFY だけを差し替えた fake（GET は使われない前提で unreachable）。
    fn fake_notify(
        f: impl Fn(&str, &[String]) -> Result<(), RequestError> + Send + 'static,
    ) -> FakeBackend {
        FakeBackend {
            get_result: Box::new(|_, _| unreachable!("get not expected in this test")),
            notify_result: Box::new(f),
        }
    }

    /// runner を spawn し、Request を 1 往復させて outcome を返す（fake backend・helper 不要）。
    fn round_trip_via_runner(backend: FakeBackend, call: ShioriCall) -> ShioriOutcome {
        let (tx, rx) = mpsc::channel::<ShioriMsg>();
        let handle = std::thread::spawn(move || run_shiori_loop(rx, backend));
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
        let backend = fake_get(|id, refs| {
            assert_eq!(id, "OnBoot");
            assert_eq!(refs, &["master".to_string()]);
            Ok(Some(r"\0hi\e".to_string()))
        });
        let outcome = round_trip_via_runner(
            backend,
            ShioriCall::Get {
                id: "OnBoot",
                references: vec!["master".to_string()],
            },
        );
        assert!(
            matches!(outcome, ShioriOutcome::Value(ref s) if s == r"\0hi\e"),
            "Ok(Some) は Value へ: got {}", describe(&outcome)
        );
    }

    #[test]
    fn get_ok_none_maps_to_no_content() {
        let backend = fake_get(|_, _| Ok(None));
        let outcome = round_trip_via_runner(
            backend,
            ShioriCall::Get {
                id: "OnFirstBoot",
                references: Vec::new(),
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
            fake_get(|_, _| Err(RequestError::Timeout)),
            ShioriCall::Get { id: "OnBoot", references: Vec::new() },
        );
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Timeout(_))),
            "Timeout: got {}", describe(&outcome)
        );

        // Handshake
        let outcome = round_trip_via_runner(
            fake_get(|_, _| Err(RequestError::Handshake(HandshakeError::Timeout))),
            ShioriCall::Get { id: "OnBoot", references: Vec::new() },
        );
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Handshake(_))),
            "Handshake: got {}", describe(&outcome)
        );

        // Ipc
        let outcome = round_trip_via_runner(
            fake_get(|_, _| Err(RequestError::Ipc(IpcError::SendFailed))),
            ShioriCall::Get { id: "OnBoot", references: Vec::new() },
        );
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Ipc(_))),
            "Ipc: got {}", describe(&outcome)
        );

        // Shiori
        let outcome = round_trip_via_runner(
            fake_get(|_, _| {
                Err(RequestError::Shiori(ShioriError::Status {
                    status: 400,
                    error_level: None,
                    error_description: None,
                }))
            }),
            ShioriCall::Get { id: "OnBoot", references: Vec::new() },
        );
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Shiori(_))),
            "Shiori: got {}", describe(&outcome)
        );
    }

    // --- NOTIFY 往復 ---------------------------------------------------------

    #[test]
    fn notify_ok_maps_to_notified() {
        let backend = fake_notify(|id, refs| {
            assert_eq!(id, "OnInitialize");
            assert!(refs.is_empty());
            Ok(())
        });
        let outcome = round_trip_via_runner(
            backend,
            ShioriCall::Notify {
                id: "OnInitialize",
                references: Vec::new(),
            },
        );
        assert!(
            matches!(outcome, ShioriOutcome::Notified),
            "Ok(()) は Notified へ: got {}", describe(&outcome)
        );
    }

    #[test]
    fn notify_err_maps_to_failed() {
        let backend = fake_notify(|_, _| Err(RequestError::Ipc(IpcError::SendFailed)));
        let outcome = round_trip_via_runner(
            backend,
            ShioriCall::Notify { id: "OnClose", references: Vec::new() },
        );
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Ipc(_))),
            "NOTIFY Err は Failed へ: got {}", describe(&outcome)
        );
    }

    // --- Unload 暫定実装 -----------------------------------------------------

    #[test]
    fn unload_returns_unloaded() {
        let backend = fake_get(|_, _| unreachable!("get not expected"));
        let (tx, rx) = mpsc::channel::<ShioriMsg>();
        let handle = std::thread::spawn(move || run_shiori_loop(rx, backend));
        let (reply, receiver) = reply_channel::<ShioriOutcome>();
        tx.send(ShioriMsg::Unload { reply }).expect("send Unload");
        let outcome = receiver.recv().expect("reply received");
        assert!(
            matches!(outcome, ShioriOutcome::Unloaded),
            "Unload は Unloaded を返す（暫定実装）: got {}", describe(&outcome)
        );
        tx.send(ShioriMsg::Close).expect("send Close");
        drop(tx);
        handle.join().expect("runner joins");
    }

    // --- 停止規約: 全 Sender drop で正常終了（on_down 非保持の含意）-----------

    #[test]
    fn all_senders_dropped_terminates_runner() {
        let backend = fake_get(|_, _| unreachable!("no request"));
        let (tx, rx) = mpsc::channel::<ShioriMsg>();
        let handle = std::thread::spawn(move || run_shiori_loop(rx, backend));
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
            call: ShioriCall::Get { id: "OnBoot", references: Vec::new() },
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
}
