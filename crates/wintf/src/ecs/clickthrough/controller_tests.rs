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
    let out = resolve_transition(Some(entity(3)), &just_ended(), DesiredState::Transparent);
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

/// テスト用 screen→client 変換の模擬。`world_with_hittable_window` が用いる
/// WindowPos.position=(100,200) を模して `client = cursor - (100,200)` を返す。
///
/// production は `screen_to_client_point`（OS `ScreenToClient`）が実 HWND の実座標で
/// 変換するが、状態機械テストは実 HWND 座標に依存させず決定的な模擬変換で検証する
/// （座標変換の正しさは OS ScreenToClient に委ねる＝4.2 実動検証で確認）。
fn sim_s2c(cursor: PhysicalPoint) -> impl Fn(HWND) -> Option<PointF> {
    move |_hwnd| {
        Some(PointF::new(
            (cursor.x - 100) as f32,
            (cursor.y - 200) as f32,
        ))
    }
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
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(150, 250)));

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
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(0, 0)));

        assert_eq!(
            reg.last_applied(window),
            Some(DesiredState::Transparent),
            "hit=None は Transparent へ収束すべき"
        );
        assert!(
            is_transparent(hwnd),
            "Transparent 適用後 TRANSPARENT はセット"
        );
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
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(150, 250)));
        assert_eq!(reg.last_applied(window), Some(DesiredState::Opaque));
        assert!(!is_transparent(hwnd));

        // 2 回目: 同一 hit・last_applied=Opaque → resolve_transition が None → 無適用・不変。
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(150, 250)));
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
    use crate::ecs::drag::DragState;
    use crate::ecs::drag::{reset_to_idle, snapshot_drag_state, update_drag_state};

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
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(150, 250)));
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

/// 現在の ex-style に LAYERED ビットが立っているか読み戻す。
fn is_layered(hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED as EX_LAYERED;
    let ex = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style") as u32;
    ex & EX_LAYERED.0 != 0
}

/// (e) LAYERED 同伴フラグ: 登録窓は初回評価で `WS_EX_LAYERED` が立ち（pilot 必須条件）、
/// レジストリの `layered_applied` が真へ倒れる。以後の評価で重複適用しない（冪等）。
#[test]
fn eval_applies_layered_companion_on_first_pass() {
    let mut world = World::new();
    let window = world_with_hittable_window(&mut world);
    let hwnd = create_test_hwnd();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert!(!is_layered(hwnd), "seed: LAYERED must start clear");

        let mut reg = ClickThroughRegistry::new();
        reg.register(window, hwnd); // layered_applied = false

        // 初回評価: hit の有無に関わらず同伴フラグが立つ（cursor は hit 位置）。
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(150, 250)));

        assert!(is_layered(hwnd), "初回評価で WS_EX_LAYERED が立つべき");
        assert_eq!(
            reg.iter().next().map(|t| t.layered_applied),
            Some(true),
            "適用成功後に layered_applied が真へ倒れるべき"
        );

        // 2 回目の評価でも LAYERED は保持されたまま（落とさない・冪等）。
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(0, 0)));
        assert!(is_layered(hwnd), "以後の評価でも LAYERED は保持される");
    }));
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

        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(10, 10)));

        // position 未確定ゆえ skip・last_applied は初期 Opaque のまま。
        assert_eq!(reg.last_applied(window), Some(DesiredState::Opaque));
    }));
    destroy_test_hwnd(hwnd);
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

// ========================================================================
// prune_dead_targets（窓破棄追随・R7.2 / Lifecycle）
// ========================================================================

use crate::ecs::window::{Window as WinComp, WindowStyle};
use windows::Win32::UI::WindowsAndMessaging::{WS_EX_LAYERED, WS_POPUP};

/// `Window` コンポーネントを持つ生きた窓 Entity を spawn する（prune テスト用最小構成）。
fn spawn_live_window(world: &mut World) -> Entity {
    world
        .spawn((
            WinComp {
                title: "PruneTest".to_string(),
                parent: None,
            },
            WindowStyle {
                style: WS_POPUP,
                ex_style: WS_EX_LAYERED,
            },
        ))
        .id()
}

