//! 決定論 spine テストハーネス（R8・tasks.md task 6.1）。
//!
//! `cargo test --workspace`（外部 CI 無し・ローカル DoD ゲート）で常設観測する、起動〜発話〜
//! 終了の全経路を **sleep 不使用**・**注入 Tick のみ**・**headless GPU（WARP・MTA）** で通す
//!
//! **「sleep 不使用」の射程（R7.9・2026-07-31 に明確化）**: 本ファイルで言う sleep 不使用とは
//! **時刻を進めるために sleep しない**ことであり（時刻前進は注入 Tick のみ＝決定論の源）、
//! **有界待機の poll-backoff に用いる短い sleep は明示例外**である。反復回数のみの上限は CPU
//! 競合下で数 ms で尽き、製品コードが正常でも偽陽性の赤を出すため、待機はすべて壁時計
//! [`SPIN_WAIT`] で有界化してある。待機の形は**3 種**（いずれも期限は [`SPIN_WAIT`]）:
//!
//! - **純粋ポーリング**（各反復が系を進めない・別スレッドの到着を読むだけ）→ [`spin_wait_until`]。
//!   密 yield（[`SPIN_YIELD_BUDGET`]）で速い経路の検出遅延を犠牲にせず、予算超過後に
//!   [`BACKOFF_SLEEP`] へ落としてコア占有をやめる（PR #96 で導入）。
//! - **ハイブリッド**（毎反復 Tick を注入して系を進めつつ別スレッドの結果も待つ）→ 各呼出点の
//!   自前ループ＋`sleep(200µs)` の poll-backoff。[`spin_wait_until`] は純粋ポーリング専用ゆえ
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
//! （`crate::emo2_boot::{PresentBridge, Emo2Wiring, run_attach_phase, ...}`）へ到達できない。
//! spine は Tick 注入＋GPU readback＋frame フェーズ直接駆動を **in-process** で行う必要があり
//! （bin を起動する外部テストでは不能）、既存 repo 慣行（`emo2_boot::wire_tests`・
//! `placement::spawn` のテスト）に従い **in-crate `#[cfg(test)]` モジュール** として置く。
//!
//! # 依存の申し送り（CONCERNS 相当）
//!
//! scripted `ShioriBackend` は `RequestError`/`ExitKind`/`ShutdownError`/`HelperStatus`
//! （`shiori_host32_host` 由来）をトレイト戻り値型として**名前で**参照する。`areka_kanade` は
//! これら host32 型を re-export しないため、`areka/Cargo.toml` の `[dev-dependencies]` へ
//! `shiori-host32-host`（既に areka-ghost/areka-kanade 経由で推移ビルド済み・x64・新規外部依存
//! ではない）を追加した。

use crate::placement::follow::OffsetBase;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_actor::{ActorError, ActorHandle};
use areka_emo_compose::{BindSet, PatternState};
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{
    GhostBootOptions, GhostRuntime, ShioriWiring, SystemVarWiring, TickerMode, boot,
};
use areka_kanade::{CloseReason, MonotonicMs, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_sakura::ActorKey;
use areka_seriko::{
    AnimationTable, BindResolver, LoopRng, SerikoLoopConfig, SerikoSink, SurfaceResolver,
    spawn_seriko,
};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use log_capture_kit::{LineFormat, capture_lines};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{DPI, GraphicsCore, Point, WindowHandle, WindowPos, WucGraphicsResource};
use wintf::executor::{FilterResult, JoinHandle, MessageLoop};

use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};

use super::adapter::PresentBridge;
use super::assets::{BootAssets, LoopTables, actor_keyed_balloon_tables, build_boot_assets};
use super::frame::{
    Emo2Wiring, run_attach_phase, run_dpi_phase, run_move_drain_phase, run_text_phase,
    run_text_scale_phase,
};
use super::move_cue::{MoveCueSink, MoveDirective};
use super::talk_clock::{ClockedTextSink, TalkClock};
use super::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};
use super::target_map::{balloon_target, shell_target};
// 進行状態の記録（第 2 系統・R3.8）の型と本体は兄弟ファイル側に置く（design D3）。
use self::conformance_support::{
    RecordedStatus, StatusLedger, record_status, snapshot_status_calls,
};

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
enum RecordedCall {
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
struct ScriptedShioriBackendBuilder {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
}

impl ScriptedShioriBackendBuilder {
    /// 空の台本（既定 status=`Running`）から開始する。
    fn new() -> Self {
        Self {
            get_scripts: HashMap::new(),
            notify_scripts: HashMap::new(),
            unload_script: None,
        }
    }

