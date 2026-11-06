//! `build` の純粋なテスト（工場だけ・装置なし・要件 1.1, 1.3, 1.5, 2.4〜2.8, 4.5, 4.6, 5.1, 5.2, 5.5〜5.7, 6.7）。
//! `fill_world_visuals` で埋めた後の値から列を組む（本番と同じ順）。

use super::super::draw_content_test_support::solid_rect_content;
use super::super::*;
use super::builder::*;
use crate::ecs::graphics::ClipShape;
use crate::ecs::layout::Arrangement;
use crate::ecs::test_support::capture_under_filter;
use crate::numerics::*;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::system::SystemState;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::core::Interface;
use windows_numerics::Matrix3x2;

const INFINITE: Aabb = Aabb::new(
    f32::NEG_INFINITY,
    f32::NEG_INFINITY,
    f32::INFINITY,
    f32::INFINITY,
);

fn rect4() -> D2D_RECT_F {
    D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 4.0,
        bottom: 4.0,
    }
}

fn factory() -> ID2D1Factory {
    unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None) }.unwrap()
}

fn unit_rect() -> ID2D1Geometry {
    let factory = factory();
    let r = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 1.0,
        bottom: 1.0,
    };
    unsafe { factory.CreateRectangleGeometry(&r) }
        .unwrap()
        .cast()
        .unwrap()
}

fn at(x: f32, y: f32, w: f32, h: f32) -> Arrangement {
    Arrangement {
        offset: crate::ecs::Offset { x, y },
        size: crate::ecs::Size {
            width: w,
            height: h,
        },
        ..Default::default()
    }
}

fn clipped() -> Visual {
    Visual {
        clip: Some(ClipShape::Rectangle),
        ..Default::default()
    }
}

fn rotated(rad: f32) -> VisualTransform {
    VisualTransform(Transform2D {
        rotation: rad,
        ..Transform2D::identity()
    })
}

/// 中身つきの節（位置 (x, y)・大きさ 4×4）。
fn leaf(world: &mut World, parent: Entity, x: f32, y: f32) -> Entity {
    world
        .spawn((
            Visual::default(),
            at(x, y, 4.0, 4.0),
            solid_rect_content(rect4()),
            ChildOf(parent),
        ))
        .id()
}

/// 中身なしの節（変形だけ）。
fn group(world: &mut World, parent: Entity, visual: Visual, arr: Arrangement) -> Entity {
    world
        .spawn((
            visual,
            arr,
            VisualTransform(Transform2D::identity()),
            ChildOf(parent),
        ))
        .id()
}

fn fill(world: &mut World, root: Entity) -> Vec<(Entity, usize)> {
    let top = world
        .get::<Children>(root)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    let mut state: SystemState<Query<FillData>> = SystemState::new(world);
    let mut nodes = state.get_mut(world).unwrap();
    let mut visited = Vec::new();
    fill_world_visuals(root, None, &mut nodes, false, &top, false, &mut visited);
    visited
}

fn build_only(
    world: &mut World,
    unit: &ID2D1Geometry,
    viewport: Aabb,
    visited: &[(Entity, usize)],
) -> Vec<DrawCommand> {
    let mut state: SystemState<Query<NodeData>> = SystemState::new(world);
    let nodes = state.get(world).unwrap();
    let mut out = vec![DrawCommand::PopClipRect]; // 先頭で消されること
    let root = Entity::PLACEHOLDER;
    build(
        &mut out,
        &factory(),
        unit,
        viewport,
        &nodes,
        root,
        None,
        visited,
    );
    out
}

fn flatten(world: &mut World, root: Entity, viewport: Aabb) -> Vec<DrawCommand> {
    let visited = fill(world, root);
    build_only(world, &unit_rect(), viewport, &visited)
}

/// 比べやすい形（命令の種類・実体・世界の外接矩形）。
fn shape(cmds: &[DrawCommand]) -> Vec<String> {
    cmds.iter()
        .map(|c| match c {
            DrawCommand::Draw(d) => format!("draw {:?} {:?}", d.entity, d.world_aabb),
            DrawCommand::PushClipRect(r) => format!("push_rect {:?}", r.world_aabb),
            DrawCommand::PopClipRect => "pop_rect".into(),
            DrawCommand::PushClipGeometry(g) => format!("push_geom {:?}", g.world_aabb),
            DrawCommand::PopClipGeometry => "pop_geom".into(),
        })
        .collect()
}

