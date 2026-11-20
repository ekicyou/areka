use std::sync::Arc;

use areka_emo_atlas::{DecodedImage, MemoryDecoder, WicDecoderArm};
use areka_emo_compose::RegionMasks;
use areka_parsers::shell::{
    Collision, CollisionBounds, CollisionName, CollisionRegion, CollisionShape,
};
use wintf::ecs::IdForm;

use super::{HIT_SHAPE_LIMIT, HitMasks, hit_shapes};
use crate::shell_target::load_shell_target;
use crate::shell_target::test_support::{claudia_shell_dir, with_com_initialized};

fn hit(shape: CollisionShape) -> Collision {
    Collision {
        index: 0,
        shape,
        name: CollisionName::new(String::new()),
    }
}

fn bounds(left: i64, top: i64, right: i64, bottom: i64) -> CollisionBounds {
    CollisionBounds {
        left,
        top,
        right,
        bottom,
    }
}

fn ids(plan: &super::HitShapePlan) -> Vec<u32> {
    plan.shapes.iter().map(|s| s.id.get()).collect()
}

#[test]
fn shapes_follow_the_table_and_skipped_entries_keep_their_seat() {
    let regions = [
        hit(CollisionShape::Rect(bounds(200, 260, 100, -5))),
        hit(CollisionShape::Rect(bounds(10, 10, 10, 50))),
        hit(CollisionShape::Ellipse(bounds(199, 50, 100, 10))),
        hit(CollisionShape::Circle {
            x: 150,
            y: 150,
            radius: -40,
        }),
        hit(CollisionShape::Circle {
            x: 150,
            y: 150,
            radius: 0,
        }),
        hit(CollisionShape::Polygon(vec![(20, 20), (80, 20)])),
        hit(CollisionShape::Polygon(vec![(20, 20), (80, 20), (-9, 900)])),
        hit(CollisionShape::Rect(bounds(400, 0, 500, 10))),
    ];
    let plan = hit_shapes(
        &regions,
        None,
        &mut HitMasks::default(),
        (300, 250),
        HIT_SHAPE_LIMIT,
    );
    assert_eq!((plan.first, plan.dropped), (0, 0));
    assert_eq!(ids(&plan), [1, 3, 4, 7]);
    let forms: Vec<&IdForm> = plan.shapes.iter().map(|s| &s.form).collect();
    assert_eq!(
        forms,
        [
            &IdForm::Rect {
                left: 100.0,
                top: 0.0,
                right: 200.0,
                bottom: 250.0
            },
            &IdForm::Ellipse {
                center: (149.5, 30.0),
                radius: (49.5, 20.0)
            },
            &IdForm::Ellipse {
                center: (150.0, 150.0),
                radius: (40.0, 40.0)
            },
            &IdForm::Polygon(vec![(20.0, 20.0), (80.0, 20.0), (-9.0, 900.0)]),
        ]
    );
}

#[test]
fn over_the_limit_keeps_the_front_most_entries() {
    let regions: Vec<Collision> = (0..5)
        .map(|i| hit(CollisionShape::Rect(bounds(i, 0, i + 1, 1))))
        .collect();
    let plan = hit_shapes(&regions, None, &mut HitMasks::default(), (10, 10), 3);
    assert_eq!((plan.first, plan.dropped), (2, 2));
    assert_eq!(ids(&plan), [1, 2, 3]);
    assert_eq!(
        plan.shapes[0].form,
        IdForm::Rect {
            left: 2.0,
            top: 0.0,
            right: 3.0,
            bottom: 1.0
        }
    );
}