    /// `id` に対する GET 応答を 1 件、応答列の末尾へ積む（複数回で FIFO 消費）。
    fn get(
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
    fn notify(mut self, id: impl Into<String>, response: Result<(), RequestError>) -> Self {
        self.notify_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `unload()` の結果を台本化する（一度きり消費・`Option::take` で払い出す）。
    fn unload(mut self, response: Result<ExitKind, ShutdownError>) -> Self {
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
    fn build(mut self) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        self.get_scripts
            .entry("username".to_string())
            .or_insert_with(|| VecDeque::from([Ok(None)]));

        let calls = Arc::new(Mutex::new(Vec::new()));
        let status_calls: StatusLedger = Arc::new(Mutex::new(Vec::new()));
        let backend = ScriptedShioriBackend {
            get_scripts: self.get_scripts,
            notify_scripts: self.notify_scripts,
            unload_script: self.unload_script,
            status: HelperStatus::Running,
            calls: Arc::clone(&calls),
            status_calls: Arc::clone(&status_calls),
        };
        let handle = ScriptedShioriHandle {
            calls,
            status_calls,
        };
        (backend, handle)
    }
}

/// 台本化したテスト専用 SHIORI backend（`areka_kanade::ShioriBackend` 実装・R8.1/R8.6）。
struct ScriptedShioriBackend {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    status: HelperStatus,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    /// 進行状態の記録（第 2 系統・R3.8）。既存 `calls` とは別の台帳で、書き込みのみ。
    status_calls: StatusLedger,
}

impl ScriptedShioriBackend {
    /// ビルダー起点。
    fn builder() -> ScriptedShioriBackendBuilder {
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
        record_status(&self.status_calls, id, status);
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
}

/// [`ScriptedShioriBackend`] をテスト側から観測するためのハンドル（`Arc` 共有）。
///
/// backend 本体を別スレッド（shiori actor）へ move した後も、このハンドルから発火列を照合できる。
#[derive(Clone)]
struct ScriptedShioriHandle {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    /// 進行状態の記録（第 2 系統・R3.8）の共有台帳。取り出し口は [`Self::status_calls`]。
    status_calls: StatusLedger,
}

impl ScriptedShioriHandle {
    /// 受領記録（非 Status のみ）のスナップショットを返す（死活監視ノイズを除外）。
    fn non_status_calls(&self) -> Vec<RecordedCall> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .cloned()
            .collect()
    }

    /// 進行状態の記録（呼出 id と組み立て済み進行状態の対）のスナップショットを返す（R3.8）。
    fn status_calls(&self) -> Vec<RecordedStatus> {
        snapshot_status_calls(&self.status_calls)
    }
}

// ===========================================================================
// GPU / fixture / 有界待機ヘルパ（draw_readback_test／ghost spine 定石の踏襲）
// ===========================================================================

/// **別スレッドの進行を待つ**有界スピンの猶予（sleep 不使用・`yield_now` のみで回す協調ループ用）。
///
/// # 反復回数で打ち切ってはならない
///
/// `yield_now()` のビジーウェイトでは **反復回数が経過時間の代理にならない**。CPU 競合下
/// （`cargo test --workspace` の並行実行・ウイルス対策の再スキャン等）では、待っている相手スレッドが
/// 一度も走らないまま数十万回の yield が尽きうる（steering
/// `areka-defender-rescan-starves-cooperative-test-loops`）。実測では旧 `for _ in 0..100_000u32` 形が
/// 並行実行時に**約 6%**（単独実行 30 回中 2 回）で待機に失敗し、`boot_calls` が空のまま照合へ落ちていた。
///
/// # 適用範囲
///
/// 分類の軸は「Tick を注入するか」ではなく **「別スレッドの進行を待っているか」** である。
///
/// - **対象（[`spin_wait_until`] を使う）**: 待機対象が別スレッドの進行であり、各反復が**何も進めない**
///   純粋なポーリング（`non_status_calls()` / `drain_received()` を読むだけのループ）。
/// - **対象（本猶予の期限だけを借りる。ただし期限は十分条件ではない）**: 各反復で
///   `inject_dispatcher_tick` により**系を進めつつ**、同じ反復で `drain_received()` 等により
///   **別スレッドの結果も待つ**ハイブリッドのループ（[`spin_wait_until`] は純粋ポーリング専用ゆえ
///   流用しない）。ここでは打ち切りを時刻期限にするだけでは足りず、**注入する simulated time が
///   待っている観測を追い越さない**ことを構造で保証しなければならない。追い越しうる時刻には必ず
///   上限（頭打ち）を置くこと。実測: S2 Phase 1 は毎反復 `now += 5` が 210 反復（実時間 ~0.6 秒）で
///   Clear cue の時刻を跨ぎ、リビール観測が間に合わないと**待っている条件そのものが破壊**されて
///   永久に不成立になる——並行実行時に約 2% 失敗し、期限を 30 秒に延ばしても 50 回中 3 回失敗した。
///   期限は「壊れていない条件を待つ」ためのものであり、条件が壊れるレースは期限では直らない。
/// - **非対象**: 別スレッドの進行を待たず、注入 Tick 列そのものが仕事量であるループ。時刻で打ち切ると
///   注入列が短くなり意味が変わる。
///
/// 猶予は通常経路（マイクロ秒〜ミリ秒）に対して桁違いに大きく取る。期限切れは呼び手の assert が
/// 落として原因を名指しするので hang しない。
const SPIN_WAIT: Duration = Duration::from_secs(30);

/// `yield_now()` の密スピンを続ける上限反復数。これを超えたら [`BACKOFF_SLEEP`] へ落とす。
///
/// # なぜ純 yield のままではいけないか
/// `yield_now()` の密ループは **1 コアを占有し続ける**。反復上限だけで打ち切っていた旧実装は
/// 早々に諦めるためこれが顕在化しなかったが、時刻期限（[`SPIN_WAIT`]）へ変えると失敗経路が
/// 数十秒フルにコアを焼き、**同一バイナリで並走する他テストを飢餓させて別の flake を生む**
/// （実測: 純 yield ＋ 30 秒期限で 50 回中 5 回・無関係な 3 テストが巻き添えで失敗し、総所要が
/// 230 秒→1490 秒へ悪化した）。待機は「速い経路を邪魔しない」と同時に「長引いたら CPU を返す」
/// 必要がある。
///
/// # 予算を旧実装の上限に揃える理由
/// 予算を小さく取る（実測: 10_000）と、**正常でも数百 ms 待つ呼出点**が [`BACKOFF_SLEEP`] の
/// 1ms 粒度に律速され、通常経路が 1 回 4.4 秒 → 10.3 秒へ倍増した。旧実装の最大予算（1_000_000）
/// をそのまま踏襲すれば、**成功する待機は旧実装と完全に同じ密スピンで完了**し、予算を使い切った
/// ——旧実装なら諦めて assert を落としていた——場合にのみ sleep へ落ちる。すなわち本ヘルパは
/// 「旧挙動 ＋ 諦めずに時刻期限まで CPU を返しながら待つ」の純増であり、通常経路を一切遅くしない。
const SPIN_YIELD_BUDGET: u32 = 1_000_000;

/// 密スピンを使い切った後の 1 回あたり待機。CPU を明け渡し、相手スレッドに実行機会を与える。
const BACKOFF_SLEEP: Duration = Duration::from_millis(1);

/// `cond` が真になるまで [`SPIN_WAIT`] の範囲で待つ。真になったら `true`、期限切れなら `false`。
///
/// 速い経路（通常はマイクロ秒）は `yield_now()` の密スピンで待ち time-to-detect を犠牲にしない。
/// [`SPIN_YIELD_BUDGET`] を超えたら [`BACKOFF_SLEEP`] の短い sleep へ落として**コア占有をやめる**。
/// 本ファイルの「sleep 不使用」規律は *系を進める* Tick 注入ループの決定論を守るためのものであり、
/// 別スレッドの進行を待つだけの本ヘルパには当たらない（待機は観測内容を変えない）。
fn spin_wait_until(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + SPIN_WAIT;
    let mut spun = 0u32;
    loop {
        if cond() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        if spun < SPIN_YIELD_BUDGET {
            spun += 1;
            std::thread::yield_now();
        } else {
            std::thread::sleep(BACKOFF_SLEEP);
        }
    }
}

/// 「尽きるのが正常」の回収（settle）が満たすべき**壁時計の最小持続**（要件 4.2・4.5）。
///
/// 回収機会を反復回数で与えると、CPU 競合下（並列 `cargo test`・ウイルス対策の再スキャン）では
/// 数 ms で反復が尽き、**残余を出す欠陥があっても空のまま緑になる**（空虚な緑）。負検証は
/// 「出ないこと」を主張するので、機会が縮む方向の変動は誰にも赤にされない。よって最低限の
/// 観測時間は壁時計で与える。
///
/// 初期値 200ms（design C3）を実測のうえ据え置いた。置き換え対象の旧ループ（`yield_now` 5,000 回）が
/// 実際に占める時間を本機（22 論理 CPU）で測ると **無負荷 0.31ms**（7/7・振れ幅 0.306〜0.314）／
/// **4 スレッド占有 0.93・8.4・22.6ms**／**22 スレッド占有 3.7・5.5・390ms**——負荷次第で 3 桁動き、
/// 平常時は 1ms 未満しか待たない。200ms の床は平常値の 600 倍以上を常に与え、上振れ（390ms）とも
/// 同じ桁なので**どの負荷でも旧形より回収機会が縮まない**（要件 4.5）。この床の内側には
/// [`BACKOFF_SLEEP`] 粒度の反復が実測 約 130 回入る（1ms 指定の sleep は実測 1.5ms 粒度）。
/// 代償は 1 呼出点あたり 0.2 秒で、対象は 2 呼出点のみ。
const SETTLE_MIN: Duration = Duration::from_millis(200);

/// 回収が尽きたと見なすのに要する**連続して 0 件だった反復数**（要件 4.2）。
///
/// 壁時計だけでは「たまたま静かな 200ms」を通してしまうので、観測量の側からも条件を置く。
/// 1 反復ごとに [`BACKOFF_SLEEP`] を挟むので、50 回は「50ms 以上どのスレッドからも 1 件も
/// 来なかった」ことを意味する。初期値（design C3）を据え置いた。無負荷では [`SETTLE_MIN`] の
/// 内側に約 130 反復入るので本条件は先に満たされるが、sleep 粒度が伸びる負荷下（Windows の
/// タイマ分解能が落ちると 1 反復 15ms 級＝200ms で 13 反復）では**こちらが効いて反復を伸ばす**。
/// 2 条件は入れ替わりで律速する関係にあり、片方だけでは負荷下の回収機会を保証できない。
const SETTLE_QUIET_ROUNDS: u32 = 50;

/// 「尽きるのが正常」の回収ループ。`step` は 1 反復分の回収（drain）を行い**回収件数**を返す。
///
/// 終了条件は **[`SETTLE_MIN`] を満たし かつ 連続 [`SETTLE_QUIET_ROUNDS`] 回 0 件**の両立で、
/// どちらか一方では返らない。両立しないまま [`SPIN_WAIT`] を超えたら必ず返る（hang しない）。
/// 反復の間は [`BACKOFF_SLEEP`] の短い sleep だけを挟む（有界 poll-backoff＝要件 4.4。
/// **時刻を進めるための sleep ではない**——時刻前進は呼出側が前段で決定論的に注入し終えており、
/// 本ヘルパは観測しかしない）。
///
/// 本ヘルパは **panic しない**。合否の宣告は呼出側の既存 assert のまま（要件 4.5・4.6）。
fn settle_bounded(step: impl FnMut() -> usize) {
    settle_bounded_with(Instant::now, step);
}

/// 時計を注入できる [`settle_bounded`] の内側（檻専用の継ぎ目・`spine_settle_tests.rs`）。
///
/// 「最小持続に達するまで返らない」を実時間の計測で確かめると並列負荷で合否が動く——本仕様が
/// 直そうとしている当の病になる。時計を差し替えられるようにして、檻の期待値を**反復回数という
/// 整数**へ落とすためだけの継ぎ目であり、本番の呼出点は常に [`settle_bounded`]（実時計）を使う。
fn settle_bounded_with(mut now: impl FnMut() -> Instant, mut step: impl FnMut() -> usize) {
    let started = now();
    let mut quiet = 0u32;
    loop {
        if step() == 0 {
            quiet += 1;
        } else {
            quiet = 0;
        }
        let elapsed = now().saturating_duration_since(started);
        if elapsed >= SETTLE_MIN && quiet >= SETTLE_QUIET_ROUNDS {
            return;
        }
        if elapsed > SPIN_WAIT {
            return;
        }
        std::thread::sleep(BACKOFF_SLEEP);
    }
}

/// `GraphicsCore`＋`WucGraphicsResource` を実資源として載せた wintf World（headless GPU・R8.4）。
///
/// 本番 UI スレッドは MTA（記憶: areka WUC は MTA スレッドで動く）。WARP 可（`GraphicsCore::new()`）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// emo2 fixture ルート（`CARGO_MANIFEST_DIR`＝`crates/areka` 相対・assets.rs テストと同一規約）。
///
/// 呼ぶたびに **ghost スコープの永続状態（`<ghost>/master/profile/areka/`）を除去**する。
/// position-persist で永続書込が実際に効くようになったため、実機実走（8.7 サインオフ）や
/// 過去のテスト実行が共有 fixture へ起動記録（`[boot] count`）と窓位置を書き残す。
/// 残ると boot が「2 回目起動」と判定して **OnFirstBoot を発行しなくなり**、scripted boot 系列
/// （OnInitialize → username → OnFirstBoot → …）を期待する spine テストが落ちる。
/// fixture は git 追跡外（gitignore 済み）ゆえ削除は安全で、テストを実行順・実機実走から独立させる。
fn emo2_root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2");
    let persist_dir = root
        .join("ghost")
        .join("master")
        .join("profile")
        .join("areka");
    let _ = std::fs::remove_dir_all(&persist_dir);
    root
}

/// emo2 fixture のバルーンルート（assets.rs テストと同一規約）。
fn emo2_balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

/// scope0/scope1 の 2 スコープぶんの合成配置（placement::spawn テストの emo2 相当値を踏襲）。
///
/// attach フェーズは窓 `Entity` のみを消費し `WindowPos`/寸法は読まないため、位置値は attach の
/// 成否に無関係（`GhostWindows::scopes()` が `[0,1]` を返すことだけが load-bearing）。両 scope の
/// `balloon_size` が同値なのは合成値をそのまま踏襲しているだけで、実 fixture の scope 別バルーン
/// 採寸（scope0 と scope1 は解決する系列が異なり `validrect` も異なる）を表すものではない。
fn two_scope_placements() -> Vec<ScopePlacement> {
    vec![
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 733 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 708 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            balloon_offset_base: OffsetBase::unpinned(PointPx { x: -412, y: -25 }),
            // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
            balloon_limit: true,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1063 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1044 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            balloon_offset_base: OffsetBase::unpinned(PointPx { x: 285, y: -19 }),
            // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
            balloon_limit: true,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
        },
    ]
}

/// 窓タイトル（emo2 相当）。
fn titles() -> GhostTitles {
    GhostTitles::from_scope_titles([(0, "むらさき".to_string()), (1, "エモ".to_string())])
}

/// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
/// （draw_readback_test/attach_wiring の決定論パターン——WM_QUIT は posted が尽きた後に配送）。
///
/// 実 `ClockedTextSink<EmoTextSink>` の UI ドレイン（`spawn_emo_text` の pump アクター）を
/// headless に駆動できることを裏付ける定石。sink 生存中は複数セッション実行できる。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// クロージャ `f` を別スレッドで実行し有界時間で完了を観測する（ghost spine の `run_bounded` 同旨）。
fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    let (done_tx, done_rx) = mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

/// `ActorHandle::join` を有界時間で観測する（ghost spine の `join_bounded` 同旨）。
fn join_bounded(what: &str, timeout: Duration, handle: ActorHandle) -> Result<(), ActorError> {
    let (res_tx, res_rx) = mpsc::sync_channel::<Result<(), ActorError>>(0);
    std::thread::spawn(move || {
        let _ = res_tx.send(handle.join());
    });
    match res_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
    }
}

