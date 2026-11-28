//! 形ごとの当たりの実 GPU のテスト。

use areka_emo_atlas::{AtlasTable, DecodedImage, WicDecoderArm};
use areka_emo_compose::EmoWorld;
use areka_parsers::shell::parse;
use bevy_ecs::world::World;
use wintf::ecs::{GraphicsCore, hit_test_entity};

use super::test_support::{
    FIVE_SHAPES_MASK, attach_and_show, bake_world, capture, five_shapes_assets, gradient,
    hit_generation, load_hit_shell, make_world_with_gpu, mask_pictures_created, mount_entities,
    name_at, paint_screen, screen_point, straight_image, warn_count,
};
use super::{EmoPresenter, TargetId};
use crate::shell_target::test_support::claudia_shell_dir;

const T: TargetId = TargetId(0);

fn lines<S: AsRef<str>>(rows: &[S]) -> String {
    rows.iter().map(|r| format!("{}\n", r.as_ref())).collect()
}

fn opaque(rows: &[&str], w: u32, h: u32) -> (EmoWorld, AtlasTable) {
    load_hit_shell(&lines(rows), (w, h, gradient(w, h, 0x15)), Vec::new())
}

fn shown(assets: (EmoWorld, AtlasTable)) -> (World, EmoPresenter) {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, assets, 0, 96);
    (world, presenter)
}

fn check(world: &mut World, presenter: &EmoPresenter, cases: &[(i64, i64, Option<&str>)]) {
    for &(x, y, want) in cases {
        assert_eq!(name_at(world, presenter, T, x, y), want, "({x},{y})");
    }
}

fn red_left_mask() -> DecodedImage {
    straight_image(16, 16, |x, _| {
        if x < 8 {
            [0, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        }
    })
}

const EDGES: [&str; 3] = [
    "collisionex0,Rc,rect,100,200,200,260",
    "collisionex1,Ell,ellipse,100,100,200,160",
    "collisionex2,Poly,polygon,20,200,80,200,80,260,20,260",
];

#[test]
fn claudia_face_0_resolves_ellipse_polygons_and_rect_names() {
    let mut world = make_world_with_gpu();
    let dec = WicDecoderArm::new().expect("WIC ファクトリが作れる");
    let mut shell = crate::shell_target::load_shell_target(&claudia_shell_dir(), &dec)
        .expect("claudia のシェルは読める");
    let assets = (shell.take_world(), shell.atlas().clone());
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, assets, 0, 96);

    for (x, y, want) in [
        (152, 483, "Foot"),
        (250, 228, "Fan"),
        (150, 435, "Skirt"),
        (165, 70, "Head"),
    ] {
        assert_eq!(
            name_at(&mut world, &presenter, T, x, y),
            Some(want),
            "({x},{y})"
        );
    }
}

#[test]
fn five_shapes_resolve_at_every_display_scale() {
    for (dpi, k) in [(96, 1), (192, 2)] {
        let mut world = make_world_with_gpu();
        let mut presenter = EmoPresenter::new();
        attach_and_show(&mut presenter, &mut world, T, five_shapes_assets(), 0, dpi);

        let (names, events) = capture(|| {
            [(4, 8), (24, 8), (40, 8), (56, 8), (76, 4), (70, 14)]
                .map(|(x, y)| name_at(&mut world, &presenter, T, x * k, y * k))
        });
        assert_eq!(
            names,
            [Some("R"), Some("B"), Some("E"), Some("C"), Some("P"), None],
            "dpi {dpi}"
        );
        assert_eq!(warn_count(&events), 0, "{events:?}");
        assert_eq!(hit_generation(&presenter, T), Some(1));
        assert_eq!(mask_pictures_created(&presenter, T), 1);
    }
}

#[test]
fn rect_ellipse_polygon_edges_follow_the_pixel_centre_rule() {
    let (mut world, presenter) = shown(opaque(&EDGES, 300, 300));
    check(
        &mut world,
        &presenter,
        &[
            (100, 230, Some("Rc")),
            (99, 230, None),
            (199, 230, Some("Rc")),
            (200, 230, None),
            (201, 230, None),
            (150, 200, Some("Rc")),
            (150, 199, None),
            (150, 259, Some("Rc")),
            (150, 260, None),
            (150, 261, None),
            (150, 130, Some("Ell")),
            (100, 130, Some("Ell")),
            (99, 130, None),
            (199, 130, Some("Ell")),
            (200, 130, None),
            (201, 130, None),
            (150, 100, Some("Ell")),
            (150, 99, None),
            (150, 159, Some("Ell")),
            (150, 160, None),
            (150, 161, None),
            (103, 103, None),
            (50, 230, Some("Poly")),
            (19, 230, None),
            (20, 230, Some("Poly")),
            (21, 230, Some("Poly")),
            (79, 230, Some("Poly")),
            (80, 230, None),
            (81, 230, None),
            (50, 199, None),
            (50, 200, Some("Poly")),
            (50, 201, Some("Poly")),
            (50, 259, Some("Poly")),
            (50, 260, None),
            (50, 261, None),
        ],
    );
}

