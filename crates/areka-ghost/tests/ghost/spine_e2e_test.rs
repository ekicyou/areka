//! 決定論 spine e2e（design.md「spine e2e（決定論・純 x64）」）。
//!
//! 本ファイルは 2 つのテスト専用型を提供する（task 4.1 の成果物）:
//! - [`ScriptedShioriBackend`] — `areka_kanade::ShioriBackend` を実装する台本 fake。
//! - [`RecordingSink`] — 演者非依存の単一出力契約 `areka_sakura::contract::CueSink` を実装する、
//!   `Clone` 可能な記録 sink（broadcast で全 cue を受ける）。
//!
//! 後続タスク（4.2〜4.7）はこのファイルへ boot〜close の各シナリオ（S1〜S6）の
//! `#[test]` を追加していく。本タスク（4.1）はその土台となる 2 型自体の構築・検証
//! （台本通りの応答・終了結果・死活遷移を任意に再現できること）のみを担う。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use areka_kanade::ShioriBackend;
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

/// host-32 IPC 有界 e2e の壁時計安全弁（ハング検出器）。兄弟 e2e 規約（inproc/real_pasta/
/// snapshot_capture = 60s）へ整合。意味論 deadline は MonotonicMs 仮想時間で注入されるため
/// この壁時計値はテスト意味論に影響せず、workspace 並列負荷の飢餓による偽赤のみを防ぐ。
const E2E_BOUND: std::time::Duration = std::time::Duration::from_secs(60);

/// talk 駆動の効果（cue 発火・TalkDone・その後段の close 握手）を待つ有界スピン。
/// **待っている間も dispatcher へ Tick を注入し続ける**のが本ヘルパの要点である。
///
/// # なぜ「待つ間も注入し続ける」必要があるか（2026-07-30・S1/S4/S5 の共通根因）
///
/// `TickerMode::Disabled` 下では仮想時刻は**注入された Tick でしか進まない**。ゆえに
/// 「最初の cue が出たら Tick を止めて `yield_now` だけで待つ」旧形は、スレッド
/// スケジューリング次第で**両方向に**壊れる:
///
/// - **注入不足**（引き渡しが速く `now` が talk horizon 未満で止まる）: 挨拶 talk
///   （`\s[0]hello\e`＝`hello` の 0.25s）が完了せず `TalkDone` が出ない。kanade は
///   DD-IT-12 により `Steady{talk: Some(greeting)}` で close を保留し続け、以降 Tick が
///   来ないので**永久に握手が始まらない**（S4 の `Unload` 待ち・S5 の `OnClose` 待ちが
///   60s 安全弁まで空転して落ちる）。
/// - **注入過多**（引き渡しが遅く、dispatcher の**無制限** inbox に Tick が溜まる）:
///   溜まった分を一気に drain して仮想時刻が horizon を飛び越え、初回起動 epilogue cue
///   （`areka.prop.set`@0.25）まで発火してしまう（S1 の cue 列アサートが 3 件期待で落ちる）。
///
/// どちらも「仮想時刻の進み具合」を**待機の副作用**に委ねていたことが原因である。本ヘルパは
/// 待機条件を `done` で明示的に表明させ、仮想時刻は常に単調前進させる——これにより
/// 「何を待っているか」と「時計をどれだけ進めたか」が分離され、結果が決定的になる。
///
/// `send_tick` は dispatcher 宛の Tick 送出のみを行うこと（kanade 宛の Tick は
/// `Steady` pump として消費され台本外の `OnSecondChange` NOTIFY を誘発する）。
fn spin_pumping_ticks(
    what: &str,
    now: &mut u64,
    mut send_tick: impl FnMut(u64),
    mut done: impl FnMut() -> bool,
) {
    let deadline = std::time::Instant::now() + E2E_BOUND;
    while std::time::Instant::now() < deadline {
        if done() {
            return;
        }
        send_tick(*now);
        *now += 1;
        std::thread::yield_now();
    }
    panic!("{what}（有界スピンが {E2E_BOUND:?} を超過）");
}

// ===================== ScriptedShioriBackend =====================

/// backend が受領した 1 呼出の記録（照合用・要件 7.1「発火内容を蓄積して照合できる」）。
///
/// `Get`/`Notify` は id・references を保持する（task-spec の「id + references 最低限」）。
/// `Unload`/`Status` は引数を持たないため variant のみで足りる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedCall {
    /// GET 呼出（応答を要するイベント）。
    Get { id: String, references: Vec<String> },
    /// NOTIFY 呼出（片道イベント）。
    Notify { id: String, references: Vec<String> },
    /// unload（正規 clean shutdown）呼出。
    Unload,
    /// status（非ブロッキング死活問い合わせ）呼出。
    Status,
}

/// [`ScriptedShioriBackend`] を組み立てるビルダー（台本の事前登録）。
///
/// GET/NOTIFY は id ごとに応答列（`VecDeque`）を積み上げ、呼出のたびに先頭から 1 件
/// 消費する（`RequestError`/`ShutdownError` は `Clone` を実装しないため、値そのものを
/// 使い切り消費する設計にする——スクリプトの再利用は想定しない）。
pub struct ScriptedShioriBackendBuilder {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    initial_status: HelperStatus,
}

impl ScriptedShioriBackendBuilder {
    /// 空の台本（既定 status=`Running`）から開始する。
    pub fn new() -> Self {
        Self {
            get_scripts: HashMap::new(),
            notify_scripts: HashMap::new(),
            unload_script: None,
            initial_status: HelperStatus::Running,
        }
    }

    /// `id` に対する GET 応答を 1 件、応答列の末尾へ積む（複数回呼べば FIFO に消費される）。
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

