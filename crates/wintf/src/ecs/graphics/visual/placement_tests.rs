//! 置き場を WUC へ写す純粋な補助のテスト（要件 4.3, 4.4, 4.6, 4.8, 5.1, 8.4）。
//! 親の決め・子の並び・変形・切り抜きの合わせの各場合。

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::{Matrix3x2, Vector3};

use super::placement::*;
use crate::ecs::graphics::ClipShape;
use crate::numerics::Aabb;

/// 実体の番号から木を組む小さな足場（親・子の並び・種類）。
#[derive(Default)]
struct Tree {
    parent: HashMap<Entity, Entity>,
    children: HashMap<Entity, Vec<Entity>>,
    class: HashMap<Entity, ChildClass>,
}

fn e(n: u32) -> Entity {
    Entity::from_raw_u32(n).unwrap()
}

impl Tree {
    fn node(mut self, n: u32, class: ChildClass, kids: &[u32]) -> Self {
        self.class.insert(e(n), class);
        let kids: Vec<Entity> = kids.iter().map(|&k| e(k)).collect();
        for &k in &kids {
            self.parent.insert(k, e(n));
        }
        self.children.insert(e(n), kids);
        self
    }
    fn host_of(&self, n: u32) -> HostAnswer {
        wuc_host_of(
            e(n),
            |x| self.parent.get(&x).copied(),
            |x| self.class.get(&x).copied().unwrap_or(ChildClass::Other),
        )
    }
    fn order(&self, host: u32) -> Vec<Entity> {
        wuc_children_order(
            e(host),
            |x| self.children.get(&x).cloned().unwrap_or_default(),
            |x| self.class.get(&x).copied().unwrap_or(ChildClass::Other),
        )
    }
}

use ChildClass::{AttachedAuto, DetachedAuto, Node, Other, Owner};

// --- 置き場の部品 ---

#[test]
fn placement_compares_by_value() {
    let embedded = |opacity| {
        SurfacePlacement(PlacementKind::Embedded {
            parent_world: Matrix3x2::translation(1.0, 2.0),
            opacity,
            visible: true,
            clip: PlacementClip::Rect(Aabb::new(0.0, 0.0, 4.0, 4.0)),
        })
    };
    let auto = SurfacePlacement(PlacementKind::Auto {
        origin_px: (3, -2),
        size_px: (10, 20),
    });
    // 値が変わったときだけ書く（1 回の判断）ための比べ
    assert_eq!(embedded(0.5), embedded(0.5));
    assert_ne!(embedded(0.5), embedded(0.25));
    assert_ne!(embedded(0.5), auto);
}

// --- 親の決め（wuc_host_of） ---

#[test]
fn host_is_the_direct_parent_owner() {
    let t = Tree::default().node(1, Owner, &[2]).node(2, Owner, &[]);
    assert_eq!(t.host_of(2), HostAnswer::Host(e(1)));
}

#[test]
fn host_passes_through_one_node() {
    let t = Tree::default()
        .node(1, Owner, &[2])
        .node(2, Node, &[3])
        .node(3, Owner, &[]);
    assert_eq!(t.host_of(3), HostAnswer::Host(e(1)));
}

#[test]
fn host_passes_through_deep_nodes() {
    let t = Tree::default()
        .node(1, Owner, &[2])
        .node(2, Node, &[3])
        .node(3, Node, &[4])
        .node(4, Node, &[5])
        .node(5, Owner, &[]);
    assert_eq!(t.host_of(5), HostAnswer::Host(e(1)));
}

#[test]
fn host_passes_through_a_preparing_auto_owner() {
    let t = Tree::default()
        .node(1, Owner, &[2])
        .node(2, DetachedAuto, &[3])
        .node(3, Node, &[4])
        .node(4, AttachedAuto, &[]);
    assert_eq!(t.host_of(4), HostAnswer::Host(e(1)));
}

#[test]
fn a_preparing_auto_owner_itself_is_detached() {
    let t = Tree::default()
        .node(1, Owner, &[2])
        .node(2, DetachedAuto, &[]);
    assert_eq!(t.host_of(2), HostAnswer::Detached);
}

