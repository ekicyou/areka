//! `fill_world_visuals` の純粋なテスト（装置なし・`World` だけ・要件 1.2〜1.11, 2.2, 3.9, 4.1〜4.4, 5.3, 5.4, 6.7）。

use super::draw_content_test_support::solid_rect_content;
use super::*;
use crate::ecs::common::MAX_ANCESTOR_DEPTH;
use crate::ecs::graphics::ClipShape;
use crate::ecs::graphics::VisualGraphics;
use crate::ecs::layout::Arrangement;
use crate::ecs::test_support::capture_under_filter;
use crate::numerics::*;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::system::SystemState;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows_numerics::{Matrix3x2, Vector2};

const DEPTH_TEXT: &str = "木の深さが上限を越えた";
const OWNER_TEXT: &str = "節の下の描画面の持ち主は支えない";
const OPACITY_TEXT: &str = "節の不透明度の層は";

fn rect4() -> D2D_RECT_F {
    D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 4.0,
        bottom: 4.0,
    }
}

fn arranged(x: f32, y: f32) -> Arrangement {
    Arrangement {
        offset: crate::ecs::Offset { x, y },
        ..Default::default()
    }
}

fn moved(x: f32, y: f32) -> VisualTransform {
    VisualTransform(Transform2D {
        offset: Vector2 { X: x, Y: y },
        ..Transform2D::identity()
    })
}

fn scaled(x: f32, y: f32) -> VisualTransform {
    VisualTransform(Transform2D {
        scale: Vector2 { X: x, Y: y },
        ..Transform2D::identity()
    })
}

/// 回をまたいで変化の検出を持つ係の代わり（`SystemState` が前の回の時刻を、
/// `ReportedIssues` が前の回の誤りを覚える）。
struct Filler(
    SystemState<Query<'static, 'static, FillData>>,
    ReportedIssues,
);

impl Filler {
    fn new(world: &mut World) -> Self {
        Self(SystemState::new(world), ReportedIssues::default())
    }

    /// 根の子を `children_of_root` として 1 回埋める。
    fn run_with(
        &mut self,
        world: &mut World,
        root: Entity,
        root_arrangement_changed: bool,
        children_of_root: &[Entity],
        root_children_changed: bool,
    ) -> (bool, Vec<(Entity, usize)>) {
        let mut nodes = self.0.get_mut(world).unwrap();
        let mut visited = Vec::new();
        let changed = fill_world_visuals(
            root,
            Some(&Name::new("root")),
            &mut nodes,
            root_arrangement_changed,
            children_of_root,
            root_children_changed,
            &mut visited,
            &mut self.1,
        );
        self.1.end_round();
        (changed, visited)
    }

    fn run(&mut self, world: &mut World, root: Entity) -> bool {
        let top = kids(world, root);
        self.run_with(world, root, false, &top, false).0
    }
}

/// 根の子を `children_of_root` として 1 回埋める（初めての回＝全部が変化して見える）。
fn fill(
    world: &mut World,
    root: Entity,
    children_of_root: &[Entity],
) -> (bool, Vec<(Entity, usize)>) {
    Filler::new(world).run_with(world, root, false, children_of_root, false)
}

fn version(world: &World, e: Entity) -> u64 {
    world.get::<VisualVersion>(e).unwrap().0
}

fn kids(world: &World, e: Entity) -> Vec<Entity> {
    world
        .get::<Children>(e)
        .map(|c| c.to_vec())
        .unwrap_or_default()
}

fn wv(world: &World, e: Entity) -> WorldVisual {
    *world.get::<WorldVisual>(e).unwrap()
}

#[test]
fn two_levels_multiply_transform_then_offset() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((
            Visual::default(),
            moved(10.0, 20.0),
            arranged(1.0, 2.0),
            ChildOf(root),
        ))
        .id();
    let b = world
        .spawn((
            Visual::default(),
            scaled(2.0, 2.0),
            arranged(3.0, 0.0),
            ChildOf(a),
        ))
        .id();
    let mut filler = Filler::new(&mut world);
    let (changed, visited) = {
        let top = kids(&world, root);
        filler.run_with(&mut world, root, false, &top, false)
    };
    assert!(changed);
    assert_eq!(visited, vec![(a, 1), (b, 2)]);
    assert_eq!(wv(&world, a).world_mat, Matrix3x2::translation(11.0, 22.0));
    // 局所 = 拡大 × 平行移動(offset)（offset は拡大されない）× 親。
    let want = Matrix3x2 {
        M11: 2.0,
        M12: 0.0,
        M21: 0.0,
        M22: 2.0,
        M31: 14.0,
        M32: 22.0,
    };
    assert_eq!(wv(&world, b).world_mat, want);
    assert!(wv(&world, b).axis_aligned);
    // 同じ値なら書かない。
    assert!(!filler.run(&mut world, root));
}