// ===========================================================================
// ログ捕捉（硬化機構の唯一の定義元 `log-capture-kit` への委譲）
//
// `run_attach_phase` の `info!(planned, attached, ...)`（DD-12 の縮退がバグを隠さない檻の
// 観測点）をこのスレッド上で捕捉して件数一致を assert する。行の形（1 イベント 1 行・
// `level=… target=…` に続けてフィールドを訪問順で ` name=value`）は移行前と 1 バイト変わらない。
//
// 「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」は**誤り**である。差し替わる
// のはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
// interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点を最初に踏んだスレッドの判定が
// 焼き付く（先着が勝つ）。捕捉窓を持たないスレッドの既定は `NoSubscriber` で判定は「不要」ゆえ、
// 先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。
// 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 捕捉窓の内側での interest 再計算 ⑶ 番兵イベントに
// よる空振り検出 の 3 点でこれを塞ぐ（機序の逐条解説は `log_capture_kit` の crate doc と
// 同 crate の `src/probe.rs`）。
// ===========================================================================

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち指定 level（例 `"ERROR"`）の件数を数える。
fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|l| l.contains(&needle)).count()
}

// ===========================================================================
// SpineHarness（再利用可能な結線ハーネス・6.2/6.3 がこの上に構築する）
// ===========================================================================