fn draw(e: Entity, l: f32, t: f32, r: f32, b: f32) -> String {
    format!("draw {:?} {:?}", e, Aabb::new(l, t, r, b))
}

fn push_rect(l: f32, t: f32, r: f32, b: f32) -> String {
    format!("push_rect {:?}", Aabb::new(l, t, r, b))
}

#[test]
fn siblings_in_children_order_parent_before_child() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = leaf(&mut world, root, 0.0, 0.0);
    let b = leaf(&mut world, a, 1.0, 0.0);
    let c = leaf(&mut world, root, 10.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(
        shape(&out),
        vec![
            draw(a, 0.0, 0.0, 4.0, 4.0),
            draw(b, 1.0, 0.0, 5.0, 4.0),
            draw(c, 10.0, 0.0, 14.0, 4.0),
        ]
    );
}

#[test]
fn node_without_content_emits_nothing_but_its_children() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let n = group(&mut world, root, Visual::default(), at(5.0, 5.0, 0.0, 0.0));
    let c = leaf(&mut world, n, 1.0, 1.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(shape(&out), vec![draw(c, 6.0, 6.0, 10.0, 10.0)]);
}

#[test]
fn clip_push_pop_pairs_and_nests_with_depth_returns() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = group(&mut world, root, clipped(), at(0.0, 0.0, 10.0, 10.0));
    let b = group(&mut world, a, clipped(), at(2.0, 2.0, 5.0, 5.0));
    let c = leaf(&mut world, b, 0.0, 0.0);
    let d = leaf(&mut world, a, 6.0, 6.0); // a の中・b の外（深さが 3 → 2 へ戻る）
    let e = leaf(&mut world, root, 20.0, 0.0); // 深さが 2 → 1 へ戻る
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(
        shape(&out),
        vec![
            push_rect(0.0, 0.0, 10.0, 10.0),
            push_rect(2.0, 2.0, 7.0, 7.0),
            draw(c, 2.0, 2.0, 6.0, 6.0),
            "pop_rect".into(),
            draw(d, 6.0, 6.0, 10.0, 10.0),
            "pop_rect".into(),
            draw(e, 20.0, 0.0, 24.0, 4.0),
        ]
    );
}

#[test]
fn trailing_open_clips_are_closed_at_the_end() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = group(&mut world, root, clipped(), at(0.0, 0.0, 10.0, 10.0));
    let b = group(&mut world, a, clipped(), at(0.0, 0.0, 10.0, 10.0));
    let c = leaf(&mut world, b, 0.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(
        shape(&out),
        vec![
            push_rect(0.0, 0.0, 10.0, 10.0),
            push_rect(0.0, 0.0, 10.0, 10.0),
            draw(c, 0.0, 0.0, 4.0, 4.0),
            "pop_rect".into(),
            "pop_rect".into(),
        ]
    );
}

#[test]
fn rotated_rect_clip_is_raised_to_unit_rect_geometry() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((
            clipped(),
            at(0.0, 0.0, 10.0, 10.0),
            rotated(std::f32::consts::FRAC_PI_4),
            ChildOf(root),
        ))
        .id();
    leaf(&mut world, a, 0.0, 0.0);
    let visited = fill(&mut world, root);
    let unit = unit_rect();
    let out = build_only(&mut world, &unit, INFINITE, &visited);
    let world_a = world.get::<WorldVisual>(a).unwrap().world_mat;
    let DrawCommand::PushClipGeometry(g) = &out[0] else {
        panic!("図形の push でない: {:?}", shape(&out));
    };
    assert_eq!(g.geometry, unit);
    assert_eq!(
        g.world_mat,
        unit_to_rect(&Aabb::new(0.0, 0.0, 10.0, 10.0)) * world_a
    );
    assert!(matches!(out[1], DrawCommand::Draw(_)));
    assert!(matches!(out[2], DrawCommand::PopClipGeometry));
    assert_eq!(out.len(), 3);
}

