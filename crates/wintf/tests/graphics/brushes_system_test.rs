//! resolve_inherited_brushes システムのギャップテスト (W3b-T)
//!
//! Brush 継承解決はデバイス非依存の純粋 ECS ロジックだが、
//! システム本体のテストが存在しなかったため追加する。
//! 対象: crates/wintf/src/ecs/graphics/systems/brushes.rs

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::resolve_inherited_brushes;
use wintf::ecs::widget::{Brush, BrushInherit, Brushes, DEFAULT_BACKGROUND, DEFAULT_FOREGROUND};

/// resolve_inherited_brushes を 1 回実行するヘルパー
fn run_resolve(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(resolve_inherited_brushes);
    schedule.run(world);
}

/// Brushes なし・親なし → デフォルト（前景=黒、背景=透明）が挿入されマーカーが除去される
#[test]
fn no_brushes_no_parent_resolves_to_defaults() {
    let mut world = World::new();
    let entity = world.spawn(BrushInherit).id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(entity).expect("Brushes inserted");
    assert_eq!(brushes.foreground, DEFAULT_FOREGROUND);
    assert_eq!(brushes.background, DEFAULT_BACKGROUND);
    assert!(
        world.get::<BrushInherit>(entity).is_none(),
        "BrushInherit marker should be removed"
    );
}

/// Brushes なし・解決済み親あり → 親の Brushes がそのまま複製される
#[test]
fn no_brushes_clones_fully_resolved_parent() {
    let mut world = World::new();
    let parent = world
        .spawn(Brushes {
            foreground: Brush::RED,
            background: Brush::BLUE,
        })
        .id();
    let child = world.spawn((BrushInherit, ChildOf(parent))).id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(child).expect("Brushes inserted");
    assert_eq!(brushes.foreground, Brush::RED);
    assert_eq!(brushes.background, Brush::BLUE);
    assert!(world.get::<BrushInherit>(child).is_none());
}

/// 両フィールド Solid の既存 Brushes → 値は変更されず、マーカーのみ除去される
#[test]
fn solid_brushes_kept_and_marker_removed() {
    let mut world = World::new();
    let entity = world
        .spawn((
            BrushInherit,
            Brushes {
                foreground: Brush::GREEN,
                background: Brush::WHITE,
            },
        ))
        .id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(entity).unwrap();
    assert_eq!(brushes.foreground, Brush::GREEN);
    assert_eq!(brushes.background, Brush::WHITE);
    assert!(world.get::<BrushInherit>(entity).is_none());
}

/// 前景のみ Inherit → 親の前景色が Solid として解決され、背景は維持される
#[test]
fn inherit_foreground_resolved_from_parent() {
    let mut world = World::new();
    let parent = world
        .spawn(Brushes {
            foreground: Brush::RED,
            background: Brush::BLUE,
        })
        .id();
    let child = world
        .spawn((
            BrushInherit,
            Brushes {
                foreground: Brush::Inherit,
                background: Brush::GREEN,
            },
            ChildOf(parent),
        ))
        .id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(child).unwrap();
    assert_eq!(brushes.foreground, Brush::RED, "fg inherited from parent");
    assert_eq!(brushes.background, Brush::GREEN, "bg kept as-is");
}

/// 背景のみ Inherit → 親の背景色が解決され、前景は維持される
#[test]
fn inherit_background_resolved_from_parent() {
    let mut world = World::new();
    let parent = world
        .spawn(Brushes {
            foreground: Brush::RED,
            background: Brush::BLUE,
        })
        .id();
    let child = world
        .spawn((
            BrushInherit,
            Brushes {
                foreground: Brush::WHITE,
                background: Brush::Inherit,
            },
            ChildOf(parent),
        ))
        .id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(child).unwrap();
    assert_eq!(brushes.foreground, Brush::WHITE, "fg kept as-is");
    assert_eq!(brushes.background, Brush::BLUE, "bg inherited from parent");
}

/// 両フィールド Inherit・親なし → デフォルト値で解決される
#[test]
fn inherit_fields_fall_back_to_defaults_without_parent() {
    let mut world = World::new();
    let entity = world.spawn((BrushInherit, Brushes::default())).id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(entity).unwrap();
    assert_eq!(brushes.foreground, DEFAULT_FOREGROUND);
    assert_eq!(brushes.background, DEFAULT_BACKGROUND);
}

/// 部分解決の親（片フィールド Inherit）はスキップされ、完全解決の祖父から継承される
#[test]
fn partially_resolved_parent_is_skipped_for_grandparent() {
    let mut world = World::new();
    let grandparent = world
        .spawn(Brushes {
            foreground: Brush::WHITE,
            background: Brush::BLUE,
        })
        .id();
    // 親は前景のみ Solid（背景 Inherit）= 未解決扱い
    let parent = world
        .spawn((
            Brushes {
                foreground: Brush::RED,
                background: Brush::Inherit,
            },
            ChildOf(grandparent),
        ))
        .id();
    let child = world.spawn((BrushInherit, ChildOf(parent))).id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(child).unwrap();
    assert_eq!(
        brushes.foreground,
        Brush::WHITE,
        "partially resolved parent must be skipped; inherit from grandparent"
    );
    assert_eq!(brushes.background, Brush::BLUE);
}

/// 部分解決の親しかいない（完全解決の祖先なし）→ 親の Solid 値ではなくデフォルトになる
///
/// find_parent_brushes は「両フィールド解決済み」の祖先のみを返すため、
/// 親の解決済みフィールド（ここでは前景 RED）は子に伝播しない。現行仕様の固定化。
#[test]
fn partially_resolved_parent_without_resolved_ancestor_yields_defaults() {
    let mut world = World::new();
    let parent = world
        .spawn(Brushes {
            foreground: Brush::RED,
            background: Brush::Inherit,
        })
        .id();
    let child = world.spawn((BrushInherit, ChildOf(parent))).id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(child).unwrap();
    assert_eq!(
        brushes.foreground, DEFAULT_FOREGROUND,
        "parent's resolved fg must NOT leak through a partially resolved parent"
    );
    assert_eq!(brushes.background, DEFAULT_BACKGROUND);
}

/// マーカーを持たないエンティティは解決対象外（Inherit のまま維持される）
#[test]
fn entity_without_marker_is_untouched() {
    let mut world = World::new();
    let entity = world.spawn(Brushes::default()).id();

    run_resolve(&mut world);

    let brushes = world.get::<Brushes>(entity).unwrap();
    assert!(brushes.foreground.is_inherit(), "fg stays Inherit");
    assert!(brushes.background.is_inherit(), "bg stays Inherit");
}

/// 複数エンティティが 1 回の実行でまとめて解決される
#[test]
fn multiple_entities_resolved_in_single_run() {
    let mut world = World::new();
    let parent = world
        .spawn(Brushes {
            foreground: Brush::RED,
            background: Brush::BLUE,
        })
        .id();
    let c1 = world.spawn((BrushInherit, ChildOf(parent))).id();
    let c2 = world.spawn((BrushInherit, ChildOf(parent))).id();
    let orphan = world.spawn(BrushInherit).id();

    run_resolve(&mut world);

    assert_eq!(world.get::<Brushes>(c1).unwrap().foreground, Brush::RED);
    assert_eq!(world.get::<Brushes>(c2).unwrap().background, Brush::BLUE);
    assert_eq!(
        world.get::<Brushes>(orphan).unwrap().foreground,
        DEFAULT_FOREGROUND
    );
    for e in [c1, c2, orphan] {
        assert!(world.get::<BrushInherit>(e).is_none());
    }
}