/// despawn 済み Entity は prune で除去される（無効 HWND への適用を未然に断つ）。
#[test]
fn prune_removes_despawned_window_entity() {
    let mut world = World::new();
    let live = spawn_live_window(&mut world);
    let dead = spawn_live_window(&mut world);

    let mut reg = ClickThroughRegistry::new();
    reg.register(live, HWND::default());
    reg.register(dead, HWND::default());
    assert_eq!(reg.len(), 2);

    // 1 窓を despawn（破棄相当）。
    world.despawn(dead);

    let removed = prune_dead_targets(&world, &mut reg);
    assert_eq!(removed, 1, "despawn 済み Entity 1 件が除去されるべき");
    assert_eq!(reg.len(), 1);
    assert!(reg.last_applied(live).is_some(), "生存窓は残る");
    assert!(reg.last_applied(dead).is_none(), "破棄窓は除去される");
}

/// `Window` コンポーネントを持たない対象でも、Entity が生存する限り prune は除去しない
/// （prune は Entity 消滅のみを破棄シグナルとする＝eval コアの純粋性・汎用対象も監視可）。
#[test]
fn prune_keeps_live_entity_without_window_component() {
    let mut world = World::new();
    let bare = world.spawn_empty().id(); // Window コンポーネントなし・生きた Entity。

    let mut reg = ClickThroughRegistry::new();
    reg.register(bare, HWND::default());
    assert_eq!(reg.len(), 1);

    let removed = prune_dead_targets(&world, &mut reg);
    assert_eq!(removed, 0, "生存 Entity は Window 有無に関わらず残す");
    assert_eq!(reg.len(), 1);
}

/// 破棄窓（despawn 済み Entity）は `evaluate_targets` の巡回対象から外れ、
/// `apply_click_through` が撃たれない。実 HWND を握って seed し、despawn 後の評価で
/// ex-style が **変化しない**ことを確認する（破棄窓へ適用が走れば seed 状態が動くため、
/// 不変 = 適用スキップの観測）。
#[test]
fn eval_skips_apply_for_destroyed_window() {
    let mut world = World::new();
    let window = world_with_hittable_window(&mut world);
    let hwnd = create_test_hwnd();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // seed: TRANSPARENT を立て last_applied=Transparent。
        apply_click_through(hwnd, true).expect("seed transparent");
        assert!(is_transparent(hwnd), "seed: TRANSPARENT set");

        let mut reg = ClickThroughRegistry::new();
        reg.register(window, hwnd);
        reg.set_last_applied(window, DesiredState::Transparent);

        // 窓破棄相当: Entity を despawn（areka の close パスと同じ正準シグナル）。
        world.despawn(window);

        // 破棄後の評価: prune で対象から外れ、hit=Some でも apply が走らない。
        // cursor screen (150,250) は本来 hit=Some → Opaque へ動くはずだが、prune 済みゆえ不変。
        evaluate_targets(&world, &mut reg, sim_s2c(PhysicalPoint::new(150, 250)));

        assert!(reg.is_empty(), "破棄窓は評価前 prune で除去される");
        assert!(
            is_transparent(hwnd),
            "破棄窓へは apply が走らず ex-style は seed のまま不変であるべき"
        );
    }));
    destroy_test_hwnd(hwnd);
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

// ========================================================================
// ClickThroughRegistryHandle（areka 登録面・NonSend リソース）
// ========================================================================

/// 公開ハンドル経由の register/remove が共有レジストリへ反映される（areka 4.1 の登録面）。
#[test]
fn registry_handle_register_remove_reflects_shared_registry() {
    let registry = StdRc::new(StdRefCell::new(ClickThroughRegistry::new()));
    let handle = ClickThroughRegistryHandle::new(StdRc::clone(&registry));

    let mut w = World::new();
    let e = w.spawn_empty().id();
    let hwnd = create_test_hwnd();

    assert!(handle.is_empty());
    handle.register(e, hwnd);
    assert_eq!(handle.len(), 1);
    assert_eq!(registry.borrow().len(), 1, "共有レジストリへ反映される");

    assert!(handle.remove(e));
    assert!(handle.is_empty());
    assert!(registry.borrow().is_empty());
    destroy_test_hwnd(hwnd);
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
    assert_eq!(
        registry.borrow().len(),
        1,
        "start 後の register が反映される"
    );
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