#[test]
fn an_attached_auto_owner_hosts_under_its_nearest_owner() {
    let t = Tree::default()
        .node(1, Owner, &[2])
        .node(2, Node, &[3])
        .node(3, AttachedAuto, &[]);
    assert_eq!(t.host_of(3), HostAnswer::Host(e(1)));
}

#[test]
fn outside_the_window_is_detached() {
    // 祖先が Visual を持たない（窓の外）
    let t = Tree::default()
        .node(1, Other, &[2])
        .node(2, Node, &[3])
        .node(3, Owner, &[]);
    assert_eq!(t.host_of(3), HostAnswer::Detached);
    // 親が無い（根・窓そのもの）
    let t = Tree::default().node(1, Owner, &[]);
    assert_eq!(t.host_of(1), HostAnswer::Detached);
    // 節だけをたどって根に着く
    let t = Tree::default().node(1, Node, &[2]).node(2, Owner, &[]);
    assert_eq!(t.host_of(2), HostAnswer::Detached);
}

// --- 子の種類（child_class） ---

#[test]
fn child_class_follows_the_parts_and_the_phase() {
    use super::promotion::Phase::{Active, Node as NodePhase, Preparing};
    // (描画面の 3 部品, 節, 手動の目印, 相) → 種類
    let cases = [
        ((true, false, false, NodePhase), Owner), // 新しい描き方の部品を持たない持ち主
        ((true, true, true, NodePhase), Owner),   // 目印か命令の列を持つ節
        ((true, true, true, Active), Owner),      // 手動の目印が相より勝つ
        ((true, true, false, Active), AttachedAuto), // 上げた
        ((true, true, false, Preparing), DetachedAuto), // 用意中
        ((true, true, false, NodePhase), DetachedAuto), // 戻した直後（3 部品は次の回に外れる）
        ((false, true, false, Active), Node),     // 3 部品がまだ無い節
        ((false, true, false, NodePhase), Node),
        ((false, false, false, NodePhase), Other), // 見た目の部品を持たない（窓の外の根など）
    ];
    for ((g, n, m, p), want) in cases {
        assert_eq!(child_class(g, n, m, p), want, "{g} {n} {m} {p:?}");
    }
}

// --- 子の並び（wuc_children_order） ---

#[test]
fn order_passes_through_nodes_in_preorder() {
    let t = Tree::default()
        .node(1, Owner, &[2, 3, 6])
        .node(2, Owner, &[])
        .node(3, Node, &[4, 5])
        .node(4, Owner, &[])
        .node(5, Owner, &[])
        .node(6, Owner, &[]);
    assert_eq!(t.order(1), vec![e(2), e(4), e(5), e(6)]);
}

#[test]
fn order_does_not_descend_into_owners_or_other() {
    let t = Tree::default()
        .node(1, Owner, &[2, 4, 5])
        .node(2, Owner, &[3])
        .node(3, Owner, &[])
        .node(4, Other, &[6])
        .node(6, Owner, &[])
        .node(5, DetachedAuto, &[7])
        .node(7, Owner, &[]);
    // 2 の子 3 は 2 の下・Other の 4 は拾わず降りない・用意中の 5 は通り抜けて 7 を拾う
    assert_eq!(t.order(1), vec![e(2), e(7)]);
}

#[test]
fn auto_owners_go_below_all_manual_owners_in_reverse_preorder() {
    let t = Tree::default()
        .node(1, Owner, &[10, 2, 3, 12])
        .node(10, AttachedAuto, &[])
        .node(2, Owner, &[])
        .node(3, Node, &[11, 4])
        .node(11, AttachedAuto, &[])
        .node(4, Owner, &[])
        .node(12, AttachedAuto, &[]);
    // 手動の持ち主を前順で先に（最初が最上）、自動は前順の逆で後に（前順で後が上）
    assert_eq!(t.order(1), vec![e(2), e(4), e(12), e(11), e(10)]);
}

