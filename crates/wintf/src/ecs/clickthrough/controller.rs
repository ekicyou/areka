//! クリック透過機構の UI スレッド側判定・適用ロジック。
//!
//! 本ファイルは 3 層で構成する:
//! 1. [`resolve_transition`]（純関数・World 非依存）: 差分ガード＋ドラッグ抑止を
//!    適用して「今回適用すべき変化」を返す。副作用・World アクセスを持たない。
//! 2. [`evaluate_targets`]（同期評価コア・テスト可能）: settled な `&World` に対し
//!    レジストリの各対象窓を 1 回巡回し、ヒットテスト→`resolve_transition`→
//!    差分時のみ `apply_click_through`→成功時のみ `last_applied` 書き戻し、を行う。
//!    `tick_bridge::tick_one_frame` と同じ「同期コア切り出し」規律でテスト隔離する。
//! 3. [`ClickThroughController::start`] / [`ClickThroughHandle`]（async ループ＋RAII）:
//!    `spawn_local` で UI スレッドに投入する二重起床（カーソル移動 notify／VSync tick）
//!    の listen-before-work ループと、その生存期間を束ねる RAII ハンドル。

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use event_listener::Event;
use tracing::{debug, trace, warn};

use crate::ecs::PointF;
use crate::ecs::drag::{DragStateSnapshot, snapshot_drag_state};
use crate::ecs::hit_test_in_window;
use crate::ecs::world::EcsWorld;
use crate::ecs::{PhysicalPoint, WindowPos};
use crate::win_style::{apply_click_through, apply_layered_companion};
use windows::Win32::Foundation::{HWND, POINT};

use super::{ClickThroughRegistry, CursorMonitorBridge, DesiredState};

/// 差分ガード＋ドラッグ抑止を適用して「今回適用すべき変化」を返す純関数。
///
/// 適用不要（差分なし or ドラッグ抑止）の場合は `None` を返す。
///
/// # 判定順序（ゲーティング）
/// 1. **ドラッグ最優先ゲート（R5.1/R5.3）**: ドラッグ移動中は透過 ON へ遷移させない。
///    移動中は望ましい状態を強制的に `Opaque` とし、`last_applied != Opaque` の
///    時のみ `Some(Opaque)`（透過を外して掴み維持）、同一なら `None`。
///    - 抑止対象は「ドラッグ移動中」の状態。design（§System Flows「ゲーティング順序」）は
///      抑止スコープを *ドラッグ中* と規定する。本実装では `Dragging`（閾値到達・移動中）に
///      加え、その直前 1 フレームの `JustStarted`（移動が始まった直後・掴み確定）も抑止対象と
///      する。両者はボタン押下＋ドラッグ開始済みの「移動中」フェーズであり、ここで透過 ON に
///      なると掴みが崩れる（R5.1 のアンチフリッカ意図）。一方 `Preparing`（押下のみ・閾値未到達）は
///      まだドラッグ開始前なので非ドラッグ写像に委ねる。
/// 2. **`JustEnded` 再収束（R5.2）**: ドラッグ終了直後は抑止を解除し、現在の `hit` に
///    基づく非ドラッグ写像へ委ねる（終了サイクルで正しい状態へ再収束する）。
/// 3. **非ドラッグ写像（R3.3/R2.1/R2.2）**: `Some(entity)` → `Opaque`（不透過・自窓で受領）、
///    `None` → `Transparent`（透過・背面プロセスへ通過）。
/// 4. **差分ガード（R3.2）**: 望ましい状態が `last_applied` と同一なら `None`（再適用しない）。
///    異なる場合のみ `Some(desired)`（ちょうど一度だけ適用・R3.3）。
///
/// # 純粋性
/// `&World`・I/O・グローバル状態・副作用を持たない決定的関数。`last_applied` の
/// 真実源（[`super::ClickThroughRegistry`]）への書き戻しは呼び出し側（タスク 3.1）の
/// 責務であり、本関数は計算のみを担う。
pub(crate) fn resolve_transition(
    hit: Option<Entity>,
    drag: &DragStateSnapshot,
    last_applied: DesiredState,
) -> Option<DesiredState> {
    // 望ましい状態を判定する。
    let desired = match drag {
        // ドラッグ移動中: 透過 ON へは絶対に遷移させない。強制 Opaque。
        DragStateSnapshot::Dragging { .. } | DragStateSnapshot::JustStarted { .. } => {
            DesiredState::Opaque
        }
        // それ以外（Idle / Preparing / JustEnded）は非ドラッグ写像に委ねる。
        // JustEnded は抑止解除サイクルとして現在の hit に従い再収束する（R5.2）。
        DragStateSnapshot::Idle
        | DragStateSnapshot::Preparing { .. }
        | DragStateSnapshot::JustEnded { .. } => match hit {
            Some(_) => DesiredState::Opaque,
            None => DesiredState::Transparent,
        },
    };

    // 差分ガード（R3.2）: 変化がある時だけ適用対象を返す。
    if desired == last_applied {
        None
    } else {
        Some(desired)
    }
}