#[test]
fn quarter_turn_rect_clip_stays_axis_aligned_push() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    world.spawn((
        clipped(),
        at(0.0, 0.0, 10.0, 4.0),
        rotated(std::f32::consts::FRAC_PI_2),
        ChildOf(root),
    ));
    let out = flatten(&mut world, root, INFINITE);
    assert!(
        matches!(
            out.as_slice(),
            [DrawCommand::PushClipRect(_), DrawCommand::PopClipRect]
        ),
        "{:?}",
        shape(&out)
    );
}

/// 無限の viewport の列から、viewport の外の描く 1 件と、外の切り抜きの push〜対の pop を除く。
fn cull_by_hand(full: &[DrawCommand], viewport: Aabb) -> Vec<String> {
    let mut kept = Vec::new();
    let mut skip_depth = 0usize; // 外の切り抜きの中に居る間の入れ子の数
    for (c, s) in full.iter().zip(shape(full)) {
        let outside = |a: Aabb| a.intersect(viewport).is_empty();
        match c {
            DrawCommand::PushClipRect(_) | DrawCommand::PushClipGeometry(_) if skip_depth > 0 => {
                skip_depth += 1;
            }
            DrawCommand::PopClipRect | DrawCommand::PopClipGeometry if skip_depth > 0 => {
                skip_depth -= 1;
            }
            _ if skip_depth > 0 => {}
            DrawCommand::PushClipRect(r) if outside(r.world_aabb) => skip_depth = 1,
            DrawCommand::PushClipGeometry(g) if outside(g.world_aabb) => skip_depth = 1,
            DrawCommand::Draw(d) if outside(d.world_aabb) => {}
            _ => kept.push(s),
        }
    }
    kept
}

#[test]
fn viewport_culling_equals_infinite_list_minus_culled_items() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = leaf(&mut world, root, 0.0, 0.0);
    leaf(&mut world, root, 100.0, 0.0); // 外
    let c = group(&mut world, root, clipped(), at(200.0, 0.0, 10.0, 10.0)); // 外の切り抜き
    leaf(&mut world, c, 0.0, 0.0);
    let n = group(
        &mut world,
        root,
        Visual::default(),
        at(300.0, 0.0, 0.0, 0.0),
    ); // 中身なし・外
    let back = leaf(&mut world, n, -290.0, 0.0); // 外の親の子でも中なら残る（切り抜きの無い節は刈らない）
    let e = leaf(&mut world, root, 48.0, 48.0); // 端で交わる
    let viewport = Aabb::new(0.0, 0.0, 50.0, 50.0);
    let full = flatten(&mut world, root, INFINITE);
    let culled = flatten(&mut world, root, viewport);
    assert_eq!(full.len(), 7, "{:?}", shape(&full));
    assert_eq!(shape(&culled), cull_by_hand(&full, viewport));
    assert_eq!(
        shape(&culled),
        vec![
            draw(a, 0.0, 0.0, 4.0, 4.0),
            draw(back, 10.0, 0.0, 14.0, 4.0),
            draw(e, 48.0, 48.0, 52.0, 52.0),
        ]
    );
}

#[test]
fn empty_viewport_emits_nothing() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    leaf(&mut world, root, 0.0, 0.0);
    let out = flatten(&mut world, root, Aabb::new(0.0, 0.0, 0.0, 0.0));
    assert!(out.is_empty(), "{:?}", shape(&out));
}

#[test]
fn hidden_node_and_its_subtree_are_not_in_the_list() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let hidden = world
        .spawn((
            Visual {
                is_visible: false,
                ..Default::default()
            },
            at(0.0, 0.0, 4.0, 4.0),
            solid_rect_content(rect4()),
            ChildOf(root),
        ))
        .id();
    leaf(&mut world, hidden, 0.0, 0.0);
    let c = leaf(&mut world, root, 10.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(shape(&out), vec![draw(c, 10.0, 0.0, 14.0, 4.0)]);
}

fn first_draw(out: &[DrawCommand]) -> &DrawItem {
    out.iter()
        .find_map(|c| match c {
            DrawCommand::Draw(d) => Some(d),
            _ => None,
        })
        .unwrap()
}