#[test]
fn reordering_children_reorders_the_answer() {
    let a = Tree::default()
        .node(1, Owner, &[2, 3])
        .node(2, Owner, &[])
        .node(3, Node, &[4])
        .node(4, Owner, &[]);
    let b = Tree::default()
        .node(1, Owner, &[3, 2])
        .node(2, Owner, &[])
        .node(3, Node, &[4])
        .node(4, Owner, &[]);
    assert_eq!(a.order(1), vec![e(2), e(4)]);
    assert_eq!(b.order(1), vec![e(4), e(2)]);
}

// --- 変形（wuc_transform） ---

#[test]
fn translation_only_becomes_an_offset_with_dpi() {
    let got = wuc_transform(Matrix3x2::translation(10.0, 20.0), 1.5);
    assert_eq!(
        got,
        WucTransform::Offset(Vector3 {
            X: 15.0,
            Y: 30.0,
            Z: 0.0
        })
    );
}

#[test]
fn rotation_becomes_a_matrix_with_scaled_translation() {
    let m = Matrix3x2::rotation(30.0) * Matrix3x2::translation(10.0, 20.0);
    let WucTransform::Matrix(got) = wuc_transform(m, 2.0) else {
        panic!("回転は行列になる");
    };
    assert_eq!(
        (got.M11, got.M12, got.M21, got.M22),
        (m.M11, m.M12, m.M21, m.M22)
    );
    assert_eq!((got.M41, got.M42, got.M43, got.M44), (20.0, 40.0, 0.0, 1.0));
    assert_eq!((got.M33, got.M13, got.M31), (1.0, 0.0, 0.0));
}

#[test]
fn scale_becomes_a_matrix_and_dpi_scales_only_the_translation() {
    let m = Matrix3x2::scale(2.0, 3.0) * Matrix3x2::translation(4.0, 5.0);
    let WucTransform::Matrix(got) = wuc_transform(m, 1.25) else {
        panic!("拡大は行列になる");
    };
    assert_eq!((got.M11, got.M22, got.M12, got.M21), (2.0, 3.0, 0.0, 0.0));
    assert_eq!((got.M41, got.M42), (5.0, 6.25));
}

// --- 持ち主へ写す値（owner_props） ---

fn embedded(parent_world: Matrix3x2, opacity: f32, visible: bool) -> PlacementKind {
    PlacementKind::Embedded {
        parent_world,
        opacity,
        visible,
        clip: PlacementClip::None,
    }
}

#[test]
fn an_owner_without_placement_keeps_the_offset_when_only_translated() {
    let got = owner_props(Matrix3x2::translation(10.0, 20.0), 0.5, 1.5, None);
    assert_eq!(
        got,
        OwnerProps {
            transform: WucTransform::Offset(Vector3 {
                X: 15.0,
                Y: 30.0,
                Z: 0.0
            }),
            opacity: 0.5,
            size_px: None,
        }
    );
}

#[test]
fn an_owner_without_placement_maps_its_visual_transform_as_a_matrix() {
    let local = Matrix3x2::rotation(30.0) * Matrix3x2::translation(10.0, 20.0);
    let got = owner_props(local, 1.0, 2.0, None);
    assert_eq!(got.transform, wuc_transform(local, 2.0));
    assert!(matches!(got.transform, WucTransform::Matrix(_)));
}

#[test]
fn an_owner_under_a_node_multiplies_the_parent_world_and_the_ancestor_opacity() {
    let place = embedded(Matrix3x2::translation(10.0, 20.0), 0.4, true);
    let got = owner_props(Matrix3x2::translation(5.0, 5.0), 0.5, 1.5, Some(&place));
    assert_eq!(
        got.transform,
        WucTransform::Offset(Vector3 {
            X: 22.5,
            Y: 37.5,
            Z: 0.0
        })
    );
    assert!((got.opacity - 0.2).abs() < 1e-6, "{}", got.opacity);
    assert_eq!(got.size_px, None);

    let world = Matrix3x2::rotation(30.0) * Matrix3x2::translation(100.0, 0.0);
    let local = Matrix3x2::translation(5.0, 0.0);
    let rotated = owner_props(local, 1.0, 1.25, Some(&embedded(world, 1.0, true)));
    assert_eq!(rotated.transform, wuc_transform(local * world, 1.25));
}

