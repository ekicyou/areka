//! 決定論 spine テストハーネス（R8・tasks.md task 6.1）のうち、本体を引かない支えの 2 層。
//!
//! - ⑴ 待ちの部品と GPU の装置の許可 `GpuPermit`（子のファイル `spine_wait.rs`）。
//! - ⑵ 台本の SHIORI（`ScriptedShioriBackend` 一式・[`HOMEURL_RESOURCE`]・進行状態の記録の型と本体）と
//!   「解かれるまで固まる」台本の置き場 [`hold_support`]（`spine_hold_support.rs`）。
//!
//! 背骨の足場（`SpineHarness`・`make_world_with_gpu` ほか＝⑶）とそれを使う spine の
//! テストは本体の部品も使うので、本体の `runtime_tests/spine.rs` に置く（spec: runtime-crate-split 設計 D4）。
//! このモジュールは `#[cfg(any(test, feature = "test-support"))]` で立ち、本体のテストは `test-support` で使う。
//!
//! **「sleep 不使用」の射程（R7.9・2026-07-31 に明確化）**: spine で言う sleep 不使用とは
//! **時刻を進めるために sleep しない**ことであり（時刻前進は注入 Tick のみ＝決定論の源）、
//! **有界待機の poll-backoff に用いる短い sleep は明示例外**である。反復回数のみの上限は CPU
//! 競合下で数 ms で尽き、製品コードが正常でも偽陽性の赤を出すため、待機はすべて壁時計で有界化してある。
//! 打ち切り方は 3 通りある。目印のある待ち（[`Progress::Count`]）は「目印が [`SPIN_WAIT`]（30 秒）進まなければ
//! 打ち切り・総時間は 300 秒まで」、GPU の許可の待ちは「許可が 500 秒返らなければ打ち切り」、
//! [`run_bounded`] は総時間の `recv_timeout` である。⑴ が持つ待ちの形は**純粋ポーリング**（各反復が系を進めない・
//! 別スレッドの到着を読むだけ）→ [`wait_until`]。密 yield（`DENSE_SPIN`（`spine_wait.rs`・60 ms）の窓）で
//! 速い経路の検出遅延を犠牲にせず、窓を過ぎたら [`BACKOFF_SLEEP`] へ落としてコア占有をやめる
//! （PR #96 で導入）。回収（settle）の形も ⑴ にあり（[`settle_bounded`]）、ハイブリッドの形は ⑶ の側（本体の `runtime_tests/spine.rs`）の説明にある。
//!
//! # 依存の申し送り（CONCERNS 相当）
//!
//! scripted `ShioriBackend` は `RequestError`/`ExitKind`/`ShutdownError`/`HelperStatus`
//! （`shiori_host32_host` 由来）をトレイト戻り値型として**名前で**参照する。`areka_kanade` は
//! これら host32 型を re-export しないため、`areka-runtime/Cargo.toml` に `shiori-host32-host`
//! （既に areka-ghost/areka-kanade 経由で推移ビルド済み・x64・新規外部依存ではない）を、
//! `test-support` のときだけ付く optional の依存と、自分のテストのための `[dev-dependencies]` の 2 行で置く。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use areka_kanade::{ShioriBackend, ShioriUnblock};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

use self::hold_support::{HoldAt, HoldGate};

// ===========================================================================
// 進行状態の記録（第 2 系統・R3.8）の型と本体
//
// 台本の SHIORI（下）が書き込み、本体の適合一周走行（`runtime_tests/spine_conformance_support.rs`）が
// 読む。定義は 1 つで、⑵ と同じ所に置く（spec: runtime-crate-split 段 2 の記録「設計からの外れ」）。
// ===========================================================================

/// 進行状態の記録 1 件（呼出 id と、その呼出に載った**組み立て済み**の進行状態の対）。
///
/// `status` は kanade が `ExecutionStatus::render()` 済みの wire 値をそのまま持つ
/// （`crates/areka-kanade/src/shiori/real.rs`）。`None` は「`Status` ヘッダ行を
/// 出さない」ことを表す値であって、記録の欠落ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedStatus {
    /// 呼出のイベント id（wire 形の逐語）。
    pub id: String,
    /// 組み立て済みの進行状態。`None`＝ヘッダ行を出さない。
    pub status: Option<String>,
}

/// 進行状態の記録の台帳（受け口本体と観測ハンドルが `Arc` で共有する）。
pub type StatusLedger = Arc<Mutex<Vec<RecordedStatus>>>;

/// 台本受け口の書き込み点の本体（照会・片道の 2 か所から呼ばれる）。
///
/// **書き込み専用**である。既存の呼出記録とは別の台帳へ積むだけで、既存の読み手を 1 つも
/// 増やさない（design D3「追補の形（挙動を変えない）」）。
pub(crate) fn record_status(ledger: &StatusLedger, id: &str, status: Option<&str>) {
    ledger
        .lock()
        .expect("status ledger mutex poisoned")
        .push(RecordedStatus {
            id: id.to_string(),
            status: status.map(str::to_string),
        });
}