#[test]
fn hash_is_the_same_for_same_version_and_transform_regardless_of_com_address() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = leaf(&mut world, root, 3.0, 0.0);
    let unit = unit_rect();
    let visited = fill(&mut world, root);
    let first = build_only(&mut world, &unit, INFINITE, &visited);
    let h1 = first_draw(&first).hash;
    let list1 = first_draw(&first).command_list.as_raw();
    // 別の命令の列（別の番地）に差し替えても、版と行列が同じなら hash は同じ。
    world.entity_mut(a).insert(solid_rect_content(rect4()));
    let second = build_only(&mut world, &unit, INFINITE, &visited);
    assert_ne!(first_draw(&second).command_list.as_raw(), list1);
    assert_eq!(first_draw(&second).hash, h1);
    // 版か行列が変われば違う。
    let item = first_draw(&second);
    let v = world.get::<VisualVersion>(a).unwrap().0;
    let hash =
        |v: u64, m: Matrix3x2| hash_draw(a, v, item.interpolation_mode, item.composite_mode, &m);
    assert_eq!(hash(v, item.world_mat), h1);
    assert_ne!(hash(v + 1, item.world_mat), h1);
    assert_ne!(hash(v, Matrix3x2::translation(4.0, 0.0)), h1);
    // -0.0 と 0.0 は同じ行列（CalcHash の正規化）。
    let mut neg = item.world_mat;
    neg.M12 = -0.0;
    assert_eq!(hash(v, neg), h1);
}

fn with_clip(clip: ClipShape) -> Visual {
    Visual {
        clip: Some(clip),
        ..Default::default()
    }
}

/// 切り抜きの節（位置 (2, 3)・大きさ 10×10）と中身の子を置き、最初の push の図形を返す。
fn geometry_clip_of(clip: ClipShape) -> (ClipGeometry, Vec<DrawCommand>) {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = group(&mut world, root, with_clip(clip), at(2.0, 3.0, 10.0, 10.0));
    leaf(&mut world, a, 0.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    let DrawCommand::PushClipGeometry(g) = &out[0] else {
        panic!("図形の push でない: {:?}", shape(&out));
    };
    assert_eq!(g.world_mat, Matrix3x2::translation(2.0, 3.0));
    assert_eq!(g.world_aabb, Aabb::new(2.0, 3.0, 12.0, 13.0));
    assert!(matches!(out[1], DrawCommand::Draw(_)));
    assert!(matches!(out[2], DrawCommand::PopClipGeometry));
    assert_eq!(out.len(), 3);
    (g.clone(), out)
}

/// 局所の点が図形の内側か。
fn inside(g: &ID2D1Geometry, x: f32, y: f32) -> bool {
    let p = windows_numerics::Vector2 { X: x, Y: y };
    unsafe { g.FillContainsPoint(p, None, D2D1_DEFAULT_FLATTENING_TOLERANCE) }
        .unwrap()
        .as_bool()
}

#[test]
fn uniform_rounded_clip_is_a_path_geometry_of_the_arrangement_size() {
    let (g, _) = geometry_clip_of(ClipShape::rounded_rectangle(3.0));
    for (x, y) in [(0.3, 0.3), (9.7, 0.3), (9.7, 9.7), (0.3, 9.7)] {
        assert!(!inside(&g.geometry, x, y), "角の外 ({x}, {y})");
    }
    for (x, y) in [(5.0, 5.0), (1.5, 1.5), (5.0, 0.1), (0.1, 5.0)] {
        assert!(inside(&g.geometry, x, y), "内側 ({x}, {y})");
    }
}

#[test]
fn individual_rounded_clip_rounds_each_corner_by_its_own_radius() {
    // 右上だけ丸い。
    let (g, _) = geometry_clip_of(ClipShape::rounded_rectangle_individual(0.0, 4.0, 0.0, 0.0));
    assert!(!inside(&g.geometry, 9.7, 0.3), "右上の角の外");
    for (x, y) in [(0.3, 0.3), (9.7, 9.7), (0.3, 9.7)] {
        assert!(inside(&g.geometry, x, y), "丸くない角の内側 ({x}, {y})");
    }
}

/// 持ち主の WUC 側と同じ見た目か: 同じ大きさ・半径の D2D 角丸矩形と内外が一致する（2.4）。
/// 半径が辺の半分を超えると X と Y で別々に頭を打ち、角は楕円になる。
#[test]
fn rounded_clip_matches_d2d_rounded_rectangle_including_over_large_radius() {
    let f = factory();
    for (w, h, r) in [
        (100.0, 40.0, 30.0),
        (100.0, 40.0, 5.0),
        (40.0, 100.0, 200.0),
    ] {
        let ours = super::clip_geometry::rounded_rect(&f, w, h, r, r, r, r).unwrap();
        let d2d: ID2D1Geometry = unsafe {
            f.CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: w,
                    bottom: h,
                },
                radiusX: r,
                radiusY: r,
            })
        }
        .unwrap()
        .cast()
        .unwrap();
        let pts = [
            (21.0, 0.5),
            (0.5, 21.0),
            (w - 21.0, h - 0.5),
            (w - 0.5, 21.0),
            (0.5, 0.5),
            (3.0, 3.0),
            (w * 0.5, h * 0.5),
            (w * 0.5, 0.5),
            (0.5, h * 0.5),
        ];
        for (x, y) in pts {
            assert_eq!(
                inside(&ours, x, y),
                inside(&d2d, x, y),
                "{w}x{h} r={r} の ({x}, {y})"
            );
        }
    }
    let ours = super::clip_geometry::rounded_rect(&f, 100.0, 40.0, 30.0, 30.0, 30.0, 30.0).unwrap();
    assert!(!inside(&ours, 21.0, 0.5), "楕円の角の外");
}