#[test]
fn an_owner_under_a_hidden_node_maps_opacity_zero() {
    let place = embedded(Matrix3x2::identity(), 1.0, false);
    let got = owner_props(Matrix3x2::identity(), 1.0, 1.0, Some(&place));
    assert_eq!(got.opacity, 0.0);
}

#[test]
fn an_auto_owner_maps_only_its_integer_origin_and_size() {
    let place = PlacementKind::Auto {
        origin_px: (13, -4),
        size_px: (40, 30),
    };
    let local = Matrix3x2::rotation(30.0) * Matrix3x2::translation(7.3, 2.1);
    let got = owner_props(local, 0.5, 1.25, Some(&place));
    assert_eq!(
        got,
        OwnerProps {
            transform: WucTransform::Offset(Vector3 {
                X: 13.0,
                Y: -4.0,
                Z: 0.0
            }),
            opacity: 1.0,
            size_px: Some((40, 30)),
        }
    );
}

// --- 切り抜きの合わせ（merged_clip） ---

const SIZE: (f32, f32) = (100.0, 50.0);

fn near(a: Aabb, b: Aabb) -> bool {
    [
        (a.left(), b.left()),
        (a.top(), b.top()),
        (a.right(), b.right()),
        (a.bottom(), b.bottom()),
    ]
    .iter()
    .all(|(x, y)| (x - y).abs() < 1e-3)
}

#[test]
fn no_placement_clip_keeps_the_own_clip() {
    let to = Matrix3x2::translation(10.0, 10.0);
    for own in [
        None,
        Some(ClipShape::Rectangle),
        Some(ClipShape::rounded_rectangle(4.0)),
    ] {
        assert_eq!(
            merged_clip(own.as_ref(), &PlacementClip::None, to, SIZE),
            ClipPlan::Own
        );
    }
}

#[test]
fn unsupported_placement_clip_stays_unsupported() {
    let to = Matrix3x2::translation(10.0, 10.0);
    assert_eq!(
        merged_clip(None, &PlacementClip::Unsupported, to, SIZE),
        ClipPlan::Unsupported
    );
}

#[test]
fn a_placement_clip_that_contains_the_owner_keeps_the_own_clip() {
    let to = Matrix3x2::translation(10.0, 10.0);
    let clip = PlacementClip::Rect(Aabb::new(0.0, 0.0, 200.0, 200.0));
    for own in [
        ClipShape::Rectangle,
        ClipShape::rounded_rectangle(4.0),
        ClipShape::rounded_rectangle_individual(1.0, 2.0, 3.0, 4.0),
    ] {
        assert_eq!(merged_clip(Some(&own), &clip, to, SIZE), ClipPlan::Own);
    }
}

#[test]
fn an_owner_without_its_own_clip_keeps_a_containing_placement_clip_for_its_children() {
    // WUC の切り抜きは子の持ち主にも効き、子は持ち主の外へ出られるので省かない
    let to = Matrix3x2::translation(10.0, 10.0);
    let clip = PlacementClip::Rect(Aabb::new(0.0, 0.0, 200.0, 200.0));
    let ClipPlan::Rect { local } = merged_clip(None, &clip, to, SIZE) else {
        panic!("局所の矩形になる");
    };
    assert!(
        near(local, Aabb::new(-10.0, -10.0, 190.0, 190.0)),
        "{local:?}"
    );

    let rotated = Matrix3x2::rotation(30.0) * Matrix3x2::translation(500.0, 500.0);
    let host = Aabb::new(0.0, 0.0, 1000.0, 1000.0);
    let ClipPlan::Transformed { host_rect, .. } =
        merged_clip(None, &PlacementClip::Rect(host), rotated, SIZE)
    else {
        panic!("回転は逆行列つきの矩形になる");
    };
    assert_eq!(host_rect, host);
}

#[test]
fn a_geometry_own_clip_never_drops_the_placement_clip() {
    // 図形は持ち主の範囲に収まるとは限らないので、含む置き場の切り抜きでも合わせられない
    let factory: ID2D1Factory1 =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None) }.unwrap();
    let rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 10.0,
        bottom: 10.0,
    };
    let g = unsafe { factory.CreateRectangleGeometry(&rect) }.unwrap();
    let own = ClipShape::Geometry(g.into());
    let to = Matrix3x2::translation(10.0, 10.0);
    for host in [
        Aabb::new(0.0, 0.0, 200.0, 200.0),
        Aabb::new(0.0, 30.0, 70.0, 100.0),
    ] {
        assert_eq!(
            merged_clip(Some(&own), &PlacementClip::Rect(host), to, SIZE),
            ClipPlan::Unsupported
        );
    }
}