/// spine ハーネスの SERIKO ループ駆動モード（task 9.4・design「結線・資産・実機経路（spine.rs）」）。
///
/// spine は loop ticker を**起動しない**。ループ駆動は `SerikoLoopConfig`（表＋乱数）を seriko へ値渡しで
/// 仕込み、tick は `SerikoSink::send_tick` を直接注入して制御する（決定論・sleep 不使用・R7.2/7.3）。
enum LoopDriver {
    /// ループ完全不活性（`SerikoLoopConfig::disabled()` 相当＝空表・ダミー乱数）。既存 spine 全テストの
    /// 非退行経路（設計 Testing Strategy E2E-3・Implementation Notes）。send_tick を注入してもループは
    /// 表示中 slot の評価対象アニメが常にゼロ＝何も発行しない（従来観測どおり）。
    Inert,
    /// 実 emo2 表（`BootAssets.loop_tables`）＋固定注入乱数列で駆動（まばたき e2e・R7.2/7.3）。
    Live(LoopRng),
}

/// scripted ghost boot＋実 sink 結線＋GPU World＋frame 結線状態を束ねた spine ハーネス。
///
/// `wire_emo2_boot`（task 5.1・production）の結線を、headless GPU World＋合成 `GhostWindows`＋
/// scripted backend＋`TickerMode::Disabled` で**テスト内 in-process** に再現したもの。frame
/// フェーズ（`run_attach_phase` 等）を直接駆動でき、`ghost`/`seriko`/`shiori_handle` で
/// ライフサイクル・発火列を観測できる。
struct SpineHarness {
    /// GPU 資源（`GraphicsCore`/`WucGraphicsResource`）＋合成 `GhostWindows`＋装着スロットを持つ World。
    world: World,
    /// frame 三相結線状態（presenter/rx/runtime/clock/assets）。`run_attach_phase` の駆動対象。
    wiring: Emo2Wiring,
    /// 文字層ランタイムの観測用クローン（6.2/6.3 が `present_frame`/`read_back` に使う）。
    #[allow(dead_code)]
    runtime: Rc<RefCell<TextLayerRuntime>>,
    /// ghost ランタイム（`dispatcher()` への Tick 注入・`shutdown()` の駆動）。
    ghost: GhostRuntime,
    /// seriko worker の join ハンドル（有界 join で終了確認）。
    seriko: ActorHandle,
    /// scripted backend の発火列観測ハンドル（boot 系列・close 系列の照合）。
    shiori_handle: ScriptedShioriHandle,
    /// 文字層 UI アクター（`spawn_emo_text` の pump アクター）の生存ハンドル（drain は `pump_text`）。
    #[allow(dead_code)]
    text_pump: JoinHandle<()>,
    /// SERIKO ループ tick の直接注入端（task 9.4）。`spawn_seriko` が返す `SerikoSink` の clone で、
    /// `inject_seriko_tick`（`send_tick` 直接注入・loop ticker 不起動・R7.2/7.3）に使う。surface_sink 本体は
    /// boot が sinks 第 1 要素として値消費するため、この clone をハーネスに保持する。全 clone は単一 seriko
    /// inbox への送信端で配送意味は同一。shutdown 時に drop して inbox 切断→worker 自然終了させる。
    tick_sink: SerikoSink,
}