#[test]
fn ellipse_geometry_clip_is_used_as_is() {
    let ellipse: ID2D1Geometry = unsafe {
        factory().CreateEllipseGeometry(&D2D1_ELLIPSE {
            point: windows_numerics::Vector2 { X: 5.0, Y: 5.0 },
            radiusX: 5.0,
            radiusY: 5.0,
        })
    }
    .unwrap()
    .cast()
    .unwrap();
    let (g, _) = geometry_clip_of(ClipShape::Geometry(ellipse.clone()));
    assert_eq!(g.geometry, ellipse);
}

const CLIP_FAIL_TEXT: &str = "切り抜きの図形を作れないか外接矩形を取れない";

#[test]
fn geometry_bounds_failure_logs_and_skips_the_subtree() {
    // 開いたまま閉じていない path geometry は GetBounds が D2DERR_WRONG_STATE（0x88990001）を返す。
    let path = unsafe { factory().CreatePathGeometry() }.unwrap();
    let _sink = unsafe { path.Open() }.unwrap();
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let clip = with_clip(ClipShape::Geometry(path.cast().unwrap()));
    let a = group(&mut world, root, clip, at(0.0, 0.0, 10.0, 10.0));
    world.entity_mut(a).insert(Name::new("broken-clip"));
    leaf(&mut world, a, 0.0, 0.0);
    let c = leaf(&mut world, root, 20.0, 0.0);
    let visited = fill(&mut world, root);
    let unit = unit_rect();
    let mut out = Vec::new();
    let log = capture_under_filter("error", || {
        out = build_only(&mut world, &unit, INFINITE, &visited)
    });
    assert_eq!(shape(&out), vec![draw(c, 20.0, 0.0, 24.0, 4.0)]);
    assert_eq!(log.matches(CLIP_FAIL_TEXT).count(), 1, "{log}");
    assert!(log.contains("broken-clip"), "実体名が要る: {log}");
    assert!(log.contains("0x88990001"), "HRESULT が要る: {log}");
}

#[test]
fn node_gone_between_fill_and_build_is_logged_and_skipped() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = leaf(&mut world, root, 0.0, 0.0);
    leaf(&mut world, a, 1.0, 0.0);
    let c = leaf(&mut world, root, 10.0, 0.0);
    let visited = fill(&mut world, root);
    world.entity_mut(a).remove::<Arrangement>();
    let unit = unit_rect();
    let mut out = Vec::new();
    let log = capture_under_filter("error", || {
        out = build_only(&mut world, &unit, INFINITE, &visited)
    });
    assert_eq!(shape(&out), vec![draw(c, 10.0, 0.0, 14.0, 4.0)]);
    assert_eq!(log.matches("[build]").count(), 1, "{log}");
}
