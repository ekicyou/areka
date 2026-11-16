//! 決定論 spine テストハーネス（R8・tasks.md task 6.1）。
//!
//! `cargo test --workspace`（外部 CI 無し・ローカル DoD ゲート）で常設観測する、起動〜発話〜
//! 終了の全経路を **sleep 不使用**・**注入 Tick のみ**・**実 GPU（MTA）** で通す
//!
//! **「sleep 不使用」の射程（R7.9・2026-07-31 に明確化）**: 本ファイルで言う sleep 不使用とは
//! **時刻を進めるために sleep しない**ことであり（時刻前進は注入 Tick のみ＝決定論の源）、
//! **有界待機の poll-backoff に用いる短い sleep は明示例外**である。反復回数のみの上限は CPU
//! 競合下で数 ms で尽き、製品コードが正常でも偽陽性の赤を出すため、待機はすべて壁時計
//! [`SPIN_WAIT`] で有界化してある。待機の形は**3 種**（いずれも期限は [`SPIN_WAIT`]）:
//!
//! - **純粋ポーリング**（各反復が系を進めない・別スレッドの到着を読むだけ）→ [`wait_until`]。
//!   密 yield（[`DENSE_SPIN`](wait::DENSE_SPIN) の窓）で速い経路の検出遅延を犠牲にせず、窓を過ぎたら
//!   [`BACKOFF_SLEEP`] へ落としてコア占有をやめる（PR #96 で導入）。
//! - **ハイブリッド**（毎反復 Tick を注入して系を進めつつ別スレッドの結果も待つ）→ 各呼出点の
//!   自前ループ＋`sleep(200µs)` の poll-backoff。[`wait_until`] は純粋ポーリング専用ゆえ
//!   流用しない。送出ごとに `yield_now` で回すと unbounded channel を洪水させつつ worker を
//!   CPU 飢餓させる二重の害があり、短い sleep でペーシングするのが根治（areka-kanade 先例）。
//!   加えて注入時刻が観測を追い越さないこと（R7.8 の頭打ち）が別途必要。
//! - **回収（settle）**（「尽きるのが正常」の残余回収＝負検証）→ [`settle_bounded`]。時刻前進は
//!   呼出側が前段で注入し終えており、本体は観測しかしない。打ち切りは反復回数ではなく
//!   **[`SETTLE_MIN`] の最小持続 かつ 連続 [`SETTLE_QUIET_ROUNDS`] 回 0 件**の両立で、反復間は
//!   [`BACKOFF_SLEEP`] の短い sleep のみ（要件 4.2・4.4）。負荷で回収機会が縮まないことが要点。
//!
//! 詳細な根拠は [`drive_shell_shown`] の doc を参照。
//! 決定論 spine の**土台**。本 task（6.1）はハーネス（scripted `ShioriBackend`＋実 sink 結線＋
//! GPU World＋frame フェーズ直接駆動）と、boot→Tick→attach 到達をスモークレベルで固定する
//! `#[test]` を所有する。豊富な観測（S1 ピクセル readback・S2 typewriter・S3 `\b` 配送・
//! S5 close 握手）は後続 task（6.2/6.3）が本ハーネスの上に構築する。
//!
//! # 構造上の逸脱（設計 File Structure との差分・CONCERNS 相当）
//!
//! design.md の File Structure は spine を外部結合テスト `tests/emo2_boot_spine_test.rs` に置くが、
//! `areka` は `[[bin]]` のみ（`[lib]` 無し）で外部 `tests/` から bin 内部項目
//! （`crate::{PresentBridge, RuntimeWiring, run_attach_phase, ...}`）へ到達できない。
//! spine は Tick 注入＋GPU readback＋frame フェーズ直接駆動を **in-process** で行う必要があり
//! （bin を起動する外部テストでは不能）、既存 repo 慣行（`runtime::wire_tests`・
//! `areka_placement::spawn` のテスト）に従い **in-crate `#[cfg(test)]` モジュール** として置く。
//!
//! # 依存の申し送り（CONCERNS 相当）
//!
//! scripted `ShioriBackend` は `RequestError`/`ExitKind`/`ShutdownError`/`HelperStatus`
//! （`shiori_host32_host` 由来）をトレイト戻り値型として**名前で**参照する。`areka_kanade` は
//! これら host32 型を re-export しないため、`areka/Cargo.toml` の `[dev-dependencies]` へ
//! `shiori-host32-host`（既に areka-ghost/areka-kanade 経由で推移ビルド済み・x64・新規外部依存
//! ではない）を追加した。

use areka_placement::follow::OffsetBase;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_actor::ActorHandle;
use areka_emo_compose::{BindSet, PatternState};
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostRuntime, ShioriWiring, TickerMode, boot_with_kanade_stop};
use areka_kanade::{CloseReason, KanadeNotice, MonotonicMs, ShioriBackend, ShioriUnblock};
use areka_sakura::ActorKey;
use areka_seriko::{
    AnimationTable, BindResolver, LoopRng, SerikoLoopConfig, SerikoSink, SurfaceResolver,
    spawn_seriko,
};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use log_capture_kit::{LineFormat, capture_lines};

