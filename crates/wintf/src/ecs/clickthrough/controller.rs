//! クリック透過機構の UI スレッド側判定・適用ロジック。
//!
//! 本ファイルは 3 層で構成する:
//! 1. [`resolve_transition`]（純関数・World 非依存）: 差分ガード＋ドラッグ抑止を
//!    適用して「今回適用すべき変化」を返す。副作用・World アクセスを持たない。
//! 2. [`evaluate_targets`]（同期評価コア・テスト可能）: settled な `&World` に対し
//!    レジストリの各対象窓を 1 回巡回し、ヒットテスト→`resolve_transition`→
//!    差分時のみ `apply_click_through`→成功時のみ `last_applied` 書き戻し、を行う。
//!    `tick_bridge::tick_one_frame` と同じ「同期コア切り出し」規律でテスト隔離する。
//! 3. [`start_click_through`] / [`ClickThroughHandle`]（async ループ＋RAII）:
//!    `spawn_local` で UI スレッドに投入する二重起床（カーソル移動 notify／VSync tick）
//!    の listen-before-work ループと、その生存期間を束ねる RAII ハンドル。

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use event_listener::Event;
use tracing::{debug, trace, warn};

use crate::ecs::PointF;
use crate::ecs::drag::{DragState, snapshot_drag_state};
use crate::ecs::hit_test_in_window;
use crate::ecs::world::EcsWorld;
use crate::ecs::{PhysicalPoint, Window, WindowHandle, WindowPos};
use crate::win_style::{apply_click_through, apply_layered_companion};
use windows::Win32::Foundation::{HWND, POINT};

use super::{ClickThroughRegistry, CursorMonitorBridge, DesiredState};

/// 差分ガード＋ドラッグ抑止を適用して「今回適用すべき変化」を返す純関数。
///
/// 適用不要（差分なし or ドラッグ抑止）の場合は `None` を返す。
///
/// # 判定順序（ゲーティング）
/// 1. **ドラッグ最優先ゲート**: ドラッグ移動中は透過 ON へ遷移させない。
///    移動中は望ましい状態を強制的に `Opaque` とし、`last_applied != Opaque` の
///    時のみ `Some(Opaque)`（透過を外して掴み維持）、同一なら `None`。
///    - 抑止対象は `Dragging`（閾値到達・移動中）に加え、閾値到達から次の tick の
///      `dispatch_drag_events` までの `JustStarted`（掴み確定）も含む。ここで透過 ON に
///      なると掴みが崩れる。`Preparing`（押下のみ・閾値未到達）はまだドラッグ開始前なので
///      非ドラッグ写像に委ねる。
/// 2. **`JustEnded` 再収束**: ドラッグ終了直後は抑止を解除し、現在の `hit` に
///    基づく非ドラッグ写像へ委ねる。
/// 3. **非ドラッグ写像**: `Some(entity)` → `Opaque`（不透過・自窓で受領）、
///    `None` → `Transparent`（透過・背面プロセスへ通過）。
/// 4. **差分ガード**: 望ましい状態が `last_applied` と同一なら `None`（再適用しない）。
///
/// `last_applied` の真実源（[`super::ClickThroughRegistry`]）への書き戻しは
/// 呼び出し側（`evaluate_targets`）の責務。
pub(crate) fn resolve_transition(
    hit: Option<Entity>,
    drag: &DragState,
    last_applied: DesiredState,
) -> Option<DesiredState> {
    let desired = match drag {
        DragState::Dragging { .. } | DragState::JustStarted { .. } => DesiredState::Opaque,
        DragState::Idle | DragState::Preparing { .. } | DragState::JustEnded { .. } => match hit {
            Some(_) => DesiredState::Opaque,
            None => DesiredState::Transparent,
        },
    };

    if desired == last_applied {
        None
    } else {
        Some(desired)
    }
}

