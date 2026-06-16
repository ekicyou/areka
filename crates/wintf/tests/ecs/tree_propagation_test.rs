//! W7b-T1: `ecs/common/tree_system.rs` ジェネリック階層伝播システムの特性化テスト
//!
//! 対象（すべて `crate::ecs::common::tree_system` の `pub` 関数。`ecs::*` から再エクスポート）:
//! - `sync_simple_transforms<L, G, M>` — 階層に属さないルート（`Without<ChildOf>` かつ
//!   `Without<Children>`）の `G` を `L.into()` で更新。`RemovedComponents<ChildOf>`（孤立）経路。
//! - `mark_dirty_trees<L, G, M>` — 変更エンティティから祖先へダーティビット（`M`）を伝播。
//! - `propagate_parent_transforms<L, G, M>` — ルートから子へ BFS で `G = parent_G * child_L` を伝播。
//!   静的サブツリー最適化（`set_if_neq` + `is_changed` ゲート）を含む。
//!
//! これらはジェネリックなため、具体型 `(Arrangement, GlobalArrangement, ArrangementTreeChanged)`
//! でインスタンス化して bevy `World` + `Schedule` 上で駆動する。Win32/GPU 非依存。
//! 期待値は `From<Arrangement> for GlobalArrangement` / `Mul<Arrangement> for GlobalArrangement`
//! の代数（`arrangement.rs`）から独立に導出する（特性化）。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::{
    Arrangement, ArrangementTreeChanged, GlobalArrangement, LayoutScale, Offset, Size,
    mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms,
};

// ---- ヘルパ ----

fn arr(x: f32, y: f32, sx: f32, sy: f32, w: f32, h: f32) -> Arrangement {
    Arrangement {
        offset: Offset { x, y },
        scale: LayoutScale { x: sx, y: sy },
        size: Size {
            width: w,
            height: h,
        },
    }
}

/// 伝播に必要な 3 コンポーネントをまとめてスポーンするバンドル。
/// （本番では `Arrangement::on_add` フックが `GlobalArrangement`/`ArrangementTreeChanged` を
/// 自動挿入するが、フックは `DeferredWorld` 経由のためここでは明示的に同梱して挙動を等価にする。）
fn node_bundle(a: Arrangement) -> (Arrangement, GlobalArrangement, ArrangementTreeChanged) {
    (a, GlobalArrangement::default(), ArrangementTreeChanged)
}

/// 本番 `arrangement_systems.rs` と同じ順序で 3 システムを直列実行するスケジュール。
fn propagation_schedule() -> Schedule {
    let mut s = Schedule::default();
    s.add_systems(
        (
            sync_simple_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>,
            mark_dirty_trees::<Arrangement, GlobalArrangement, ArrangementTreeChanged>,
            propagate_parent_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>,
        )
            .chain(),
    );
    s
}

fn global(world: &World, e: Entity) -> GlobalArrangement {
    *world.get::<GlobalArrangement>(e).unwrap()
}

// =============================================================================
// sync_simple_transforms: ルート（ChildOf なし・Children なし）の G 更新
// =============================================================================

/// 階層に属さない単一エンティティ（ルート）の `GlobalArrangement` が
/// `Arrangement.into()` で更新される（`sync_simple_transforms` の p0 経路）。
#[test]
fn sync_simple_updates_orphan_root_from_local() {
    let mut world = World::new();
    let e = world.spawn(node_bundle(arr(10.0, 20.0, 1.0, 1.0, 100.0, 50.0))).id();

    let mut s = Schedule::default();
    s.add_systems(sync_simple_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>);
    s.run(&mut world);

    // From<Arrangement>: translation*scale。scale=1 なので bounds = local_bounds + offset。
    let g = global(&world, e);
    assert_eq!(g.transform.M31, 10.0);
    assert_eq!(g.transform.M32, 20.0);
    assert_eq!(g.bounds.left, 10.0);
    assert_eq!(g.bounds.top, 20.0);
    assert_eq!(g.bounds.right, 110.0);
    assert_eq!(g.bounds.bottom, 70.0);
}