/// World 上に生存しない対象窓をレジストリから刈り取る（窓破棄追随・R7.2 / Lifecycle）。
///
/// 除去条件: window Entity が既に despawn 済み（`world.get_entity(window).is_err()`）。
/// これが「窓破棄」の正準シグナルである。areka の窓 close は `world.despawn(entity)` で
/// 行われ（`on_window_handle_remove`→`WM_CLOSE`→`DestroyWindow` の起点）、Entity と共に
/// 対象が消える。したがって Entity 生存確認だけで破棄追随が成立する。
///
/// これを [`evaluate_targets`] の巡回前に呼ぶことで、破棄済み窓の無効 HWND へ
/// `apply_click_through` を撃つ経路（`Err` スパム）を構造的に断つ。除去件数を返す
/// （テスト・観測用）。areka の窓登録／破棄（task 4.1）は非同期ゆえ、機構側は「今 World に
/// 在る Entity の対象だけを見る」ことで窓ライフサイクルに追随する（既存窓ライフサイクル
/// ファイルは不変）。
///
/// NOTE: `remove::<Window>` のみ（Entity は残す）で HWND が破棄される稀な過渡状態は、
/// `evaluate_targets` の `apply_click_through` が `Err` をグレースフルに warn+skip する
/// 既存経路（`last_applied` 据え置き・次サイクル再試行）が受け止める。prune は「Entity 消滅」
/// という単一・明快な破棄シグナルに限定し、`Window` コンポーネント有無へ結合しない
/// （eval コアの純粋性を保ち、`Window` を持たない汎用対象も破棄までは監視できる）。
pub(crate) fn prune_dead_targets(world: &World, registry: &mut ClickThroughRegistry) -> usize {
    // Entity が World 上に生存している対象のみ残す（despawn 済みは刈り取る）。
    registry.retain(|t| world.get_entity(t.window).is_ok())
}

