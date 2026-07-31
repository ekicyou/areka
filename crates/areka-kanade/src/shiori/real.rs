//! real shiori アクター（`src/shiori/real.rs`）: 既存 SHIORI 出口 API
//! （`shiori-host32-host` の [`Shiori3Client`]）を専有スレッドで包む。
//!
//! **本ファイルは areka-kanade 内で host32 型（[`Shiori3Client`] / [`RequestError`] /
//! [`ParentMessageWindow`] / [`HelperLifecycle`]）を import してよい唯一の場所である**
//! （Boundary Commitment）。呼出結果と区別失敗語彙は既存 API の戻り値をそのまま機械的に
//! 写像して [`ShioriOutcome`] へ載せる（status 判定・区別語彙の再実装をしない・Req 5.3/6.1）。
//!
//! # 構造（backend 抽象）
//!
//! `ShioriMsg` の dispatch／写像ロジックは backend 抽象（[`ShioriBackend`]）越しに書かれ、
//! [`run_shiori_loop`] が唯一の受信ループとして所有する。本番は [`ShioriConnection`]（実
//! `Shiori3Client`／`HelperLifecycle` を呼ぶ）を、テストはスクリプト化した fake backend を
//! **同一の runner**へ結線する——mapping と往復は本番と同一コードパス上で検証される
//! （実 32bit helper を要さない）。backend は窓所有スレッド上でのみ生きるため `Send` を
//! 要求しない（`spawn_shiori_actor` の connect closure が返す `Box<dyn ShioriBackend>` が
//! 純 x64 の偽装注入シームになる）。
//!
//! アクター境界の受理規約（envelope・停止・on_down の寿命）は親モジュール
//! [`crate::shiori`] の rustdoc に記す。

use std::sync::mpsc::{Receiver, Sender};

use areka_actor::{ActorHandle, spawn_actor};
use shiori_host32_host::{
    ExitKind, HelperLifecycle, HelperStatus, ParentMessageWindow, RequestError, Shiori3Client,
    ShutdownError,
};

use crate::msg::{KanadeMsg, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome};

/// 接続済み SHIORI 一式（`!Send` 資材はスレッド内で connect が生成する）。
///
/// `window`（[`ParentMessageWindow`]・`!Send`）と `helper`（[`HelperLifecycle`]）を所有し、
/// アクタースレッド終了時（Close／全 Sender drop）の drop で RAII teardown される。
pub struct ShioriConnection {
    /// HELLO ハンドシェイク済みの親メッセージ窓（`Shiori3Client` が借用する送信経路）。
    pub window: ParentMessageWindow,
    /// helper ライフサイクル監視の器（正規 clean shutdown／死活監視を担う）。
    pub helper: HelperLifecycle,
}

/// `ShioriMsg` dispatch の背後にある呼出面（本番＝[`ShioriConnection`]・テスト＝scripted fake）。
///
/// 窓所有スレッド上でのみ生きるため `Send` を要求しない（thread-local）。呼出ごとに
/// sticky 状態（helper 死活キャッシュ）を更新し得るため各メソッドは `&mut self` を取る。
pub trait ShioriBackend {
    /// 応答を要するイベント（GET）。`Ok(Some)`＝Value・`Ok(None)`＝204・`Err`＝失敗。
    ///
    /// `status` は `ExecutionStatus::render()` 済みの wire 値（`None`＝`Status` ヘッダ行なし・Req 2.3）。
    /// 語彙は kanade が所有し、backend は解釈せず host32 へそのまま転記する（DD-IT-1 語彙非漏洩・Req 2.2）。
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError>;
    /// 片道イベント（NOTIFY）。`Ok(())`＝完了・`Err`＝失敗。
    ///
    /// `status` は GET と同じく render 済みの wire 値（`None`＝ヘッダ行なし）——backend は解釈せず
    /// host32 へそのまま転記する（DD-IT-1 語彙非漏洩・Req 2.2/2.3）。
    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError>;
    /// 正規 clean shutdown（unload → helper 正常終了観測）。
    fn unload(&mut self) -> Result<ExitKind, ShutdownError>;
    /// 非ブロッキング死活問い合わせ（sticky）。
    fn status(&mut self) -> HelperStatus;
}