impl SpineHarness {
    /// 標準台本（boot 系列＋`\s[0]\e` OnBoot＋ForceQuit close 系列）で spine ハーネスを起動する。
    ///
    /// `on_boot` は OnBoot GET が返す応答スクリプト。6.1 は最小 talk（`\s[0]\e`）を渡す。
    /// SERIKO ループは **不活性**（`LoopDriver::Inert`）で駆動する——既存 spine 全テストの非退行経路
    /// （設計 Testing Strategy E2E-3）。ループを実表で活性化するまばたき e2e は [`Self::boot_live`]。
    fn boot(on_boot: &str) -> SpineHarness {
        let (backend, shiori_handle) = Self::standard_backend(on_boot);
        Self::boot_with(backend, shiori_handle, LoopDriver::Inert)
    }

    /// 実 emo2 表＋固定注入乱数列で SERIKO ループを**活性化**して spine ハーネスを起動する（task 9.4）。
    ///
    /// `rng` は 1/N 抽選の固定注入列（決定論・実 entropy 非依存・R7.1/7.2）。tick は loop ticker を起動せず
    /// [`Self::inject_seriko_tick`]（`send_tick` 直接注入）で制御する（sleep 不使用・R7.3）。まばたき e2e が
    /// 使う（実 kero/sakura まばたき 1 周の full golden は task 10.2）。
    fn boot_live(on_boot: &str, rng: LoopRng) -> SpineHarness {
        let (backend, shiori_handle) = Self::standard_backend(on_boot);
        Self::boot_with(backend, shiori_handle, LoopDriver::Live(rng))
    }