#[test]
fn circle_edges_follow_the_pixel_centre_rule() {
    let (mut world, presenter) = shown(opaque(&["collisionex0,Cir,circle,150,150,40"], 300, 300));
    check(
        &mut world,
        &presenter,
        &[
            (150, 150, Some("Cir")),
            (110, 150, Some("Cir")),
            (150, 110, Some("Cir")),
            (109, 150, None),
            (150, 109, None),
            (189, 150, Some("Cir")),
            (150, 189, Some("Cir")),
            (190, 150, None),
            (150, 190, None),
            (191, 150, None),
            (150, 191, None),
            (178, 178, None),
            (179, 179, None),
            (170, 170, Some("Cir")),
        ],
    );
}

#[test]
fn reversed_rect_and_ellipse_and_negative_circle_are_normalised() {
    let (mut world, presenter) = shown(opaque(
        &[
            "collisionex0,CirNeg,circle,50,250,-20",
            "collisionex1,EllInv,ellipse,280,160,240,100",
            "collisionex2,RcInv,rect,280,260,240,200",
        ],
        300,
        300,
    ));
    check(
        &mut world,
        &presenter,
        &[
            (50, 250, Some("CirNeg")),
            (40, 250, Some("CirNeg")),
            (30, 250, Some("CirNeg")),
            (69, 250, Some("CirNeg")),
            (50, 230, Some("CirNeg")),
            (29, 250, None),
            (70, 250, None),
            (50, 229, None),
            (50, 270, None),
            (260, 130, Some("EllInv")),
            (240, 130, Some("EllInv")),
            (260, 159, Some("EllInv")),
            (259, 100, Some("EllInv")),
            (239, 130, None),
            (260, 160, None),
            (259, 99, None),
            (260, 230, Some("RcInv")),
            (240, 230, Some("RcInv")),
            (260, 200, Some("RcInv")),
            (260, 259, Some("RcInv")),
            (239, 230, None),
            (260, 199, None),
            (260, 260, None),
            (279, 259, Some("RcInv")),
            (280, 230, None),
        ],
    );
}

#[test]
fn empty_shapes_never_hit_and_do_not_hide_the_rest() {
    let (mut world, presenter) = shown(opaque(
        &[
            "collisionex0,Under,rect,0,0,300,300",
            "collisionex1,Cir0,circle,50,50,0",
            "collisionex2,EllThin,ellipse,250,20,250,80",
            "collisionex3,PolyLine,polygon,120,280,180,280,150,280",
            "collisionex4,RcThin,rect,200,200,200,260",
            "collisionex5,RcFlat,rect,100,200,160,200",
            "collisionex6,PolyDot,polygon,70,70,70,70,70,70",
        ],
        300,
        300,
    ));
    check(
        &mut world,
        &presenter,
        &[
            (50, 50, Some("Under")),
            (51, 50, Some("Under")),
            (49, 50, Some("Under")),
            (250, 50, Some("Under")),
            (249, 50, Some("Under")),
            (251, 50, Some("Under")),
            (150, 280, Some("Under")),
            (150, 279, Some("Under")),
            (150, 281, Some("Under")),
            (200, 230, Some("Under")),
            (199, 230, Some("Under")),
            (130, 200, Some("Under")),
            (130, 199, Some("Under")),
            (70, 70, Some("Under")),
        ],
    );
}

#[test]
fn pentagram_centre_hits_and_gaps_inside_the_hull_do_not() {
    let (mut world, presenter) = shown(opaque(
        &["collisionex0,Star,polygon,150,50,209,231,55,119,245,119,91,231"],
        300,
        300,
    ));
    check(
        &mut world,
        &presenter,
        &[
            (150, 150, Some("Star")),
            (150, 60, Some("Star")),
            (200, 219, Some("Star")),
            (190, 95, None),
            (110, 95, None),
            (150, 220, None),
            (5, 5, None),
            (150, 40, None),
        ],
    );
}

#[test]
fn shapes_overflowing_the_face_hit_only_inside_it() {
    let (mut world, presenter) = shown(opaque(
        &[
            "collisionex0,Neg,rect,-50,-50,40,40",
            "collisionex1,Far,ellipse,60,60,300,300",
            "collisionex2,Tri,polygon,-60,60,40,60,-60,160",
        ],
        100,
        100,
    ));
    check(
        &mut world,
        &presenter,
        &[
            (0, 0, Some("Neg")),
            (10, 10, Some("Neg")),
            (39, 39, Some("Neg")),
            (40, 40, None),
            (99, 99, Some("Far")),
            (70, 70, None),
            (5, 60, Some("Tri")),
            (5, 59, None),
            (10, 70, Some("Tri")),
            (30, 80, None),
            (-1, 5, None),
            (5, -1, None),
            (100, 5, None),
            (5, 100, None),
        ],
    );
}