/// 対象のうち、`Window` と、登録の HWND と同じ `WindowHandle` を持つ窓だけを残す
/// （窓の破棄と付け替えに追随）。対象は wintf の窓だけ。除去件数を返す。
///
/// 刈る条件: window Entity が despawn 済み・`Window` を外した（`Window` に `on_remove` の
/// フックが無いので `WindowHandle` は残る）・`WindowHandle` の HWND が登録の HWND と違う。
///
/// [`evaluate_targets`] の巡回前に呼び、破棄済み・付け替え前の HWND へ
/// `ScreenToClient`／`apply_click_through` を撃つ経路を断つ。
pub(crate) fn prune_dead_targets(world: &World, registry: &mut ClickThroughRegistry) -> usize {
    registry.retain(|t| {
        world.get::<Window>(t.window).is_some()
            && world
                .get::<WindowHandle>(t.window)
                .is_some_and(|h| h.hwnd == t.hwnd)
    })
}

/// settled な `&World` に対しレジストリの全対象窓を 1 回巡回評価する同期コア
/// （post-tick 評価の本体）。
///
/// `tick_bridge::tick_one_frame` と同じく、async ループ（[`run_click_through`]）から
/// 同期コアを切り出してテスト隔離する。
///
/// # 引数
/// - `world`: ECS tick 完了後（`GlobalArrangement`／`AlphaMask`／`DragState` 確定後）の
///   settled World 参照。単一 UI スレッドで tick と排他されるため中間状態を読まない。
/// - `registry`: 監視対象窓と `last_applied`。差分ガードの真実源。
/// - `screen_to_client`: 対象 HWND の screen physical カーソルを client physical へ変換する
///   関数（production は `screen_to_client_point`）。`None` は当該窓 skip（無効 HWND 等）。
///   テストは決定的な模擬変換を注入する。
///
/// # 各対象窓の処理
/// 0. `layered_applied` が偽なら `apply_layered_companion` で `WS_EX_LAYERED` 同伴フラグを
///    1 回立てる（DComp 窓は `WS_EX_TRANSPARENT` 単独ではマウス透過が効かない）。
///    成功時のみ真へ書き戻し、失敗時は当該窓を skip（次サイクル再試行）。
/// 1. `WindowPos.position` が無い未マップ窓はスキップ。
/// 2. `screen_to_client(hwnd)` で client physical を得て `hit_test_in_window` を呼ぶ。
/// 3. `snapshot_drag_state()` と `last_applied` を渡して [`resolve_transition`] で desired を
///    決める。`None`（差分なし or ドラッグ抑止）は skip。
/// 4. 変化時のみ `apply_click_through` を **1 回** 適用し、**`Ok` の時だけ** レジストリの
///    `last_applied` を書き戻す。`Err` は `warn!` でログし据え置き（次サイクルで再試行）。
///
/// ドラッグ状態は 1 パス内で全窓に同一スナップショットを用いるため、ループ前に一度だけ読む。
///
/// 表示層（GPU 合成 visual/content）には一切触れない（ex-style トグルのみ）。
pub(crate) fn evaluate_targets(
    world: &World,
    registry: &mut ClickThroughRegistry,
    mut screen_to_client: impl FnMut(HWND) -> Option<PointF>,
) {
    prune_dead_targets(world, registry);

    let drag = snapshot_drag_state();

    // `iter` の不変借用と `set_last_applied` の可変借用が重なるので、対象を先に集める。
    let targets: Vec<(Entity, windows::Win32::Foundation::HWND, DesiredState, bool)> = registry
        .iter()
        .map(|t| (t.window, t.hwnd, t.last_applied, t.layered_applied))
        .collect();

    for (window, hwnd, last_applied, layered_applied) in targets {
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

        if world
            .get::<WindowPos>(window)
            .and_then(|wp| wp.position)
            .is_none()
        {
            trace!(?window, "clickthrough: WindowPos.position 未確定 — skip");
            continue;
        }

        let Some(client) = screen_to_client(hwnd) else {
            trace!(?window, "clickthrough: screen→client 変換失敗 — skip");
            continue;
        };

        let hit = hit_test_in_window(world, window, client);

        let Some(desired) = resolve_transition(hit, &drag, last_applied) else {
            continue;
        };

        match apply_click_through(hwnd, desired == DesiredState::Transparent) {
            Ok(()) => {
                debug!(?window, ?desired, "clickthrough: ex-style トグル適用");
                registry.set_last_applied(window, desired);
                // 透過を外した直後は WM_SETCURSOR が次の移動まで来ないので、その場で出し直す。
                if desired == DesiredState::Opaque {
                    crate::ecs::cursor::answer::push_cursor(
                        world,
                        window,
                        crate::ecs::cursor::CursorTrigger::Move,
                    );
                }
            }
            Err(e) => {
                warn!(?window, error = %e, "clickthrough: apply_click_through 失敗 — skip");
            }
        }
    }
}

