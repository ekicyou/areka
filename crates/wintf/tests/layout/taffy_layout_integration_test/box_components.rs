//! テスト2.x: 高レベルレイアウトコンポーネント

use super::*;

/// テスト2.1: BoxSize値オブジェクトの実装を検証（BoxStyle経由でComponentとして登録）
#[test]
fn test_box_size_component() {
    let mut world = World::new();

    // BoxStyleを使ってBoxSizeを含むエンティティを作成
    let entity = world
        .spawn(BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(200.0)),
                height: Some(Dimension::Px(100.0)),
            }),
            ..Default::default()
        })
        .id();

    assert!(world.entity(entity).contains::<BoxStyle>());

    // BoxStyleからBoxSizeを取得して検証
    let style = world.get::<BoxStyle>(entity).unwrap();
    assert_eq!(
        style.size.as_ref().unwrap().width,
        Some(Dimension::Px(200.0))
    );
    assert_eq!(
        style.size.as_ref().unwrap().height,
        Some(Dimension::Px(100.0))
    );

    // Defaultは両方None
    let default_size = BoxSize::default();
    assert_eq!(default_size.width, None);
    assert_eq!(default_size.height, None);
}

/// テスト2.2: BoxMargin値オブジェクトの実装を検証（BoxStyle経由でComponentとして登録）
#[test]
fn test_box_margin_component() {
    let mut world = World::new();

    let margin = BoxMargin(Rect {
        left: LengthPercentageAuto::Px(10.0),
        right: LengthPercentageAuto::Px(10.0),
        top: LengthPercentageAuto::Px(5.0),
        bottom: LengthPercentageAuto::Px(5.0),
    });

    // BoxStyleを使ってBoxMarginを含むエンティティを作成
    let entity = world
        .spawn(BoxStyle {
            margin: Some(margin),
            ..Default::default()
        })
        .id();
    assert!(world.entity(entity).contains::<BoxStyle>());

    // Defaultはauto（taffy標準に従う）
    let default_margin = BoxMargin::default();
    assert_eq!(default_margin.0.left, LengthPercentageAuto::Auto);
}

/// テスト2.3: BoxPadding値オブジェクトの実装を検証（BoxStyle経由でComponentとして登録）
#[test]
fn test_box_padding_component() {
    let mut world = World::new();

    let padding = BoxPadding(Rect {
        left: LengthPercentage::Px(10.0),
        right: LengthPercentage::Px(10.0),
        top: LengthPercentage::Px(5.0),
        bottom: LengthPercentage::Px(5.0),
    });

    // BoxStyleを使ってBoxPaddingを含むエンティティを作成
    let entity = world
        .spawn(BoxStyle {
            padding: Some(padding),
            ..Default::default()
        })
        .id();
    assert!(world.entity(entity).contains::<BoxStyle>());
}

/// テスト2.4: FlexContainer値オブジェクトの実装を検証（BoxStyle経由でComponentとして登録）
#[test]
fn test_flex_container_component() {
    let mut world = World::new();

    // BoxStyleを使ってFlexContainer相当のプロパティを設定
    let entity = world
        .spawn(BoxStyle {
            flex_direction: Some(FlexDirection::Column),
            justify_content: Some(JustifyContent::CENTER),
            align_items: Some(AlignItems::CENTER),
            ..Default::default()
        })
        .id();
    assert!(world.entity(entity).contains::<BoxStyle>());

    // BoxStyleからプロパティを取得して検証
    let style = world.get::<BoxStyle>(entity).unwrap();
    assert_eq!(style.flex_direction, Some(FlexDirection::Column));
    assert_eq!(style.justify_content, Some(JustifyContent::CENTER));
    assert_eq!(style.align_items, Some(AlignItems::CENTER));

    // FlexContainer値オブジェクトのDefaultチェック
    let default_container = FlexContainer::default();
    assert_eq!(default_container.direction, FlexDirection::Row);
    assert_eq!(default_container.justify_content, None);
    assert_eq!(default_container.align_items, None);
}

/// テスト2.5: FlexItem値オブジェクトの実装を検証（BoxStyle経由でComponentとして登録）
#[test]
fn test_flex_item_component() {
    let mut world = World::new();

    // BoxStyleを使ってFlexItem相当のプロパティを設定
    let entity = world
        .spawn(BoxStyle {
            flex_grow: Some(1.0),
            flex_shrink: Some(0.5),
            flex_basis: Some(Dimension::Px(100.0)),
            align_self: Some(AlignSelf::END),
            ..Default::default()
        })
        .id();
    assert!(world.entity(entity).contains::<BoxStyle>());

    // BoxStyleからプロパティを取得して検証
    let style = world.get::<BoxStyle>(entity).unwrap();
    assert_eq!(style.flex_grow, Some(1.0));
    assert_eq!(style.flex_shrink, Some(0.5));
    assert_eq!(style.flex_basis, Some(Dimension::Px(100.0)));
    assert_eq!(style.align_self, Some(AlignSelf::END));

    // FlexItem値オブジェクトのDefaultチェック
    let default_item = FlexItem::default();
    assert_eq!(default_item.grow, 0.0);
    assert_eq!(default_item.shrink, 1.0);
    assert_eq!(default_item.basis, Dimension::Auto);
    assert_eq!(default_item.align_self, None);
}
