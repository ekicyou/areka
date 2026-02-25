//! WindowDragging フィルタ統合テスト
//!
//! Task 5.3: `WindowDragging` マーカーにより
//! ドラッグ中のウィンドウが ECS レイアウトパイプラインをバイパスすることを検証する。
//!
//! - `window_pos_sync_system`: ドラッグ中はサイズのみ更新（位置はスキップ）
//! - `sync_window_arrangement_from_window_pos`: ドラッグ中は完全スキップ（位置同期のみのため）

use bevy_ecs::prelude::*;
use wintf::ecs::drag::WindowDragging;
use wintf::ecs::layout::systems::window_pos_sync_system;
use wintf::ecs::layout::{Arrangement, GlobalArrangement, sync_window_arrangement_from_window_pos};
use wintf::ecs::window::{DPI, Window, WindowPos};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{LayoutScale, Offset, Point, Size, SizeI};

/// ヘルパー: テスト用 World を基本リソース付きで生成
fn make_world() -> World {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    world
}

// =============================================================================
// window_pos_sync_system: ドラッグ中はサイズのみ更新テスト
// =============================================================================

/// WindowDragging 付きエンティティは window_pos_sync_system で位置はスキップ、サイズは更新される
#[test]
fn window_pos_sync_updates_size_only_for_dragging_entity() {
    let mut world = make_world();

    // Window entity（ドラッグ中）: GlobalArrangement が変更されたら
    // サイズは更新されるが、位置は元のまま
    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement {
                bounds: wintf::ecs::layout::D2DRect {
                    left: 500.0,
                    top: 600.0,
                    right: 1300.0,
                    bottom: 1200.0,
                },
                ..Default::default()
            },
            WindowDragging, // ← ドラッグ中マーカー
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(window_pos_sync_system);
    schedule.run(&mut world);

    // 位置は元の値のまま変更されていないこと
    let wp = world.get::<WindowPos>(entity).unwrap();
    assert_eq!(
        wp.position,
        Some(Point { x: 100, y: 200 }),
        "ドラッグ中の entity は位置をスキップすべき"
    );
    // サイズは GlobalArrangement から更新されること（DPI変更対応）
    assert_eq!(
        wp.size,
        Some(SizeI {
            width: 800,
            height: 600
        }),
        "ドラッグ中の entity はサイズを更新すべき"
    );
}

/// WindowDragging なしエンティティは window_pos_sync_system で通常更新される
#[test]
fn window_pos_sync_processes_non_dragging_entity() {
    let mut world = make_world();

    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement::default(),
            // WindowDragging なし
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(window_pos_sync_system);

    // 1回目: Added → Changed として検知されるが bounds がゼロなのでスキップ
    schedule.run(&mut world);

    // GlobalArrangement を有効な値に更新 → Changed が発火
    world.get_mut::<GlobalArrangement>(entity).unwrap().bounds = wintf::ecs::layout::D2DRect {
        left: 500.0,
        top: 600.0,
        right: 1300.0,
        bottom: 1200.0,
    };

    // 2回目: Changed<GlobalArrangement> で WindowPos を更新
    schedule.run(&mut world);

    // WindowPos が GlobalArrangement.bounds から更新されること
    let wp = world.get::<WindowPos>(entity).unwrap();
    assert_eq!(
        wp.position,
        Some(Point { x: 500, y: 600 }),
        "非ドラッグ entity は GlobalArrangement から WindowPos を更新すべき"
    );
    assert_eq!(
        wp.size,
        Some(SizeI {
            width: 800,
            height: 600
        }),
        "非ドラッグ entity は GlobalArrangement から WindowPos.size を更新すべき"
    );
}

/// WindowDragging 除去後に window_pos_sync_system が位置更新も再開すること
#[test]
fn window_pos_sync_resumes_after_dragging_removed() {
    let mut world = make_world();

    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement {
                bounds: wintf::ecs::layout::D2DRect {
                    left: 500.0,
                    top: 600.0,
                    right: 1300.0,
                    bottom: 1200.0,
                },
                ..Default::default()
            },
            WindowDragging,
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(window_pos_sync_system);

    // 1回目: WindowDragging あり → 位置スキップ、サイズは更新
    schedule.run(&mut world);
    let wp = world.get::<WindowPos>(entity).unwrap();
    assert_eq!(
        wp.position,
        Some(Point { x: 100, y: 200 }),
        "ドラッグ中は位置がスキップされるべき"
    );
    assert_eq!(
        wp.size,
        Some(SizeI {
            width: 800,
            height: 600
        }),
        "ドラッグ中でもサイズは更新されるべき"
    );

    // WindowDragging を除去
    world.entity_mut(entity).remove::<WindowDragging>();
    // GlobalArrangement を変更して Changed トリガーを再発火
    world.get_mut::<GlobalArrangement>(entity).unwrap().bounds = wintf::ecs::layout::D2DRect {
        left: 700.0,
        top: 800.0,
        right: 1500.0,
        bottom: 1400.0,
    };

    // 2回目: WindowDragging なし → 位置・サイズ両方更新される
    schedule.run(&mut world);
    let wp = world.get::<WindowPos>(entity).unwrap();
    assert_eq!(
        wp.position,
        Some(Point { x: 700, y: 800 }),
        "WindowDragging 除去後は位置も更新すべき"
    );
    assert_eq!(
        wp.size,
        Some(SizeI {
            width: 800,
            height: 600
        }),
        "WindowDragging 除去後はサイズも更新すべき"
    );
}