    /// 初期 `status()` を台本化する（既定は `Running`）。
    pub fn status(mut self, status: HelperStatus) -> Self {
        self.initial_status = status;
        self
    }

    /// backend 本体（アクタースレッドへ move する側）と、テストが状態変更・照合に使う
    /// [`ScriptedShioriHandle`] のペアを構築する。
    pub fn build(self) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        let status = Arc::new(Mutex::new(self.initial_status));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let backend = ScriptedShioriBackend {
            get_scripts: self.get_scripts,
            notify_scripts: self.notify_scripts,
            unload_script: self.unload_script,
            status: Arc::clone(&status),
            calls: Arc::clone(&calls),
        };
        let handle = ScriptedShioriHandle { status, calls };
        (backend, handle)
    }
}

impl Default for ScriptedShioriBackendBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 台本化したテスト専用 SHIORI backend（`areka_kanade::ShioriBackend` 実装・要件 7.1/7.6）。
///
/// プロセス spawn・実窓・i686 成果物を一切要さない（純 x64・要件 7.6）。応答・終了結果は
/// [`ScriptedShioriBackendBuilder`] で事前登録し、`status()` は [`ScriptedShioriHandle`]
/// 経由でテスト自身のスレッドから途中差し替え可能（helper がシナリオ途中で死ぬ様子を
/// 再現するための capability・後続タスクの S3 が利用する）。
pub struct ScriptedShioriBackend {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    status: Arc<Mutex<HelperStatus>>,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriBackend {
    /// ビルダー起点（[`ScriptedShioriBackendBuilder::new`] の別名）。
    pub fn builder() -> ScriptedShioriBackendBuilder {
        ScriptedShioriBackendBuilder::new()
    }
}

impl ShioriBackend for ScriptedShioriBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Get {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.get_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::get(\"{id}\"): no scripted response left (never configured or script exhausted)")
            })
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Notify {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.notify_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::notify(\"{id}\"): no scripted response left (never configured or script exhausted)")
            })
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Unload);
        self.unload_script.take().unwrap_or_else(|| {
            panic!("ScriptedShioriBackend::unload(): no scripted response configured")
        })
    }

    fn status(&mut self) -> HelperStatus {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Status);
        *self.status.lock().expect("status mutex poisoned")
    }
}

/// [`ScriptedShioriBackend`] をテスト側から観測・操作するためのハンドル。
///
/// backend 本体（`Box<dyn ShioriBackend>` としてアクタースレッドへ move される）とは
/// 独立に、`Arc` 共有を通じて status の途中差し替え・呼出記録の照合を行える。
#[derive(Clone)]
pub struct ScriptedShioriHandle {
    status: Arc<Mutex<HelperStatus>>,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriHandle {
    /// `status()` が以後返す値を差し替える（helper がシナリオ途中で死ぬ様子の再現）。
    /// テスト自身のスレッドから、backend が別スレッド（shiori actor）で生きている間に
    /// 呼べる。
    pub fn set_status(&self, status: HelperStatus) {
        *self.status.lock().expect("status mutex poisoned") = status;
    }

    /// 受領記録（`Arc` クローン）を返す。backend を別スレッドへ move した後も、
    /// このハンドルから発火列を照合できる。
    pub fn calls(&self) -> Arc<Mutex<Vec<RecordedCall>>> {
        Arc::clone(&self.calls)
    }
}

// ===================== RecordingSink =====================

/// テスト専用の `Clone` 可能な記録 sink（演者非依存の単一出力契約 [`CueSink`] を実装する）。
///
/// sakura の `MockSink`（`tests/ghost/` から見て他クレートの凍結面）とは同型だが
/// `Clone` を実装しない。dispatcher の per-talk 注入（`S: CueSink + Clone`/`T: CueSink + Clone`）
/// を満たすため、`tests/ghost/` 側で定義し直す（sakura の `sink.rs` には手を入れない・
/// design.md 「spine e2e」参照）。broadcast ゆえ登録された全 sink が全 cue を受ける
/// （surface/text スロットの別なく同一の全 cue が届く・演者側 relevance が action を選別する）。
#[derive(Clone)]
pub struct RecordingSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl RecordingSink {
    /// 空の共有蓄積を持つ sink を生成する。
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 共有蓄積の `Arc` クローンを返す。`RecordingSink` を clone してアクタースレッドへ
    /// 渡した後も、このハンドルから発火列を照合できる。
    pub fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }
}

impl Default for RecordingSink {
    fn default() -> Self {
        Self::new()
    }
}

impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("records mutex poisoned")
            .push(cue);
    }
}

#[cfg(test)]
#[path = "spine_e2e_test_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "spine_e2e_test_broadcast_relevance_partition.rs"]
mod broadcast_relevance_partition;

#[cfg(test)]
#[path = "spine_e2e_test_s1_boot_success.rs"]
mod s1_boot_success;

#[cfg(test)]
#[path = "spine_e2e_test_s2_connect_failure.rs"]
mod s2_connect_failure;

#[cfg(test)]
#[path = "spine_e2e_test_s3_helper_liveness_detected.rs"]
mod s3_helper_liveness_detected;

#[cfg(test)]
#[path = "spine_e2e_test_s4_close_handshake.rs"]
mod s4_close_handshake;

#[cfg(test)]
#[path = "spine_e2e_test_s5_close_deadline.rs"]
mod s5_close_deadline;

#[cfg(test)]
#[path = "spine_e2e_test_s6_full_disconnect.rs"]
mod s6_full_disconnect;

#[cfg(test)]
#[path = "spine_e2e_test_global_log_probe.rs"]
mod global_log_probe;

#[cfg(test)]
#[path = "spine_e2e_test_s7_second_boot_record_present.rs"]
mod s7_second_boot_record_present;