use super::sample_test_support::acquire_emo2;
use sample_ghost_kit::SampleRoot;
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{DPI, GraphicsCore, Point, WindowHandle, WindowPos, WucGraphicsResource};
use wintf::executor::{FilterResult, JoinHandle, MessageLoop};

use areka_placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use areka_placement::source::GhostTitles;
use areka_placement::spawn::{GhostWindows, spawn_ghost_windows};

use super::adapter::PresentBridge;
use super::assets::{BootAssets, LoopTables, actor_keyed_balloon_tables, build_boot_assets};
use super::frame::{
    RuntimeWiring, all_shown, run_attach_phase, run_dpi_phase, run_move_drain_with, run_text_phase,
    run_text_scale_phase,
};
use super::move_cue::{MoveCueSink, PositionCue};
use super::talk_clock::{ClockedTextSink, TalkClock};
use super::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};
use super::target_map::{balloon_target, shell_target};
use crate::session::ghost_quit::KanadeNoticeRx;
// 進行状態の記録（第 2 系統・R3.8）の型と本体は兄弟ファイル側に置く（design D3）。
use self::conformance_support::{
    RecordedStatus, StatusLedger, record_status, snapshot_status_calls,
};
use self::hold_support::{HoldAt, HoldGate};

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
pub(crate) enum RecordedCall {
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
/// GET/NOTIFY は id ごとに応答列（`VecDeque`）を積み、呼出のたびに先頭から 1 件消費する
/// （`RequestError`/`ShutdownError` は `Clone` 非実装ゆえ値を使い切り消費する設計）。
pub(crate) struct ScriptedShioriBackendBuilder {
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
    pub(crate) fn get(
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
    pub(crate) fn notify(
        mut self,
        id: impl Into<String>,
        response: Result<(), RequestError>,
    ) -> Self {
        self.notify_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `unload()` の結果を台本化する（一度きり消費・`Option::take` で払い出す）。
    pub(crate) fn unload(mut self, response: Result<ExitKind, ShutdownError>) -> Self {
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
    pub(crate) fn build(mut self) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
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
            status: HelperStatus::Running,
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
pub(crate) const HOMEURL_RESOURCE: &str = "homeurl";

/// 台本化したテスト専用 SHIORI backend（`areka_kanade::ShioriBackend` 実装・R8.1/R8.6）。
pub(crate) struct ScriptedShioriBackend {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    status: HelperStatus,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    /// 進行状態の記録（第 2 系統・R3.8）。既存 `calls` とは別の台帳で、書き込みのみ。
    status_calls: StatusLedger,
    /// 「解かれるまで固まる」台本と解く手の置き場（要件 7.1・`spine_hold_support.rs`）。
    gate: Arc<HoldGate>,
}

impl ScriptedShioriBackend {
    /// ビルダー起点。
    pub(crate) fn builder() -> ScriptedShioriBackendBuilder {
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
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Status);
        self.status
    }

    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        Some(self.gate.unblock())
    }
}

/// [`ScriptedShioriBackend`] をテスト側から観測するためのハンドル（`Arc` 共有）。
///
/// backend 本体を別スレッド（shiori actor）へ move した後も、このハンドルから発火列を照合できる。
#[derive(Clone)]
pub(crate) struct ScriptedShioriHandle {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    /// 進行状態の記録（第 2 系統・R3.8）の共有台帳。取り出し口は [`Self::status_calls`]。
    status_calls: StatusLedger,
    gate: Arc<HoldGate>,
}

impl ScriptedShioriHandle {
    /// 受領記録（非 Status のみ）のスナップショットを返す（死活監視ノイズを除外）。
    pub(crate) fn non_status_calls(&self) -> Vec<RecordedCall> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .cloned()
            .collect()
    }

    /// 受けた呼び出し（`Get`・`Notify`・`Unload`）の数。状態の問い合わせ `Status` は数えず、記録の写しも
    /// 作らない（待ちの進みの目印に毎回読む口・areka-P0-ghost-session-test-load-flake 3.1）。
    pub(crate) fn call_count(&self) -> u64 {
        let calls = self.calls.lock().expect("calls mutex poisoned");
        calls
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .count() as u64
    }

    /// 進行状態の記録（呼出 id と組み立て済み進行状態の対）のスナップショットを返す（R3.8）。
    fn status_calls(&self) -> Vec<RecordedStatus> {
        snapshot_status_calls(&self.status_calls)
    }
}

// ===========================================================================
// GPU / fixture / 有界待機ヘルパ（draw_readback_test／ghost spine 定石の踏襲）
// ===========================================================================

#[path = "spine_wait.rs"]
mod wait;

// 待ちの部品は子のファイル `spine_wait.rs` に置き、同じ名前で出し直す（兄弟のテストは `super::SPIN_WAIT` などで引く）。
use self::wait::{BACKOFF_SLEEP, SPIN_WAIT, join_bounded};
pub(crate) use self::wait::{
    GpuPermit, Progress, WaitFailure, join_thread_bounded, run_bounded, run_bounded_watching,
    wait_recv, wait_until, wait_until_with,
};

#[path = "spine_hold_support.rs"]
pub(crate) mod hold_support;
#[cfg(test)]
#[path = "spine_wait_tests.rs"]
mod wait_tests;