/// ドラッグ中に DPI が変わっても GlobalArrangement のサイズ変更が WindowPos に反映されること
#[test]
fn window_pos_sync_propagates_size_change_during_drag() {
    let mut world = make_world();

    // 初期状態: DPI 200% → 400x300 ウィンドウ
    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI {
                    width: 400,
                    height: 300,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement {
                bounds: wintf::ecs::layout::D2DRect {
                    left: 100.0,
                    top: 200.0,
                    right: 500.0,
                    bottom: 500.0,
                },
                ..Default::default()
            },
            WindowDragging, // ドラッグ中
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(window_pos_sync_system);

    // 1回目: Added → Changed で初回処理（サイズ同じなので変化なし）
    schedule.run(&mut world);
    let wp = world.get::<WindowPos>(entity).unwrap();
    assert_eq!(
        wp.size,
        Some(SizeI {
            width: 400,
            height: 300
        })
    );
    assert_eq!(wp.position, Some(Point { x: 100, y: 200 }));

    // DPI 125% に変更 → レイアウト再計算でサイズが小さくなる (250x187)
    world.get_mut::<GlobalArrangement>(entity).unwrap().bounds = wintf::ecs::layout::D2DRect {
        left: 100.0, // 位置はドラッグ中のカーソル位置
        top: 200.0,
        right: 350.0,  // 幅 250
        bottom: 387.0, // 高さ 187
    };

    // 2回目: Changed<GlobalArrangement> → サイズのみ更新
    schedule.run(&mut world);
    let wp = world.get::<WindowPos>(entity).unwrap();
    assert_eq!(
        wp.position,
        Some(Point { x: 100, y: 200 }),
        "ドラッグ中は位置を変更しない"
    );
    assert_eq!(
        wp.size,
        Some(SizeI {
            width: 250,
            height: 187
        }),
        "ドラッグ中でも DPI 変更によるサイズ変更は反映すべき"
    );
}

// =============================================================================
// sync_window_arrangement_from_window_pos: Without<WindowDragging> テスト
// =============================================================================

/// WindowDragging 付きエンティティは sync_window_arrangement_from_window_pos でスキップ
#[test]
fn sync_arrangement_skips_dragging_entity() {
    let mut world = make_world();

    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 500, y: 600 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement {
                offset: Offset { x: 100.0, y: 200.0 },
                scale: LayoutScale { x: 1.0, y: 1.0 },
                size: Size {
                    width: 800.0,
                    height: 600.0,
                },
            },
            GlobalArrangement::default(),
            WindowDragging, // ドラッグ中
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);
    schedule.run(&mut world);

    // Arrangement.offset は元の値のまま
    let arr = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arr.offset,
        Offset { x: 100.0, y: 200.0 },
        "ドラッグ中の entity は sync_window_arrangement でスキップされるべき"
    );
}

/// WindowDragging なしエンティティは sync_window_arrangement_from_window_pos で更新される
#[test]
fn sync_arrangement_processes_non_dragging_entity() {
    let mut world = make_world();

    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 500, y: 600 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement {
                offset: Offset { x: 100.0, y: 200.0 },
                scale: LayoutScale { x: 1.0, y: 1.0 },
                size: Size {
                    width: 800.0,
                    height: 600.0,
                },
            },
            GlobalArrangement::default(),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);
    schedule.run(&mut world);

    // Arrangement.offset が WindowPos.position に同期されること
    let arr = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(
        arr.offset,
        Offset { x: 500.0, y: 600.0 },
        "非ドラッグ entity の Arrangement.offset は WindowPos.position に同期されるべき"
    );
}

/// 複数エンティティ: ドラッグ中はスキップ、非ドラッグは更新
#[test]
fn mixed_entities_only_non_dragging_updated() {
    let mut world = make_world();

    // エンティティ A: ドラッグ中（WindowDragging あり）
    let entity_a = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 500, y: 600 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement {
                offset: Offset { x: 0.0, y: 0.0 },
                scale: LayoutScale { x: 1.0, y: 1.0 },
                size: Size {
                    width: 800.0,
                    height: 600.0,
                },
            },
            GlobalArrangement::default(),
            WindowDragging,
        ))
        .id();

    // エンティティ B: 非ドラッグ
    let entity_b = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 300, y: 400 }),
                size: Some(SizeI {
                    width: 640,
                    height: 480,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement {
                offset: Offset { x: 0.0, y: 0.0 },
                scale: LayoutScale { x: 1.0, y: 1.0 },
                size: Size {
                    width: 640.0,
                    height: 480.0,
                },
            },
            GlobalArrangement::default(),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);
    schedule.run(&mut world);

    // A: スキップ → offset 変更なし
    let arr_a = world.get::<Arrangement>(entity_a).unwrap();
    assert_eq!(
        arr_a.offset,
        Offset { x: 0.0, y: 0.0 },
        "ドラッグ中エンティティ A の offset は変更されないべき"
    );

    // B: 更新 → offset = WindowPos.position
    let arr_b = world.get::<Arrangement>(entity_b).unwrap();
    assert_eq!(
        arr_b.offset,
        Offset { x: 300.0, y: 400.0 },
        "非ドラッグエンティティ B の offset は WindowPos.position に同期されるべき"
    );
}