/// screen physical → client physical を OS(`ScreenToClient`)へ委譲する変換（production 用）。
///
/// NCHITTEST キャッシュ（`pointer::nchittest_cache`）・OS 入力経路（`window_proc::mouse_click`）と
/// 同一の変換で、frame/DPI/マルチモニタを OS に委ねる。`WindowPos.position` の手引き算は、
/// 高 DPI・マルチモニタで position が窓の真の物理クライアント原点とズレると判定と表示がずれるため
/// 採らない。無効 HWND 等で失敗した場合は `None`。
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
/// - **単一 wake event を引数で受ける**: async ループはただ 1 つの共有
///   `Arc<event_listener::Event>` を listen する。これを (a) カーソルワーカ
///   （`CursorMonitorBridge::spawn(wake_event.clone())`）と (b) VSync tick 源（`WinApp::run` が
///   起こす中継 `tick_bridge::relay_vblank` が vblank ごとに notify）の **両方** が notify することで、
///   futures/select コンビネータ（tokio/futures 非依存）無しに二重起床を実現する。tick 源への
///   結線は `WinApp` の責務（`wire_click_through` が event を作り、`run` が `relay_vblank` へ
///   渡す）なので、`start` は wake event を **引数** で受ける。
/// - **レジストリを共有可能化**（`Rc<RefCell<ClickThroughRegistry>>`）: areka は窓を
///   非同期生成するため、アプリ側が `start` **後** に対象窓を登録できるよう、呼び手が
///   同じ表を [`ClickThroughRegistryHandle`] に包んで World へ据える。
///
/// - `world`: UI スレッド所有 World への `Weak`（`run_async_tick` と同じ寿命規律・
///   shutdown で `upgrade()` が `None` を返しループが安全終了）。
/// - `running`: `WinApp::run` のループがまだ回っているか。下りた後の起床では判定を
///   回さずに終える（[`run_click_through`] 参照）。
///
/// # Postconditions
/// カーソルワーカ稼働・async ループ稼働。返した `ClickThroughHandle` の drop で
/// 機構停止（ワーカ join・async ループは `run()` が旗を下ろした後の最初の起床か、world drop で終了）。
pub(crate) fn start_click_through(
    world: Weak<RefCell<EcsWorld>>,
    registry: Rc<RefCell<ClickThroughRegistry>>,
    wake_event: Arc<Event>,
    running: Rc<Cell<bool>>,
) -> ClickThroughHandle {
    // join は handle 単独が駆動するので、handle が唯一の強 `Rc` を持ち、ループには `Weak` を
    // 渡す（handle drop で即 stop/join・ループが park 中でも遅延しない）。
    let monitor = Rc::new(CursorMonitorBridge::spawn(Arc::clone(&wake_event)));

    let join = crate::executor::spawn_local(run_click_through(
        Arc::clone(&wake_event),
        Weak::clone(&world),
        Rc::clone(&registry),
        Rc::downgrade(&monitor),
        running,
    ));

    debug!("click-through started (UI-thread eval loop + cursor worker)");

    ClickThroughHandle {
        _monitor: monitor,
        _join: join,
    }
}

/// クリック透過機構の生存期間を束ねる RAII ハンドル。持つだけで、口は無い
/// （登録・除去は呼び手が同じ表を包んだ [`ClickThroughRegistryHandle`] で行う）。
///
/// - `_monitor`: カーソルワーカの唯一の強参照。drop でワーカを stop/join（`CursorMonitorBridge::Drop`）。
/// - `_join`: async ループの JoinHandle。ループは `world` の `Weak` が upgrade 不能に
///   なった時点（shutdown）で自ら終了する（`run_async_tick` と同じ終了規律）。
pub(crate) struct ClickThroughHandle {
    _monitor: Rc<CursorMonitorBridge>,
    _join: crate::executor::JoinHandle<()>,
}

