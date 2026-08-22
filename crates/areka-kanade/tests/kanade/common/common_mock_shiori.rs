//! mock shiori アクター 3 種（即応・失敗注入・ブロッキング）。
//!
//! `common/mod.rs`（1,657 行）から責務単位で切り出した子モジュール（タスク 8.2）。
//! 項目は親のファサードから再輸出されるため、消費側の `super::common::X` は不変である。

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use areka_actor::{ActorHandle, spawn_actor};
use areka_kanade::{ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome};

use super::fixture::FixtureState;
use super::{Fixture, RecordedCall, expected_unload};

// ============================================================================
// mock shiori アクター
// ============================================================================

/// mock shiori アクターのハンドル群（送信端・join ハンドル・記録アクセサ）。
pub struct MockShiori {
    /// kanade へ渡す shiori inbox の送信端（real と同一の [`ShioriMsg`] 型）。
    pub sender: Sender<ShioriMsg>,
    /// アクタースレッドの join ハンドル。
    pub handle: ActorHandle,
    /// 受理した呼出の記録列（`(method, id, references)`）。
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl MockShiori {
    /// 受理済み呼出の記録スナップショットを返す。
    pub fn recorded(&self) -> Vec<RecordedCall> {
        self.calls.lock().expect("mock shiori calls mutex").clone()
    }
}

/// mock shiori アクターを起動する。
///
/// real と同一の [`ShioriMsg`] 型を受け、[`Fixture`] 表に従い同梱 `reply` へ即時応答する。
/// 各 [`ShioriMsg::Request`] は `(method, id, references)` を記録列へ蓄積し、fixture の
/// 応答を `reply.send` で返す。[`ShioriMsg::Unload`] は記録の上 `Unloaded` を返す。
/// [`ShioriMsg::Close`] で受信ループを終了する。
pub fn spawn_mock_shiori(fixture: Fixture) -> MockShiori {
    let calls: Arc<Mutex<Vec<RecordedCall>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_body = Arc::clone(&calls);

    let (sender, handle) = spawn_actor::<ShioriMsg, _>("mock-shiori", move |rx| {
        let mut state = FixtureState::new(fixture);
        // 受信ループ: 即時応答（sleep なし・決定的）。
        while let Ok(msg) = rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(RecordedCall::from_call(&call));
                    let outcome = state.respond(&call);
                    let _ = reply.send(outcome);
                }
                ShioriMsg::Unload { reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(expected_unload());
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                ShioriMsg::Close => break,
            }
        }
    });

    MockShiori {
        sender,
        handle,
        calls,
    }
}

// ============================================================================
// 失敗注入付き mock shiori（4.6 専用・区別語彙ごとの呼出失敗を観測する）
// ============================================================================

/// 失敗注入の区別語彙（[`ShioriFailure`] の 5 種に 1:1 対応する COPYABLE 記述子）。
///
/// [`ShioriFailure`] 自体は `Clone` を実装しないため（かつ `Fixture` は `Clone` 派生ゆえ
/// 中に持てない）、fixture 側にはこの Copy な種別のみを持たせ、実 `ShioriFailure` は mock の
/// `respond` 内でその都度 fresh に構築する（設計「Error Categories」の 5 語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailKind {
    /// [`ShioriFailure::Handshake`]（接続確立失敗）。
    Handshake,
    /// [`ShioriFailure::Timeout`]（タイムアウト）。
    Timeout,
    /// [`ShioriFailure::Ipc`]（helper 死活の一態様）。
    Ipc,
    /// [`ShioriFailure::Shiori`]（SHIORI エラー応答）。
    Shiori,
    /// [`ShioriFailure::Internal`]（kanade 内部規律違反・境界写像では生成されない・DD-IT-11）。
    /// 5.1 は記述子のみ追加し語彙を完全化する（実掃引は 5.2 の担当）。
    Internal,
}

impl FailKind {
    /// この種別に対応する fresh な [`ShioriFailure`] を構築する（`Clone` 不要ゆえ都度生成）。
    fn make(self) -> ShioriFailure {
        match self {
            FailKind::Handshake => ShioriFailure::Handshake("injected handshake failure".into()),
            FailKind::Timeout => ShioriFailure::Timeout("injected timeout".into()),
            FailKind::Ipc => ShioriFailure::Ipc("injected ipc failure".into()),
            FailKind::Shiori => ShioriFailure::Shiori("injected shiori error".into()),
            FailKind::Internal => ShioriFailure::Internal("injected internal violation".into()),
        }
    }
}

/// 失敗注入 mock shiori の記述子（どのイベント id の呼出をどの語彙で落とすか）。
///
/// `fail_on_id` に一致する **最初の** GET／NOTIFY 呼出で [`ShioriOutcome::Failed`] を返し、
/// それ以外の呼出は良性応答（[`FixtureState::respond`] と同一の既定表）を返す。全て Copy／
/// `&'static str` で構成され `ShioriFailure`（非 Clone）を保持しないため、`Fixture` 同様に
/// 気軽に複製できる。
#[derive(Debug, Clone, Copy)]
pub struct FailOn {
    /// 落とす対象のイベント id（例: `"OnInitialize"`）。
    pub id: &'static str,
    /// 返す失敗の区別語彙。
    pub kind: FailKind,
}

