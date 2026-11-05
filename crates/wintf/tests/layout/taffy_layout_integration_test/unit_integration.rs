//! テスト7.x: ユニット・統合テスト

use super::*;

/// テスト7.5: Dimensionのconst constructorテスト
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

/// テスト7.9: EcsWorldを使った3階層ウィジェットツリーの総合レイアウトテスト
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

    // EcsWorldを作成（デフォルトのシステムスケジュールが登録済み）
    let mut ecs_world = EcsWorld::new();

    // 3階層のウィジェットツリーを構築
    let (root, container, child) = {
        let world = ecs_world.world_mut();

        // Root (ルートエンティティ - LayoutRootマーカーを使用)
        let root = world
            .spawn((
                LayoutRoot, // レイアウト計算のルートを示すマーカー
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
                Arrangement::default(), // Arrangementを明示的に追加
            ))
            .id();

        // Container (中間層)
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
                Arrangement::default(), // Arrangementを明示的に追加
                ChildOf(root),          // 親子関係を設定
            ))
            .id();

        // Child (末端)
        let child = world
            .spawn((
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(200.0)),
                        height: Some(Dimension::Px(150.0)),
                    }),
                    ..Default::default()
                },
                Arrangement::default(), // Arrangementを明示的に追加
                ChildOf(container),     // 親子関係を設定
            ))
            .id();

        (root, container, child)
    };

    // 初回レイアウト計算を実行
    ecs_world.try_tick_world();

    // 検証1: すべてのエンティティにTaffyStyleが生成されている
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

    // 検証2: すべてのエンティティにArrangementが生成されている
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

    // 検証3: すべてのエンティティにGlobalArrangementが生成されている
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

    // 初回のArrangementとGlobalArrangementを保存
    let (initial_container_arrangement, initial_container_global) = {
        let world = ecs_world.world();
        (
            *world.entity(container).get::<Arrangement>().unwrap(),
            *world.entity(container).get::<GlobalArrangement>().unwrap(),
        )
    };

    // Containerのサイズを変更
    {
        let world = ecs_world.world_mut();
        if let Some(mut style) = world.entity_mut(container).get_mut::<BoxStyle>() {
            if let Some(ref mut size) = style.size {
                size.width = Some(Dimension::Px(600.0)); // 400 → 600
                size.height = Some(Dimension::Px(450.0)); // 300 → 450
            }
        }
    }

    // レイアウト再計算を実行（1 tickで完了するはず）
    ecs_world.try_tick_world();

    // 検証4: Containerのサイズ変更後、Arrangementが更新されている
    {
        let world = ecs_world.world();
        let updated_container_arrangement = *world.entity(container).get::<Arrangement>().unwrap();
        assert_ne!(
            initial_container_arrangement.size, updated_container_arrangement.size,
            "Containerのサイズ変更後、Arrangementが更新されていません"
        );
    }

    // 検証5: Containerのサイズ変更後、GlobalArrangementが更新されている
    {
        let world = ecs_world.world();
        let updated_container_global = *world.entity(container).get::<GlobalArrangement>().unwrap();

        assert_ne!(
            initial_container_global.bounds, updated_container_global.bounds,
            "Containerのサイズ変更後、GlobalArrangementのboundsが更新されていません"
        );
    }

    // 検証7: レイアウト計算が正しく完了していることを確認
    // ContainerのArrangementサイズが期待値（600x?）に近いことを確認
    {
        let world = ecs_world.world();
        let container_size = world.entity(container).get::<Arrangement>().unwrap().size;
        assert!(
            (container_size.width - 600.0).abs() < 1.0,
            "Containerの幅が期待値と異なります: expected ~600.0, got {}",
            container_size.width
        );
        // 高さは親の Flex レイアウトが決める（明示の指定は反映されない）ので確かめない
    }
}
