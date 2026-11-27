//! `fill_world_visuals` の走査の列のテスト（装置なし・`World` だけ）。
//! 置き場の材料（親の節の世界の行列・可視・不透明度の積・矩形の切り抜きの交わり）・自動の持ち主の
//! 通り抜け・「この回の更新」が祖先の動きでは立たないことを確かめる。

use super::draw_content_test_support::solid_rect_content;
use super::promotion::{Phase, SurfaceReadiness, Verdict};
use super::world_visual_tests::{Filler, rect4};
use super::*;
use crate::ecs::graphics::ClipShape;
use crate::ecs::graphics::VisualGraphics;
use crate::ecs::layout::Arrangement;
use crate::numerics::*;
use bevy_ecs::hierarchy::ChildOf;
use windows::Win32::Graphics::Direct2D::Common::D2D1_BLEND_MODE_MULTIPLY;
use windows_numerics::{Matrix3x2, Vector2};

fn placed(x: f32, y: f32, w: f32, h: f32) -> Arrangement {
    Arrangement {
        offset: crate::ecs::Offset { x, y },
        size: crate::ecs::Size {
            width: w,
            height: h,
        },
        ..Default::default()
    }
}

fn moved(x: f32, y: f32) -> VisualTransform {
    VisualTransform(Transform2D {
        offset: Vector2 { X: x, Y: y },
        ..Transform2D::identity()
    })
}

fn entry(scan: &TreeScan, e: Entity) -> &ScanEntry {
    scan.entries.iter().find(|x| x.entity == e).unwrap()
}

fn updated(scan: &TreeScan) -> Vec<(Entity, bool)> {
    scan.entries.iter().map(|e| (e.entity, e.updated)).collect()
}

#[test]
fn every_node_carries_auto_promotion_from_birth() {
    let mut world = World::new();
    let a = world
        .spawn((Visual::default(), WorldVisual::default()))
        .id();
    let b = world
        .spawn((Visual::default(), solid_rect_content(rect4())))
        .id();
    assert!(world.get::<AutoPromotion>(a).is_some());
    assert!(world.get::<AutoPromotion>(b).is_some());
}

#[test]
fn owner_entry_carries_placement_material() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((
            Visual {
                opacity: 0.5,
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            WorldVisual::default(),
            moved(10.0, 0.0),
            placed(0.0, 0.0, 20.0, 20.0),
            ChildOf(root),
        ))
        .id();
    let b = world
        .spawn((
            Visual::default(),
            WorldVisual::default(),
            placed(1.0, 2.0, 0.0, 0.0),
            ChildOf(a),
        ))
        .id();
    let owner = world
        .spawn((
            Visual::default(),
            OwnSurface,
            VisualGraphics::default(),
            placed(3.0, 0.0, 4.0, 4.0),
            ChildOf(b),
        ))
        .id();
    let c = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(a)))
        .id();
    let scan = Filler::new(&mut world).scan(&mut world, root);
    let order: Vec<_> = scan
        .entries
        .iter()
        .map(|e| (e.entity, e.depth, e.parent, e.end))
        .collect();
    assert_eq!(
        order,
        vec![
            (a, 1, None, 4),
            (b, 2, Some(0), 3),
            (owner, 3, Some(1), 3),
            (c, 2, Some(0), 4),
        ]
    );
    assert_eq!(
        scan.visible_nodes().collect::<Vec<_>>(),
        vec![(a, 1), (b, 2), (c, 2)]
    );
    let o = entry(&scan, owner);
    assert_eq!(
        o.kind,
        ScanKind::Owner {
            parent_world: Matrix3x2::translation(11.0, 2.0)
        }
    );
    assert!(o.visible);
    assert!(!o.updated, "持ち主の項は更新を数えない");
    assert_eq!(o.world.world_mat, Matrix3x2::translation(14.0, 2.0));
    assert_eq!(o.world.content_world_aabb, Aabb::new(14.0, 2.0, 18.0, 6.0));
    assert_eq!(
        o.ancestors,
        AncestorAcc {
            opacity: 0.5,
            layer: true,
            rect_clip: Some(Aabb::new(10.0, 0.0, 30.0, 20.0)),
            rect_clip_rotated: false,
            blend: false,
        }
    );
    assert_eq!(entry(&scan, a).ancestors, AncestorAcc::ROOT);
    assert_eq!(entry(&scan, a).own_clip, ClipClass::Rect);
    // 持ち主の矩形は親の節の部分木の外接矩形に足さない（描く列の答えを変えない）。
    assert!(
        world
            .get::<WorldVisual>(a)
            .unwrap()
            .subtree_world_aabb
            .is_empty()
    );
}

