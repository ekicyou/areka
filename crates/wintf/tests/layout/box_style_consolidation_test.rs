//! BoxStyle統合コンポーネントのテスト
//!
//! Requirements: 1.1, 1.2, 1.4, 3.2

// タスク1.1: BoxStyle構造体の定義テスト

/// テスト: BoxStyleがデフォルトで全フィールドNoneであること
#[test]
fn test_box_style_default_all_none() {
    use wintf::ecs::layout::BoxStyle;

    let style = BoxStyle::default();

    // Box系プロパティがNone
    assert!(style.size.is_none());
    assert!(style.margin.is_none());
    assert!(style.padding.is_none());
    assert!(style.position.is_none());
    assert!(style.inset.is_none());

    // Flex系プロパティがNone
    assert!(style.flex_direction.is_none());
    assert!(style.justify_content.is_none());
    assert!(style.align_items.is_none());
    assert!(style.flex_grow.is_none());
    assert!(style.flex_shrink.is_none());
    assert!(style.flex_basis.is_none());
    assert!(style.align_self.is_none());
}

// タスク1.3: From変換トレイトテスト

/// テスト: BoxStyleからtaffy::Styleへの変換（全フィールドNone時）
#[test]
fn test_box_style_to_taffy_style_default() {
    use wintf::ecs::layout::BoxStyle;

    let box_style = BoxStyle::default();
    let taffy_style: taffy::Style = (&box_style).into();

    // デフォルト値がtaffyデフォルトと互換
    assert_eq!(taffy_style.flex_grow, 0.0);
    assert_eq!(taffy_style.flex_shrink, 1.0);
}

/// テスト: BoxStyleからtaffy::Styleへの変換（サイズ設定時）
#[test]
fn test_box_style_to_taffy_style_with_size() {
    use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension};

    let box_style = BoxStyle {
        size: Some(BoxSize {
            width: Some(Dimension::Px(200.0)),
            height: Some(Dimension::Px(100.0)),
        }),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();

    assert_eq!(taffy_style.size.width, taffy::Dimension::length(200.0));
    assert_eq!(taffy_style.size.height, taffy::Dimension::length(100.0));
}