#[test]
fn flip_gives_ordered_world_bounds() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let n = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            scaled(-1.0, 1.0),
            ChildOf(root),
        ))
        .id();
    {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    let w = wv(&world, n);
    assert!(w.axis_aligned);
    assert_eq!(w.content_world_aabb, Aabb::new(-4.0, 0.0, 0.0, 4.0));
}

#[test]
fn subtree_is_union_of_content_and_children_in_preorder() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let p = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            ChildOf(root),
        ))
        .id();
    let c1 = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            arranged(10.0, 10.0),
            ChildOf(p),
        ))
        .id();
    // 中身の無い節は和に何も足さない。
    let c2 = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(p)))
        .id();
    let (_, visited) = {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    assert_eq!(visited, vec![(p, 1), (c1, 2), (c2, 2)]);
    assert_eq!(
        wv(&world, p).world_mat,
        Matrix3x2::identity(),
        "変形なしは恒等"
    );
    assert_eq!(
        wv(&world, p).content_world_aabb,
        Aabb::new(0.0, 0.0, 4.0, 4.0)
    );
    assert_eq!(
        wv(&world, p).subtree_world_aabb,
        Aabb::new(0.0, 0.0, 14.0, 14.0)
    );
    assert_eq!(
        wv(&world, c1).subtree_world_aabb,
        Aabb::new(10.0, 10.0, 14.0, 14.0)
    );
    assert!(wv(&world, c2).subtree_world_aabb.is_empty());
}

#[test]
fn invisible_node_skips_its_subtree() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((Visual::default(), moved(1.0, 0.0), ChildOf(root)))
        .id();
    let hidden = world
        .spawn((
            Visual {
                is_visible: false,
                ..Default::default()
            },
            moved(2.0, 0.0),
            ChildOf(a),
        ))
        .id();
    let under = world
        .spawn((Visual::default(), moved(3.0, 0.0), ChildOf(hidden)))
        .id();
    let (_, visited) = {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    assert_eq!(visited, vec![(a, 1)]);
    assert_eq!(wv(&world, hidden).world_mat, Matrix3x2::identity());
    assert_eq!(wv(&world, under).world_mat, Matrix3x2::identity());
}

#[test]
fn depth_over_limit_stops_with_one_error() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let mut parent = root;
    for _ in 0..MAX_ANCESTOR_DEPTH + 5 {
        parent = world
            .spawn((Visual::default(), WorldVisual::default(), ChildOf(parent)))
            .id();
    }
    let top = kids(&world, root);
    let mut visited = Vec::new();
    let log = capture_under_filter("error", || visited = fill(&mut world, root, &top).1);
    assert_eq!(visited.len(), MAX_ANCESTOR_DEPTH);
    assert_eq!(visited.last().unwrap().1, MAX_ANCESTOR_DEPTH);
    assert_eq!(log.matches(DEPTH_TEXT).count(), 1, "{log}");
    assert!(log.contains("root"), "根の実体名が要る: {log}");
}

#[test]
fn parent_cycle_does_not_hang() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((Visual::default(), WorldVisual::default()))
        .id();
    let b = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(a)))
        .id();
    world.entity_mut(a).insert(ChildOf(b));
    let mut visited = Vec::new();
    let log = capture_under_filter("error", || visited = fill(&mut world, root, &[a]).1);
    assert_eq!(visited.len(), MAX_ANCESTOR_DEPTH);
    assert_eq!(log.matches(DEPTH_TEXT).count(), 1, "{log}");
}

#[test]
fn owner_under_node_is_cut_with_error_but_not_under_root() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let node = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(root)))
        .id();
    // 根の直下の持ち主は今の WUC の木の子なので記録しない。
    world.spawn((Visual::default(), Name::new("root-owner"), ChildOf(root)));
    let owner = world
        .spawn((
            Visual::default(),
            WorldVisual::default(),
            VisualGraphics::default(),
            Name::new("deep-owner"),
            ChildOf(node),
        ))
        .id();
    world.spawn((Visual::default(), WorldVisual::default(), ChildOf(owner)));
    let top = kids(&world, root);
    let mut visited = Vec::new();
    let log = capture_under_filter("error", || visited = fill(&mut world, root, &top).1);
    assert_eq!(visited, vec![(node, 1)]);
    assert_eq!(log.matches(OWNER_TEXT).count(), 1, "{log}");
    assert!(log.contains("deep-owner"), "実体名が要る: {log}");
    assert!(!log.contains("root-owner"), "{log}");
}