/// 観測ハンドルの取り出し口の本体（進行状態の記録のスナップショットを呼出順で返す）。
pub(crate) fn snapshot_status_calls(ledger: &StatusLedger) -> Vec<RecordedStatus> {
    ledger.lock().expect("status ledger mutex poisoned").clone()
}

// ===========================================================================
// ScriptedShioriBackend（DD-11・areka 側 spine ローカルの最小 fake）
//
// `areka_kanade::ShioriBackend` を実装する台本 fake。プロセス spawn・実窓・i686 成果物を
// 一切要さない（純 x64・R8.6）。応答・終了結果は事前登録し、`status()` は固定値を返す。
// トレイト形状・戻り値型は `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の
// `ScriptedShioriBackend`（task 4.1 成果物）を正典として写した最小版。
// ===========================================================================

/// backend が受領した 1 呼出の記録（照合用）。`Get`/`Notify` は id・references を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedCall {
    /// GET 呼出（応答を要するイベント）。
    Get { id: String, references: Vec<String> },
    /// NOTIFY 呼出（片道イベント）。
    Notify { id: String, references: Vec<String> },
    /// unload（正規 clean shutdown）呼出。
    Unload,
}

/// [`ScriptedShioriBackend`] を組み立てるビルダー（台本の事前登録）。
///
/// GET/NOTIFY は id ごとに応答列（`VecDeque`）を積み、呼出のたびに先頭から 1 件消費する
/// （`RequestError`/`ShutdownError` は `Clone` 非実装ゆえ値を使い切り消費する設計）。
pub struct ScriptedShioriBackendBuilder {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    hold: Option<HoldAt>,
}

impl ScriptedShioriBackendBuilder {
    /// 空の台本（既定 status=`Running`）から開始する。
    fn new() -> Self {
        Self {
            get_scripts: HashMap::new(),
            notify_scripts: HashMap::new(),
            unload_script: None,
            hold: None,
        }
    }

    /// `id` に対する GET 応答を 1 件、応答列の末尾へ積む（複数回で FIFO 消費）。
    pub fn get(
        mut self,
        id: impl Into<String>,
        response: Result<Option<String>, RequestError>,
    ) -> Self {
        self.get_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `id` に対する NOTIFY 応答を 1 件、応答列の末尾へ積む。
    pub fn notify(mut self, id: impl Into<String>, response: Result<(), RequestError>) -> Self {
        self.notify_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `unload()` の結果を台本化する（一度きり消費・`Option::take` で払い出す）。
    pub fn unload(mut self, response: Result<ExitKind, ShutdownError>) -> Self {
        self.unload_script = Some(response);
        self
    }

    /// backend 本体（アクタースレッドへ move する側）と、テストが照合に使う
    /// [`ScriptedShioriHandle`] のペアを構築する。
    ///
    /// # username prefetch の既定台本（task 8.2・R9.1/9.2・kanade prefetch boot 図）
    ///
    /// sylphya 機能（task 6.2/8.2）が正典 boot 系列へ **username SHIORI Resource GET prefetch**
    /// （OnInitialize NOTIFY の後・OnFirstBoot GET の前）を追加した。全 boot がこの prefetch を発行
    /// するため、テストが `username` GET を明示台本化していなければ **既定で `Ok(None)`（NoContent）**
    /// を 1 件補う。「カスタム username 無し」＝既定 username 世界（204→既定・既定は sakura 常駐）を
    /// faithful に再現する DRY 既定であり、将来の spine テストが username GET を毎回書かずとも
    /// backend が panic しない（`boot_with` に手組み backend を渡す経路も同じ既定で保護される）。
    /// テストが独自 username 応答を要すれば `.get("username", …)` で明示上書きでき、その場合は
    /// 既に登録済みゆえ既定は補われない。
    pub fn build(mut self) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        self.get_scripts
            .entry("username".to_string())
            .or_insert_with(|| VecDeque::from([Ok(None)]));

        let calls = Arc::new(Mutex::new(Vec::new()));
        let status_calls: StatusLedger = Arc::new(Mutex::new(Vec::new()));
        let gate = Arc::new(HoldGate::new(self.hold));
        let backend = ScriptedShioriBackend {
            get_scripts: self.get_scripts,
            notify_scripts: self.notify_scripts,
            unload_script: self.unload_script,
            calls: Arc::clone(&calls),
            status_calls: Arc::clone(&status_calls),
            gate: Arc::clone(&gate),
        };
        let handle = ScriptedShioriHandle {
            calls,
            status_calls,
            gate,
        };
        (backend, handle)
    }
}

/// 更新の窓口が定常到達のたびに照会する Resource の名前（台本に無ければ NoContent で答える）。
pub const HOMEURL_RESOURCE: &str = "homeurl";

/// 台本化したテスト専用 SHIORI backend（`areka_kanade::ShioriBackend` 実装・R8.1/R8.6）。
///
/// `OnBalloonTimeout` だけは列の照合の外（実時間で決まるので、台本に無ければ記録せず 204 で答える）。
pub struct ScriptedShioriBackend {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    /// 進行状態の記録（第 2 系統・R3.8）。既存 `calls` とは別の台帳で、書き込みのみ。
    status_calls: StatusLedger,
    /// 「解かれるまで固まる」台本と解く手の置き場（要件 7.1・`spine_hold_support.rs`）。
    gate: Arc<HoldGate>,
}

impl ScriptedShioriBackend {
    /// ビルダー起点。
    pub fn builder() -> ScriptedShioriBackendBuilder {
        ScriptedShioriBackendBuilder::new()
    }
}

impl ShioriBackend for ScriptedShioriBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        // 台本に無い OnBalloonTimeout は、記録にも門にも載せずに 204 で答える。台詞の後のバルーンの時間切れ
        // （既定 30 秒）は実時間で決まり、負荷の下では降ろす前や呼び出しの列の途中に割り込む＝来るかどうかが
        // 時刻しだいの呼び出しで、足場のテストが確かめる列の外（送る正しさは areka-kanade のテストが固定）。
        // 台本に書いたテストには今までどおり記録する（areka-P0-ghost-session-test-load-flake の負荷の回）。
        if id == "OnBalloonTimeout" && self.get_scripts.get(id).is_none_or(VecDeque::is_empty) {
            return Ok(None);
        }
        record_status(&self.status_calls, id, status);
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Get {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.gate.pass_get(id)?;
        self.get_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            // 回数は定常到達・台詞の数で決まる＝台本に無ければ homeurl も OnTranslate も 204 で答える。
            .or_else(|| (id == HOMEURL_RESOURCE || id == "OnTranslate").then_some(Ok(None)))
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::get(\"{id}\"): no scripted response left")
            })
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        record_status(&self.status_calls, id, status);
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Notify {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.gate.pass_notify(id)?;
        self.notify_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::notify(\"{id}\"): no scripted response left")
            })
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Unload);
        self.gate.pass_unload()?;
        self.unload_script.take().unwrap_or_else(|| {
            panic!("ScriptedShioriBackend::unload(): no scripted response configured")
        })
    }

    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }

    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        Some(self.gate.unblock())
    }
}

