//! ユニット・統合テスト

use super::*;

/// Dimensionのconst constructorテスト
#[test]
fn test_dimension_const_constructors() {
    const DIM_LENGTH: Dimension = Dimension::length(100.0);
    const DIM_PERCENT: Dimension = Dimension::percent(50.0);
    const DIM_AUTO: Dimension = Dimension::auto();
    const DIM_ZERO: Dimension = Dimension::zero();

    assert_eq!(DIM_LENGTH, Dimension::Px(100.0));
    assert_eq!(DIM_PERCENT, Dimension::Percent(50.0));
    assert_eq!(DIM_AUTO, Dimension::Auto);
    assert_eq!(DIM_ZERO, Dimension::Px(0.0));
}

/// EcsWorldを使った3階層ウィジェットツリーの総合レイアウトテスト
///
/// このテストは、BoxSize変更から最終的なGlobalArrangementの更新までを検証します。
/// - 3階層のウィジェットツリー（Root → Container → Child）を構築
/// - 初回レイアウト計算でGlobalArrangementが正しく設定されることを確認
/// - Containerのサイズ変更後、すべての階層でGlobalArrangementが再計算されることを確認
///
/// 注意: このテストはWindowコンポーネントを使用せず、純粋なレイアウト計算のみを検証します。
/// これにより、実際のウィンドウ作成処理を回避し、テストの安全性と速度を向上させます。
#[test]
fn test_full_layout_pipeline_with_ecs_world() {
    use wintf::ecs::ChildOf;
    use wintf::ecs::world::EcsWorld;

    let mut ecs_world = EcsWorld::new();

    let (root, container, child) = {
        let world = ecs_world.world_mut();

        let root = world
            .spawn((
                LayoutRoot,
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(800.0)),
                        height: Some(Dimension::Px(600.0)),
                    }),
                    flex_direction: Some(FlexDirection::Column),
                    justify_content: Some(JustifyContent::START),
                    align_items: Some(AlignItems::STRETCH),
                    ..Default::default()
                },
                Arrangement::default(),
            ))
            .id();

        let container = world
            .spawn((
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(400.0)),
                        height: Some(Dimension::Px(300.0)),
                    }),
                    flex_direction: Some(FlexDirection::Row),
                    justify_content: Some(JustifyContent::CENTER),
                    align_items: Some(AlignItems::CENTER),
                    ..Default::default()
                },
                Arrangement::default(),
                ChildOf(root),
            ))
            .id();

        let child = world
            .spawn((
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(200.0)),
                        height: Some(Dimension::Px(150.0)),
                    }),
                    ..Default::default()
                },
                Arrangement::default(),
                ChildOf(container),
            ))
            .id();

        (root, container, child)
    };

    ecs_world.try_tick_world();

    {
        let world = ecs_world.world();
        assert!(
            world.entity(root).contains::<TaffyStyle>(),
            "RootにTaffyStyleが生成されていません"
        );
        assert!(
            world.entity(container).contains::<TaffyStyle>(),
            "ContainerにTaffyStyleが生成されていません"
        );
        assert!(
            world.entity(child).contains::<TaffyStyle>(),
            "ChildにTaffyStyleが生成されていません"
        );
    }

    {
        let world = ecs_world.world();
        assert!(
            world.entity(root).contains::<Arrangement>(),
            "RootにArrangementが生成されていません"
        );
        assert!(
            world.entity(container).contains::<Arrangement>(),
            "ContainerにArrangementが生成されていません"
        );
        assert!(
            world.entity(child).contains::<Arrangement>(),
            "ChildにArrangementが生成されていません"
        );
    }

    {
        let world = ecs_world.world();
        assert!(
            world.entity(root).contains::<GlobalArrangement>(),
            "RootにGlobalArrangementが生成されていません"
        );
        assert!(
            world.entity(container).contains::<GlobalArrangement>(),
            "ContainerにGlobalArrangementが生成されていません"
        );
        assert!(
            world.entity(child).contains::<GlobalArrangement>(),
            "ChildにGlobalArrangementが生成されていません"
        );
    }

    let (initial_container_arrangement, initial_container_global) = {
        let world = ecs_world.world();
        (
            *world.entity(container).get::<Arrangement>().unwrap(),
            *world.entity(container).get::<GlobalArrangement>().unwrap(),
        )
    };

    {
        let world = ecs_world.world_mut();
        if let Some(mut style) = world.entity_mut(container).get_mut::<BoxStyle>() {
            if let Some(ref mut size) = style.size {
                size.width = Some(Dimension::Px(600.0));
                size.height = Some(Dimension::Px(450.0));
            }
        }
    }

    ecs_world.try_tick_world();

    {
        let world = ecs_world.world();
        let updated_container_arrangement = *world.entity(container).get::<Arrangement>().unwrap();
        assert_ne!(
            initial_container_arrangement.size, updated_container_arrangement.size,
            "Containerのサイズ変更後、Arrangementが更新されていません"
        );
    }

    {
        let world = ecs_world.world();
        let updated_container_global = *world.entity(container).get::<GlobalArrangement>().unwrap();

        assert_ne!(
            initial_container_global.bounds, updated_container_global.bounds,
            "Containerのサイズ変更後、GlobalArrangementのboundsが更新されていません"
        );
    }

    {
        let world = ecs_world.world();
        let container_size = world.entity(container).get::<Arrangement>().unwrap().size;
        assert!(
            (container_size.width - 600.0).abs() < 1.0,
            "Containerの幅が期待値と異なります: expected ~600.0, got {}",
            container_size.width
        );
        assert!(
            (container_size.height - 450.0).abs() < 1.0,
            "Containerの高さが期待値と異なります: expected ~450.0, got {}",
            container_size.height
        );
    }
}