/// settled な `&World` に対しレジストリの全対象窓を 1 回巡回評価する同期コア
/// （post-tick 評価の本体・独立テスト可能・要件 2.1/2.2/2.4/3.2/3.3/5.x/8.x）。
///
/// `tick_bridge::tick_one_frame` と同じく「async ループから同期コアを切り出し」て
/// テスト隔離する。async ループ（[`run_click_through`]）は起床ごとに本関数へ
/// 「その時点のワーカ最新カーソル座標」を渡して 1 パスを回す。
///
/// # 引数
/// - `world`: ECS tick 完了後（`GlobalArrangement`／`AlphaMask`／`DragState` 確定後）の
///   settled World 参照。単一 UI スレッドで tick と排他されるため中間状態を読まない。
/// - `registry`: 監視対象窓と `last_applied`。差分ガードの真実源。
/// - `screen_to_client`: 対象 HWND の screen physical カーソルを client physical へ変換する
///   関数（production は `screen_to_client_point`＝OS `ScreenToClient` に委譲）。`None` は
///   当該窓 skip（無効 HWND 等）。テストは決定的な模擬変換を注入して状態機械のみ検証する。
///
/// # 各対象窓の処理
/// 0. `layered_applied` が偽なら `apply_layered_companion` で `WS_EX_LAYERED` 同伴フラグを
///    1 回立てる（pilot REPORT 必須条件: DComp 窓は `WS_EX_TRANSPARENT` 単独ではマウス
///    透過が効かない）。成功時のみ真へ書き戻し、失敗時は当該窓を skip（次サイクル再試行）。
/// 1. `WindowPos.position` の有無で未マップ窓を判定し、無ければスキップ（mapped-guard）。
/// 2. `screen_to_client(hwnd)` で client physical を得て `hit_test_in_window` を呼ぶ。座標変換は
///    OS(`ScreenToClient`)へ委譲する（NCHITTEST キャッシュ・OS 入力経路と同一）。`cursor -
///    WindowPos.position` の手引き算は、高 DPI・マルチモニタで position が窓の真の物理
///    クライアント原点とズレると判定と表示がずれるため採らない（4.2 実動検証で発覚）。
/// 3. `snapshot_drag_state()`（一度読み・下記）と `last_applied` を渡して
///    [`resolve_transition`] で desired を決める。`None`（差分なし or ドラッグ抑止）は skip。
/// 4. 変化時のみ `apply_click_through` を **1 回** 適用し、**`Ok` の時だけ** レジストリの
///    `last_applied` を書き戻す（単一所有・design line 367）。`Err` は `warn!` でログし
///    `last_applied` を据え置き（次サイクルで再試行）。
///
/// # ドラッグスナップショットの単一読み
/// ドラッグ状態は thread_local（UI スレッド・読み取り専用）。1 パス内で全窓に対し
/// 同一スナップショットを用いるため、ループ前に **一度だけ** 読む。
///
/// 表示層（GPU 合成 visual/content）には一切触れない（ex-style トグルのみ）。
pub(crate) fn evaluate_targets(
    world: &World,
    registry: &mut ClickThroughRegistry,
    mut screen_to_client: impl FnMut(HWND) -> Option<PointF>,
) {
    // 窓破棄追随（R7.2 / Error Handling: Lifecycle）: 巡回前に、World 上に生存しない対象
    // （despawn 済み Entity・`Window` コンポーネント喪失）をレジストリから刈り取る。これにより
    // 破棄済み窓の無効 HWND へ `apply_click_through` を撃たない（Err スパム回避）。
    prune_dead_targets(world, registry);

    // ドラッグスナップショットは 1 パスで一度だけ読む（全窓で同一・UI スレッド）。
    let drag = snapshot_drag_state();

    // 巡回中に `registry` を可変借用して書き戻すため、対象の列挙を先にスナップショット
    // する（`iter` の不変借用と `set_last_applied` の可変借用の重複を避ける）。
    // 対象は areka で 2 窓のみ・汎用でも小数のため Vec 収集のコストは無視できる。
    let targets: Vec<(Entity, windows::Win32::Foundation::HWND, DesiredState, bool)> = registry
        .iter()
        .map(|t| (t.window, t.hwnd, t.last_applied, t.layered_applied))
        .collect();

    for (window, hwnd, last_applied, layered_applied) in targets {
        // `WS_EX_LAYERED` 同伴フラグを初回評価で 1 回立てる（pilot REPORT 必須条件:
        // DComp 窓は TRANSPARENT 単独ではマウス透過が効かない）。適用成功時のみ真へ倒し、
        // 失敗はこの窓を当該サイクル skip（`last_applied` と同じ据え置き・次サイクル再試行）。
        if !layered_applied {
            match apply_layered_companion(hwnd) {
                Ok(()) => {
                    debug!(?window, "clickthrough: WS_EX_LAYERED 同伴フラグ適用");
                    registry.mark_layered_applied(window);
                }
                Err(e) => {
                    warn!(?window, error = %e, "clickthrough: apply_layered_companion 失敗 — skip");
                    continue;
                }
            }
        }

        // 未マップ窓（WindowPos.position 未確定）はスキップ（mapped-guard）。
        if world
            .get::<WindowPos>(window)
            .and_then(|wp| wp.position)
            .is_none()
        {
            trace!(?window, "clickthrough: WindowPos.position 未確定 — skip");
            continue;
        }

        // screen physical カーソル → 当該窓の client physical。変換は OS(ScreenToClient)へ
        // 委譲する（NCHITTEST キャッシュ・OS 入力経路と同一）。`cursor - WindowPos.position` の
        // 手引き算は高 DPI・マルチモニタで position が窓の真の物理クライアント原点とズレると
        // 判定領域が表示とズレるため使わない（4.2 実動検証で発覚）。失敗（無効 HWND 等）は skip。
        let Some(client) = screen_to_client(hwnd) else {
            trace!(?window, "clickthrough: screen→client 変換失敗 — skip");
            continue;
        };

        let hit = hit_test_in_window(world, window, client);

        // 差分ガード＋ドラッグ抑止（純関数）。None は skip（再適用不要）。
        let Some(desired) = resolve_transition(hit, &drag, last_applied) else {
            continue;
        };

        // 変化時のみトグル API を 1 回適用。成功時のみレジストリへ書き戻す単一経路。
        match apply_click_through(hwnd, desired == DesiredState::Transparent) {
            Ok(()) => {
                debug!(?window, ?desired, "clickthrough: ex-style トグル適用");
                // 単一所有（design line 367）: 適用成功後にのみ last_applied を更新。
                registry.set_last_applied(window, desired);
            }
            Err(e) => {
                // グレースフル: 失敗窓のみ skip、last_applied は据え置き（次サイクル再試行）。
                warn!(?window, error = %e, "clickthrough: apply_click_through 失敗 — skip");
            }
        }
    }
}

