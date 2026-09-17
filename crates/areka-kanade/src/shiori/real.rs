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

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use areka_actor::{ActorHandle, spawn_actor};
use shiori_host32_host::{
    CharsetNegotiator, ExitKind, HelperLifecycle, HelperStatus, ParentMessageWindow, RequestError,
    Shiori3Client, ShutdownError,
};

use crate::msg::{KanadeMsg, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome};

/// 接続済み SHIORI 一式（`!Send` 資材はスレッド内で connect が生成する）。
///
/// `window`（[`ParentMessageWindow`]・`!Send`）と `helper`（[`HelperLifecycle`]）を所有し、
/// アクタースレッド終了時（Close／全 Sender drop）の drop で RAII teardown される。
/// # 文字コードの交渉状態（areka-P0-charset-canon 要件 5.1）
/// [`CharsetNegotiator`] は**接続の持ち物**である。[`Shiori3Client`] は 1 往復ごとに作り捨て
/// られるため、そちらに置くと採用結果がイベントをまたいで保たれない。接続 1 つ（＝SHIORI の
/// load から unload まで）につき交渉状態は 1 つで、応答待ちのイベント（GET）と片道のイベント
/// （NOTIFY）が同じものを共有する。初期値の決定は結線層（`areka-ghost` の `shiori_wiring`）が
/// 行い、本層は**規則を持たない**（受け取って持ち、結線へ貸すだけ）。
pub struct ShioriConnection {
    /// HELLO ハンドシェイク済みの親メッセージ窓（`Shiori3Client` が借用する送信経路）。
    pub window: ParentMessageWindow,
    /// helper ライフサイクル監視の器（正規 clean shutdown／死活監視を担う）。
    pub helper: HelperLifecycle,
    /// 文字コードの交渉状態（接続と同寿命・GET/NOTIFY 共通・要件 5.1）。
    pub negotiator: CharsetNegotiator,
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
    /// アクターが手空き（inbox が [`IDLE_INTERVAL`] の間空）のたびに呼ばれる保守の機会。
    ///
    /// backend が自分のスレッド上で周期的に行うべき軽い仕事（例: 自分が所有する資材の
    /// 応答性の維持）に使う。ブロックしない・失敗を返さない・異常は [`Self::status`] で報告する
    /// （直後に必ず確認される）。往復（`get`／`notify`／`unload`）の内側からは呼ばれない。
    /// 既定は何もしない。
    fn on_idle(&mut self) {}
}

/// backend の保守周期（手空きを検出する受信待ちの上限）。
///
/// 値の根拠: backend が外から見て応答可能でいるために要する保守の間隔の、現状で最も厳しい
/// 要求は host32 backend の「資材を所有するスレッドが 5 秒以上処理を止めると OS が応答なしと
/// 判定しうる」である。その 1/10 を取り、負荷やスケジューラの遅れに対する余裕と、速い決定論
/// テストの上限（4 倍＝2 秒・全体 5 秒以内）を同時に満たす。往復の所要には影響しない（inbox
/// 到達は送信で即起きる）。正常時の手空きはログを出さないので、この周期を短くしてもログは
/// 増えない。
pub const IDLE_INTERVAL: Duration = Duration::from_millis(500);

impl ShioriBackend for ShioriConnection {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        // フィールド別借用（窓は共有・交渉状態は可変で、別フィールドゆえ同時に借りられる）。
        // 文字コードの規則は結線側が持ち、本層には書かない。
        Shiori3Client::new(&self.window, &mut self.negotiator).get(id, references, status)
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        // GET と同じ交渉状態を渡す——双方のイベントが同じ文字コードで送る（要件 5.1）。
        Shiori3Client::new(&self.window, &mut self.negotiator).notify(id, references, status)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.helper.request_clean_shutdown(&self.window)
    }

    fn status(&mut self) -> HelperStatus {
        self.helper.status()
    }

    fn on_idle(&mut self) {
        // 手空きの間も窓を所有するこのスレッドがメッセージを取り出し続け、OS の応答なし判定に
        // 落ちないようにする（往復の外でしか呼ばれない契約ゆえ `clear→store→take` は崩れない）。
        self.window.pump_pending_messages();
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

/// 死活監視の本体: `backend.status()` が `Exited(kind)` を初めて返したとき、`error!` 記録と
/// `on_down` への [`KanadeMsg::ShioriDown`] 送出を**一度だけ**行う。
///
/// `unloaded`（正規終了の確定後）または `*down_reported`（報告済み）なら何もしない（sticky）。
/// 毎回呼んでよい。
fn report_exit_once(
    backend: &mut dyn ShioriBackend,
    unloaded: bool,
    down_reported: &mut bool,
    on_down: &Sender<KanadeMsg>,
) {
    if !unloaded && !*down_reported {
        if let HelperStatus::Exited(kind) = backend.status() {
            *down_reported = true;
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
}

/// shiori アクターの受信ループ（本番・テスト共通の唯一の dispatch 経路）。
///
/// `ShioriMsg` を [`IDLE_INTERVAL`] を上限とする有限待ち（`recv_timeout`）で受け、
/// [`handle_call`] の結果を同梱 `reply` へちょうど 1 回送る。`Unload` は `backend.unload()`
/// （正規 clean shutdown）へ委譲し、成功／異常終了／失敗をそれぞれログ区分した上で応答する。
/// `Close` は即時停止する。全 `Sender<ShioriMsg>` drop（`Disconnected`）でも正常終了する。
///
/// # 手空き
/// 待ちが [`IDLE_INTERVAL`] の間メッセージなしで明けるたび（`Timeout`）、
/// [`ShioriBackend::on_idle`] を呼び、続けて死活監視を行ってから待ち直す。ログは出さない。
/// 手空きの処理は待ちの腕の中だけで行い、往復（`get`／`notify`／`unload`）の内側からは呼ばない。
///
/// # 死活監視（設計ディスカッション #2）
/// メッセージ到達のたびに冒頭で、また手空きのたびに `backend.status()` を確認する。
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
    loop {
        let msg = match rx.recv_timeout(IDLE_INTERVAL) {
            Ok(msg) => msg,
            Err(RecvTimeoutError::Timeout) => {
                // 手空き: 保守の機会 → 死活監視 → 待ち直す（ログは出さない・往復の外でだけ呼ぶ）。
                backend.on_idle();
                report_exit_once(backend.as_mut(), unloaded, &mut down_reported, &on_down);
                continue;
            }
            // 全 Sender<ShioriMsg> drop: 正常終了（on_down もここで自然に drop される）。
            Err(RecvTimeoutError::Disconnected) => return,
        };
        // 死活監視: 正規終了が確定するまで、メッセージ到達のたびに sticky 状態を確認する。
        report_exit_once(backend.as_mut(), unloaded, &mut down_reported, &on_down);
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

#[cfg(test)]
#[path = "real_idle_tests.rs"]
mod idle_tests;
