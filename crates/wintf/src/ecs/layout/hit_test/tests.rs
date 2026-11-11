use super::*;
use crate::ecs::types::{Point, SizeI};
use bevy_ecs::world::World;
use windows_numerics::Matrix3x2;

use crate::ecs::types::Rect;

/// テスト用ヘルパー: 指定した bounds を持つ GlobalArrangement を作成
fn make_global_arrangement(left: f32, top: f32, right: f32, bottom: f32) -> GlobalArrangement {
    GlobalArrangement {
        transform: Matrix3x2::translation(left, top),
        bounds: Rect {
            left,
            top,
            right,
            bottom,
        },
    }
}

// ========================================================================
// hit_test_entity テスト
// ========================================================================

/// ヒットあり（bounds 内）
#[test]
fn test_hit_test_entity_hit_inside() {
    let mut world = World::new();

    // GlobalArrangement を直接設定（伝播システムは使用しない）
    let entity = world
        .spawn((
            make_global_arrangement(10.0, 20.0, 110.0, 70.0), // 100x50
            HitTest::bounds(),
        ))
        .id();

    let point = PointF::new(50.0, 40.0);
    assert!(hit_test_entity(&world, entity, point));
}

/// ヒットなし（bounds 外）
#[test]
fn test_hit_test_entity_miss_outside() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(10.0, 20.0, 110.0, 70.0),
            HitTest::bounds(),
        ))
        .id();

    // bounds外の座標
    let point = PointF::new(200.0, 200.0);
    assert!(!hit_test_entity(&world, entity, point));
}

/// HitTestMode::None のスキップ
#[test]
fn test_hit_test_entity_skip_none_mode() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(10.0, 20.0, 110.0, 70.0),
            HitTest::none(), // ヒットテスト対象外
        ))
        .id();

    // bounds内だがHitTestMode::Noneなのでヒットしない
    let point = PointF::new(50.0, 40.0);
    assert!(!hit_test_entity(&world, entity, point));
}

/// HitTest コンポーネントなし（デフォルト動作 = Bounds）
#[test]
fn test_hit_test_entity_no_component_defaults_to_bounds() {
    let mut world = World::new();

    let entity = world
        .spawn(make_global_arrangement(10.0, 20.0, 110.0, 70.0))
        .id();

    // HitTestコンポーネントなしでもBoundsとして扱う
    let point = PointF::new(50.0, 40.0);
    assert!(hit_test_entity(&world, entity, point));
}

/// GlobalArrangement なしの場合
#[test]
fn test_hit_test_entity_no_global_arrangement() {
    let mut world = World::new();

    // GlobalArrangement なしのエンティティ
    let entity = world.spawn(HitTest::bounds()).id();

    let point = PointF::new(50.0, 40.0);
    assert!(!hit_test_entity(&world, entity, point));
}

// ========================================================================
// hit_test テスト
// ========================================================================

/// ヒットあり（最前面優先）
#[test]
fn test_hit_test_front_priority() {
    let mut world = World::new();

    // 背面エンティティ
    let back = world
        .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
        .id();

    // 前面エンティティ
    let front = world
        .spawn(make_global_arrangement(20.0, 20.0, 80.0, 80.0))
        .id();

    // ルートエンティティ
    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[back, front]);

    // 両方の bounds 内の座標
    let point = PointF::new(50.0, 50.0);
    let result = hit_test(&world, root, point);

    // 最前面の front がヒットする
    assert_eq!(result, Some(front));
}

/// ヒットあり（背面のみ）
#[test]
fn test_hit_test_back_only() {
    let mut world = World::new();

    // 背面エンティティ
    let back = world
        .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
        .id();

    // 前面エンティティ
    let front = world
        .spawn(make_global_arrangement(50.0, 50.0, 100.0, 100.0))
        .id();

    // ルートエンティティ
    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[back, front]);

    // back のみの bounds 内の座標
    let point = PointF::new(10.0, 10.0);
    let result = hit_test(&world, root, point);

    // back がヒットする
    assert_eq!(result, Some(back));
}

/// ヒットなし
#[test]
fn test_hit_test_none() {
    let mut world = World::new();

    let entity = world
        .spawn(make_global_arrangement(10.0, 10.0, 60.0, 60.0))
        .id();

    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[entity]);

    // 全ての bounds 外の座標
    let point = PointF::new(200.0, 200.0);
    let result = hit_test(&world, root, point);

    assert_eq!(result, None);
}

/// HitTestMode::None のスキップ（子は引き続き調査）
#[test]
fn test_hit_test_skip_none_mode() {
    let mut world = World::new();

    // 背面エンティティ
    let back = world
        .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
        .id();

    // オーバーレイ（HitTestMode::None）
    let overlay = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::none(),
        ))
        .id();

    // ルートエンティティ（overlay が最前面）
    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[back, overlay]);

    let point = PointF::new(50.0, 50.0);
    let result = hit_test(&world, root, point);

    // overlay はスキップされ、back がヒットする
    assert_eq!(result, Some(back));
}