#[test]
fn extreme_coordinates_neither_fail_nor_panic() {
    let shapes = [
        "rect,-9223372036854775808,-9223372036854775808,9223372036854775807,9223372036854775807",
        "ellipse,-9223372036854775808,-9223372036854775808,9223372036854775807,9223372036854775807",
        "circle,0,0,9223372036854775807",
        "circle,0,0,-9223372036854775808",
        "polygon,-9223372036854775808,-9223372036854775808,9223372036854775807,-9223372036854775808,9223372036854775807,9223372036854775807,-9223372036854775808,9223372036854775807",
    ];
    let mut world = make_world_with_gpu();
    for (i, shape) in shapes.iter().enumerate() {
        let target = TargetId(i as u32);
        let mut presenter = EmoPresenter::new();
        let row = format!("collisionex0,Big,{shape}");
        let (names, events) = capture(|| {
            attach_and_show(
                &mut presenter,
                &mut world,
                target,
                opaque(&[row.as_str()], 100, 100),
                0,
                96,
            );
            [(0, 0), (50, 50), (99, 99)].map(|(x, y)| name_at(&mut world, &presenter, target, x, y))
        });
        assert_eq!(names, [Some("Big"); 3], "{shape}");
        assert_eq!(warn_count(&events), 0, "{shape}: {events:?}");
    }
}

#[test]
fn five_shapes_and_collision_rects_overlap_in_written_order() {
    let rows = [
        "collision0,0,0,100,100,Base",
        "collisionex1,Rg,region,mask.png,255,0,0",
        "collisionex2,Rc,rect,20,0,60,40",
        "collisionex3,El,ellipse,40,20,100,60",
        "collisionex4,Ci,circle,70,70,20",
        "collisionex5,Po,polygon,0,50,50,100,0,100",
        "collision6,80,80,100,100,Top",
    ];
    let assets = load_hit_shell(
        &lines(&rows),
        (100, 100, gradient(100, 100, 0x15)),
        vec![(FIVE_SHAPES_MASK, red_left_mask())],
    );
    let (mut world, presenter) = shown(assets);
    check(
        &mut world,
        &presenter,
        &[
            (3, 3, Some("Rg")),
            (12, 3, Some("Base")),
            (30, 10, Some("Rc")),
            (45, 5, Some("Rc")),
            (50, 30, Some("El")),
            (70, 45, Some("El")),
            (95, 40, Some("El")),
            (95, 10, Some("Base")),
            (70, 55, Some("Ci")),
            (70, 70, Some("Ci")),
            (75, 75, Some("Ci")),
            (10, 90, Some("Po")),
            (30, 60, Some("Base")),
            (80, 80, Some("Top")),
            (90, 90, Some("Top")),
            (85, 85, Some("Top")),
        ],
    );
}

#[test]
fn missing_top_shape_lets_the_lower_name_through() {
    let (mut world, presenter) = shown(opaque(
        &[
            "collisionex0,Low,rect,0,0,100,100",
            "collisionex1,Up,ellipse,0,0,100,100",
        ],
        100,
        100,
    ));
    check(
        &mut world,
        &presenter,
        &[
            (50, 50, Some("Up")),
            (2, 2, Some("Low")),
            (97, 97, Some("Low")),
            (50, 2, Some("Up")),
        ],
    );
}

#[test]
fn shapes_imported_from_a_child_hit_at_the_placed_position() {
    let text = lines(&[
        "surface0",
        "{",
        "element0,overlay,p.png,0,0",
        "element1,overlay,10,20,10",
        "collisionex0,Own,rect,0,0,10,10",
        "}",
        "surface10",
        "{",
        "element0,overlay,c.png,0,0",
        "collisionex0,CE,ellipse,0,0,20,20",
        "collisionex1,CP,polygon,0,0,20,0,0,20",
        "}",
    ]);
    let assets = bake_world(
        parse(&text),
        vec![
            ("p.png", 100, 100, gradient(100, 100, 0x15)),
            ("c.png", 20, 20, gradient(20, 20, 0x33)),
        ],
    );
    let (mut world, presenter) = shown(assets);
    check(
        &mut world,
        &presenter,
        &[
            (5, 5, Some("Own")),
            (23, 13, Some("CP")),
            (21, 11, Some("CP")),
            (20, 10, Some("CP")),
            (32, 22, Some("CE")),
            (30, 20, Some("CE")),
            (39, 20, Some("CE")),
            (45, 20, None),
            (19, 20, None),
        ],
    );
}