impl FailOn {
    /// `OnInitialize`（boot 最初の NOTIFY）を指定語彙で落とす記述子を返す。
    ///
    /// boot 系列の最初の呼出（`OnInitialize` NOTIFY・Req 1.1）を対象にすることで、Boot 駆動が
    /// 確実にこの呼出へ到達し、失敗が boot 応答待ち（`awaits_reply`）で受領され Unloading{Fault}
    /// へ倒れる経路を確実に踏む（設計「Error Categories」ShioriFailure 行）。
    pub fn on_initialize(kind: FailKind) -> Self {
        FailOn {
            id: "OnInitialize",
            kind,
        }
    }
}

/// 失敗注入付き mock shiori アクターを起動する（[`spawn_mock_shiori`] の派生・4.6 専用）。
///
/// `fixture` の良性応答表に加え、`fail_on.id` に一致する最初の呼出で
/// [`ShioriOutcome::Failed`]（`fail_on.kind` に対応する fresh な [`ShioriFailure`]）を返す。
/// 呼出の記録・[`ShioriMsg::Unload`]／[`ShioriMsg::Close`] の扱いは [`spawn_mock_shiori`] と同一
/// （Unload は best-effort で `Unloaded` を返し記録する）。これにより「区別語彙ごとの呼出失敗
/// → 観測可能な終了（Unloading{Fault}→Unload→Stopped）」を統合層で観測できる（Req 6.1）。
pub fn spawn_mock_shiori_failing(fixture: Fixture, fail_on: FailOn) -> MockShiori {
    let calls: Arc<Mutex<Vec<RecordedCall>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_body = Arc::clone(&calls);

    let (sender, handle) = spawn_actor::<ShioriMsg, _>("mock-shiori-failing", move |rx| {
        let mut state = FixtureState::new(fixture);
        let mut failed_once = false;
        while let Ok(msg) = rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(RecordedCall::from_call(&call));
                    // 対象イベントの最初の呼出のみ失敗させる（以降は良性表へ戻す）。
                    let outcome = if !failed_once && call_id(&call) == fail_on.id {
                        failed_once = true;
                        ShioriOutcome::Failed(fail_on.kind.make())
                    } else {
                        state.respond(&call)
                    };
                    let _ = reply.send(outcome);
                }
                ShioriMsg::Unload { reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(expected_unload());
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                ShioriMsg::Close => break,
            }
        }
    });

    MockShiori {
        sender,
        handle,
        calls,
    }
}

/// [`ShioriCall`] のイベント id の wire 形を取り出す（GET／NOTIFY 共通・失敗注入の突合用）。
fn call_id(call: &ShioriCall) -> &str {
    match call {
        ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => id.as_str(),
    }
}

// ============================================================================
// ブロッキング mock shiori（6.3 専用・呼出ブロック中の Tick catch-up を観測する）
// ============================================================================