/// screen physical → client physical を OS(`ScreenToClient`)へ委譲する変換（production 用）。
///
/// NCHITTEST キャッシュ（`pointer::nchittest_cache`）・OS 入力経路（`window_proc::mouse_click`）と
/// 同一の変換で、frame/DPI/マルチモニタを OS に委ねる。`WindowPos.position` を手引き算する旧実装は、
/// 高 DPI・マルチモニタで position が窓の真の物理クライアント原点とズレると判定と表示がずれるため
/// 採らない（4.2 実動検証で発覚）。無効 HWND 等で失敗した場合は `None`。
fn screen_to_client_point(hwnd: HWND, screen: PhysicalPoint) -> Option<PointF> {
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    let mut pt = POINT {
        x: screen.x,
        y: screen.y,
    };
    // SAFETY: Win32 境界。ScreenToClient は hwnd と POINT への有効ポインタを要する。所有権不変。
    if unsafe { ScreenToClient(hwnd, &mut pt).as_bool() } {
        Some(PointF::new(pt.x as f32, pt.y as f32))
    } else {
        None
    }
}

/// UI スレッドでクリック透過の判定・適用ループを駆動するエントリ。
///
/// design の Service Interface（`start(world, registry) -> ClickThroughHandle`）を、
/// 二重起床（カーソル移動 notify＋VSync tick）と start 後の窓登録を実現するため
/// **最小限に精緻化**する（本コメントで根拠を明示・design §Service Interface 準拠）:
///
/// - **単一 wake event を引数で受ける**: async ループはただ 1 つの共有
///   `Arc<event_listener::Event>` を listen する。この wake event を (a) カーソルワーカ
///   （`CursorMonitorBridge::spawn(wake_event.clone())`）と (b) VSync tick 源（後続
///   タスク 3.2 が post-tick で notify）の **両方** が notify することで、futures/select
///   コンビネータ（tokio/futures 非依存・R4.2）無しに二重起床を実現する。tick 源への
///   結線は task 3.2 の責務のため、`start` は wake event を **引数** で受ける。
/// - **レジストリを共有可能化**（`Rc<RefCell<ClickThroughRegistry>>`）: areka は窓を
///   非同期生成するため、task 3.2/4.1 が `start` **後** に対象窓を登録／破棄時に除去
///   できるよう、返す [`ClickThroughHandle`] 経由で register/remove を公開する。
pub(crate) struct ClickThroughController;

impl ClickThroughController {
    /// UI スレッドで機構を起動する。ワーカ生成・event 共有・async ループの
    /// `spawn_local` を束ね、[`ClickThroughHandle`]（RAII）を返す。
    ///
    /// - `world`: UI スレッド所有 World への `Weak`（`run_async_tick` と同じ寿命規律・
    ///   shutdown で `upgrade()` が `None` を返しループが安全終了）。
    /// - `registry`: 共有レジストリ（start 後の登録／除去を許すため `Rc<RefCell<..>>`）。
    /// - `wake_event`: 二重起床の単一 wake event。ワーカが notify し、VSync tick 源
    ///   （task 3.2）も同一 event を post-tick で notify する。
    ///
    /// # Postconditions
    /// カーソルワーカ稼働・async ループ稼働。返した `ClickThroughHandle` の drop で
    /// 機構停止（ワーカ join・async ループは world drop で終了）。
    pub(crate) fn start(
        world: Weak<RefCell<EcsWorld>>,
        registry: Rc<RefCell<ClickThroughRegistry>>,
        wake_event: Arc<Event>,
    ) -> ClickThroughHandle {
        // ワーカは同一 wake event で起床通知する（カーソル移動 → notify）。
        // ワーカは handle（RAII で唯一の強所有・drop で stop/join）と async ループ（座標読み）
        // で共有する。**join を handle 単独が駆動する**ため、handle が唯一の強 `Rc` を持ち、
        // ループには `Weak` を渡す。こうすることで:
        //   - handle drop → 強 `Rc` が 0 → ワーカ即 stop/join（ループが park 中でも遅延しない）。
        //   - ループは毎起床 `upgrade()` し、`None`（handle drop 済み）なら shutdown として終了。
        // UI スレッド単独所有ゆえ `Rc`/`Weak`（非 `Arc`）で十分（内部 `latest_pos` は `Arc<AtomicI64>`）。
        let monitor = Rc::new(CursorMonitorBridge::spawn(Arc::clone(&wake_event)));

        // async 判定・適用ループを UI スレッドへ投入（実行はメッセージループに委ねる）。
        let join = crate::executor::spawn_local(run_click_through(
            Arc::clone(&wake_event),
            Weak::clone(&world),
            Rc::clone(&registry),
            Rc::downgrade(&monitor),
        ));

        debug!("ClickThroughController started (UI-thread eval loop + cursor worker)");

        ClickThroughHandle {
            monitor,
            registry,
            wake_event,
            _join: join,
        }
    }
}

