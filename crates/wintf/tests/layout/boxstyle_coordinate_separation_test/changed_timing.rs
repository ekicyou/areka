//! Task 7.2: Changed<BoxStyle> 発火タイミングテスト

use bevy_ecs::prelude::*;
use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension, sync_window_arrangement_from_window_pos};
use wintf::ecs::window::{DPI, Window, WindowPos};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{Point, SizeI};
use wintf::ecs::{Arrangement, GlobalArrangement};

/// 位置のみ変更時に Changed<BoxStyle> が発火しないことを検証
///
/// WindowPos.position の変更は sync_window_arrangement_from_window_pos 経由で
/// Arrangement.offset を更新するが、BoxStyle には触らない。
#[test]
fn test_changed_boxstyle_not_fired_on_position_only_change() {
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
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(800.0)),
                    height: Some(Dimension::Px(600.0)),
                }),
                ..Default::default()
            },
            Arrangement::default(),
            GlobalArrangement::default(),
        ))
        .id();

    // Changed<BoxStyle> 検知用システム
    fn detect_boxstyle_change(
        query: Query<Entity, Changed<BoxStyle>>,
        mut changed_count: ResMut<ChangedBoxStyleCount>,
    ) {
        for _entity in query.iter() {
            changed_count.0 += 1;
        }
    }

    #[derive(Resource, Default)]
    struct ChangedBoxStyleCount(u32);

    world.insert_resource(ChangedBoxStyleCount::default());

    let mut schedule = Schedule::default();
    schedule.add_systems((
        sync_window_arrangement_from_window_pos,
        detect_boxstyle_change.after(sync_window_arrangement_from_window_pos),
    ));

    // 初回実行: Added 扱いで Changed<BoxStyle> が 1回発火
    schedule.run(&mut world);
    let initial_count = world.resource::<ChangedBoxStyleCount>().0;

    // WindowPos.position のみ変更（BoxStyle は触らない）
    world.get_mut::<WindowPos>(entity).unwrap().position = Some(Point { x: 500, y: 300 });
    schedule.run(&mut world);

    let after_pos_change = world.resource::<ChangedBoxStyleCount>().0;
    assert_eq!(
        initial_count, after_pos_change,
        "位置のみ変更で Changed<BoxStyle> が発火してはいけない"
    );
}

/// サイズ変更時に Changed<BoxStyle> が発火することを検証
#[test]
fn test_changed_boxstyle_fired_on_size_change() {
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
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(800.0)),
                    height: Some(Dimension::Px(600.0)),
                }),
                ..Default::default()
            },
            Arrangement::default(),
            GlobalArrangement::default(),
        ))
        .id();

    #[derive(Resource, Default)]
    struct ChangedBoxStyleCount2(u32);

    fn detect_boxstyle_change2(
        query: Query<Entity, Changed<BoxStyle>>,
        mut changed_count: ResMut<ChangedBoxStyleCount2>,
    ) {
        for _entity in query.iter() {
            changed_count.0 += 1;
        }
    }

    world.insert_resource(ChangedBoxStyleCount2::default());

    let mut schedule = Schedule::default();
    schedule.add_systems(detect_boxstyle_change2);

    // 初回実行（Added → Changed 発火）
    schedule.run(&mut world);
    let count_after_first = world.resource::<ChangedBoxStyleCount2>().0;
    assert_eq!(count_after_first, 1, "初回は Added で発火");

    // 2回目: 変更なし → 発火しない
    schedule.run(&mut world);
    let count_after_second = world.resource::<ChangedBoxStyleCount2>().0;
    assert_eq!(count_after_second, 1, "変更なしで発火してはいけない");

    // BoxStyle.size を変更 → Changed<BoxStyle> 発火
    world.get_mut::<BoxStyle>(entity).unwrap().size = Some(BoxSize {
        width: Some(Dimension::Px(600.0)),
        height: Some(Dimension::Px(400.0)),
    });

    schedule.run(&mut world);
    let count_after_size = world.resource::<ChangedBoxStyleCount2>().0;
    assert_eq!(
        count_after_size, 2,
        "サイズ変更後に Changed<BoxStyle> が発火すること"
    );
}