/// スケール付きルートでも `sync_simple_transforms` が `From` 代数どおりに G を設定する
/// （offset はスケール適用後 M31 = offset.x * scale.x）。
#[test]
fn sync_simple_applies_scale_to_root_global() {
    let mut world = World::new();
    let e = world.spawn(node_bundle(arr(10.0, 20.0, 2.0, 3.0, 100.0, 50.0))).id();

    let mut s = Schedule::default();
    s.add_systems(sync_simple_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>);
    s.run(&mut world);

    let g = global(&world, e);
    assert_eq!(g.transform.M11, 2.0);
    assert_eq!(g.transform.M22, 3.0);
    assert_eq!(g.transform.M31, 20.0); // 10 * 2
    assert_eq!(g.transform.M32, 60.0); // 20 * 3
    // bounds = local_bounds(0,0,100,50) を scale(2,3)+translate(20,60) で変換
    assert_eq!(g.bounds.left, 20.0);
    assert_eq!(g.bounds.top, 60.0);
    assert_eq!(g.bounds.right, 220.0); // 100*2 + 20
    assert_eq!(g.bounds.bottom, 210.0); // 50*3 + 60
}

/// `sync_simple_transforms` はルート（Children なし）のみを対象とし、
/// 子を持つエンティティ（`Without<Children>` で除外）には触れない。
#[test]
fn sync_simple_ignores_entity_with_children() {
    let mut world = World::new();
    let parent = world.spawn(node_bundle(arr(10.0, 20.0, 2.0, 2.0, 100.0, 50.0))).id();
    world.spawn((node_bundle(arr(0.0, 0.0, 1.0, 1.0, 10.0, 10.0)).0, ChildOf(parent)));

    let mut s = Schedule::default();
    s.add_systems(sync_simple_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>);
    s.run(&mut world);

    // parent は Children を持つため sync_simple_transforms の対象外 → G は既定（恒等）のまま。
    let g = global(&world, parent);
    assert_eq!(g.transform, GlobalArrangement::default().transform);
    assert_eq!(g.bounds.right, 0.0);
}