#[test]
fn bound_edges_and_many_entries_get_distinct_ids() {
    let regions: Vec<Collision> = (0..300)
        .map(|i| hit(CollisionShape::Rect(bounds(i, 0, i + 1, 1))))
        .collect();
    let all = hit_shapes(
        &regions,
        None,
        &mut HitMasks::default(),
        (400, 10),
        HIT_SHAPE_LIMIT,
    );
    assert_eq!((all.first, all.dropped), (0, 0));
    assert_eq!(ids(&all), (1..=300).collect::<Vec<u32>>());
    let lefts: Vec<f32> = all
        .shapes
        .iter()
        .map(|s| match s.form {
            IdForm::Rect { left, .. } => left,
            _ => panic!("rect"),
        })
        .collect();
    assert_eq!(lefts, (0..300).map(|i| i as f32).collect::<Vec<_>>());

    let exact = hit_shapes(&regions, None, &mut HitMasks::default(), (400, 10), 300);
    assert_eq!(
        (exact.first, exact.dropped, exact.shapes.len()),
        (0, 0, 300)
    );

    let last_two = hit_shapes(&regions, None, &mut HitMasks::default(), (400, 10), 2);
    assert_eq!((last_two.first, last_two.dropped), (298, 298));
    assert_eq!(ids(&last_two), [1, 2]);
    assert_eq!(
        last_two.shapes[1].form,
        IdForm::Rect {
            left: 299.0,
            top: 0.0,
            right: 300.0,
            bottom: 1.0
        }
    );

    let none = hit_shapes(&regions, None, &mut HitMasks::default(), (400, 10), 0);
    assert_eq!((none.first, none.dropped, none.shapes.len()), (300, 300, 0));
}

#[test]
fn skipped_entries_inside_the_bound_keep_their_seat() {
    let regions = [
        hit(CollisionShape::Rect(bounds(0, 0, 1, 1))),
        hit(CollisionShape::Rect(bounds(50, 50, 60, 60))),
        hit(CollisionShape::Rect(bounds(2, 0, 3, 1))),
    ];
    let plan = hit_shapes(&regions, None, &mut HitMasks::default(), (10, 10), 2);
    assert_eq!((plan.first, plan.dropped), (1, 1));
    assert_eq!(ids(&plan), [2]);

    let regions = [
        hit(CollisionShape::Region(region("r.png", 0, 0))),
        hit(CollisionShape::Rect(bounds(0, 0, 1, 1))),
    ];
    let plan = hit_shapes(
        &regions,
        None,
        &mut HitMasks::default(),
        (10, 10),
        HIT_SHAPE_LIMIT,
    );
    assert_eq!(ids(&plan), [2]);
}

#[test]
fn extreme_coordinates_saturate_without_panic() {
    let regions = [
        hit(CollisionShape::Rect(bounds(
            i64::MIN,
            i64::MIN,
            i64::MAX,
            i64::MAX,
        ))),
        hit(CollisionShape::Circle {
            x: 0,
            y: 0,
            radius: i64::MIN,
        }),
        hit(CollisionShape::Polygon(vec![
            (i64::MIN, 0),
            (i64::MAX, 0),
            (0, i64::MAX),
        ])),
    ];
    let plan = hit_shapes(
        &regions,
        None,
        &mut HitMasks::default(),
        (10, 10),
        HIT_SHAPE_LIMIT,
    );
    let limit = (1i64 << 24) as f32;
    assert_eq!(
        plan.shapes
            .iter()
            .map(|s| s.form.clone())
            .collect::<Vec<_>>(),
        [
            IdForm::Rect {
                left: 0.0,
                top: 0.0,
                right: 10.0,
                bottom: 10.0
            },
            IdForm::Ellipse {
                center: (0.0, 0.0),
                radius: (limit, limit)
            },
            IdForm::Polygon(vec![(-limit, 0.0), (limit, 0.0), (0.0, limit)]),
        ]
    );
}

fn region(file: &str, x: i64, y: i64) -> CollisionRegion {
    CollisionRegion {
        file: file.to_string(),
        red: 255,
        green: 0,
        blue: 0,
        invert: false,
        x,
        y,
    }
}