#[test]
fn an_axis_aligned_placement_clip_maps_into_local() {
    // 局所 → 親の DIP は 2 倍して (10,10) へ。親の矩形 (30,20)-(70,60) は局所の (10,5)-(30,25)
    let to = Matrix3x2::scale(2.0, 2.0) * Matrix3x2::translation(10.0, 10.0);
    let clip = PlacementClip::Rect(Aabb::new(30.0, 20.0, 70.0, 60.0));
    let ClipPlan::Rect { local } = merged_clip(None, &clip, to, SIZE) else {
        panic!("軸に沿う矩形になる");
    };
    assert!(near(local, Aabb::new(10.0, 5.0, 30.0, 25.0)), "{local:?}");
}

#[test]
fn an_own_rectangle_intersects_with_the_placement_clip() {
    let to = Matrix3x2::translation(10.0, 10.0);
    // 親の矩形は局所の (-10,20)-(60,90)、自分の矩形 (0,0)-(100,50) と交わり (0,20)-(60,50)
    let clip = PlacementClip::Rect(Aabb::new(0.0, 30.0, 70.0, 100.0));
    let ClipPlan::Rect { local } = merged_clip(Some(&ClipShape::Rectangle), &clip, to, SIZE) else {
        panic!("交わりの矩形になる");
    };
    assert!(near(local, Aabb::new(0.0, 20.0, 60.0, 50.0)), "{local:?}");
}

#[test]
fn a_rounded_own_clip_cannot_merge_with_a_cutting_placement_clip() {
    let to = Matrix3x2::translation(10.0, 10.0);
    let clip = PlacementClip::Rect(Aabb::new(0.0, 30.0, 70.0, 100.0));
    let own = ClipShape::rounded_rectangle(4.0);
    assert_eq!(
        merged_clip(Some(&own), &clip, to, SIZE),
        ClipPlan::Unsupported
    );
}

#[test]
fn a_rotated_owner_gets_the_host_rect_with_the_inverse_matrix() {
    let to = Matrix3x2::rotation(30.0) * Matrix3x2::translation(50.0, 50.0);
    let host = Aabb::new(40.0, 40.0, 90.0, 90.0);
    let clip = PlacementClip::Rect(host);
    let ClipPlan::Transformed { host_rect, inverse } = merged_clip(None, &clip, to, SIZE) else {
        panic!("回転は逆行列つきの矩形になる");
    };
    assert_eq!(host_rect, host);
    let id = to * inverse;
    for (got, want) in [
        (id.M11, 1.0),
        (id.M12, 0.0),
        (id.M21, 0.0),
        (id.M22, 1.0),
        (id.M31, 0.0),
        (id.M32, 0.0),
    ] {
        assert!((got - want).abs() < 1e-4, "{id:?}");
    }
}

#[test]
fn a_rotated_owner_with_its_own_clip_cannot_merge() {
    let to = Matrix3x2::rotation(30.0) * Matrix3x2::translation(50.0, 50.0);
    let clip = PlacementClip::Rect(Aabb::new(40.0, 40.0, 90.0, 90.0));
    assert_eq!(
        merged_clip(Some(&ClipShape::Rectangle), &clip, to, SIZE),
        ClipPlan::Unsupported
    );
}

#[test]
fn a_degenerate_matrix_shows_nothing_and_keeps_the_own_clip() {
    // 潰れた行列では持ち主の範囲が空になり、切り抜くものが無い
    let to = Matrix3x2::scale(0.0, 1.0) * Matrix3x2::translation(500.0, 0.0);
    let clip = PlacementClip::Rect(Aabb::new(0.0, 0.0, 10.0, 10.0));
    assert_eq!(merged_clip(None, &clip, to, SIZE), ClipPlan::Own);
}
