//! マルチウィンドウイベント処理の統合テスト
//!
//! 以下を検証:
//! - find_owner_window: エンティティの所属ウィンドウ特定
//! - build_bubble_path: Window 境界での伝播停止
//! - WM_MOUSELEAVE スコーピング: ウィンドウ別 PointerState クリア
//! - ドラッグ HWND ガード: 異ウィンドウからの end_dragging スキップ

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::pointer::{build_bubble_path, PointerState};
use wintf::ecs::window::{find_owner_window, Window};

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

// ============================================================================
// Task 7.2: WM_MOUSELEAVE スコーピングテスト
// ============================================================================

/// 2つの Window 配下にそれぞれ PointerState を持つエンティティを配置し、
/// Window A のスコープ付きクリア処理を模擬実行
#[test]
fn test_mouseleave_scoped_pointer_clear() {
    let mut world = World::new();

    // Window A とそのウィジェット
    let window_a = world
        .spawn(Window {
            title: "Window A".to_string(),
            ..Default::default()
        })
        .id();
    let widget_a = world
        .spawn((ChildOf(window_a), PointerState::default()))
        .id();

    // Window B とそのウィジェット
    let window_b = world
        .spawn(Window {
            title: "Window B".to_string(),
            ..Default::default()
        })
        .id();
    let widget_b = world
        .spawn((ChildOf(window_b), PointerState::default()))
        .id();

    // Window A のスコープ付き PointerState クリアを模擬実行
    // (handlers.rs の WM_MOUSELEAVE と同じロジック)
    let mut entities_to_clear = Vec::new();
    {
        let mut query = world.query::<(Entity, &PointerState)>();
        for (e, _) in query.iter(&world) {
            if find_owner_window(&world, e) == Some(window_a) {
                entities_to_clear.push(e);
            }
        }
    }
    for entity in entities_to_clear {
        if let Ok(mut entity_ref) = world.get_entity_mut(entity) {
            entity_ref.remove::<PointerState>();
        }
    }

    // 検証: Window A のウィジェットは PointerState が削除された
    assert!(
        world.get::<PointerState>(widget_a).is_none(),
        "Window A のウィジェットの PointerState は削除された"
    );

    // 検証: Window B のウィジェットは PointerState が維持された
    assert!(
        world.get::<PointerState>(widget_b).is_some(),
        "Window B のウィジェットの PointerState は維持される"
    );
}

// ============================================================================
// Task 5.2 / 7.4: ドラッグ HWND ガードテスト
// ============================================================================

/// Preparing/JustStarted 状態のドラッグで、異なるウィンドウからの
/// ボタンアップが end_dragging をスキップすることを検証。
///
/// handlers.rs 内の HWND ガードロジック:
///   Preparing { entity, .. } / JustStarted { entity, .. }:
///     find_owner_window(world, entity) == Some(window_entity) の場合のみ end_dragging 実行
///
/// ここでは find_owner_window によるウィンドウ所有権チェックを模擬実行し、
/// 異なるウィンドウの entity では guard が不成立であることを純粋 ECS で検証する。
#[test]
fn test_drag_hwnd_guard_owner_window_check() {
    let mut world = World::new();

    // Window A とそのウィジェット（ドラッグ対象）
    let window_a = world
        .spawn(Window {
            title: "Window A".to_string(),
            ..Default::default()
        })
        .id();
    let drag_target = world.spawn(ChildOf(window_a)).id();

    // Window B（ボタンアップが来るウィンドウ）
    let window_b = world
        .spawn(Window {
            title: "Window B".to_string(),
            ..Default::default()
        })
        .id();

    // Preparing/JustStarted 状態での entity 所有者チェック模擬
    // ドラッグ対象は Window A に属する
    let drag_entity_owner = find_owner_window(&world, drag_target);
    assert_eq!(
        drag_entity_owner,
        Some(window_a),
        "ドラッグ対象は Window A に属する"
    );

    // Window B からのボタンアップ: ガード不成立
    let should_end_from_b = drag_entity_owner == Some(window_b);
    assert!(
        !should_end_from_b,
        "Window B からのボタンアップではドラッグ終了しない"
    );

    // Window A からのボタンアップ: ガード成立
    let should_end_from_a = drag_entity_owner == Some(window_a);
    assert!(
        should_end_from_a,
        "Window A からのボタンアップではドラッグ終了する"
    );
}
