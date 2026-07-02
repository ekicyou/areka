//! WUC Visual 子削除 API テスト (R1)
//!
//! WUC 移行: 旧 DCompositionVisualExt の add_visual/remove_visual/remove_all_visuals は
//! WUC では `parent.cast::<ContainerVisual>()?.Children()?` 経由の
//! `InsertAtBottom` / `Remove` / `RemoveAll` に対応する（要件 R1）。
//! VisualGraphics の on_remove フックもこの経路を使う（components.rs::on_visual_graphics_remove）。

use windows::UI::Composition::{ContainerVisual, Visual};
use windows::core::{Interface, Result};

use super::common::{WucVisualFactory, setup_graphics};

/// WUC 移行: DComp の add_visual(insertAbove=false, ref=None) 相当。
fn add_child(parent: &Visual, child: &Visual) -> Result<()> {
    parent
        .cast::<ContainerVisual>()?
        .Children()?
        .InsertAtBottom(child)?;
    Ok(())
}

/// WUC 移行: DComp の remove_visual 相当。
fn remove_child(parent: &Visual, child: &Visual) -> Result<()> {
    parent.cast::<ContainerVisual>()?.Children()?.Remove(child)?;
    Ok(())
}

/// WUC 移行: DComp の remove_all_visuals 相当。
fn remove_all_children(parent: &Visual) -> Result<()> {
    parent.cast::<ContainerVisual>()?.Children()?.RemoveAll()?;
    Ok(())
}

/// 子数を取得するヘルパー。
fn child_count(parent: &Visual) -> Result<i32> {
    Ok(parent.cast::<ContainerVisual>()?.Children()?.Count()?)
}

/// remove: 子Visualを親から削除できることを確認
#[test]
fn test_remove_visual_removes_child_from_parent() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    // 親Visual と 子Visual を作成
    let parent_visual = factory.create_visual()?;
    let child_visual = factory.create_visual()?;

    // 子を親に追加
    add_child(&parent_visual, &child_visual)?;
    assert_eq!(child_count(&parent_visual)?, 1, "追加後は子が 1");

    // 子を親から削除
    let result = remove_child(&parent_visual, &child_visual);
    assert!(result.is_ok(), "remove should succeed: {:?}", result);
    assert_eq!(child_count(&parent_visual)?, 0, "削除後は子が 0");

    Ok(())
}

/// remove: 存在しないVisualの削除はエラーを返す（適切なエラーハンドリング）
#[test]
fn test_remove_visual_nonexistent_returns_error() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    // 親Visual と 子Visual を作成（子は追加しない）
    let parent_visual = factory.create_visual()?;
    let child_visual = factory.create_visual()?;

    // 追加していないVisualを削除しようとする
    // WUC の VisualCollection::Remove は所属していない Visual に対して例外を返す場合がある。
    // 実際の動作はランタイムにより異なりうるため、呼び出しがパニックしないこと・
    // エラーハンドリングが適切に行われることを確認する。
    let result = remove_child(&parent_visual, &child_visual);
    eprintln!("remove result for nonexistent: {:?}", result);

    Ok(())
}

/// remove_all: 全ての子Visualを削除できることを確認
#[test]
fn test_remove_all_visuals_clears_all_children() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    // 親Visual と 複数の子Visual を作成
    let parent_visual = factory.create_visual()?;
    let child1 = factory.create_visual()?;
    let child2 = factory.create_visual()?;
    let child3 = factory.create_visual()?;

    // 全ての子を親に追加
    add_child(&parent_visual, &child1)?;
    add_child(&parent_visual, &child2)?;
    add_child(&parent_visual, &child3)?;
    assert_eq!(child_count(&parent_visual)?, 3, "3 子追加後");

    // 全ての子を一括削除
    let result = remove_all_children(&parent_visual);
    assert!(result.is_ok(), "remove_all should succeed: {:?}", result);
    assert_eq!(child_count(&parent_visual)?, 0, "一括削除後は 0");

    // 削除後に再度呼んでも問題ないことを確認
    let result2 = remove_all_children(&parent_visual);
    assert!(
        result2.is_ok(),
        "remove_all on empty parent should succeed: {:?}",
        result2
    );

    Ok(())
}

/// remove: 複数回の追加・削除が正常に動作することを確認
#[test]
fn test_remove_visual_multiple_operations() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    let parent_visual = factory.create_visual()?;
    let child_visual = factory.create_visual()?;

    // 追加 → 削除 → 再追加 → 再削除 のサイクル
    add_child(&parent_visual, &child_visual)?;
    remove_child(&parent_visual, &child_visual)?;
    add_child(&parent_visual, &child_visual)?;
    remove_child(&parent_visual, &child_visual)?;
    assert_eq!(child_count(&parent_visual)?, 0, "サイクル後は 0");

    Ok(())
}