/// アプリ側（`crates/areka`）が監視対象窓を登録するための **公開** ハンドル。
///
/// `WinApp::run`（`runtime/mod.rs` 結線点）が共有レジストリをこの newtype に包み、World へ
/// **NonSend リソース**として挿入する。areka の `run_setup` は
/// `world.get_non_send::<ClickThroughRegistryHandle>()` で取得し、生成した shell/balloon
/// の 2 窓（window Entity ＋ HWND）を [`register`](Self::register) で登録する。
///
/// # なぜ NonSend リソースか
/// areka は窓を **非同期生成**し、[`start_click_through`] は `run()` 冒頭で走る。登録面を
/// World に置けば start 後の任意タイミングで登録できる。レジストリ本体は `Rc`/`RefCell` で
/// `!Send`。
///
/// # 破棄追随
/// 除去の口は無い。機構内部の [`prune_dead_targets`] が毎評価で、`Window` と登録の HWND と
/// 同じ `WindowHandle` を持つ wintf の窓だけを残して刈り取る（despawn で十分）。
pub struct ClickThroughRegistryHandle {
    registry: Rc<RefCell<ClickThroughRegistry>>,
}

impl ClickThroughRegistryHandle {
    /// 共有レジストリを包んで登録面を作る（`runtime/mod.rs` 結線点が呼ぶ）。
    pub(crate) fn new(registry: Rc<RefCell<ClickThroughRegistry>>) -> Self {
        Self { registry }
    }

    /// 監視対象窓を登録する（areka の非同期窓生成に追随）。
    ///
    /// 同一 window Entity の再登録は重複させず、HWND が変わったときだけ記録を戻す
    /// （同じ HWND なら何もしない）。
    pub fn register(&self, window: Entity, hwnd: windows::Win32::Foundation::HWND) {
        self.registry.borrow_mut().register(window, hwnd);
    }

    /// 登録済み対象窓数。使い手はテストだけ。
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.registry.borrow().len()
    }

    /// 登録済み対象が無いか。使い手はテストだけ。
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.registry.borrow().is_empty()
    }
}

/// 二重起床の判定・適用ループ本体（`spawn_local` で UI スレッドに駆動される async）。
///
/// `run_async_tick` と同一の listen-before-work 規律: await の **前** に
/// `listener = wake_event.listen()` を arm する（処理中に届く notify を落とさない）。
/// 起きたらまず `running` を見て、下りていれば判定を回さずに終える。World とカーソル監視の
/// `Weak` は起きてから `upgrade()` し（待ちの間に強参照を握らない）、`None`（shutdown で
/// strong 所有者 drop 済み）でも終える。
///
/// 起床後は当該フレームの **ECS tick 完了後（post-tick）** の settled World を 1 回だけ評価する。
async fn run_click_through(
    wake_event: Arc<Event>,
    world: Weak<RefCell<EcsWorld>>,
    registry: Rc<RefCell<ClickThroughRegistry>>,
    monitor: Weak<CursorMonitorBridge>,
    running: Rc<Cell<bool>>,
) {
    debug!("ClickThrough eval loop started (dual-wake: cursor notify + VSync tick)");
    loop {
        let listener = wake_event.listen();

        // 待ちの間は `Weak` のまま（強参照を握ると handle drop でワーカが止まらない）。
        listener.await;

        // `WinApp::run` のループが戻った後の起床では判定を回さない（tick タスクと同じ守り）。
        if !running.get() {
            debug!("ClickThrough eval loop stopping (message loop has returned)");
            return;
        }
        let (Some(world_rc), Some(monitor_rc)) = (world.upgrade(), monitor.upgrade()) else {
            debug!("ClickThrough eval loop stopping (world/handle dropped — shutdown)");
            return;
        };

        // World 借用に失敗（tick 進行中等）したら当該サイクルを飛ばし、次の起床で評価する。
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
