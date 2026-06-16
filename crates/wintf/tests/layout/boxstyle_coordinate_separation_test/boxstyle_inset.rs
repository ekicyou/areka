//! Task 7.1: BoxStyle.inset 不変性テスト

use bevy_ecs::prelude::*;
use wintf::ecs::layout::{BoxStyle, sync_window_arrangement_from_window_pos};
use wintf::ecs::window::{DPI, Window, WindowPos};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{Point, SizeI};
use wintf::ecs::{Arrangement, GlobalArrangement, LayoutScale, Offset, Size};

/// WM_WINDOWPOSCHANGED 相当の処理後（PostLayoutパイプライン経由）で
/// Window entity の BoxStyle.inset が変更されていないことを検証
#[test]
fn test_boxstyle_inset_unchanged_after_window_pos_update() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));

    // Window entity を作成（BoxStyle.inset は None のまま）
    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI { width: 800, height: 600 }),
                ..Default::default()
            },
            DPI::default(),
            BoxStyle::default(),
            Arrangement {
                offset: Offset { x: 0.0, y: 0.0 },
                scale: LayoutScale { x: 1.0, y: 1.0 },
                size: Size {
                    width: 800.0,
                    height: 600.0,
                },
            },
            GlobalArrangement::default(),
        ))
        .id();

    // PostLayout スケジュール（sync_window_arrangement_from_window_pos）を実行
    let mut schedule = Schedule::default();
    schedule.add_systems(sync_window_arrangement_from_window_pos);
    schedule.run(&mut world);

    // BoxStyle.inset は変更されていないこと（None のまま）
    let box_style = world.get::<BoxStyle>(entity).unwrap();
    assert!(
        box_style.inset.is_none(),
        "BoxStyle.inset は sync_window_arrangement_from_window_pos 後も None であるべき"
    );

    // Arrangement.offset は WindowPos.position に追従していること
    let arr = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(arr.offset.x, 100.0);
    assert_eq!(arr.offset.y, 200.0);
}

/// WindowPos.position を変更してもBoxStyle.insetが影響を受けないことを検証
#[test]
fn test_boxstyle_inset_unaffected_by_window_position_change() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));

    let entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI { width: 800, height: 600 }),
                ..Default::default()
            },
            DPI::default(),
            BoxStyle::default(),
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

    // 初回実行（Added 扱い）
    schedule.run(&mut world);

    // WindowPos.position を変更（ウィンドウ移動を模擬）
    world.get_mut::<WindowPos>(entity).unwrap().position = Some(Point { x: 500, y: 300 });
    schedule.run(&mut world);

    // BoxStyle.inset は変更されていないこと
    let box_style = world.get::<BoxStyle>(entity).unwrap();
    assert!(
        box_style.inset.is_none(),
        "位置変更後も BoxStyle.inset は None であるべき"
    );

    // Arrangement.offset は新しい位置に更新されること
    let arr = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(arr.offset.x, 500.0);
    assert_eq!(arr.offset.y, 300.0);
}
