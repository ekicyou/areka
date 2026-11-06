//! マルチウィンドウイベント処理の統合テスト
//!
//! 以下を検証:
//! - find_owner_window: エンティティの所属ウィンドウ特定
//! - build_bubble_path: Window 境界での伝播停止

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::pointer::build_bubble_path;
use wintf::ecs::window::{Window, find_owner_window};

// ============================================================================
// Task 1.2: find_owner_window ユニットテスト
// ============================================================================

/// 2つの Window エンティティとそれぞれの子ウィジェットを構築し、
/// 各ウィジェットの find_owner_window が正しい Window を返すことを検証
#[test]
fn test_find_owner_window_basic() {
    let mut world = World::new();

    // Window A とその子
    let window_a = world
        .spawn(Window {
            title: "Window A".to_string(),
            ..Default::default()
        })
        .id();

    let container_a = world.spawn(ChildOf(window_a)).id();
    let widget_a = world.spawn(ChildOf(container_a)).id();

    // Window B とその子
    let window_b = world
        .spawn(Window {
            title: "Window B".to_string(),
            ..Default::default()
        })
        .id();

    let widget_b = world.spawn(ChildOf(window_b)).id();

    // 検証: Window A の子孫は Window A を返す
    assert_eq!(
        find_owner_window(&world, widget_a),
        Some(window_a),
        "widget_a は Window A に属する"
    );
    assert_eq!(
        find_owner_window(&world, container_a),
        Some(window_a),
        "container_a は Window A に属する"
    );

    // 検証: Window B の子孫は Window B を返す
    assert_eq!(
        find_owner_window(&world, widget_b),
        Some(window_b),
        "widget_b は Window B に属する"
    );

    // 検証: Window 自身は自身を返す
    assert_eq!(
        find_owner_window(&world, window_a),
        Some(window_a),
        "Window A 自身は自身を返す"
    );
    assert_eq!(
        find_owner_window(&world, window_b),
        Some(window_b),
        "Window B 自身は自身を返す"
    );
}

/// LayoutRoot 直下のエンティティ（Window を持たない）は None を返す
#[test]
fn test_find_owner_window_no_window() {
    let mut world = World::new();

    // LayoutRoot 相当（Window を持たないルートエンティティ）
    let layout_root = world.spawn_empty().id();

    // LayoutRoot 直下のエンティティ
    let orphan = world.spawn(ChildOf(layout_root)).id();

    // LayoutRoot 自身も Window ではない
    assert_eq!(
        find_owner_window(&world, layout_root),
        None,
        "Window を持たないエンティティは None"
    );
    assert_eq!(
        find_owner_window(&world, orphan),
        None,
        "Window でないルートの子エンティティは None"
    );
}

/// 孤立エンティティ（ChildOf なし、Window でもない）は None を返す
#[test]
fn test_find_owner_window_isolated_entity() {
    let mut world = World::new();
    let isolated = world.spawn_empty().id();

    assert_eq!(
        find_owner_window(&world, isolated),
        None,
        "孤立エンティティは None"
    );
}

// ============================================================================
// Task 2.2: build_bubble_path の Window 境界停止テスト
// ============================================================================

/// LayoutRoot → Window → Container → Widget の階層で、
/// Widget からの bubble path が Window で停止することを検証
#[test]
fn test_build_bubble_path_stops_at_window() {
    let mut world = World::new();

    // LayoutRoot → Window → Container → Widget
    let layout_root = world.spawn_empty().id();
    let window = world
        .spawn((
            Window {
                title: "Test Window".to_string(),
                ..Default::default()
            },
            ChildOf(layout_root),
        ))
        .id();
    let container = world.spawn(ChildOf(window)).id();
    let widget = world.spawn(ChildOf(container)).id();

    let path = build_bubble_path(&world, widget);

    // パスは widget → container → window で、layout_root は含まない
    assert_eq!(path.len(), 3, "パスの長さは 3（widget, container, window）");
    assert_eq!(path[0], widget, "パスの先頭は widget");
    assert_eq!(path[1], container, "パスの2番目は container");
    assert_eq!(path[2], window, "パスの最後は window");
    assert!(
        !path.contains(&layout_root),
        "LayoutRoot はパスに含まれない"
    );
}

/// Window 自身から開始する場合、パスは [window] のみ
#[test]
fn test_build_bubble_path_from_window() {
    let mut world = World::new();

    let layout_root = world.spawn_empty().id();
    let window = world
        .spawn((
            Window {
                title: "Test Window".to_string(),
                ..Default::default()
            },
            ChildOf(layout_root),
        ))
        .id();

    let path = build_bubble_path(&world, window);

    assert_eq!(path.len(), 1, "Window 自身からのパスは長さ 1");
    assert_eq!(path[0], window, "パスは [window] のみ");
}

/// Window がない階層（LayoutRoot 直下）では従来通り全パスを返す
#[test]
fn test_build_bubble_path_no_window() {
    let mut world = World::new();

    let root = world.spawn_empty().id();
    let child = world.spawn(ChildOf(root)).id();

    let path = build_bubble_path(&world, child);

    // Window がないので root まで含む従来動作
    assert_eq!(path.len(), 2, "Window なしの場合は全パス");
    assert_eq!(path[0], child);
    assert_eq!(path[1], root);
}
