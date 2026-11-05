/// Layout-to-Graphics同期システムのテスト
///
/// このテストは、レイアウト計算結果が正しくグラフィックスコンポーネントに
/// 伝播することを検証します。
use bevy_ecs::prelude::*;
use bevy_ecs::system::IntoSystem;
use wintf::ecs::*;
use wintf::ecs::{Point, SizeI};

// sync_visual_from_layout_rootは廃止され、Arrangementから直接Surfaceサイズを取得するようになりました。
// Visual.sizeは削除され、Arrangement.sizeがSingle Source of Truthです。

#[test]
fn test_sync_window_pos() {
    use bevy_ecs::schedule::Schedule;

    // WorldとScheduleを準備
    let mut world = World::new();
    world.insert_resource(wintf::ecs::world::FrameCount(1)); // window_pos_sync_systemに必要
    let mut schedule = Schedule::default();
    schedule.add_systems(wintf::ecs::window_pos_sync_system);

    // Window、Visual、WindowPosを持つエンティティを作成
    // Note: Visual::on_addがArrangement::default()を自動挿入し、
    //       Arrangement::on_addがGlobalArrangement::default()を自動挿入する
    let entity = world
        .spawn((Window::default(), Visual::default(), WindowPos::default()))
        .id();

    // Commands適用のためにflush
    world.flush();

    // 自動挿入されたArrangementとGlobalArrangementを上書き
    world.entity_mut(entity).insert((
        layout::Arrangement {
            offset: layout::Offset { x: 100.0, y: 50.0 },
            scale: layout::LayoutScale::default(),
            size: layout::Size {
                width: 800.0,
                height: 600.0,
            },
        },
        layout::GlobalArrangement {
            transform: windows_numerics::Matrix3x2::identity(),
            bounds: layout::D2DRect {
                left: 100.0,
                top: 50.0,
                right: 900.0,
                bottom: 650.0,
            },
        },
    ));

    // スケジュールを実行（GlobalArrangementがChangedとして検出される）
    schedule.run(&mut world);

    // WindowPosが更新されていることを確認
    let window_pos = world.entity(entity).get::<WindowPos>().unwrap();
    assert_eq!(window_pos.position, Some(Point { x: 100, y: 50 }));
    assert_eq!(
        window_pos.size,
        Some(SizeI {
            width: 800,
            height: 600
        })
    );
}

#[test]
fn test_is_self_initiated_default_false() {
    // TLS フラグの初期値は false であること
    assert!(
        !wintf::ecs::is_self_initiated(),
        "is_self_initiated() の初期値は false であるべき"
    );
}

#[test]
fn test_skip_invalid_bounds() {
    // WorldとScheduleを準備（FrameCount が無いと引数の検査で system が走らない）
    let mut world = World::new();
    world.insert_resource(wintf::ecs::world::FrameCount(1));

    // 無効なbounds (0,0,0,0)を持つエンティティを作成
    let entity = world
        .spawn((
            Window::default(),
            layout::LayoutRoot,
            layout::GlobalArrangement {
                transform: windows_numerics::Matrix3x2::identity(),
                bounds: layout::D2DRect {
                    left: 0.0,
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                },
            },
            Visual::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 100 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
        ))
        .id();

    // window_pos_sync_systemシステムを実行
    let mut system = IntoSystem::into_system(wintf::ecs::window_pos_sync_system);
    system.initialize(&mut world);
    system.run((), &mut world).unwrap();
    system.apply_deferred(&mut world);

    // 無効なboundsの場合、WindowPosは更新されないべき
    let window_pos = world.entity(entity).get::<WindowPos>().unwrap();
    assert_eq!(window_pos.position, Some(Point { x: 100, y: 100 }));
    assert_eq!(
        window_pos.size,
        Some(SizeI {
            width: 800,
            height: 600
        })
    );
}