/// クリック透過機構の生存期間を束ねる RAII ハンドル。
///
/// - `monitor`: カーソルワーカ。drop でワーカを stop/join（`CursorMonitorBridge::Drop`）。
/// - `registry`: 共有レジストリ。[`register`]/[`remove`] で start 後の窓登録／除去を行う。
/// - `wake_event`: 手動起床（テスト・強制再評価）にも使える wake event の保持。
/// - `_join`: async ループの JoinHandle。ループは `world` の `Weak` が upgrade 不能に
///   なった時点（shutdown）で自ら終了する（`run_async_tick` と同じ終了規律）。
///
/// [`register`]: ClickThroughHandle::register
/// [`remove`]: ClickThroughHandle::remove
pub(crate) struct ClickThroughHandle {
    monitor: Rc<CursorMonitorBridge>,
    registry: Rc<RefCell<ClickThroughRegistry>>,
    wake_event: Arc<Event>,
    _join: crate::executor::JoinHandle<()>,
}

impl ClickThroughHandle {
    /// 監視対象窓を登録する（start 後・areka の非同期窓生成に追随）。
    pub(crate) fn register(&self, window: Entity, hwnd: windows::Win32::Foundation::HWND) {
        self.registry.borrow_mut().register(window, hwnd);
    }

    /// 監視対象窓を除去する（ウィンドウ破棄時・R7.2 非破壊）。存在すれば `true`。
    pub(crate) fn remove(&self, window: Entity) -> bool {
        self.registry.borrow_mut().remove(window)
    }

    /// 共有レジストリへの参照（結線・テスト用）。
    pub(crate) fn registry(&self) -> &Rc<RefCell<ClickThroughRegistry>> {
        &self.registry
    }

    /// 共有 wake event への参照（VSync tick 源の結線・強制再評価・テスト用）。
    pub(crate) fn wake_event(&self) -> &Arc<Event> {
        &self.wake_event
    }

    /// 手動でループを 1 回起床する（テスト・強制再評価用）。
    pub(crate) fn wake(&self) {
        self.wake_event.notify(usize::MAX);
    }

    /// ワーカ最新カーソル座標（screen physical）を読む（テスト・結線用）。
    pub(crate) fn latest_cursor(&self) -> PhysicalPoint {
        self.monitor.latest_cursor()
    }
}

/// アプリ側（`crates/areka`・task 4.1）が監視対象窓を登録／除去するための **公開** ハンドル。
///
/// クリック透過機構は `WinApp::run`（`runtime/mod.rs` 結線点）で起動され、その共有レジストリ
/// （`Rc<RefCell<ClickThroughRegistry>>`）がこの newtype に包まれて World へ **NonSend リソース**
/// として挿入される。areka の `run_setup`（`&mut World` を持つ）は
/// `world.get_non_send::<ClickThroughRegistryHandle>()` で取得し、生成した shell/balloon
/// の 2 窓（window Entity ＋ HWND）を [`register`](Self::register) で登録する。
///
/// # なぜ NonSend リソースか
/// areka は窓を **非同期生成**する（`run_setup` は command 経由で後から `&mut World` を得る）。
/// `ClickThroughController::start` は `run()` 冒頭で走るため、登録面を World に置いておけば
/// start 後の任意タイミングで登録できる。レジストリ本体は UI スレッド単独所有（`!Send`）ゆえ
/// NonSend リソースが妥当（`Rc`/`RefCell` は `!Send`）。
///
/// # 破棄追随
/// 登録済み窓が破棄されても、[`remove`](Self::remove) の明示除去に加え、機構内部の
/// [`prune_dead_targets`] が毎評価で World 生存を確認して自動的に刈り取る（二重の安全弁）。
pub struct ClickThroughRegistryHandle {
    registry: Rc<RefCell<ClickThroughRegistry>>,
}

