//! taffy の木のつなぎのテスト（audit-wintf-layout-widget の 1・2 番）
//!
//! 配置の 5 つの係を本番と同じ順（`ecs/world/mod.rs` の `Layout`）に `.chain()` で並べて回す。

use super::*;
use crate::ecs::layout::BoxSize;
use crate::ecs::test_support::capture_under_filter;

/// 本番の `Layout` と同じ順に配置の係を並べる。
/// 変化の検知は係ごとの前回の実行が基準なので、同じスケジュールを回し続ける
/// （毎回作り直すと、どの部品も「付いたばかり」に見える）。
fn layout_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    // 記録の捕捉はスレッド局所なので、係を呼んだスレッドで回す
    schedule.set_executor(bevy_ecs::schedule::SingleThreadedExecutor::new());
    schedule.add_systems(
        (
            build_taffy_styles_system,
            sync_taffy_tree_system,
            compute_taffy_layout_system,
            update_arrangements_system,
            cleanup_removed_entities_system,
        )
            .chain(),
    );
    schedule
}

fn px_box(width: f32, height: f32) -> BoxStyle {
    BoxStyle {
        size: Some(BoxSize {
            width: Some(Dimension::Px(width)),
            height: Some(Dimension::Px(height)),
        }),
        ..Default::default()
    }
}

/// 横に並べる根（200×100）を作る。
fn spawn_row_root(world: &mut World) -> Entity {
    world
        .spawn((
            LayoutRoot,
            BoxStyle {
                flex_direction: Some(FlexDirection::Row),
                ..px_box(200.0, 100.0)
            },
        ))
        .id()
}

fn new_world() -> World {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();
    world
}

fn computed(world: &World, entity: Entity) -> taffy::Layout {
    world
        .get::<TaffyComputedLayout>(entity)
        .expect("TaffyComputedLayout があるはず")
        .0
}

/// 1 番: 木に入った後で子へ `BoxStyle` を付けると、付けた大きさで配置される。
#[test]
fn box_style_added_to_child_later_is_laid_out() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let child = world.spawn(ChildOf(root)).id();
    layout.run(&mut world);

    world.entity_mut(child).insert(px_box(50.0, 20.0));
    layout.run(&mut world);

    let layout = computed(&world, child);
    assert_eq!(
        (layout.size.width, layout.size.height),
        (50.0, 20.0),
        "後から付けた BoxStyle の大きさで配置されていない"
    );
    let arrangement = world.get::<Arrangement>(child).expect("Arrangement");
    assert_eq!(arrangement.size.width, 50.0);
}

/// 1 番: 親へ後から `BoxStyle` を付けても、親が根につながり、前から居る子も親につながる。
#[test]
fn box_style_added_to_parent_later_links_parent_and_children() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let middle = world.spawn(ChildOf(root)).id();
    let leaf = world.spawn((px_box(30.0, 10.0), ChildOf(middle))).id();
    layout.run(&mut world);

    world.entity_mut(middle).insert(BoxStyle {
        margin: Some(crate::ecs::layout::BoxMargin(crate::ecs::layout::Rect {
            left: crate::ecs::layout::LengthPercentageAuto::Px(15.0),
            right: crate::ecs::layout::LengthPercentageAuto::Px(0.0),
            top: crate::ecs::layout::LengthPercentageAuto::Px(0.0),
            bottom: crate::ecs::layout::LengthPercentageAuto::Px(0.0),
        })),
        ..px_box(100.0, 50.0)
    });
    layout.run(&mut world);

    let middle_layout = computed(&world, middle);
    assert_eq!(
        (
            middle_layout.location.x,
            middle_layout.size.width,
            middle_layout.size.height
        ),
        (15.0, 100.0, 50.0),
        "後から BoxStyle を付けた親が根の下で配置されていない"
    );
    let leaf_layout = computed(&world, leaf);
    assert_eq!(
        (leaf_layout.size.width, leaf_layout.size.height),
        (30.0, 10.0),
        "前から居た子が、後から BoxStyle を付けた親につながっていない"
    );
}

/// 1 番: 根でない部品の `BoxStyle` を外すと `TaffyStyle` が外れ、残った兄弟がその回のうちに詰まる。
/// `Arrangement` は最後の値のまま残る。
#[test]
fn box_style_removed_detaches_node_and_siblings_close_up() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let first = world.spawn((px_box(50.0, 20.0), ChildOf(root))).id();
    let second = world.spawn((px_box(50.0, 20.0), ChildOf(root))).id();
    layout.run(&mut world);
    assert_eq!(computed(&world, second).location.x, 50.0);

    world.entity_mut(first).remove::<BoxStyle>();
    layout.run(&mut world);

    assert!(
        world.get::<TaffyStyle>(first).is_none(),
        "BoxStyle を外しても TaffyStyle が残っている"
    );
    assert!(world.get::<TaffyComputedLayout>(first).is_none());
    assert_eq!(
        world.get::<Arrangement>(first).map(|a| a.size.width),
        Some(50.0),
        "Arrangement は最後の値のまま残る"
    );
    assert_eq!(
        computed(&world, second).location.x,
        0.0,
        "BoxStyle を外した兄弟の後ろが詰まっていない"
    );
}

/// 1 番: 根の `BoxStyle` を外すと、`TaffyStyle` は既定のスタイルへ戻る（`LayoutRoot` だけの部品と同じ）。
#[test]
fn box_style_removed_from_root_resets_to_default_style() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    layout.run(&mut world);

    world.entity_mut(root).remove::<BoxStyle>();
    layout.run(&mut world);

    let style = world
        .get::<TaffyStyle>(root)
        .expect("根の TaffyStyle は残る");
    assert_eq!(style.0, taffy::Style::default());
}