    /// 標準 scripted backend（boot 系列＋OnBoot talk＋ForceQuit close 系列）を組む。
    ///
    /// boot 系列（OnInitialize→[username prefetch]→OnFirstBoot→OnBoot→basewareversion）＋ shutdown
    /// （`GhostRuntime::shutdown` は常に ForceQuit 経路＝OnClose NOTIFY→Unload・ghost spine S1 と同旨）を
    /// 台本化する。OnSecondChange は kanade へ Tick を送らないため不要。
    /// username prefetch GET（OnInitialize 後・OnFirstBoot 前・sylphya task 8.2・R9.1/9.2）は `build()` が
    /// 既定 `Ok(None)`（NoContent＝カスタム username 無し）を自動補填するため明示台本化しない。
    fn standard_backend(on_boot: &str) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(on_boot.to_string())))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
            .build()
    }

    /// 任意の scripted backend＋ループ駆動モードで spine ハーネスを起動する（6.2/6.3 が独自台本を注入する口）。
    fn boot_with(
        backend: ScriptedShioriBackend,
        shiori_handle: ScriptedShioriHandle,
        driver: LoopDriver,
    ) -> SpineHarness {
        // ── headless GPU World（MTA COM＋WARP 可・R8.4）＋合成 GhostWindows（scope [0,1]） ──
        let mut world = make_world_with_gpu();
        spawn_ghost_windows(&mut world, &two_scope_placements(), &titles());

        // ── 構築入力（実 emo2 fixture・COM は make_world_with_gpu で初期化済み） ──
        // 作者基準 DPI は emo2 fixture の実測既定（shell/balloon とも無宣言＝96・task 2.1）。
        let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
            .expect("emo2 fixture の BootAssets 組立は成功する");

        // ── presenter／文字層ランタイム／実 EmoTextSink（テストスレッド＝UI pump スレッド） ──
        let presenter = EmoPresenter::new();
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        let (emo_text_sink, text_pump) =
            spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump (test) thread");

        // ── talk 時刻源（本番同様 dola clock 注入）＋実 ClockedTextSink<EmoTextSink> ──
        let clock_fn: Arc<dyn Fn() -> f64 + Send + Sync> = Arc::new(dola::runtime::clock::now);
        let clock = TalkClock::new(clock_fn);
        let clocked_text_sink = ClockedTextSink::new(emo_text_sink, clock.clone());

        // ── mpsc＋PresentBridge→spawn_seriko（実 SurfaceOutput 経路）。SurfaceResolver は
        //    非 Clone ゆえ spawn_seriko が値消費し、Emo2Wiring 側は空 alias のプレースホルダで埋める
        //    （attach は resolver を読まない・Task 4.1 申し送り・wire_emo2_boot 手順4 と同型）。 ──
        let (tx, rx) = mpsc::channel::<PresentCommand>();
        let bridge = PresentBridge::new(tx);
        let BootAssets {
            shells,
            balloons,
            resolver,
            static_binds,
            bind_resolver,
            loop_tables,
            shell_author_dpi,
            balloon_author_dpi,
        } = assets;
        // SERIKO ループ構成（task 9.4・design「結線・資産・実機経路（spine.rs）」）: 実 emo2 表
        // （`BootAssets.loop_tables`＝task 9.1 が `EmoWorld` スナップショットから `from_world` で構築）＋
        // 固定注入乱数列（`driver`）で `loop_config` を組む。既存 spine テストは Inert（`disabled()` 相当＝
        // 空表・ダミー乱数）でループ完全不活性＝従来観測どおり非退行（設計 Testing Strategy E2E-3・
        // Implementation Notes）。まばたき e2e（`boot_live`）のみ実表＋固定 rng で駆動する（本番 mod.rs は
        // 実 entropy・spine は固定注入列で決定論・R7.1/7.2/7.3）。
        let LoopTables {
            shell: shell_table,
            balloon: balloon_scope_tables,
        } = loop_tables;
        // バルーン表の転送（要件 5.6）: production `wire_emo2_boot` と同型で、`build_boot_assets` が
        // scope ごとに導出済みの写像を、scope キーだけアクタ鍵語彙へ写して渡す（値移送）。
        let balloon_tables = actor_keyed_balloon_tables(balloon_scope_tables);
        let loop_config = match driver {
            LoopDriver::Inert => SerikoLoopConfig::disabled(),
            LoopDriver::Live(rng) => SerikoLoopConfig {
                shell_table,
                balloon_tables,
                rng,
            },
        };
        // 実名前解決表（BootAssets.bind_resolver・task 7.1 が emo2 fixture の MountModel から構築）＋
        // loop_config を seriko の起動へ値渡しで配線する（production wire_emo2_boot と同型・task 7.2/9.4）。
        let (surface_sink, seriko) = spawn_seriko(
            resolver,
            static_binds.clone(),
            bind_resolver,
            loop_config,
            bridge,
        );
        // loop tick 直接注入端（task 9.4）: SerikoSink を 1 本 clone してハーネスへ保持する。surface_sink 本体は
        // 下の boot_options が sinks 第 1 要素として値消費するため、この clone を `inject_seriko_tick`
        // （send_tick 直接注入・loop ticker 不起動・R7.2/7.3）に使う。全 clone は単一 seriko inbox への送信端で
        // 配送意味は同一。shutdown 時に drop して inbox 切断→worker 自然終了させる（下記 shutdown_bounded）。
        let tick_sink = surface_sink.clone();
        let wiring_assets = BootAssets {
            shells,
            balloons,
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds,
            // 実 bind_resolver は seriko が値消費済み（attach は bind_resolver を読まない）ため空表プレースホルダ。
            bind_resolver: BindResolver::empty(),
            // 実 loop_tables は loop_config へ移送済み（attach は loop_tables を読まない）ため空表プレースホルダ。
            loop_tables: LoopTables {
                shell: AnimationTable::empty(),
                balloon: BTreeMap::new(),
            },
            // 作者基準 DPI は搬送のみ（本相は値を解釈しない）。
            shell_author_dpi,
            balloon_author_dpi,
        };

        // ── move channel＋実 MoveCueSink（wire_emo2_boot 手順4 と同型・S-3 形＝task 9.3） ──
        // talk スレッドの MoveCueSink が送出端、UI スレッドの Emo2Wiring が受信端 move_rx（frame 相
        // drain＝run_move_drain_phase・task 9.2）を保持する。9.1 の throwaway 送出端を実 MoveCueSink へ
        // 差し替え、production `wire_emo2_boot` の sink 構成（下記のとおり現在は 4 本）を spine でも
        // 忠実に再現する。
        let (move_tx, move_rx) = mpsc::channel::<MoveDirective>();
        let move_sink = MoveCueSink::new(move_tx);

        // ── lifecycle channel＋実 BalloonLifecycleSink（wire_emo2_boot 手順4 と同型・task 4.1） ──
        // talk スレッドの BalloonLifecycleSink が送出端、UI スレッドの Emo2Wiring が受信端
        // lifecycle_rx（バルーン可視性相＝task 4.4 が drain）を保持する。production と同じ 4-sink 構成を
        // spine でも忠実に再現するため、throwaway ではなく実 sink を登録する。
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
        let lifecycle_sink = BalloonLifecycleSink::new(lifecycle_tx);

        // ── scripted boot（実 sink 注入・TickerMode::Disabled＝Tick 注入で駆動・R8.3） ──
        // sinks は broadcast 登録先で surface（seriko）／text（ClockedTextSink）／move（MoveCueSink）／
        // lifecycle（BalloonLifecycleSink）の 4 sink を第 1〜4 要素として渡す（production mod.rs と同順・S-3 形）。
        let options = GhostBootOptions {
            ghost_root: emo2_root(),
            default_encoding: DefaultEncoding::Ansi,
            shiori: ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![
                Box::new(surface_sink),
                Box::new(clocked_text_sink),
                Box::new(move_sink),
                Box::new(lifecycle_sink),
            ],
            // scripted spine harness: 本番 provider 経路（FromSylphya）を忠実に再現する。boot が
            // 内部で sylphya を起動し selfname 系／username を publish・provider を鏡像由来に据える。
            // App スコープ root は不要（None＝App 層不在縮退・ghost/shell スコープは emo2 mount 由来）。
            system_vars: SystemVarWiring::FromSylphya,
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };
        let ghost = boot(options).expect("scripted boot は解決可能な emo2 ghost_root で成功する");

        // ── frame 三相結線状態（wire_emo2_boot 手順6 相当・System 登録はせず直接駆動する） ──
        // Emo2Wiring は move の受信端 move_rx を保持し frame 相 drain（run_move_drain_phase・task 9.2）に
        // 備える。move の spine e2e（task 9.3）は上の実 MoveCueSink 経由で cue→channel→drain を通す。
        // 表示ライフサイクルの受信端 lifecycle_rx も同様に保持し、可視性相（task 4.4）の drain に備える。
        let wiring = Emo2Wiring::new(
            presenter,
            rx,
            move_rx,
            lifecycle_rx,
            mpsc::channel::<crate::emo2_boot::zorder_cue::ZOrderDirective>().1,
            Rc::clone(&runtime),
            clock,
            wiring_assets,
        );

        SpineHarness {
            world,
            wiring,
            runtime,
            ghost,
            seriko,
            shiori_handle,
            text_pump,
            tick_sink,
        }
    }

    /// 文字層 UI アクターの pending メッセージを headless に drain する（実 ClockedTextSink 経路）。
    fn pump_text(&self) {
        pump_until_idle();
    }

    /// seriko ループへ tick を 1 発直接注入する（loop ticker 不起動・`SerikoSink::send_tick`・R7.2/7.3）。
    ///
    /// 本番（mod.rs）は `spawn_loop_ticker` の worker スレッドが実時刻で `send_tick` するが、spine は
    /// 決定論のため ticker を起動せず、テストスレッドから注入時刻のみで `send_tick` を直接呼ぶ（sleep 不使用）。
    /// dispatcher 経路（`inject_dispatcher_tick`）は talk/cue clock を進めるのみで seriko ループには届かない
    /// （ghost 側にループ結線なし）——ループ tick はこの直接注入だけが供給する。
    fn inject_seriko_tick(&self, now_ms: u64) {
        self.tick_sink.send_tick(now_ms);
    }

    /// dispatcher へ Tick を 1 発注入する（sleep 不使用・注入時刻のみで進める・R8.3）。
    fn inject_dispatcher_tick(&self, now: u64) {
        self.ghost
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher actor should still be alive to accept an injected Tick");
    }

    /// 正規終了＋全ハンドル有界 join でハーネスを畳む（hang させない）。
    ///
    /// `GhostRuntime::shutdown(User)` は ForceQuit 経路（OnClose NOTIFY→Unload）で ghost 一式を
    /// join し、dispatcher が保持する `SerikoSink` クローンを drop する→seriko worker の inbox 切断→
    /// 自然終了。続けて seriko を有界 join する（ghost spine S1/S2 の後片付け技法）。
    fn shutdown_bounded(self) {
        let SpineHarness {
            world,
            wiring,
            runtime,
            ghost,
            seriko,
            shiori_handle,
            text_pump,
            tick_sink,
        } = self;

        run_bounded("spine ghost shutdown", Duration::from_secs(10), move || {
            // 正規 close（DD-10 と同じ User）。ForceQuit ゆえ OnClose は NOTIFY で消化される。
            let _ = ghost.shutdown(CloseReason::User);
        });
        // loop tick 直接注入端の clone を明示 drop（task 9.4）: ghost.shutdown が dispatcher 保持の
        // SerikoSink クローンを drop しても、ハーネス保持の tick_sink clone が生きていると seriko inbox が
        // 切断されず worker が終了しない。全 Sender drop で自然終了させるため seriko join の前に drop する。
        drop(tick_sink);
        join_bounded("spine seriko join", Duration::from_secs(10), seriko).expect(
            "seriko worker should terminate once all SerikoSink clones drop after shutdown",
        );

        // 残り（!Send・テストスレッド常駐）を明示 drop（UI アクター/presenter/Rc runtime/World）。
        drop(wiring);
        drop(world);
        drop(runtime);
        drop(text_pump);
        let _ = shiori_handle;
    }
}