impl ShioriBackend for ShioriConnection {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        Shiori3Client::new(&self.window).get(id, references, status)
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        Shiori3Client::new(&self.window).notify(id, references, status)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.helper.request_clean_shutdown(&self.window)
    }

    fn status(&mut self) -> HelperStatus {
        self.helper.status()
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
///
/// `status`（[`crate::status::ExecutionStatus`]）は呼出直前に `render()` して wire 値
/// （`Option<&str>`・`None`＝ヘッダ行なし）へ落とし、そのまま backend へ渡す
/// （語彙は kanade 所有・Req 2.2/2.3・DD-IT-1）。
fn handle_call(backend: &mut dyn ShioriBackend, call: ShioriCall) -> ShioriOutcome {
    match call {
        ShioriCall::Get { id, references, status } => {
            let status_wire = status.render();
            // wire 形（`as_str()`）のみを backend へ渡す——出所カテゴリは境界を跨がない（DD-1）。
            match backend.get(id.as_str(), &references, status_wire.as_deref()) {
                Ok(Some(value)) => ShioriOutcome::Value(value),
                Ok(None) => ShioriOutcome::NoContent,
                Err(e) => ShioriOutcome::Failed(map_error(e)),
            }
        }
        ShioriCall::Notify { id, references, status } => {
            let status_wire = status.render();
            // GET と同じく wire 形のみを渡す（DD-1）。
            match backend.notify(id.as_str(), &references, status_wire.as_deref()) {
                Ok(()) => ShioriOutcome::Notified,
                Err(e) => ShioriOutcome::Failed(map_error(e)),
            }
        }
    }
}

/// shiori アクターの受信ループ（本番・テスト共通の唯一の dispatch 経路）。
///
/// `ShioriMsg` を blocking `recv` で受け、[`handle_call`] の結果を同梱 `reply` へちょうど 1 回
/// 送る。`Unload` は `backend.unload()`（正規 clean shutdown）へ委譲し、成功／異常終了／失敗を
/// それぞれログ区分した上で応答する。`Close` は即時停止する。全 `Sender<ShioriMsg>` drop
/// （`recv` が `Err`）でも正常終了する。
///
/// # 死活監視（設計ディスカッション #2）
/// メッセージ到達のたびに冒頭で `backend.status()` を確認する（タイマー poll は持たない）。
/// `Exited(kind)` を初回観測したら `error!`＋`on_down` へ `ShioriDown` を**一度だけ**送る
/// （sticky）。unload 成功後（`unloaded` フラグ確定後）は死活報告を発火しない（正規終了は
/// 死ではない）。`on_down` は受信ループの生存期間中保持し、ループを抜ける（関数から return
/// する）際に自然に drop される。
fn run_shiori_loop(
    rx: Receiver<ShioriMsg>,
    mut backend: Box<dyn ShioriBackend>,
    on_down: Sender<KanadeMsg>,
) {
    let mut unloaded = false;
    let mut down_reported = false;
    while let Ok(msg) = rx.recv() {
        // 死活監視: 正規終了が確定するまで、メッセージ到達のたびに sticky 状態を確認する。
        if !unloaded && !down_reported {
            if let HelperStatus::Exited(kind) = backend.status() {
                down_reported = true;
                tracing::error!(
                    target: "shiori-actor",
                    event = "helper_exited",
                    exit = ?kind,
                    "helper の異常終了を検出——死活報告（ShioriDown）を送出（以後は再報告しない）"
                );
                let _ = on_down.send(KanadeMsg::ShioriDown {
                    reason: format!("helper exited unexpectedly: {kind:?}"),
                });
            }
        }
        match msg {
            ShioriMsg::Request { call, reply } => {
                let outcome = handle_call(backend.as_mut(), call);
                // envelope 規約: ちょうど 1 回応答する。要求側の取消／切断による send Err は無視。
                let _ = reply.send(outcome);
            }
            ShioriMsg::Unload { reply } => match backend.unload() {
                Ok(ExitKind::Clean) => {
                    unloaded = true;
                    tracing::info!(
                        target: "shiori-actor",
                        event = "unload_clean",
                        "正規 clean shutdown 完了（unload → helper 正常終了 exit(0)）"
                    );
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                Ok(other_kind) => {
                    unloaded = true;
                    tracing::warn!(
                        target: "shiori-actor",
                        event = "unload_non_clean",
                        exit = ?other_kind,
                        "unload は完了したが終了種別が Clean でない"
                    );
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                Err(shutdown_error) => {
                    tracing::error!(
                        target: "shiori-actor",
                        event = "unload_failed",
                        error = %shutdown_error,
                        "正規 clean shutdown に失敗"
                    );
                    let _ = reply.send(ShioriOutcome::Failed(ShioriFailure::Ipc(
                        shutdown_error.to_string(),
                    )));
                }
            },
            ShioriMsg::Close => {
                tracing::info!(
                    target: "shiori-actor",
                    event = "close",
                    "停止指示（Close）を受領——即時停止（接続資材を RAII teardown）"
                );
                // Break: backend（＝接続資材）は関数終了で drop され RAII teardown される。
                // on_down もここで自然に drop される。
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
/// `on_down`（kanade inbox の送信端）は接続確立成功後も**受信ループの生存期間中保持する**
/// （死活監視の届け先・Req 3.4）。この保持は kanade→shiori→on_down の Sender 環を作るが、
/// ループは Close 受領または全 `Sender<ShioriMsg>` drop で終了し、その時点で `on_down` は
/// 自然に drop される（「アクター別の停止経路」マトリクス参照）。
///
/// `connect` は本番では実 [`ShioriConnection`] を返すが、純 x64 の偽装注入シームとして
/// `Box<dyn ShioriBackend>` へ一般化されている（Req 7.1/7.6）。
///
/// inbox の送信端（[`Sender<ShioriMsg>`]）と [`ActorHandle`] を返す。
pub fn spawn_shiori_actor(
    connect: impl FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send + 'static,
    on_down: Sender<KanadeMsg>,
) -> (Sender<ShioriMsg>, ActorHandle) {
    spawn_actor("shiori", move |rx| {
        // 接続はアクタースレッド上で一度だけ実行（!Send window）。
        match connect() {
            Ok(backend) => {
                // on_down は受信ループの生存期間中保持する（死活報告の届け先・Req 3.4）。
                run_shiori_loop(rx, backend, on_down);
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
                status: ExecutionStatus::derive(&ExecutionSnapshot { talk_active: true }),
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
}
