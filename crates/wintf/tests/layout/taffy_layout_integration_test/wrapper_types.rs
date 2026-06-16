//! テスト1.x: TaffyStyle / TaffyComputedLayout ラッパー型テスト

use super::*;

/// テスト1.1: BoxStyleがTaffyStyleに名称変更されていることを検証
#[test]
fn test_taffy_style_renamed_from_box_style() {
    // TaffyStyleがComponentとして登録可能であることを確認
    let mut world = World::new();
    let entity = world.spawn(TaffyStyle::default()).id();

    // TaffyStyleコンポーネントが存在することを確認
    assert!(world.entity(entity).contains::<TaffyStyle>());
}

/// テスト1.2: TaffyStyleが#[repr(transparent)]であることを検証
#[test]
fn test_taffy_style_transparent_wrapper() {
    use std::mem::size_of;

    // TaffyStyleとStyle (taffy)のメモリサイズが同じであることを確認
    assert_eq!(size_of::<TaffyStyle>(), size_of::<Style>());
}

/// テスト1.3: TaffyStyleのDefaultトレイト実装を検証
#[test]
fn test_taffy_style_default_implementation() {
    let taffy_style = TaffyStyle::default();

    // デフォルト値が作成できることを確認（内部のStyle::default()と同じ構造）
    // PartialEqトレイトでデフォルト同士の比較が可能
    assert_eq!(taffy_style, TaffyStyle::default());
}

/// テスト1.4: TaffyStyleが必要なトレイト（Clone, Debug, PartialEq）を実装していることを検証
#[test]
fn test_taffy_style_trait_implementations() {
    let style1 = TaffyStyle::default();
    let style2 = style1.clone(); // Clone

    // PartialEq
    assert_eq!(style1, style2);

    // Debug (panic時に出力されることを確認)
    let debug_str = format!("{:?}", style1);
    assert!(debug_str.contains("TaffyStyle"));
}

/// テスト1.5: BoxComputedLayoutがTaffyComputedLayoutに名称変更されていることを検証
#[test]
fn test_taffy_computed_layout_renamed_from_box_computed_layout() {
    let mut world = World::new();
    let entity = world.spawn(TaffyComputedLayout::default()).id();

    assert!(world.entity(entity).contains::<TaffyComputedLayout>());
}

/// テスト1.6: TaffyComputedLayoutが#[repr(transparent)]であることを検証
#[test]
fn test_taffy_computed_layout_transparent_wrapper() {
    use std::mem::size_of;

    assert_eq!(size_of::<TaffyComputedLayout>(), size_of::<Layout>());
}

/// テスト1.7: TaffyComputedLayoutのDefaultトレイト実装を検証
#[test]
fn test_taffy_computed_layout_default_implementation() {
    let computed = TaffyComputedLayout::default();

    // デフォルト値が作成できることを確認
    assert_eq!(computed, TaffyComputedLayout::default());
}

/// テスト1.8: TaffyComputedLayoutが必要なトレイト（Clone, Debug, PartialEq, Copy）を実装していることを検証
#[test]
fn test_taffy_computed_layout_trait_implementations() {
    let layout1 = TaffyComputedLayout::default();
    let layout2 = layout1.clone(); // Clone
    let layout3 = layout1; // Copy

    // PartialEq
    assert_eq!(layout1, layout2);
    assert_eq!(layout1, layout3);

    // Debug
    let debug_str = format!("{:?}", layout1);
    assert!(debug_str.contains("TaffyComputedLayout"));
}