#[test]
fn owner_under_node_errors_once_while_it_persists() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let node = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(root)))
        .id();
    let owner = world
        .spawn((
            Visual::default(),
            WorldVisual::default(),
            VisualGraphics::default(),
            ChildOf(node),
        ))
        .id();
    let mut filler = Filler::new(&mut world);
    let log = capture_under_filter("error", || {
        filler.run(&mut world, root);
        filler.run(&mut world, root);
    });
    assert_eq!(log.matches(OWNER_TEXT).count(), 1, "続く間は 1 回: {log}");
    world.entity_mut(owner).remove::<VisualGraphics>();
    let log = capture_under_filter("error", || {
        filler.run(&mut world, root);
    });
    assert!(!log.contains(OWNER_TEXT), "{log}");
    world.entity_mut(owner).insert(VisualGraphics::default());
    let log = capture_under_filter("error", || {
        filler.run(&mut world, root);
    });
    assert_eq!(
        log.matches(OWNER_TEXT).count(),
        1,
        "戻ったらまた記録: {log}"
    );
}

#[test]
fn depth_over_limit_errors_once_while_it_persists() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let mut chain = vec![root];
    for _ in 0..MAX_ANCESTOR_DEPTH + 5 {
        let e = world
            .spawn((
                Visual::default(),
                WorldVisual::default(),
                ChildOf(*chain.last().unwrap()),
            ))
            .id();
        chain.push(e);
    }
    let mut filler = Filler::new(&mut world);
    let log = capture_under_filter("error", || {
        filler.run(&mut world, root);
        filler.run(&mut world, root);
    });
    assert_eq!(log.matches(DEPTH_TEXT).count(), 1, "続く間は 1 回: {log}");
    let cut = chain[3];
    world.entity_mut(cut).remove::<ChildOf>();
    let log = capture_under_filter("error", || {
        filler.run(&mut world, root);
    });
    assert!(!log.contains(DEPTH_TEXT), "{log}");
    world.entity_mut(cut).insert(ChildOf(chain[2]));
    let log = capture_under_filter("error", || {
        filler.run(&mut world, root);
    });
    assert_eq!(
        log.matches(DEPTH_TEXT).count(),
        1,
        "戻ったらまた記録: {log}"
    );
}

/// 中身を持つ節を 1 つ根の下に置き、1 回埋めて落ち着かせる。
fn settled_node(world: &mut World) -> (Filler, Entity, Entity) {
    let root = world.spawn(Visual::default()).id();
    let n = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            moved(1.0, 0.0),
            arranged(0.0, 0.0),
            ChildOf(root),
        ))
        .id();
    let mut filler = Filler::new(world);
    assert!(filler.run(world, root), "入った回は変化あり");
    assert!(!filler.run(world, root), "何も変わらない回は変化なし");
    (filler, root, n)
}

#[test]
fn transform_only_change_does_not_bump_version() {
    let mut world = World::new();
    let (mut filler, root, n) = settled_node(&mut world);
    let v0 = version(&world, n);
    *world.get_mut::<VisualTransform>(n).unwrap() = moved(5.0, 0.0);
    assert!(filler.run(&mut world, root), "行列の変化は変化あり");
    assert_eq!(version(&world, n), v0, "変形だけでは版を上げない");
}

#[test]
fn content_change_bumps_version_once() {
    let mut world = World::new();
    let (mut filler, root, n) = settled_node(&mut world);
    let v0 = version(&world, n);
    world.get_mut::<VisualDrawContent>(n).unwrap().set_changed();
    assert!(filler.run(&mut world, root));
    assert_eq!(version(&world, n), v0 + 1);
    assert!(!filler.run(&mut world, root));
    assert_eq!(version(&world, n), v0 + 1, "変わらない回は上げない");
}