/// [`ScriptedShioriBackend`] をテスト側から観測するためのハンドル（`Arc` 共有）。
///
/// backend 本体を別スレッド（shiori actor）へ move した後も、このハンドルから発火列を照合できる。
#[derive(Clone)]
pub struct ScriptedShioriHandle {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    /// 進行状態の記録（第 2 系統・R3.8）の共有台帳。取り出し口は [`Self::status_calls`]。
    status_calls: StatusLedger,
    gate: Arc<HoldGate>,
}

impl ScriptedShioriHandle {
    /// 受領記録（`Get`・`Notify`・`Unload`）のスナップショットを返す。状態の問い合わせは記録しない。
    pub fn non_status_calls(&self) -> Vec<RecordedCall> {
        self.calls.lock().expect("calls mutex poisoned").clone()
    }

    /// 受けた呼び出し（`Get`・`Notify`・`Unload`）の数。記録の写しは作らない
    /// （待ちの進みの目印に毎回読む口・areka-P0-ghost-session-test-load-flake 3.1）。
    pub fn call_count(&self) -> u64 {
        self.calls.lock().expect("calls mutex poisoned").len() as u64
    }

    /// 進行状態の記録（呼出 id と組み立て済み進行状態の対）のスナップショットを返す（R3.8）。
    pub fn status_calls(&self) -> Vec<RecordedStatus> {
        snapshot_status_calls(&self.status_calls)
    }
}

// ===========================================================================
// GPU / fixture / 有界待機ヘルパ（draw_readback_test／ghost spine 定石の踏襲）
// ===========================================================================

#[path = "spine_wait.rs"]
mod wait;

// 待ちの部品は子のファイル `spine_wait.rs` に置き、同じ名前で出し直す（兄弟のテストは `super::SPIN_WAIT` などで引く）。
pub use self::wait::{BACKOFF_SLEEP, SPIN_WAIT, join_bounded};
pub use self::wait::{
    GpuPermit, Progress, WaitFailure, join_thread_bounded, run_bounded, run_bounded_watching,
    settle_bounded, settle_bounded_with, wait_recv, wait_until, wait_until_with,
};

#[path = "spine_hold_support.rs"]
pub mod hold_support;
#[cfg(test)]
#[path = "spine_wait_tests.rs"]
mod wait_tests;

// 本体のテスト（`crates/areka/src/runtime_tests/`）が使う私有の欄へそのまま渡す口（`test-support` のときだけ・
// 本番の可視性は変えない・spec: runtime-crate-split 設計 D3 ⑷）。
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::{ScriptedShioriHandle, StatusLedger};

    /// [`ScriptedShioriHandle`] の進行状態の記録の共有台帳（`status_calls`）。
    pub fn status_calls(handle: &ScriptedShioriHandle) -> &StatusLedger {
        &handle.status_calls
    }
}