#[test]
fn ancestor_opacity_is_the_product_over_every_node_above_the_owner() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let faded = |opacity| Visual {
        opacity,
        ..Default::default()
    };
    let a = world
        .spawn((faded(0.5), WorldVisual::default(), ChildOf(root)))
        .id();
    let b = world
        .spawn((faded(0.8), WorldVisual::default(), ChildOf(a)))
        .id();
    let owner = world
        .spawn((Visual::default(), OwnSurface, ChildOf(b)))
        .id();
    let scan = Filler::new(&mut world).scan(&mut world, root);
    let got = entry(&scan, owner).ancestors.opacity;
    assert!((got - 0.4).abs() < 1e-6, "0.5 × 0.8 の積: {got}");
}

#[test]
fn owner_under_hidden_node_is_recorded_invisible() {
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
    let owner = world
        .spawn((Visual::default(), OwnSurface, ChildOf(hidden)))
        .id();
    let clear = world
        .spawn((
            Visual {
                opacity: 0.0,
                ..Default::default()
            },
            WorldVisual::default(),
            ChildOf(root),
        ))
        .id();
    let owner2 = world.spawn((Visual::default(), ChildOf(clear))).id();
    let scan = Filler::new(&mut world).scan(&mut world, root);
    assert_eq!(scan.visible_nodes().count(), 0);
    for e in [hidden, owner, clear, owner2] {
        assert!(!entry(&scan, e).visible, "{e:?}");
    }
    assert_eq!(entry(&scan, owner).parent, Some(0));
    assert_eq!(entry(&scan, owner2).parent, Some(2));
}

#[test]
fn rotated_clip_rounded_clip_and_blend_accumulate_for_descendants() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let r = world
        .spawn((
            Visual {
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            WorldVisual::default(),
            VisualTransform(Transform2D {
                rotation: 0.3,
                ..Transform2D::identity()
            }),
            placed(0.0, 0.0, 8.0, 8.0),
            ChildOf(root),
        ))
        .id();
    let mut content = solid_rect_content(rect4());
    content.mix_mode = MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY);
    let s = world
        .spawn((
            Visual {
                clip: Some(ClipShape::RoundedRectangle { radius: 2.0 }),
                ..Default::default()
            },
            content,
            ChildOf(r),
        ))
        .id();
    let owner = world.spawn((Visual::default(), ChildOf(s))).id();
    let scan = Filler::new(&mut world).scan(&mut world, root);
    let se = entry(&scan, s);
    assert_eq!(se.own_clip, ClipClass::Rounded);
    assert!(!se.source_over);
    assert!(entry(&scan, r).source_over, "中身が無ければ SOURCE_OVER");
    assert!(se.ancestors.rect_clip_rotated);
    assert_eq!(se.ancestors.rect_clip, None);
    assert!(!se.ancestors.layer && !se.ancestors.blend);
    let o = entry(&scan, owner).ancestors;
    assert!(o.rect_clip_rotated && o.layer && o.blend);
    assert_eq!(o.opacity, 1.0);
}

#[test]
fn auto_promoted_owner_subtree_is_walked_through() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let p = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            VisualGraphics::default(),
            ChildOf(root),
        ))
        .id();
    let q = world
        .spawn((Visual::default(), solid_rect_content(rect4()), ChildOf(p)))
        .id();
    {
        let mut a = world.get_mut::<AutoPromotion>(p).unwrap();
        let mut last = Verdict::Stay;
        for i in 0..8i64 {
            last = a.rule.step(
                dola::time::testing::at_ms(i * 100),
                true,
                || Ok(()),
                SurfaceReadiness::Pending,
            );
        }
        assert_eq!(last, Verdict::Prepare);
        let v = a.rule.step(
            dola::time::testing::at_ms(800),
            false,
            || Ok(()),
            SurfaceReadiness::Ready,
        );
        assert_eq!(v, Verdict::Activate);
    }
    let scan = Filler::new(&mut world).scan(&mut world, root);
    assert_eq!(
        entry(&scan, p).kind,
        ScanKind::Node {
            phase: Phase::Active
        }
    );
    assert_eq!(entry(&scan, q).parent, Some(0));
    assert_eq!(
        scan.visible_nodes().collect::<Vec<_>>(),
        vec![(p, 1), (q, 2)]
    );
}