// テーマ単位のテストモジュール接続宣言（areka-P0-file-slimming タスク 8.1・要件 1.7 / 3.1 / 3.2）。
// 本ファイルはハーネス本体（scripted backend・fixture・有界待機・ログ捕捉・SpineHarness）を保持し、
// 観測ケースはテーマごとの兄弟ファイル `spine_<テーマ>_tests.rs` に置く。
#[cfg(test)]
#[path = "spine_boot_smoke_tests.rs"]
mod boot_smoke_tests;
#[cfg(test)]
#[path = "spine_conformance_script.rs"]
mod conformance_script;
#[cfg(test)]
#[path = "spine_conformance_support.rs"]
mod conformance_support;
#[cfg(test)]
#[path = "spine_display_tests.rs"]
mod display_tests;
#[cfg(test)]
#[path = "spine_move_cue_tests.rs"]
mod move_cue_tests;
#[cfg(test)]
#[path = "spine_seriko_loop_tests.rs"]
mod seriko_loop_tests;
#[cfg(test)]
#[path = "spine_settle_tests.rs"]
mod settle_tests;
#[cfg(test)]
#[path = "spine_talk_close_tests.rs"]
mod talk_close_tests;
#[cfg(test)]
#[path = "spine_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "spine_text_scale_tests.rs"]
mod text_scale_tests;
