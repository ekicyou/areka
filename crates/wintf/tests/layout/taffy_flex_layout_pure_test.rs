/// Taffy Flexレイアウトの純粋なテスト（WindowやRectangle要素を排除）
///
/// このテストは以下を検証します:
/// 1. Taffyレイアウト計算が正しく動作すること
/// 2. ArrangementとGlobalArrangementが正しく計算されること
use bevy_ecs::prelude::*;
use wintf::ecs::ChildOf;
use wintf::ecs::layout::*;
use wintf::ecs::world::EcsWorld;

/// grow 1 の item2 の幅（100 + 400 × 1/3 を taffy が丸めた値）
const ITEM2_W: f32 = 233.0;

#[test]
fn test_taffy_flex_layout_pure() {
    // EcsWorldを作成（デフォルトのシステムスケジュールが登録済み）
    let mut ecs_world = EcsWorld::new();

    // 1. 初期エンティティツリーを構築（taffy_flex_demoと同じ構造）
    let (root, flex_container, item1, item2, item3) = {
        let world = ecs_world.world_mut();

        // ルートエンティティ (Window相当、800x600)
        let root = world
            .spawn((
                LayoutRoot,
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(800.0)),
                        height: Some(Dimension::Px(600.0)),
                    }),
                    ..Default::default()
                },
                Arrangement::default(),
            ))
            .id();

        // FlexContainer (横並び、100%x100%)
        let flex_container = world
            .spawn((
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Percent(100.0)),
                        height: Some(Dimension::Percent(100.0)),
                    }),
                    flex_direction: Some(FlexDirection::Row),
                    justify_content: Some(JustifyContent::SPACE_EVENLY),
                    align_items: Some(AlignItems::CENTER),
                    ..Default::default()
                },
                Arrangement::default(),
                ChildOf(root),
            ))
            .id();
        // Flexアイテム1（赤相当、固定200px幅）
        let item1 = world
            .spawn((
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(200.0)),
                        height: Some(Dimension::Px(150.0)),
                    }),
                    flex_grow: Some(0.0),
                    flex_shrink: Some(0.0),
                    flex_basis: Some(Dimension::Px(200.0)),
                    ..Default::default()
                },
                Arrangement::default(),
                ChildOf(flex_container),
            ))
            .id();

        // Flexアイテム2（緑相当、grow=1）
        let item2 = world
            .spawn((
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(100.0)),
                        height: Some(Dimension::Px(200.0)),
                    }),
                    flex_grow: Some(1.0),
                    flex_shrink: Some(1.0),
                    flex_basis: Some(Dimension::Auto),
                    ..Default::default()
                },
                Arrangement::default(),
                ChildOf(flex_container),
            ))
            .id();

        // Flexアイテム3（青相当、grow=2）
        let item3 = world
            .spawn((
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(100.0)),
                        height: Some(Dimension::Px(100.0)),
                    }),
                    flex_grow: Some(2.0),
                    flex_shrink: Some(1.0),
                    flex_basis: Some(Dimension::Auto),
                    ..Default::default()
                },
                Arrangement::default(),
                ChildOf(flex_container),
            ))
            .id();

        (root, flex_container, item1, item2, item3)
    };

    ecs_world.try_tick_world();

    verify_initial_layout(&ecs_world, root, flex_container, item1, item2, item3);
}

/// 初期レイアウト結果を検証
fn verify_initial_layout(
    ecs_world: &EcsWorld,
    root: Entity,
    flex_container: Entity,
    item1: Entity,
    item2: Entity,
    item3: Entity,
) {
    let world = ecs_world.world();
    let arr = |e: Entity| *world.get::<Arrangement>(e).expect("Arrangement");
    let global = |e: Entity| {
        *world
            .get::<GlobalArrangement>(e)
            .expect("GlobalArrangement")
    };

    // Root: 800x600
    let root_arr = arr(root);
    assert_eq!(root_arr.offset.x, 0.0);
    assert_eq!(root_arr.offset.y, 0.0);
    assert_eq!(root_arr.size.width, 800.0);
    assert_eq!(root_arr.size.height, 600.0);

    // Percent(100%)x100%はマージン・パディングなしで親(800x600)と同じサイズになる
    let container_arr = arr(flex_container);
    assert_eq!(container_arr.offset.x, 0.0);
    assert_eq!(container_arr.offset.y, 0.0);
    assert_eq!(container_arr.size.width, 800.0);
    assert_eq!(container_arr.size.height, 600.0);
    let parent_global = global(flex_container);

    // 余白 400px を grow 1:2 で分ける: 幅 200・233・367、交差軸は中央寄せ
    let expected = [
        (item1, 0.0, 225.0, 200.0, 150.0),
        (item2, 200.0, 200.0, ITEM2_W, 200.0),
        (
            item3,
            200.0 + ITEM2_W,
            250.0,
            800.0 - 200.0 - ITEM2_W,
            100.0,
        ),
    ];
    for (item, x, y, w, h) in expected {
        let a = arr(item);
        assert_eq!(
            (a.offset.x, a.offset.y, a.size.width, a.size.height),
            (x, y, w, h)
        );
        // GlobalArrangementは親座標+相対オフセット
        let g = global(item);
        assert_eq!(g.bounds.left, parent_global.bounds.left + a.offset.x);
        assert_eq!(g.bounds.top, parent_global.bounds.top + a.offset.y);
    }
}