#[test]
fn three_hundred_shapes_return_one_distinct_name_each() {
    let rows: Vec<String> = (0..300)
        .map(|i| {
            let (x, y) = ((i % 20) * 10, (i / 20) * 10);
            let form = if i % 2 == 0 { "rect" } else { "ellipse" };
            format!("collisionex{i},N{i},{form},{x},{y},{},{}", x + 10, y + 10)
        })
        .collect();
    let (mut world, presenter) = shown(load_hit_shell(
        &lines(&rows),
        (200, 150, gradient(200, 150, 0x15)),
        Vec::new(),
    ));
    for i in 0..300i64 {
        let (x, y) = ((i % 20) * 10, (i / 20) * 10);
        let name = format!("N{i}");
        assert_eq!(
            name_at(&mut world, &presenter, T, x + 5, y + 5),
            Some(name.as_str()),
            "{i}"
        );
        let corner = (i % 2 == 0).then_some(name.as_str());
        assert_eq!(
            name_at(&mut world, &presenter, T, x, y),
            corner,
            "corner {i}"
        );
    }
}

#[test]
fn collision_sort_does_not_change_the_answers() {
    let answers: Vec<Vec<Option<String>>> = [None, Some("ascend"), Some("descend")]
        .iter()
        .map(|sort| {
            let mut rows = vec!["charset,UTF-8".to_string()];
            rows.extend(sort.map(|s| format!("collision-sort,{s}")));
            rows.extend(
                [
                    "surface0",
                    "{",
                    "element0,overlay,p.png,0,0",
                    "collision0,0,0,60,60,A",
                    "collisionex1,B,ellipse,20,20,80,80",
                    "collisionex2,C,circle,50,50,10",
                    "}",
                ]
                .map(String::from),
            );
            let assets = bake_world(
                parse(&lines(&rows)),
                vec![("p.png", 100, 100, gradient(100, 100, 0x15))],
            );
            let (mut world, presenter) = shown(assets);
            [(50, 50), (30, 30), (70, 70), (5, 5), (90, 10)]
                .map(|(x, y)| name_at(&mut world, &presenter, T, x, y).map(String::from))
                .to_vec()
        })
        .collect();
    let want = [Some("C"), Some("B"), Some("B"), Some("A"), None].map(|n| n.map(String::from));
    assert_eq!(answers[0], want);
    assert_eq!(answers[1], want);
    assert_eq!(answers[2], want);
}

#[test]
fn same_input_gives_the_same_answers_every_time() {
    let sweep = |world: &mut World, presenter: &EmoPresenter| -> Vec<Option<String>> {
        (0..15)
            .flat_map(|j| (0..15).map(move |i| (i * 20 + 1, j * 20 + 1)))
            .chain([(100, 230), (20, 230), (150, 130)])
            .map(|(x, y)| name_at(world, presenter, T, x, y).map(String::from))
            .collect()
    };
    let (mut w1, first) = shown(opaque(&EDGES, 300, 300));
    let (mut w2, second) = shown(opaque(&EDGES, 300, 300));
    let a = sweep(&mut w1, &first);
    assert!(a.iter().any(|n| n.is_some()) && a.iter().any(|n| n.is_none()));
    assert_eq!(sweep(&mut w1, &first), a);
    assert_eq!(sweep(&mut w2, &second), a);
}

/// 入れ物が読めない間は、クリックの透過も名前の解決も当たり無しで、`error!` は 2 つ合わせて 1 件。
/// 読めた後にまた読めなくなれば、もう 1 件。
#[test]
fn unreadable_surface_misses_both_paths_with_one_error_while_unreadable() {
    let (mut world, presenter) = shown(five_shapes_assets());
    paint_screen(&mut world);
    let surface = mount_entities(&presenter, T).0;
    let point = screen_point(&world, surface, 24, 8);
    let both = |world: &World| {
        (
            hit_test_entity(world, surface, point),
            presenter.hit_region_client(world, T, 24, 8).region,
        )
    };
    assert_eq!(both(&world), (true, Some("B")), "前提: 読めれば当たる");

    let ((), events) = capture(|| {
        for _ in 0..2 {
            let core = world
                .remove_resource::<GraphicsCore>()
                .expect("前提: GraphicsCore が載っている");
            for _ in 0..3 {
                assert_eq!(both(&world), (false, None));
            }
            world.insert_resource(core);
            assert_eq!(both(&world), (true, Some("B")));
        }
    });
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .map(|e| e.message())
        .collect();
    assert_eq!(
        errors, ["[hit_test] surface alpha unreadable, no hit"; 2],
        "{events:?}"
    );
}