#[test]
fn region_is_placed_at_its_position_and_converted_once() {
    let mut dec = MemoryDecoder::new();
    dec.insert_unmultiplied(
        std::path::Path::new("shell").join("r.png"),
        DecodedImage {
            width: 2,
            height: 1,
            stride: 8,
            bgra: vec![0, 0, 255, 255, 0, 0, 0, 255],
            has_alpha: true,
        },
    );
    let placed = region("r.png", -1, 2);
    let unreadable = region("missing.png", 0, 0);
    let masks = RegionMasks::load(
        [&placed, &unreadable].into_iter(),
        std::path::Path::new("shell"),
        &dec,
    );
    let regions = [
        hit(CollisionShape::Region(unreadable)),
        hit(CollisionShape::Region(placed.clone())),
        hit(CollisionShape::Region(placed)),
    ];
    let mut id_masks = HitMasks::default();
    let plan = hit_shapes(
        &regions,
        Some(&masks),
        &mut id_masks,
        (10, 10),
        HIT_SHAPE_LIMIT,
    );
    assert_eq!(ids(&plan), [2, 3]);
    let (IdForm::Mask { mask: a, left, top }, IdForm::Mask { mask: b, .. }) =
        (&plan.shapes[0].form, &plan.shapes[1].form)
    else {
        panic!("region はマスクの形になる: {plan:?}");
    };
    assert_eq!((*left, *top, a.width(), a.height()), (-1, 2, 2, 1));
    assert!(Arc::ptr_eq(a, b), "同じ実体は 1 度だけ直す");

    id_masks.discard_pictures();
    let again = hit_shapes(
        &regions,
        Some(&masks),
        &mut id_masks,
        (10, 10),
        HIT_SHAPE_LIMIT,
    );
    let IdForm::Mask { mask: c, .. } = &again.shapes[0].form else {
        panic!("region はマスクの形になる: {again:?}");
    };
    assert!(Arc::ptr_eq(a, c), "GPU の絵を捨てても直したマスクは残る");
    assert_eq!(id_masks.pictures().created(), 0);
}

#[test]
fn imported_region_is_placed_at_its_shifted_position() {
    let mut dec = MemoryDecoder::new();
    dec.insert_unmultiplied(
        std::path::Path::new("shell").join("r.png"),
        DecodedImage {
            width: 2,
            height: 1,
            stride: 8,
            bgra: vec![0, 0, 255, 255, 0, 0, 0, 255],
            has_alpha: true,
        },
    );
    let r = region("r.png", 0, 0);
    let masks = RegionMasks::load([&r].into_iter(), std::path::Path::new("shell"), &dec);
    let shifted = CollisionRegion {
        x: 3,
        y: 4,
        ..r.clone()
    };
    let regions = [
        hit(CollisionShape::Region(r)),
        hit(CollisionShape::Region(shifted)),
    ];
    let plan = hit_shapes(
        &regions,
        Some(&masks),
        &mut HitMasks::default(),
        (10, 10),
        HIT_SHAPE_LIMIT,
    );
    let (
        IdForm::Mask {
            mask: a,
            left: l0,
            top: t0,
        },
        IdForm::Mask {
            mask: b,
            left: l1,
            top: t1,
        },
    ) = (&plan.shapes[0].form, &plan.shapes[1].form)
    else {
        panic!("region はマスクの形になる: {plan:?}");
    };
    assert_eq!(((*l0, *t0), (*l1, *t1)), ((0, 0), (3, 4)));
    assert!(Arc::ptr_eq(a, b));
}

#[test]
fn claudia_face_0_builds_ellipse_and_polygon_shapes() {
    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("COM 初期化下で WIC ファクトリが作れる");
        let target =
            load_shell_target(&claudia_shell_dir(), &dec).expect("claudia のシェルは読める");
        let world = target.build_world();
        let regions = world.hit_regions(0).expect("claudia に面 0 が在る");
        let plan = hit_shapes(
            regions,
            world.region_masks().map(|m| &**m),
            &mut HitMasks::default(),
            (333, 500),
            HIT_SHAPE_LIMIT,
        );
        assert_eq!(plan.shapes.len(), regions.len(), "{plan:?}");
        assert_eq!(ids(&plan), (1..=regions.len() as u32).collect::<Vec<_>>());
        let has = |f: fn(&IdForm) -> bool| plan.shapes.iter().any(|s| f(&s.form));
        assert!(has(|f| matches!(f, IdForm::Ellipse { .. })), "{plan:?}");
        assert!(has(|f| matches!(f, IdForm::Polygon(_))), "{plan:?}");
    });
}
