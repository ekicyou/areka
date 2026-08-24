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
        ShioriCall::Get {
            id,
            references,
            status,
        } => {
            let status_wire = status.render();
            // wire 形（`as_str()`）のみを backend へ渡す——出所カテゴリは境界を跨がない（DD-1）。
            match backend.get(id.as_str(), &references, status_wire.as_deref()) {
                Ok(Some(value)) => ShioriOutcome::Value(value),
                Ok(None) => ShioriOutcome::NoContent,
                Err(e) => ShioriOutcome::Failed(map_error(e)),
            }
        }
        ShioriCall::Notify {
            id,
            references,
            status,
        } => {
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
#[path = "real_tests.rs"]
mod tests;