/// ブロック対象の記述子（どの呼出を握り続けるか・COPYABLE）。
///
/// 実経路の SHIORI 呼出は本質的にブロックし得る（別プロセス往復）。mock は既定で即応するため
/// その窓が存在しない。本記述子で「特定の 1 呼出」を選び、その往復を明示的な解放まで握って
/// kanade を drive ループの `ReplyReceiver::recv` で止め、以降の Tick が inbox に溜まる catch-up
/// 窓を決定的に作る。ブロックするのは各種別で**最初の**一致呼出のみ（以降は即応へ戻す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOn {
    /// 指定 id の最初の GET 呼出をブロックする（例: `"OnSecondChange"`）。
    Get(&'static str),
    /// 指定 id の最初の NOTIFY 呼出をブロックする。
    Notify(&'static str),
}

impl BlockOn {
    /// この呼出が本記述子の対象か（method＋id 一致）を判定する。
    fn matches(&self, call: &ShioriCall) -> bool {
        match (self, call) {
            (BlockOn::Get(id), ShioriCall::Get { id: cid, .. }) => cid.as_str() == *id,
            (BlockOn::Notify(id), ShioriCall::Notify { id: cid, .. }) => cid.as_str() == *id,
            _ => false,
        }
    }
}

/// ブロック解放を mock shiori の受信ループへ伝えるゲートの共有部（Mutex＋Condvar）。
struct ShioriGateShared {
    inner: Mutex<ShioriGateInner>,
    cvar: std::sync::Condvar,
}

/// [`ShioriGateShared`] の Mutex 保護部（受領フラグ・解放フラグ）。
struct ShioriGateInner {
    /// 対象呼出を受領し、往復を握った（＝kanade が round-trip でブロック中）なら true。
    /// `wait_until_blocked` はこの成立を待つ（決定的バリア）。
    blocked_arrived: bool,
    /// テストが `release()` を呼んだら true（対象呼出の応答を送ってよい合図）。
    /// flag は Mutex 下で立ててから `notify_all`（lost wakeup 対策・SakuraGate と同一規律）。
    released: bool,
}

/// ブロッキング mock shiori のゲート（6.3 専用）。
///
/// [`wait_until_blocked`](ShioriGate::wait_until_blocked) で「kanade が対象呼出の往復で
/// ブロック中」であることを有界に確認し、[`release`](ShioriGate::release) でその往復に
/// fixture 既定の応答を送らせる。sleep も wall-clock も用いない（Mutex＋Condvar のみ）。
pub struct ShioriGate {
    shared: Arc<ShioriGateShared>,
}

impl ShioriGate {
    /// 対象呼出が mock へ到達し握られる（＝kanade が round-trip でブロック済み）まで有界に待つ。
    ///
    /// 期限内に成立しなければ `false`（テストは assert で失敗にする）。成立すれば `true`。
    /// flag は受信ループが Mutex 下で立ててから notify するため、本メソッドが wait に入る前に
    /// 到達しても取りこぼさない（lost wakeup なし）。
    pub fn wait_until_blocked(&self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        let mut inner = self.shared.inner.lock().expect("shiori gate mutex");
        while !inner.blocked_arrived {
            let now = std::time::Instant::now();
            if now >= deadline {
                return false;
            }
            let (guard, res) = self
                .shared
                .cvar
                .wait_timeout(inner, deadline - now)
                .expect("shiori gate condvar wait_timeout");
            inner = guard;
            if res.timed_out() && !inner.blocked_arrived {
                return false;
            }
        }
        true
    }

    /// 握られている対象呼出に fixture 既定の応答を送らせる（catch-up を解禁する）。
    ///
    /// flag を Mutex 下で立ててから `notify_all` するため、受信ループがまだ wait に入る前に
    /// 呼ばれても取りこぼさない。以降の（非対象）呼出は即応へ戻る。
    pub fn release(&self) {
        let mut inner = self.shared.inner.lock().expect("shiori gate mutex");
        inner.released = true;
        self.shared.cvar.notify_all();
    }
}

/// ブロッキング mock shiori アクターを起動する（[`spawn_mock_shiori`] の派生・6.3 専用）。
///
/// `block_on` が指す**最初の**呼出を受領したとき、記録した上で応答を送らず、受信ループ内で
/// [`ShioriGate::release`] まで Condvar で待つ（＝kanade の drive ループが `ReplyReceiver::recv`
/// で同期ブロックする窓を作る）。ブロック中に kanade inbox へ溜まった Tick は、解放後に順次
/// 処理される（catch-up・in-flight ≤ 1）。解放後の応答は当該呼出に対する fixture 既定
/// （[`FixtureState::respond`]）を用いる——非ブロック呼出（boot 各種・後続 OnSecondChange・
/// OnClose・Unload）は従来どおり即応する。
///
/// # デッドロック安全
/// 受信ループ内の Condvar 待ちは `release()` で解ける。テストが `release()` を呼ばずに終えても、
/// StartTalk 送信端 drop 等で kanade 側 round-trip が畳まれるだけでは本スレッドは解けないが、
/// テストは必ず `release()` を呼ぶ契約であり、全 join はテストの [`DEFAULT_TIMEOUT`] で有界。
pub fn spawn_mock_shiori_blocking(fixture: Fixture, block_on: BlockOn) -> (MockShiori, ShioriGate) {
    let calls: Arc<Mutex<Vec<RecordedCall>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_body = Arc::clone(&calls);

    let shared = Arc::new(ShioriGateShared {
        inner: Mutex::new(ShioriGateInner {
            blocked_arrived: false,
            released: false,
        }),
        cvar: std::sync::Condvar::new(),
    });
    let shared_body = Arc::clone(&shared);

    let (sender, handle) = spawn_actor::<ShioriMsg, _>("mock-shiori-blocking", move |rx| {
        let mut state = FixtureState::new(fixture);
        let mut blocked_once = false;
        while let Ok(msg) = rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(RecordedCall::from_call(&call));
                    // 対象の最初の呼出のみ握る（応答は release まで送らない）。
                    if !blocked_once && block_on.matches(&call) {
                        blocked_once = true;
                        // fixture 既定の応答を今のうちに決めておく（respond の出現カウンタは
                        // 呼出順に前進させたいので、ここで消費する＝実経路の 1 呼出＝1 応答）。
                        let outcome = state.respond(&call);
                        // 受領を通知（wait_until_blocked を起こす）→ release まで待つ。
                        {
                            let mut inner = shared_body.inner.lock().expect("shiori gate mutex");
                            inner.blocked_arrived = true;
                            shared_body.cvar.notify_all();
                            while !inner.released {
                                inner = shared_body
                                    .cvar
                                    .wait(inner)
                                    .expect("shiori gate condvar wait");
                            }
                        }
                        let _ = reply.send(outcome);
                    } else {
                        let outcome = state.respond(&call);
                        let _ = reply.send(outcome);
                    }
                }
                ShioriMsg::Unload { reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(expected_unload());
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                ShioriMsg::Close => break,
            }
        }
    });

    (
        MockShiori {
            sender,
            handle,
            calls,
        },
        ShioriGate { shared },
    )
}