#[test]
fn hiding_a_node_bumps_version_and_counts_as_change() {
    let mut world = World::new();
    let (mut filler, root, n) = settled_node(&mut world);
    let v0 = version(&world, n);
    world.get_mut::<Visual>(n).unwrap().is_visible = false;
    assert!(filler.run(&mut world, root));
    assert_eq!(version(&world, n), v0 + 1);
}

#[test]
fn root_arrangement_change_counts_without_bump() {
    let mut world = World::new();
    let (mut filler, root, n) = settled_node(&mut world);
    let v0 = version(&world, n);
    let top = kids(&world, root);
    assert!(filler.run_with(&mut world, root, true, &top, false).0);
    assert!(filler.run_with(&mut world, root, false, &top, true).0);
    assert_eq!(version(&world, n), v0, "根の配置・子の変化では版を上げない");
}

#[test]
fn clipped_node_arrangement_change_counts_without_bump() {
    let mut world = World::new();
    let (mut filler, root, n) = settled_node(&mut world);
    // 切り抜きの無い節の大きさの変化は木に効かない。
    world.get_mut::<Arrangement>(n).unwrap().size.width = 9.0;
    assert!(!filler.run(&mut world, root));
    world.get_mut::<Visual>(n).unwrap().clip = Some(ClipShape::Rectangle);
    filler.run(&mut world, root);
    let v0 = version(&world, n);
    world.get_mut::<Arrangement>(n).unwrap().size.width = 7.0;
    assert!(
        filler.run(&mut world, root),
        "切り抜きの大きさの変化は変化あり"
    );
    assert_eq!(version(&world, n), v0);
}

#[test]
fn new_child_counts_as_change() {
    let mut world = World::new();
    let (mut filler, root, n) = settled_node(&mut world);
    // 親の平行移動を打ち消し、世界の行列が初期値と同じ＝行列の差では気付けない子。
    world.spawn((Visual::default(), moved(-1.0, 0.0), ChildOf(n)));
    assert!(filler.run(&mut world, root));
    assert!(!filler.run(&mut world, root));
}

#[test]
fn node_opacity_warns_once_per_change() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let n = world
        .spawn((
            Visual {
                opacity: 0.5,
                ..Default::default()
            },
            solid_rect_content(rect4()),
            Name::new("half"),
            ChildOf(root),
        ))
        .id();
    let mut filler = Filler::new(&mut world);
    let log = capture_under_filter("warn", || {
        filler.run(&mut world, root);
        filler.run(&mut world, root);
    });
    assert_eq!(log.matches(OPACITY_TEXT).count(), 1, "{log}");
    assert!(log.contains("half"), "実体名が要る: {log}");
    world.get_mut::<Visual>(n).unwrap().opacity = 1.0;
    let log = capture_under_filter("warn", || {
        filler.run(&mut world, root);
    });
    assert!(!log.contains(OPACITY_TEXT), "1.0 は記録しない: {log}");
}

#[test]
fn content_change_while_hidden_bumps_version() {
    let mut world = World::new();
    let (mut filler, root, n) = settled_node(&mut world);
    let child = world
        .spawn((Visual::default(), solid_rect_content(rect4()), ChildOf(n)))
        .id();
    filler.run(&mut world, root);
    world.get_mut::<Visual>(n).unwrap().is_visible = false;
    filler.run(&mut world, root);
    let v0 = version(&world, child);
    world
        .get_mut::<VisualDrawContent>(child)
        .unwrap()
        .set_changed();
    filler.run(&mut world, root);
    world.get_mut::<Visual>(n).unwrap().is_visible = true;
    let top = kids(&world, root);
    let (_, visited) = filler.run_with(&mut world, root, false, &top, false);
    assert_eq!(
        version(&world, child),
        v0 + 1,
        "隠れている間の変化も版に残す"
    );
    assert_eq!(visited, vec![(n, 1), (child, 2)]);
}

#[test]
fn hidden_node_opacity_warns_once() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let hidden = world
        .spawn((
            Visual {
                is_visible: false,
                ..Default::default()
            },
            WorldVisual::default(),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual {
            opacity: 0.5,
            ..Default::default()
        },
        WorldVisual::default(),
        Name::new("half-hidden"),
        ChildOf(hidden),
    ));
    let mut filler = Filler::new(&mut world);
    let log = capture_under_filter("warn", || {
        filler.run(&mut world, root);
        filler.run(&mut world, root);
    });
    assert_eq!(log.matches(OPACITY_TEXT).count(), 1, "{log}");
    assert!(log.contains("half-hidden"), "実体名が要る: {log}");
}