/// テスト: BoxStyleからtaffy::Styleへの変換（Flexコンテナープロパティ設定時にdisplay: Flex）
#[test]
fn test_box_style_to_taffy_style_flex_container_sets_display() {
    use wintf::ecs::layout::{BoxStyle, FlexDirection};

    let box_style = BoxStyle {
        flex_direction: Some(FlexDirection::Column),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();

    // flex_direction設定時にdisplay: Flexが自動設定される
    assert_eq!(taffy_style.display, taffy::Display::Flex);
    assert_eq!(taffy_style.flex_direction, taffy::FlexDirection::Column);
}

/// テスト: BoxStyleからtaffy::Styleへの変換（絶対配置時）
#[test]
fn test_box_style_to_taffy_style_absolute_position() {
    use wintf::ecs::layout::{BoxInset, BoxPosition, BoxStyle, LengthPercentageAuto, Rect};

    let box_style = BoxStyle {
        position: Some(BoxPosition::Absolute),
        inset: Some(BoxInset(Rect {
            left: LengthPercentageAuto::Px(100.0),
            top: LengthPercentageAuto::Px(50.0),
            right: LengthPercentageAuto::Auto,
            bottom: LengthPercentageAuto::Auto,
        })),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();

    assert_eq!(taffy_style.position, taffy::Position::Absolute);
    assert_eq!(
        taffy_style.inset.left,
        taffy::LengthPercentageAuto::length(100.0)
    );
    assert_eq!(
        taffy_style.inset.top,
        taffy::LengthPercentageAuto::length(50.0)
    );
}

// ===== W4a ギャップテスト: BoxStyle → taffy::Style 変換の未カバー分岐 =====

/// BoxStyle::new() は Default と等価
#[test]
fn test_box_style_new_equals_default() {
    use wintf::ecs::layout::BoxStyle;

    assert_eq!(BoxStyle::new(), BoxStyle::default());
}

/// min_size / max_size が taffy::Style の min_size / max_size に変換される
#[test]
fn test_box_style_to_taffy_style_min_max_size() {
    use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension};

    let box_style = BoxStyle {
        min_size: Some(BoxSize {
            width: Some(Dimension::Px(50.0)),
            height: Some(Dimension::Px(30.0)),
        }),
        max_size: Some(BoxSize {
            width: Some(Dimension::Px(500.0)),
            height: Some(Dimension::Px(300.0)),
        }),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();

    assert_eq!(taffy_style.min_size.width, taffy::Dimension::length(50.0));
    assert_eq!(taffy_style.min_size.height, taffy::Dimension::length(30.0));
    assert_eq!(taffy_style.max_size.width, taffy::Dimension::length(500.0));
    assert_eq!(taffy_style.max_size.height, taffy::Dimension::length(300.0));
    // size 未指定部分は taffy デフォルト（auto）のまま
    assert_eq!(taffy_style.size.width, taffy::Dimension::auto());
}

/// min_size / max_size の片側 None はその辺だけ taffy デフォルトを維持する
#[test]
fn test_box_style_to_taffy_style_partial_min_max_size() {
    use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension};

    let box_style = BoxStyle {
        min_size: Some(BoxSize {
            width: Some(Dimension::Px(50.0)),
            height: None,
        }),
        max_size: Some(BoxSize {
            width: None,
            height: Some(Dimension::Percent(80.0)),
        }),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();

    assert_eq!(taffy_style.min_size.width, taffy::Dimension::length(50.0));
    assert_eq!(taffy_style.min_size.height, taffy::Dimension::auto());
    assert_eq!(taffy_style.max_size.width, taffy::Dimension::auto());
    // Dimension::Percent は ÷100 正規化される
    assert_eq!(taffy_style.max_size.height, taffy::Dimension::percent(0.8));
}

/// size の Percent 指定は Dimension 変換を経由して ÷100 正規化される
#[test]
fn test_box_style_to_taffy_style_percent_size() {
    use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension};

    let box_style = BoxStyle {
        size: Some(BoxSize {
            width: Some(Dimension::Percent(100.0)),
            height: Some(Dimension::Percent(50.0)),
        }),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();

    assert_eq!(taffy_style.size.width, taffy::Dimension::percent(1.0));
    assert_eq!(taffy_style.size.height, taffy::Dimension::percent(0.5));
}

/// position: Relative の明示指定は taffy::Position::Relative に変換される
#[test]
fn test_box_style_to_taffy_style_relative_position() {
    use wintf::ecs::layout::{BoxPosition, BoxStyle};

    let box_style = BoxStyle {
        position: Some(BoxPosition::Relative),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();
    assert_eq!(taffy_style.position, taffy::Position::Relative);
}

/// Flex アイテム系プロパティ（flex_grow 等）のみの指定では display の自動 Flex 設定は
/// 行われない（コンテナー系 3 プロパティのみが display を設定する）。
/// taffy::Style::default().display と比較することで現状仕様を固定する。
#[test]
fn test_box_style_flex_item_props_keep_default_display() {
    use wintf::ecs::layout::{BoxStyle, Dimension};

    let box_style = BoxStyle {
        flex_grow: Some(1.0),
        flex_shrink: Some(2.0),
        flex_basis: Some(Dimension::Px(100.0)),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();

    let default_style: taffy::Style = Default::default();
    assert_eq!(taffy_style.display, default_style.display);
    assert_eq!(taffy_style.flex_grow, 1.0);
    assert_eq!(taffy_style.flex_shrink, 2.0);
    assert_eq!(taffy_style.flex_basis, taffy::Dimension::length(100.0));
}

/// flex_basis の Percent 指定も ÷100 正規化される
#[test]
fn test_box_style_to_taffy_style_percent_flex_basis() {
    use wintf::ecs::layout::{BoxStyle, Dimension};

    let box_style = BoxStyle {
        flex_basis: Some(Dimension::Percent(30.0)),
        ..Default::default()
    };

    let taffy_style: taffy::Style = (&box_style).into();
    assert_eq!(taffy_style.flex_basis, taffy::Dimension::percent(0.3));
}
