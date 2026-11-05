/// フィードバックループ収束検証テスト
///
/// R4-AC1: ユーザーがウィンドウをドラッグ移動した場合、
/// WindowPos → Arrangement → GlobalArrangement → WindowPos の更新連鎖が
/// 1フレーム内で収束し、次フレームで同一値の再更新が発生しないこと
///
/// R1-AC1: Changed<WindowPos> フィルタにより WindowPos が変更された Window のみ処理すること
///
/// 同じ値の書き直しは値の比較では見えないので、変更の刻み（`get_change_ticks`）を比べる。
use bevy_ecs::change_detection::Tick;
use bevy_ecs::prelude::*;
use wintf::ecs::layout::{
    BoxStyle, mark_dirty_arrangement_trees, propagate_global_arrangements,
    sync_simple_arrangements, sync_window_arrangement_from_window_pos, window_pos_sync_system,
};
use wintf::ecs::window::{DPI, Window, WindowPos};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{Arrangement, GlobalArrangement, LayoutScale, Offset, Size};
use wintf::ecs::{Point, SizeI};

/// 本番の PostLayout（`src/ecs/world/mod.rs` の `.chain()`）と同じ並びのスケジュール。
fn post_layout_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            sync_window_arrangement_from_window_pos,
            sync_simple_arrangements,
            mark_dirty_arrangement_trees,
            propagate_global_arrangements,
            window_pos_sync_system,
        )
            .chain(),
    );
    schedule
}

/// `FrameCount` を入れた World。
fn new_world() -> World {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    world
}

/// 論理 800x600 の Window を作る。物理の大きさと `Arrangement.scale` は DPI から決める。
fn spawn_window(world: &mut World, position: Option<Point>, dpi: u32, offset: Offset) -> Entity {
    let scale = dpi as f32 / 96.0;
    world
        .spawn((
            Window::default(),
            WindowPos {
                position,
                size: Some(SizeI {
                    width: (800.0 * scale) as i32,
                    height: (600.0 * scale) as i32,
                }),
                ..Default::default()
            },
            DPI {
                dpi_x: dpi as u16,
                dpi_y: dpi as u16,
            },
            Arrangement {
                offset,
                scale: LayoutScale { x: scale, y: scale },
                size: Size {
                    width: 800.0,
                    height: 600.0,
                },
            },
            GlobalArrangement::default(),
        ))
        .id()
}

/// 部品 `T` が最後に変更の印を受けた刻み。
fn changed_tick<T: Component>(world: &World, e: Entity) -> Tick {
    world.entity(e).get_change_ticks::<T>().unwrap().changed
}

// =============================================================================
// Task 1.1: Changed<WindowPos> フィルタの検証
// =============================================================================

/// sync_window_arrangement_from_window_pos が Changed<WindowPos> フィルタを持ち、
/// WindowPos が変更されたエンティティのみ処理することを検証する
#[test]
fn test_sync_window_arrangement_only_processes_changed_window_pos() {
    let mut world = new_world();
    let entity = spawn_window(
        &mut world,
        Some(Point { x: 100, y: 200 }),
        96,
        Offset { x: 0.0, y: 0.0 },
    );
    world.entity_mut(entity).insert(BoxStyle::default());
    let mut schedule = post_layout_schedule();

    // --- 初回実行: WindowPos は "新規追加" なので Changed 扱い ---
    schedule.run(&mut world);

    // Arrangement.offset が WindowPos.position / scale = (100, 200) / 1.0 に更新されていること
    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arrangement.offset.x, 100.0,
        "初回: offset.x should be 100.0"
    );
    assert_eq!(
        arrangement.offset.y, 200.0,
        "初回: offset.y should be 200.0"
    );

    // --- 2回目実行: WindowPos 未変更 → Arrangement に書かない（刻みが動かない） ---
    let tick_before = changed_tick::<Arrangement>(&world, entity);
    schedule.run(&mut world);
    assert_eq!(
        tick_before,
        changed_tick::<Arrangement>(&world, entity),
        "2回目: WindowPos未変更のためArrangementに書かないこと"
    );

    // --- 濾し: WindowPos を変えずに Arrangement を別の値にすると、その値が残る ---
    world.get_mut::<Arrangement>(entity).unwrap().offset = Offset { x: 7.0, y: 8.0 };
    schedule.run(&mut world);
    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arrangement.offset,
        Offset { x: 7.0, y: 8.0 },
        "WindowPos未変更なら Arrangement.offset を WindowPos から書き戻さないこと"
    );

    // --- WindowPos を変更 → Changed<WindowPos> 発火 → 処理される ---
    {
        let mut wp = world.get_mut::<WindowPos>(entity).unwrap();
        wp.position = Some(Point { x: 300, y: 400 });
    }
    schedule.run(&mut world);

    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arrangement.offset.x, 300.0,
        "3回目: offset.x should be 300.0"
    );
    assert_eq!(
        arrangement.offset.y, 400.0,
        "3回目: offset.y should be 400.0"
    );

    // BoxStyle.inset は位置の同期で触られない（None のまま）
    assert!(
        world.get::<BoxStyle>(entity).unwrap().inset.is_none(),
        "BoxStyle.inset は位置の同期の後も None であるべき"
    );
}