impl ClickThroughRegistryHandle {
    /// 共有レジストリを包んで登録面を作る（`runtime/mod.rs` 結線点が呼ぶ）。
    pub(crate) fn new(registry: Rc<RefCell<ClickThroughRegistry>>) -> Self {
        Self { registry }
    }

    /// 監視対象窓を登録する（areka の非同期窓生成に追随・task 4.1）。
    ///
    /// 同一 window Entity の再登録は HWND 更新＋`last_applied` リセット（dedupe）。
    pub fn register(&self, window: Entity, hwnd: windows::Win32::Foundation::HWND) {
        self.registry.borrow_mut().register(window, hwnd);
    }

    /// 監視対象窓を明示除去する（窓破棄時・R7.2 非破壊）。存在すれば `true`。
    pub fn remove(&self, window: Entity) -> bool {
        self.registry.borrow_mut().remove(window)
    }

    /// 登録済み対象窓数（テスト・観測用）。
    pub fn len(&self) -> usize {
        self.registry.borrow().len()
    }

    /// 登録済み対象が無いか（テスト・観測用）。
    pub fn is_empty(&self) -> bool {
        self.registry.borrow().is_empty()
    }
}

/// 二重起床の判定・適用ループ本体（`spawn_local` で UI スレッドに駆動される async）。
///
/// `run_async_tick` と同一の listen-before-work 規律: await の **前** に
/// `listener = wake_event.listen()` を arm する（処理中に届く notify を落とさない・
/// 二重起床の取りこぼし防止）。World の `Weak` は毎起床 `upgrade()` し、`None`
/// （shutdown で strong 所有者 drop 済み）なら安全にループを終了する。
///
/// 起床後は当該フレームの **ECS tick 完了後（post-tick）** の settled World を 1 回だけ
/// 評価する。wake event を tick 源が post-tick で notify する結線は task 3.2 の責務で
/// あり、本ループは「起床したら settled World を評価する」ことのみ担う。
async fn run_click_through(
    wake_event: Arc<Event>,
    world: Weak<RefCell<EcsWorld>>,
    registry: Rc<RefCell<ClickThroughRegistry>>,
    monitor: Weak<CursorMonitorBridge>,
) {
    debug!("ClickThrough eval loop started (dual-wake: cursor notify + VSync tick)");
    loop {
        // 先に listen() を arm（処理中に届く notify を落とさない）。
        let listener = wake_event.listen();

        // strong 所有者が生存しているか確認。いずれか None なら shutdown — 終了。
        // - world: UI スレッド World の strong 所有者 drop（アプリ終了）。
        // - monitor: handle drop（ワーカは既に stop/join 済み）。
        let (Some(world_rc), Some(monitor_rc)) = (world.upgrade(), monitor.upgrade()) else {
            debug!("ClickThrough eval loop stopping (world/handle dropped — shutdown)");
            return;
        };

        // 起床を待機（カーソル移動 notify or VSync tick まで UI スレッドを譲る）。
        listener.await;

        // ワーカ最新カーソル座標を読み（store→notify 規律で最新を観測）、settled World を
        // 1 回評価する。World 借用に失敗（tick 進行中等）した場合は当該サイクルを安全側
        // スキップする（次起床で再評価・post-tick では通常競合しない）。
        let cursor = monitor_rc.latest_cursor();
        match world_rc.try_borrow() {
            Ok(ecs_world) => {
                let mut reg = registry.borrow_mut();
                evaluate_targets(ecs_world.world(), &mut reg, |hwnd| {
                    screen_to_client_point(hwnd, cursor)
                });
            }
            Err(_) => {
                trace!("clickthrough: World borrow 失敗 — このサイクルを安全スキップ");
            }
        }
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