/// 孤立経路: `ChildOf` を除去されたエンティティ（`RemovedComponents<ChildOf>`）が
/// 次回 `sync_simple_transforms` でルートとして再同期される（p1 経路）。
#[test]
fn sync_simple_resyncs_orphaned_entity_via_removed_components() {
    let mut world = World::new();
    let parent = world.spawn(node_bundle(arr(0.0, 0.0, 2.0, 2.0, 100.0, 100.0))).id();
    let child = world
        .spawn((node_bundle(arr(5.0, 5.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(parent)))
        .id();

    let mut s = Schedule::default();
    s.add_systems(sync_simple_transforms::<Arrangement, GlobalArrangement, ArrangementTreeChanged>);

    // 初回: child は Children なし・ChildOf あり → p0/p1 いずれの対象でもなく未更新。
    s.run(&mut world);

    // child を親から切り離す（ChildOf 除去）。これで child はルートになり、
    // RemovedComponents<ChildOf> に拾われる。
    world.entity_mut(child).remove::<ChildOf>();
    s.run(&mut world);

    // 孤立経路で child の G が child 自身の Arrangement.into() に再同期される。
    let g = global(&world, child);
    assert_eq!(g.bounds.left, 5.0);
    assert_eq!(g.bounds.top, 5.0);
    assert_eq!(g.bounds.right, 15.0);
    assert_eq!(g.bounds.bottom, 15.0);
}

// =============================================================================
// mark_dirty_trees: 変更エンティティから祖先へのダーティビット伝播
// =============================================================================

/// 深いリーフの `Arrangement` 変更が `mark_dirty_trees` で祖先（ルート含む）の
/// `ArrangementTreeChanged` を `Changed` にする（祖先方向への伝播）。
/// ルートの M が Changed になることを検出システムで観測する。
#[test]
fn mark_dirty_propagates_change_from_leaf_to_root() {
    let mut world = World::new();
    let root = world.spawn(node_bundle(arr(0.0, 0.0, 1.0, 1.0, 100.0, 100.0))).id();
    let mid = world
        .spawn((node_bundle(arr(0.0, 0.0, 1.0, 1.0, 50.0, 50.0)), ChildOf(root)))
        .id();
    let leaf = world
        .spawn((node_bundle(arr(0.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(mid)))
        .id();

    // 変更追跡をリセットするため一度 tick を進める。
    let mut clear = Schedule::default();
    clear.add_systems(|_q: Query<&ArrangementTreeChanged>| {});
    clear.run(&mut world);
    world.clear_trackers();

    // リーフの Arrangement を変更。
    world.get_mut::<Arrangement>(leaf).unwrap().offset.x = 7.0;

    // mark_dirty_trees のみ実行。
    let mut s = Schedule::default();
    s.add_systems(mark_dirty_trees::<Arrangement, GlobalArrangement, ArrangementTreeChanged>);
    s.run(&mut world);

    // ルートの M が Changed になっているか検出（リーフ変更が祖先 root まで伝播）。
    world.insert_resource(DirtyProbe(false));
    let root_id = root;
    let mut detect = Schedule::default();
    detect.add_systems(
        move |q: Query<(Entity, Ref<ArrangementTreeChanged>)>, mut done: ResMut<DirtyProbe>| {
            for (e, m) in q.iter() {
                if e == root_id && m.is_changed() {
                    done.0 = true;
                }
            }
        },
    );
    detect.run(&mut world);

    assert!(
        world.resource::<DirtyProbe>().0,
        "leaf の Arrangement 変更が mark_dirty_trees で root の ArrangementTreeChanged まで伝播するはず"
    );
}

#[derive(Resource)]
struct DirtyProbe(bool);

/// `mark_dirty_trees` は `ChildOf` 変更（再ペアレント）でもダーティビットを伝播する
/// （`Changed<ChildOf>` 経路）。新しい親の祖先チェーンが Changed になることを観測する。
#[test]
fn mark_dirty_reacts_to_childof_change() {
    let mut world = World::new();
    let root_a = world.spawn(node_bundle(arr(0.0, 0.0, 1.0, 1.0, 100.0, 100.0))).id();
    let root_b = world.spawn(node_bundle(arr(0.0, 0.0, 1.0, 1.0, 100.0, 100.0))).id();
    let child = world
        .spawn((node_bundle(arr(0.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(root_a)))
        .id();

    // 一度実行して変更追跡をクリア。
    let mut s = propagation_schedule();
    s.run(&mut world);
    world.clear_trackers();

    // child を root_b へ再ペアレント（ChildOf 変更）。
    world.entity_mut(child).insert(ChildOf(root_b));

    let mut md = Schedule::default();
    md.add_systems(mark_dirty_trees::<Arrangement, GlobalArrangement, ArrangementTreeChanged>);
    md.run(&mut world);

    // 新しい親 root_b の M が Changed になっている（child の ChildOf 変更が祖先へ伝播）。
    world.insert_resource(DirtyProbe(false));
    let target = root_b;
    let mut detect = Schedule::default();
    detect.add_systems(
        move |q: Query<(Entity, Ref<ArrangementTreeChanged>)>, mut done: ResMut<DirtyProbe>| {
            for (e, m) in q.iter() {
                if e == target && m.is_changed() {
                    done.0 = true;
                }
            }
        },
    );
    detect.run(&mut world);
    assert!(
        world.resource::<DirtyProbe>().0,
        "ChildOf 変更が新しい親 root_b の ArrangementTreeChanged まで伝播するはず"
    );
}

// =============================================================================
// propagate_parent_transforms: ルート→子の BFS グローバル変換伝播
// =============================================================================

/// 2 階層（ルート→子）でルートのスケールが子の bounds に積算される。
/// `Mul<Arrangement>` 代数（scaled_offset = offset * parent_scale）と一致する。
#[test]
fn propagate_two_level_applies_parent_scale() {
    let mut world = World::new();
    let root = world.spawn(node_bundle(arr(0.0, 0.0, 2.0, 2.0, 100.0, 100.0))).id();
    let child = world
        .spawn((node_bundle(arr(10.0, 10.0, 1.0, 1.0, 30.0, 20.0)), ChildOf(root)))
        .id();

    let mut s = propagation_schedule();
    s.run(&mut world);

    // root（伝播ルート）の G も Arrangement.into() に更新される。
    let rg = global(&world, root);
    assert_eq!(rg.bounds.right, 200.0); // 100 * 2
    assert_eq!(rg.bounds.bottom, 200.0);

    // child: parent_scale=2 → scaled_offset=(20,20)、result_scale=2 → size*2=(60,40)
    let cg = global(&world, child);
    assert_eq!(cg.bounds.left, 20.0);
    assert_eq!(cg.bounds.top, 20.0);
    assert_eq!(cg.bounds.right, 80.0); // 20 + 30*2
    assert_eq!(cg.bounds.bottom, 60.0); // 20 + 20*2
}

/// 3 階層（ルート→子→孫）で累積変換が孫まで伝播する。
/// hierarchical_bounds_test の代数値（root2x → child → grandchild）と一致。
#[test]
fn propagate_three_level_chain_accumulates() {
    let mut world = World::new();
    // root: offset(10,20) scale(2,2) size(100,50)
    let root = world.spawn(node_bundle(arr(10.0, 20.0, 2.0, 2.0, 100.0, 50.0))).id();
    // child: offset(5,10) scale(1,1) size(30,20)
    let child = world
        .spawn((node_bundle(arr(5.0, 10.0, 1.0, 1.0, 30.0, 20.0)), ChildOf(root)))
        .id();
    // grandchild: offset(2,3) scale(1.5,1.5) size(10,8)
    let grandchild = world
        .spawn((node_bundle(arr(2.0, 3.0, 1.5, 1.5, 10.0, 8.0)), ChildOf(child)))
        .id();

    let mut s = propagation_schedule();
    s.run(&mut world);

    // child: parent.bounds.left=20、scaled_offset=(5*2,10*2)=(10,20)、result_scale=2
    let cg = global(&world, child);
    assert_eq!(cg.bounds.left, 30.0);
    assert_eq!(cg.bounds.top, 60.0);
    assert_eq!(cg.bounds.right, 90.0); // 30 + 30*2
    assert_eq!(cg.bounds.bottom, 100.0); // 60 + 20*2

    // grandchild: parent(child).bounds.left=30、child の累積 scale=2、
    // scaled_offset=(2*2,3*2)=(4,6)、result_scale=2*1.5=3
    let gg = global(&world, grandchild);
    assert_eq!(gg.bounds.left, 34.0);
    assert_eq!(gg.bounds.top, 66.0);
    assert_eq!(gg.bounds.right, 64.0); // 34 + 10*3
    assert_eq!(gg.bounds.bottom, 90.0); // 66 + 8*3
}

/// 幅広ツリー: ルートの複数の子がそれぞれ独立に正しい global を得る
/// （BFS が全 Children を漏れなく処理する）。
#[test]
fn propagate_wide_tree_processes_all_children() {
    let mut world = World::new();
    let root = world.spawn(node_bundle(arr(0.0, 0.0, 2.0, 2.0, 100.0, 100.0))).id();
    let c1 = world
        .spawn((node_bundle(arr(1.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(root)))
        .id();
    let c2 = world
        .spawn((node_bundle(arr(0.0, 2.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(root)))
        .id();
    let c3 = world
        .spawn((node_bundle(arr(3.0, 4.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(root)))
        .id();

    let mut s = propagation_schedule();
    s.run(&mut world);

    // 各子: scaled_offset = offset * 2、size*2 = 20
    let g1 = global(&world, c1);
    assert_eq!(g1.bounds.left, 2.0); // 1*2
    assert_eq!(g1.bounds.right, 22.0); // 2 + 20

    let g2 = global(&world, c2);
    assert_eq!(g2.bounds.top, 4.0); // 2*2
    assert_eq!(g2.bounds.bottom, 24.0);

    let g3 = global(&world, c3);
    assert_eq!(g3.bounds.left, 6.0); // 3*2
    assert_eq!(g3.bounds.top, 8.0); // 4*2
}

/// 静的サブツリー最適化: 収束後に変更なしで再実行しても global は不変
/// （`set_if_neq` + `Changed<M>` ゲートにより伝播がスキップされる）。
#[test]
fn propagate_is_idempotent_when_nothing_changes() {
    let mut world = World::new();
    let root = world.spawn(node_bundle(arr(0.0, 0.0, 2.0, 2.0, 100.0, 100.0))).id();
    let child = world
        .spawn((node_bundle(arr(10.0, 10.0, 1.0, 1.0, 30.0, 20.0)), ChildOf(root)))
        .id();

    let mut s = propagation_schedule();
    s.run(&mut world);
    let c_first = global(&world, child);
    let r_first = global(&world, root);

    // 変更追跡をクリアして再実行（何も変えない）。
    world.clear_trackers();
    s.run(&mut world);

    assert_eq!(global(&world, child), c_first, "再実行で child global は不変");
    assert_eq!(global(&world, root), r_first, "再実行で root global は不変");
}

/// 片方のブランチのみ変更しても、もう片方の兄弟ブランチの global は破壊されない。
#[test]
fn propagate_single_branch_change_preserves_sibling() {
    let mut world = World::new();
    let root = world.spawn(node_bundle(arr(0.0, 0.0, 1.0, 1.0, 100.0, 100.0))).id();
    let left = world
        .spawn((node_bundle(arr(10.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(root)))
        .id();
    let right = world
        .spawn((node_bundle(arr(50.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(root)))
        .id();

    let mut s = propagation_schedule();
    s.run(&mut world);
    let right_before = global(&world, right);
    assert_eq!(right_before.bounds.left, 50.0);

    // left のみ変更。
    world.clear_trackers();
    world.get_mut::<Arrangement>(left).unwrap().offset.x = 25.0;
    s.run(&mut world);

    // left は更新、right は不変。
    assert_eq!(global(&world, left).bounds.left, 25.0, "left は新 offset を反映");
    assert_eq!(global(&world, right), right_before, "right（兄弟）は不変");
}

/// 再ペアレント: 子を別の親へ移すと、新しい親の累積変換で再計算される。
#[test]
fn propagate_recomputes_child_after_reparent() {
    let mut world = World::new();
    // root_a: scale 2、root_b: scale 3
    let root_a = world.spawn(node_bundle(arr(0.0, 0.0, 2.0, 2.0, 100.0, 100.0))).id();
    let root_b = world.spawn(node_bundle(arr(0.0, 0.0, 3.0, 3.0, 100.0, 100.0))).id();
    let child = world
        .spawn((node_bundle(arr(10.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(root_a)))
        .id();

    let mut s = propagation_schedule();
    s.run(&mut world);
    // root_a(scale2) 下: scaled_offset=20, size*2=20 → left=20, right=40
    assert_eq!(global(&world, child).bounds.left, 20.0);
    assert_eq!(global(&world, child).bounds.right, 40.0);

    // root_b へ再ペアレント。
    world.clear_trackers();
    world.entity_mut(child).insert(ChildOf(root_b));
    s.run(&mut world);

    // root_b(scale3) 下: scaled_offset=30, size*3=30 → left=30, right=60
    let g = global(&world, child);
    assert_eq!(g.bounds.left, 30.0, "再ペアレント後は新親 root_b の scale で再計算");
    assert_eq!(g.bounds.right, 60.0);
}

/// 単一ルート（子なし）は `propagate_parent_transforms` の roots クエリ（`&Children` 必須）に
/// 当たらないが、`sync_simple_transforms` 経路で global が設定される（3 システム協調）。
#[test]
fn propagate_full_pipeline_handles_childless_root() {
    let mut world = World::new();
    let e = world.spawn(node_bundle(arr(5.0, 7.0, 1.0, 1.0, 20.0, 30.0))).id();

    let mut s = propagation_schedule();
    s.run(&mut world);

    // sync_simple_transforms が処理する（Without<Children>）。
    let g = global(&world, e);
    assert_eq!(g.bounds.left, 5.0);
    assert_eq!(g.bounds.top, 7.0);
    assert_eq!(g.bounds.right, 25.0);
    assert_eq!(g.bounds.bottom, 37.0);
}

/// 深い 4 階層チェーンで `propagate_descendants_unchecked` の DFS ループが
/// 全階層を 1 ルートの伝播で消化する（max_depth 経路の連続反復）。
#[test]
fn propagate_deep_chain_propagates_all_levels() {
    let mut world = World::new();
    // すべて scale 1、各レベル offset +10 だけずらす。
    let a = world.spawn(node_bundle(arr(10.0, 0.0, 1.0, 1.0, 10.0, 10.0))).id();
    let b = world
        .spawn((node_bundle(arr(10.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(a)))
        .id();
    let c = world
        .spawn((node_bundle(arr(10.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(b)))
        .id();
    let d = world
        .spawn((node_bundle(arr(10.0, 0.0, 1.0, 1.0, 10.0, 10.0)), ChildOf(c)))
        .id();

    let mut s = propagation_schedule();
    s.run(&mut world);

    // scale すべて 1 なので bounds.left は offset の累積和。
    assert_eq!(global(&world, a).bounds.left, 10.0);
    assert_eq!(global(&world, b).bounds.left, 20.0);
    assert_eq!(global(&world, c).bounds.left, 30.0);
    assert_eq!(global(&world, d).bounds.left, 40.0);
    // 最深ノードの右端も累積（40 + size10）。
    assert_eq!(global(&world, d).bounds.right, 50.0);
}