#[test]
fn updated_is_set_only_by_own_changes_not_by_ancestor_motion() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let p = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            moved(1.0, 0.0),
            ChildOf(root),
        ))
        .id();
    let q = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            placed(0.0, 0.0, 4.0, 4.0),
            ChildOf(p),
        ))
        .id();
    let mut filler = Filler::new(&mut world);
    assert_eq!(
        updated(&filler.scan(&mut world, root)),
        vec![(p, true), (q, true)],
        "入った回は更新"
    );
    assert_eq!(
        updated(&filler.scan(&mut world, root)),
        vec![(p, false), (q, false)]
    );
    let before = *world.get::<WorldVisual>(q).unwrap();
    *world.get_mut::<VisualTransform>(p).unwrap() = moved(5.0, 0.0);
    assert_eq!(
        updated(&filler.scan(&mut world, root)),
        vec![(p, true), (q, false)],
        "祖先の動きでは子孫に立たない"
    );
    assert_ne!(*world.get::<WorldVisual>(q).unwrap(), before);
    world.get_mut::<VisualDrawContent>(q).unwrap().set_changed();
    assert_eq!(
        updated(&filler.scan(&mut world, root)),
        vec![(p, false), (q, true)],
        "版の上がり"
    );
    world.get_mut::<Arrangement>(q).unwrap().offset.x = 2.0;
    assert_eq!(
        updated(&filler.scan(&mut world, root)),
        vec![(p, false), (q, true)],
        "自分の配置の変化"
    );
    world.get_mut::<Visual>(p).unwrap().is_visible = false;
    world.get_mut::<VisualDrawContent>(q).unwrap().set_changed();
    assert_eq!(
        updated(&filler.scan(&mut world, root)),
        vec![(p, false), (q, false)],
        "見えない節は数えない"
    );
}

/// まとめの節の子孫は祖先の層の中と同じく上げない。`SOURCE_OVER` でないまとめの節は自分も
/// `SOURCE_OVER` でない件と同じ扱い。
#[test]
fn group_descendants_are_not_promoted() {
    use super::promotion_shape::{Ineligible, ShapeInput, check_shape};
    use windows::Win32::Graphics::Direct2D::Common::{
        D2D1_COMPOSITE_MODE_SOURCE_ATOP, D2D1_COMPOSITE_MODE_SOURCE_OVER,
    };
    for (mix, group_source_over) in [
        (D2D1_COMPOSITE_MODE_SOURCE_OVER, true),
        (D2D1_COMPOSITE_MODE_SOURCE_ATOP, false),
    ] {
        let mut world = World::new();
        let root = world.spawn(Visual::default()).id();
        let g = world
            .spawn((
                Visual::default(),
                WorldVisual::default(),
                VisualGroup {
                    mix: MixMode::Composite(mix),
                },
                placed(10.0, 10.0, 20.0, 20.0),
                ChildOf(root),
            ))
            .id();
        let c = world
            .spawn((
                Visual::default(),
                solid_rect_content(rect4()),
                placed(2.0, 2.0, 4.0, 4.0),
                ChildOf(g),
            ))
            .id();
        let scan = Filler::new(&mut world).scan(&mut world, root);
        assert_eq!(entry(&scan, g).source_over, group_source_over, "{mix:?}");
        assert!(!entry(&scan, g).ancestors.layer, "{mix:?}");
        assert!(entry(&scan, c).ancestors.layer, "{mix:?}");
        let idx = scan.entries.iter().position(|e| e.entity == c).unwrap();
        let input = ShapeInput {
            scan: &scan,
            idx,
            scale: 1.0,
            host_px: (100, 100),
        };
        assert_eq!(
            check_shape(&input),
            Err(Ineligible::AncestorLayer),
            "{mix:?}"
        );
    }
}
