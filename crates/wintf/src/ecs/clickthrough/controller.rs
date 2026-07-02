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
use crate::win_style::apply_click_through;

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
/// - `cursor_screen`: ワーカ最新カーソル座標（screen physical・`i32`）。
///
/// # 各対象窓の処理
/// 1. `WindowPos.position`（クライアント原点・screen physical）を World から取得。
///    無い窓（未マップ等）はスキップ。
/// 2. `client = cursor_screen - position` を計算し（座標変換は既存 `hit_test_in_window`
///    へ委譲＝screen 前提の変換チェーンをそのまま再利用・R8）、`hit_test_in_window` を呼ぶ。
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
    cursor_screen: PhysicalPoint,
) {
    // ドラッグスナップショットは 1 パスで一度だけ読む（全窓で同一・UI スレッド）。
    let drag = snapshot_drag_state();

    // 巡回中に `registry` を可変借用して書き戻すため、対象の列挙を先にスナップショット
    // する（`iter` の不変借用と `set_last_applied` の可変借用の重複を避ける）。
    // 対象は areka で 2 窓のみ・汎用でも小数のため Vec 収集のコストは無視できる。
    let targets: Vec<(Entity, windows::Win32::Foundation::HWND, DesiredState)> = registry
        .iter()
        .map(|t| (t.window, t.hwnd, t.last_applied))
        .collect();

    for (window, hwnd, last_applied) in targets {
        // 窓クライアント原点（screen physical）。未マップ等で無ければスキップ。
        let Some(pos) = world.get::<WindowPos>(window).and_then(|wp| wp.position) else {
            trace!(?window, "clickthrough: WindowPos.position 未確定 — skip");
            continue;
        };

        // screen physical カーソル → 当該窓のクライアント physical（cursor - 原点）。
        // 座標系対応（DPI/マルチモニタ/移動）は既存 hit_test_in_window の変換チェーンへ委譲。
        let client = PointF::new(
            (cursor_screen.x - pos.x) as f32,
            (cursor_screen.y - pos.y) as f32,
        );

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
        let join = wintf_winmsg_executor::spawn_local(run_click_through(
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
    _join: wintf_winmsg_executor::JoinHandle<()>,
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
                evaluate_targets(ecs_world.world(), &mut reg, cursor);
            }
            Err(_) => {
                trace!("clickthrough: World borrow 失敗 — このサイクルを安全スキップ");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Point;
    use crate::ecs::pointer::PhysicalPoint;
    use std::time::Instant;
    use windows::Win32::Foundation::HWND;

    fn entity(id: u32) -> Entity {
        // bevy_ecs 0.18: from_raw_u32 は index から Entity を生成する（None は
        // 予約インデックスのみ）。テスト用ダミーなので unwrap で十分。
        Entity::from_raw_u32(id).expect("valid test entity index")
    }

    fn ppoint() -> PhysicalPoint {
        PhysicalPoint { x: 0, y: 0 }
    }

    /// 閾値到達・移動中の `Dragging` スナップショット。
    fn dragging() -> DragStateSnapshot {
        DragStateSnapshot::Dragging {
            entity: entity(1),
            start_pos: ppoint(),
            current_pos: ppoint(),
            prev_pos: ppoint(),
            start_time: Instant::now(),
            hwnd: HWND::default(),
            initial_window_pos: Point { x: 0, y: 0 },
            move_window: true,
            constraint: None,
        }
    }

    /// ドラッグ終了直後の `JustEnded` スナップショット。
    fn just_ended() -> DragStateSnapshot {
        DragStateSnapshot::JustEnded {
            entity: entity(1),
            position: ppoint(),
            cancelled: false,
        }
    }

    // --- 非ドラッグ写像（R3.3） ---

    #[test]
    fn maps_hit_some_to_opaque_when_last_was_transparent() {
        let out = resolve_transition(
            Some(entity(7)),
            &DragStateSnapshot::Idle,
            DesiredState::Transparent,
        );
        assert_eq!(out, Some(DesiredState::Opaque));
    }

    #[test]
    fn maps_hit_none_to_transparent_when_last_was_opaque() {
        let out = resolve_transition(None, &DragStateSnapshot::Idle, DesiredState::Opaque);
        assert_eq!(out, Some(DesiredState::Transparent));
    }

    // --- 差分ガード（R3.2） ---

    #[test]
    fn diff_guard_hit_some_already_opaque_returns_none() {
        let out = resolve_transition(
            Some(entity(7)),
            &DragStateSnapshot::Idle,
            DesiredState::Opaque,
        );
        assert_eq!(out, None);
    }

    #[test]
    fn diff_guard_hit_none_already_transparent_returns_none() {
        let out = resolve_transition(None, &DragStateSnapshot::Idle, DesiredState::Transparent);
        assert_eq!(out, None);
    }

    // --- ドラッグ抑止（R5.1/R5.3） ---

    #[test]
    fn dragging_never_goes_transparent_even_when_hit_none() {
        // コア R5 アンチフリッカ: 移動中にカーソルがキャラから外れても透過に落とさない。
        let out = resolve_transition(None, &dragging(), DesiredState::Opaque);
        assert_eq!(out, None);
    }

    #[test]
    fn dragging_forces_opaque_when_last_was_transparent() {
        // 移動中は強制 Opaque。透過状態から入ったら不透過へ引き戻す（透過にはしない）。
        let out = resolve_transition(None, &dragging(), DesiredState::Transparent);
        assert_eq!(out, Some(DesiredState::Opaque));
    }

    #[test]
    fn just_started_also_suppresses_transparent() {
        // JustStarted（移動開始直後）も抑止対象。
        let just_started = DragStateSnapshot::JustStarted {
            entity: entity(1),
            start_pos: ppoint(),
            current_pos: ppoint(),
            start_time: Instant::now(),
        };
        assert_eq!(
            resolve_transition(None, &just_started, DesiredState::Opaque),
            None
        );
    }

    // --- JustEnded 再収束（R5.2） ---

    #[test]
    fn just_ended_reconverges_to_transparent_when_hit_none() {
        // 抑止解除後、現在 hit=None なので透過へ再収束する。
        let out = resolve_transition(None, &just_ended(), DesiredState::Opaque);
        assert_eq!(out, Some(DesiredState::Transparent));
    }

    #[test]
    fn just_ended_reconverges_to_opaque_when_hit_some() {
        let out = resolve_transition(
            Some(entity(3)),
            &just_ended(),
            DesiredState::Transparent,
        );
        assert_eq!(out, Some(DesiredState::Opaque));
    }

    // --- Preparing は非ドラッグ写像として振る舞う ---

    #[test]
    fn preparing_behaves_as_non_drag_mapping() {
        let preparing = DragStateSnapshot::Preparing {
            entity: entity(1),
            start_pos: ppoint(),
            start_time: Instant::now(),
        };
        // hit=None → Transparent（押下のみ・閾値未到達なのでまだドラッグではない）。
        assert_eq!(
            resolve_transition(None, &preparing, DesiredState::Opaque),
            Some(DesiredState::Transparent)
        );
        // hit=Some → Opaque。
        assert_eq!(
            resolve_transition(Some(entity(2)), &preparing, DesiredState::Transparent),
            Some(DesiredState::Opaque)
        );
    }

    // ========================================================================
    // evaluate_targets（同期評価コア・real World + real HWND）
    // ========================================================================

    use crate::api::get_window_long_ptr;
    use crate::ecs::layout::GlobalArrangement;
    use crate::ecs::types::{Rect, SizeI};
    use crate::ecs::window::WindowPos;
    use crate::ecs::world::EcsWorld;
    use bevy_ecs::world::World;
    use std::cell::RefCell as StdRefCell;
    use std::rc::Rc as StdRc;
    use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, WS_EX_TRANSPARENT};

    /// 指定 bounds の `GlobalArrangement` を作る（hit_test テストと同じヘルパー）。
    fn global_arrangement(left: f32, top: f32, right: f32, bottom: f32) -> GlobalArrangement {
        GlobalArrangement {
            transform: windows_numerics::Matrix3x2::translation(left, top),
            bounds: Rect {
                left,
                top,
                right,
                bottom,
            },
        }
    }

    /// 実所有のテスト HWND（非表示ポップアップ "Static"）を生成する。
    /// task 1.1 の `apply_click_through` テストと同じ生成レシピ。呼び出し側が破棄する。
    fn create_test_hwnd() -> HWND {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
        };
        use windows::core::w;
        // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ポップアップを生成する。
        unsafe {
            let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Static"),
                w!("wintf-clickthrough-eval-test"),
                WINDOW_STYLE(WS_POPUP.0),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
            .expect("CreateWindowExW should create a hidden test window")
        }
    }

    /// テスト HWND を破棄する（後始末）。
    fn destroy_test_hwnd(hwnd: HWND) {
        use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
        // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する。
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }

    /// 現在の ex-style に TRANSPARENT ビットが立っているか読み戻す。
    fn is_transparent(hwnd: HWND) -> bool {
        let ex = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style") as u32;
        ex & WS_EX_TRANSPARENT.0 != 0
    }

    /// 原点 (100,200) の窓に、client (50,50)=screen (150,250) で当たる子を仕込んだ
    /// World を作る。窓 Entity を返す。cursor screen (150,250) が hit、(0,0) が no-hit。
    fn world_with_hittable_window(world: &mut World) -> Entity {
        let window = world
            .spawn((
                global_arrangement(100.0, 200.0, 500.0, 500.0),
                WindowPos {
                    position: Some(Point { x: 100, y: 200 }),
                    size: Some(SizeI {
                        width: 400,
                        height: 300,
                    }),
                    ..Default::default()
                },
            ))
            .id();
        let widget = world
            .spawn(global_arrangement(150.0, 250.0, 250.0, 300.0))
            .id();
        world.entity_mut(window).add_children(&[widget]);
        window
    }

    /// (a) hit（Some）かつ last_applied=Transparent → Opaque へ・ex-style TRANSPARENT クリア。
    #[test]
    fn eval_hit_some_from_transparent_becomes_opaque() {
        let mut world = World::new();
        let window = world_with_hittable_window(&mut world);
        let hwnd = create_test_hwnd();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // 前提: TRANSPARENT を立てて last_applied=Transparent の状態から開始。
            apply_click_through(hwnd, true).expect("seed transparent");
            assert!(is_transparent(hwnd), "seed: TRANSPARENT must be set");

            let mut reg = ClickThroughRegistry::new();
            reg.register(window, hwnd);
            reg.set_last_applied(window, DesiredState::Transparent);

            // cursor screen (150,250) → client (50,50) → widget にヒット。
            evaluate_targets(&world, &mut reg, PhysicalPoint::new(150, 250));

            assert_eq!(
                reg.last_applied(window),
                Some(DesiredState::Opaque),
                "hit=Some は Opaque へ収束すべき"
            );
            assert!(!is_transparent(hwnd), "Opaque 適用後 TRANSPARENT はクリア");
        }));
        destroy_test_hwnd(hwnd);
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    /// (b) no-hit（None）かつ last_applied=Opaque → Transparent へ・ex-style TRANSPARENT セット。
    #[test]
    fn eval_hit_none_from_opaque_becomes_transparent() {
        let mut world = World::new();
        let window = world_with_hittable_window(&mut world);
        let hwnd = create_test_hwnd();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            apply_click_through(hwnd, false).expect("seed opaque");
            assert!(!is_transparent(hwnd), "seed: TRANSPARENT must be clear");

            let mut reg = ClickThroughRegistry::new();
            reg.register(window, hwnd); // 既定 Opaque

            // cursor screen (0,0) → client (-100,-200) → widget に当たらない（no-hit）。
            evaluate_targets(&world, &mut reg, PhysicalPoint::new(0, 0));

            assert_eq!(
                reg.last_applied(window),
                Some(DesiredState::Transparent),
                "hit=None は Transparent へ収束すべき"
            );
            assert!(is_transparent(hwnd), "Transparent 適用後 TRANSPARENT はセット");
        }));
        destroy_test_hwnd(hwnd);
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    /// (c) 差分ガード: 同一入力で 2 回評価しても 2 回目は状態不変（余計な変化なし）。
    #[test]
    fn eval_diff_guard_stable_on_repeat() {
        let mut world = World::new();
        let window = world_with_hittable_window(&mut world);
        let hwnd = create_test_hwnd();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            apply_click_through(hwnd, true).expect("seed transparent");

            let mut reg = ClickThroughRegistry::new();
            reg.register(window, hwnd);
            reg.set_last_applied(window, DesiredState::Transparent);

            // 1 回目: hit → Opaque へ変化。
            evaluate_targets(&world, &mut reg, PhysicalPoint::new(150, 250));
            assert_eq!(reg.last_applied(window), Some(DesiredState::Opaque));
            assert!(!is_transparent(hwnd));

            // 2 回目: 同一 hit・last_applied=Opaque → resolve_transition が None → 無適用・不変。
            evaluate_targets(&world, &mut reg, PhysicalPoint::new(150, 250));
            assert_eq!(
                reg.last_applied(window),
                Some(DesiredState::Opaque),
                "同一入力の再評価で状態は不変であるべき（差分ガード）"
            );
            assert!(!is_transparent(hwnd), "再評価でも ex-style は不変");
        }));
        destroy_test_hwnd(hwnd);
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    /// (d) ドラッグ経路: `JustEnded`（capture_guard 不要の実 thread_local 状態）を仕込むと
    /// 抑止解除サイクルとして現在 hit に基づき再収束する（eval コアが drag スナップショットを
    /// 尊重することの確認）。Dragging 抑止自体は `resolve_transition` の in-source テストで網羅。
    #[test]
    fn eval_honors_drag_snapshot_just_ended_reconverges() {
        use crate::ecs::drag::{reset_to_idle, snapshot_drag_state, update_drag_state};
        use crate::ecs::drag::DragState;

        let mut world = World::new();
        let window = world_with_hittable_window(&mut world);
        let hwnd = create_test_hwnd();

        // thread_local を JustEnded に設定（capture_guard を持たない変種ゆえ直接構築可能）。
        update_drag_state(|state| {
            *state = DragState::JustEnded {
                entity: entity(1),
                position: ppoint(),
                cancelled: false,
            };
        });
        // 前提確認: スナップショットが JustEnded であること。
        assert!(matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded { .. }
        ));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            apply_click_through(hwnd, true).expect("seed transparent");
            let mut reg = ClickThroughRegistry::new();
            reg.register(window, hwnd);
            reg.set_last_applied(window, DesiredState::Transparent);

            // JustEnded は抑止解除。hit=Some（screen 150,250）→ Opaque へ再収束。
            evaluate_targets(&world, &mut reg, PhysicalPoint::new(150, 250));
            assert_eq!(
                reg.last_applied(window),
                Some(DesiredState::Opaque),
                "JustEnded サイクルは現在 hit に基づき再収束すべき"
            );
        }));

        // thread_local を Idle に戻す（他テストへの汚染防止）。JustEnded→reset_to_idle。
        reset_to_idle();
        destroy_test_hwnd(hwnd);
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    /// WindowPos.position が無い窓はスキップされ、状態が据え置かれること（グレースフル）。
    #[test]
    fn eval_skips_window_without_position() {
        let mut world = World::new();
        // position を持たない窓（未マップ相当）。
        let window = world.spawn(global_arrangement(0.0, 0.0, 100.0, 100.0)).id();
        let hwnd = create_test_hwnd();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut reg = ClickThroughRegistry::new();
            reg.register(window, hwnd); // Opaque

            evaluate_targets(&world, &mut reg, PhysicalPoint::new(10, 10));

            // position 未確定ゆえ skip・last_applied は初期 Opaque のまま。
            assert_eq!(reg.last_applied(window), Some(DesiredState::Opaque));
        }));
        destroy_test_hwnd(hwnd);
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    // ========================================================================
    // ClickThroughController::start / ClickThroughHandle（RAII・shutdown）
    // ========================================================================

    /// RAII: `start`→`drop(handle)` でカーソルワーカが確実に stop/join され、ハングしない。
    /// executor を回さなくても spawn_local タスクは投入されるだけ（未実行）で問題ない。
    #[test]
    fn start_then_drop_joins_worker_without_hanging() {
        let world = StdRc::new(StdRefCell::new(EcsWorld::new()));
        let registry = StdRc::new(StdRefCell::new(ClickThroughRegistry::new()));
        let wake = Arc::new(Event::new());

        let handle = ClickThroughController::start(
            StdRc::downgrade(&world),
            StdRc::clone(&registry),
            Arc::clone(&wake),
        );

        // handle 経由の register/remove が共有レジストリへ反映されること。
        let mut w = World::new();
        let e = w.spawn_empty().id();
        let hwnd = create_test_hwnd();
        handle.register(e, hwnd);
        assert_eq!(registry.borrow().len(), 1, "start 後の register が反映される");
        assert!(handle.remove(e), "start 後の remove が反映される");
        assert!(registry.borrow().is_empty());
        destroy_test_hwnd(hwnd);

        // drop で唯一の強 Rc<CursorMonitorBridge> が落ち、ワーカが stop/join。
        // ハングせず復帰すれば RAII は健全。
        drop(handle);
    }

    /// shutdown 終了条件: world strong 所有者を drop すると、次起床でループ本体の
    /// `world.upgrade()` が `None` を返し評価が走らない。ここでは executor を回さず、
    /// 終了条件そのもの（Weak upgrade が None）を sync レベルで確認する（フル async 駆動は
    /// メッセージループが要るため、`run_async_tick` テストと同様に条件を直接検証する）。
    #[test]
    fn shutdown_condition_world_weak_upgrade_none() {
        let world = StdRc::new(StdRefCell::new(EcsWorld::new()));
        let weak = StdRc::downgrade(&world);
        assert!(weak.upgrade().is_some(), "生存中は upgrade できる");

        // strong 所有者を drop（アプリ shutdown 相当）。
        drop(world);
        assert!(
            weak.upgrade().is_none(),
            "world drop 後は Weak upgrade が None（ループはこの契機で終了する）"
        );
    }
}