/// 親子関係での優先度確認（子が親より優先）
#[test]
fn test_hit_test_child_priority_over_parent() {
    let mut world = World::new();

    // 親エンティティ
    let parent = world
        .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
        .id();

    // 子エンティティ
    let child = world
        .spawn(make_global_arrangement(10.0, 10.0, 90.0, 90.0))
        .id();

    world.entity_mut(parent).add_children(&[child]);

    // 両方の bounds 内の座標
    let point = PointF::new(50.0, 50.0);
    let result = hit_test(&world, parent, point);

    // 子が親より優先
    assert_eq!(result, Some(child));
}

// ========================================================================
// hit_test_in_window テスト
// ========================================================================

/// クライアント座標からスクリーン座標への変換確認
#[test]
fn test_hit_test_in_window_coordinate_conversion() {
    let mut world = World::new();

    // ウィンドウエンティティ
    let window = world
        .spawn((
            make_global_arrangement(100.0, 200.0, 500.0, 500.0),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI {
                    width: 400,
                    height: 300,
                }),
                ..Default::default()
            },
        ))
        .id();

    // ウィンドウ内のウィジェット（スクリーン座標）
    let widget = world
        .spawn(make_global_arrangement(150.0, 250.0, 250.0, 300.0)) // 100x50
        .id();

    world.entity_mut(window).add_children(&[widget]);

    // クライアント座標 (50, 50) → スクリーン座標 (150, 250)
    let client_point = PointF::new(50.0, 50.0);
    let result = hit_test_in_window(&world, window, client_point);

    assert_eq!(result, Some(widget));
}

/// WindowPos なしの場合の None 返却
#[test]
fn test_hit_test_in_window_no_window_pos() {
    let mut world = World::new();

    // WindowPos なしのエンティティ
    let window = world
        .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
        .id();

    let client_point = PointF::new(50.0, 50.0);
    let result = hit_test_in_window(&world, window, client_point);

    assert_eq!(result, None);
}

/// WindowPos.position が None の場合
#[test]
fn test_hit_test_in_window_position_none() {
    let mut world = World::new();

    let window = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            WindowPos {
                position: None, // position が None
                ..Default::default()
            },
        ))
        .id();

    let client_point = PointF::new(50.0, 50.0);
    let result = hit_test_in_window(&world, window, client_point);

    assert_eq!(result, None);
}

// ========================================================================
// 2 つの判定の一致（14 番）
// ========================================================================

/// 2x2 PBGRA32 の全透明・全不透明のマスク
fn mask_2x2(alpha: u8) -> crate::ecs::widget::bitmap_source::AlphaMask {
    let pixels = [0, 0, 0, alpha].repeat(4);
    crate::ecs::widget::bitmap_source::AlphaMask::from_pbgra32(&pixels, 2, 2, 8)
}

/// 位置 (0,0) の窓の下に `HitTest` と透明・不透明のマスクを付けた部品を 1 つ置く
fn window_with_masked_child(mode: HitTest, alpha: u8) -> (World, Entity, Entity) {
    let mut world = World::new();
    let window = world
        .spawn(WindowPos {
            position: Some(Point { x: 0, y: 0 }),
            ..Default::default()
        })
        .id();
    let mut mask = AlphaMaskResource::new();
    mask.set(mask_2x2(alpha));
    let surface = world
        .spawn((make_global_arrangement(0.0, 0.0, 2.0, 2.0), mode, mask))
        .id();
    world.entity_mut(window).add_children(&[surface]);
    (world, window, surface)
}

/// `NamedRegions` は αのマスクを見ない: 透明なマスクを付けても、
/// `hit_test_in_window`（クリックの素通し）と `hit_test_in_window_ex`（領域）が揃って当たる。
#[test]
fn test_named_regions_ignores_alpha_mask_both_paths_agree() {
    let (world, window, surface) = window_with_masked_child(HitTest::named_regions(), 0);
    let point = PointF::new(0.5, 0.5);

    assert_eq!(hit_test_in_window(&world, window, point), Some(surface));
    let ex = hit_test_in_window_ex(&world, window, point).expect("_ex も当たる");
    assert_eq!(ex.entity, surface);
}

/// `_ex` の `AlphaMask` の分岐: 透明なマスクは外れ、不透明なマスクは当たる。
#[test]
fn test_hit_test_in_window_ex_alpha_mask_branch() {
    let point = PointF::new(0.5, 0.5);

    let (world, window, _) = window_with_masked_child(HitTest::alpha_mask(), 0);
    assert!(hit_test_in_window_ex(&world, window, point).is_none());

    let (world, window, surface) = window_with_masked_child(HitTest::alpha_mask(), 255);
    let ex = hit_test_in_window_ex(&world, window, point).expect("不透明なら当たる");
    assert_eq!((ex.entity, ex.region), (surface, None));
}