// =============================================================================
// Task 1.1 + R2: DPI 変換の正確性検証
// =============================================================================

/// DPI 192 (200%) 環境で WindowPos.position がそのまま Arrangement.offset に設定されること
/// Window は LayoutRoot (scale=1.0) の子なので offset = position（DPI除算なし）
#[test]
fn test_sync_window_arrangement_dpi_192_no_division() {
    let mut world = new_world();
    let entity = spawn_window(
        &mut world,
        Some(Point { x: 200, y: 400 }),
        192,
        Offset { x: 0.0, y: 0.0 },
    );

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);

    schedule.run(&mut world);

    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arrangement.offset.x, 200.0,
        "DPI 192: offset.x = 200.0 (物理px、レイアウト入力と同じ)"
    );
    assert_eq!(
        arrangement.offset.y, 400.0,
        "DPI 192: offset.y = 400.0 (物理px、レイアウト入力と同じ)"
    );
}

// =============================================================================
// Task 1.1 + R3: エッジケース検証
// =============================================================================

/// WindowPos.position が None の場合、Arrangement を更新しないこと
#[test]
fn test_sync_window_arrangement_skips_none_position() {
    let mut world = new_world();
    let entity = spawn_window(&mut world, None, 96, Offset { x: 50.0, y: 60.0 });

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);

    schedule.run(&mut world);

    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arrangement.offset.x, 50.0,
        "None position: offset unchanged"
    );
    assert_eq!(
        arrangement.offset.y, 60.0,
        "None position: offset unchanged"
    );
}

/// WindowPos.position が CW_USEDEFAULT の場合、Arrangement を更新しないこと
#[test]
fn test_sync_window_arrangement_skips_cw_usedefault() {
    use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;

    let mut world = new_world();
    let entity = spawn_window(
        &mut world,
        Some(Point {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
        }),
        96,
        Offset { x: 50.0, y: 60.0 },
    );

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);

    schedule.run(&mut world);

    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arrangement.offset.x, 50.0,
        "CW_USEDEFAULT: offset unchanged"
    );
    assert_eq!(
        arrangement.offset.y, 60.0,
        "CW_USEDEFAULT: offset unchanged"
    );
}

/// 等値チェック: Arrangement.offset が変換後の値と同一の場合、書かないこと（刻みが動かない）
#[test]
fn test_sync_window_arrangement_skips_equal_offset() {
    let mut world = new_world();
    // 事前にオフセットが既に正しい値に設定されている
    let entity = spawn_window(
        &mut world,
        Some(Point { x: 100, y: 200 }),
        96,
        Offset { x: 100.0, y: 200.0 },
    );

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);

    // 初回: Added 扱いだが等値なので Arrangement は DerefMut されない
    // 最初の system は生成と同じ刻みで走るので、刻みを 1 つ進めて書き込みを見分ける
    world.increment_change_tick();
    let tick_before = changed_tick::<Arrangement>(&world, entity);
    schedule.run(&mut world);

    assert_eq!(
        tick_before,
        changed_tick::<Arrangement>(&world, entity),
        "等値なら Arrangement に書かないこと"
    );
    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(arrangement.offset.x, 100.0);
    assert_eq!(arrangement.offset.y, 200.0);
}

// =============================================================================
// Task 3.1: フィードバックループ収束検証（R4-AC1）
// =============================================================================

/// 3 つの部品の変更の刻み。
fn ticks(world: &World, e: Entity) -> (Tick, Tick, Tick) {
    (
        changed_tick::<Arrangement>(world, e),
        changed_tick::<GlobalArrangement>(world, e),
        changed_tick::<WindowPos>(world, e),
    )
}