/// 2 番: 横に 3 つ並べた真ん中を消すと、その回のうちに 3 つ目が詰まる。
/// 消した後の次の回は計算しない（消えた記録の読み位置が進む）。
#[test]
fn despawned_middle_child_closes_up_once_then_no_recompute() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let first = world.spawn((px_box(50.0, 20.0), ChildOf(root))).id();
    let middle = world.spawn((px_box(50.0, 20.0), ChildOf(root))).id();
    let third = world.spawn((px_box(50.0, 20.0), ChildOf(root))).id();
    layout.run(&mut world);
    assert_eq!(computed(&world, third).location.x, 100.0);

    world.entity_mut(middle).despawn();
    layout.run(&mut world);
    assert_eq!(
        computed(&world, third).location.x,
        50.0,
        "真ん中を消した回のうちに 3 つ目が詰まっていない"
    );

    // ECS を通さずに taffy の節だけを変える。計算が走れば答えが変わる。
    {
        let mut taffy_res = world.resource_mut::<TaffyLayoutResource>();
        let node = taffy_res.get_node(first).expect("first の節");
        let mut style = taffy_res.taffy().style(node).unwrap().clone();
        style.size.width = taffy::Dimension::length(80.0);
        taffy_res.taffy_mut().set_style(node, style).unwrap();
    }
    layout.run(&mut world);
    assert_eq!(
        computed(&world, first).size.width,
        50.0,
        "何も変わっていない回に計算し直している（消えた記録を読み切っていない）"
    );
}

const ORPHAN_PARENT_WARN: &str = "Parent has no taffy node";
const ORPHAN_CHILDREN_WARN: &str = "BoxStyle removed while children still have BoxStyle";

/// 38 番: 中ほどの部品から `BoxStyle` を外すと、`BoxStyle` を持つ子が配置から外れる。
/// 配置の振る舞いは変えず、外した回に一度だけ実体名つきで warn を出す。
#[test]
fn box_style_removed_from_container_with_styled_children_warns_once() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let container = world
        .spawn((
            Name::new("orphan-container"),
            px_box(100.0, 50.0),
            ChildOf(root),
        ))
        .id();
    world.spawn((px_box(30.0, 10.0), ChildOf(container)));
    layout.run(&mut world);

    world.entity_mut(container).remove::<BoxStyle>();
    let log = capture_under_filter("warn", || {
        layout.run(&mut world);
        layout.run(&mut world);
    });
    assert_eq!(log.matches(ORPHAN_CHILDREN_WARN).count(), 1, "{log}");
    assert!(log.contains("orphan-container"), "実体名が要る: {log}");
}

/// 55 番: `BoxStyle` を外した回に `BoxStyle` 付きの子を足すと、その子もまだ `TaffyStyle` が無いが
/// 配置から外れるので warn を一度出す。
#[test]
fn box_style_removed_while_new_styled_child_spawned_warns_once() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let container = world
        .spawn((
            Name::new("orphan-container"),
            px_box(100.0, 50.0),
            ChildOf(root),
        ))
        .id();
    layout.run(&mut world);

    world.entity_mut(container).remove::<BoxStyle>();
    world.spawn((px_box(30.0, 10.0), ChildOf(container)));
    let log = capture_under_filter("warn", || {
        layout.run(&mut world);
        layout.run(&mut world);
    });
    assert_eq!(log.matches(ORPHAN_CHILDREN_WARN).count(), 1, "{log}");
    assert!(log.contains("orphan-container"), "実体名が要る: {log}");
}

/// 55 番: 部品と子の両方から同じ回に `BoxStyle` を外したら、配置から外れる子は居ないので warn は出ない。
#[test]
fn box_style_removed_from_container_and_child_together_does_not_warn() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let container = world.spawn((px_box(100.0, 50.0), ChildOf(root))).id();
    let child = world.spawn((px_box(30.0, 10.0), ChildOf(container))).id();
    layout.run(&mut world);

    world.entity_mut(container).remove::<BoxStyle>();
    world.entity_mut(child).remove::<BoxStyle>();
    let log = capture_under_filter("warn", || layout.run(&mut world));
    assert_eq!(log.matches(ORPHAN_CHILDREN_WARN).count(), 0, "{log}");
}

/// 38 番: 子の居ない部品から `BoxStyle` を外しても warn は出ない。
#[test]
fn box_style_removed_from_leaf_does_not_warn() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let leaf = world.spawn((px_box(30.0, 10.0), ChildOf(root))).id();
    layout.run(&mut world);

    world.entity_mut(leaf).remove::<BoxStyle>();
    let log = capture_under_filter("warn", || layout.run(&mut world));
    assert_eq!(log.matches(ORPHAN_CHILDREN_WARN).count(), 0, "{log}");
}

/// 38 番: `BoxStyle` の無い親の下に置いた部品は配置されない。節を作った回に一度だけ warn を出す。
/// 親も同じ回に節を作るなら（根と子を一度に作る）出さない。
#[test]
fn node_under_parent_without_taffy_node_warns_once() {
    let mut world = new_world();
    let mut layout = layout_schedule();
    let root = spawn_row_root(&mut world);
    let plain = world.spawn((Name::new("plain-parent"), ChildOf(root))).id();
    world.spawn((px_box(30.0, 10.0), ChildOf(root)));
    world.spawn((px_box(30.0, 10.0), ChildOf(plain)));
    let log = capture_under_filter("warn", || {
        layout.run(&mut world);
        layout.run(&mut world);
    });
    assert_eq!(log.matches(ORPHAN_PARENT_WARN).count(), 1, "{log}");
    assert!(log.contains("plain-parent"), "親の実体名が要る: {log}");
}