/// ユーザーがウィンドウを移動した場合の収束テスト
/// WindowPos 更新 → sync_window_arrangement → propagate → window_pos_sync の連鎖で
/// 1フレーム内で収束し、次フレームで再更新が発生しないこと
#[test]
fn test_feedback_loop_converges_in_one_frame_dpi_96() {
    let mut world = new_world();
    let entity = spawn_window(
        &mut world,
        Some(Point { x: 0, y: 0 }),
        96,
        Offset { x: 0.0, y: 0.0 },
    );
    let mut schedule = post_layout_schedule();

    // 初回実行で初期化（Added による初期処理）
    schedule.run(&mut world);

    // --- ユーザーがウィンドウをドラッグ: position を変更 ---
    {
        let mut wp = world.get_mut::<WindowPos>(entity).unwrap();
        wp.position = Some(Point { x: 500, y: 300 });
    }
    let wp_tick_moved = changed_tick::<WindowPos>(&world, entity);

    // フレーム1: 変更が伝播
    schedule.run(&mut world);

    let arr1 = *world.get::<Arrangement>(entity).unwrap();
    assert_eq!(arr1.offset.x, 500.0, "Frame1: Arrangement.offset.x = 500.0");
    assert_eq!(arr1.offset.y, 300.0, "Frame1: Arrangement.offset.y = 300.0");
    assert_eq!(
        wp_tick_moved,
        changed_tick::<WindowPos>(&world, entity),
        "Frame1: 同じ位置を WindowPos へ書き戻さないこと"
    );

    // フレーム2: 収束確認（どの部品にも書かない）
    let before = ticks(&world, entity);
    schedule.run(&mut world);
    assert_eq!(
        before,
        ticks(&world, entity),
        "Frame2: Arrangement/GlobalArrangement/WindowPos に書かないこと (converged)"
    );
}

/// DPI 192 (scale=2.0) でのフィードバックループ収束テスト
/// 実アプリと同じ構造: Window は LayoutRoot (scale=1.0) の子エンティティ
/// ChildOf(LayoutRoot) により propagate_global_arrangements で bounds 計算される
/// bounds.left = parent.bounds.left + offset × parent_scale = offset × 1.0 = offset
#[test]
fn test_feedback_loop_converges_dpi_192() {
    use bevy_ecs::hierarchy::ChildOf;
    use wintf::ecs::layout::LayoutRoot;

    let mut world = new_world();

    // LayoutRoot (scale=1.0) — 仮想デスクトップ原点
    let layout_root = world
        .spawn((
            LayoutRoot,
            Arrangement {
                offset: Offset { x: 0.0, y: 0.0 },
                scale: LayoutScale { x: 1.0, y: 1.0 },
                size: Size {
                    width: 3840.0,
                    height: 2160.0,
                },
            },
            GlobalArrangement::default(),
        ))
        .id();

    // Window entity — LayoutRoot の子
    let entity = spawn_window(
        &mut world,
        Some(Point { x: 0, y: 0 }),
        192,
        Offset { x: 0.0, y: 0.0 },
    );
    world.entity_mut(entity).insert(ChildOf(layout_root));
    let mut schedule = post_layout_schedule();

    // 初回実行
    schedule.run(&mut world);

    // ユーザーがウィンドウをドラッグ: (200, 400) 物理px
    {
        let mut wp = world.get_mut::<WindowPos>(entity).unwrap();
        wp.position = Some(Point { x: 200, y: 400 });
    }
    let wp_tick_moved = changed_tick::<WindowPos>(&world, entity);

    // フレーム1
    schedule.run(&mut world);

    let arr1 = *world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arr1.offset.x, 200.0,
        "DPI 192: offset.x = 200.0 (物理px、LayoutRoot子なのでDPI除算なし)"
    );
    assert_eq!(
        arr1.offset.y, 400.0,
        "DPI 192: offset.y = 400.0 (物理px、LayoutRoot子なのでDPI除算なし)"
    );
    assert_eq!(
        wp_tick_moved,
        changed_tick::<WindowPos>(&world, entity),
        "DPI 192: 同じ位置を WindowPos へ書き戻さないこと"
    );

    // フレーム2: 収束確認
    let before = ticks(&world, entity);
    schedule.run(&mut world);
    assert_eq!(
        before,
        ticks(&world, entity),
        "DPI 192: Frame2 でどの部品にも書かないこと (converged in one frame)"
    );
}
