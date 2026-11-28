//! `region` の当たりの実 GPU のテスト。

use areka_emo_atlas::{DecodedImage, MemoryDecoder};
use bevy_ecs::world::World;

use super::test_support::{
    attach_and_show, gradient, load_hit_shell, make_world_with_gpu, name_at, straight_image,
};
use super::{EmoPresenter, TargetId};

const T: TargetId = TargetId(0);

const WHITE: [u8; 4] = [255, 255, 255, 255];
const RED: [u8; 4] = [0, 0, 255, 255];
const DARK_RED: [u8; 4] = [0, 0, 254, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [255, 0, 0, 255];
const YELLOW: [u8; 4] = [0, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

type Block = (u32, u32, u32, u32, [u8; 4]);

fn canvas(w: u32, h: u32, ground: [u8; 4], blocks: &[Block]) -> DecodedImage {
    straight_image(w, h, |x, y| {
        blocks
            .iter()
            .rev()
            .find(|&&(x0, y0, x1, y1, _)| (x0..x1).contains(&x) && (y0..y1).contains(&y))
            .map_or(ground, |&(.., color)| color)
    })
}

fn lines<S: AsRef<str>>(rows: &[S]) -> String {
    rows.iter().map(|r| format!("{}\n", r.as_ref())).collect()
}

fn shown_with(
    rows: &[&str],
    (w, h): (u32, u32),
    images: Vec<(&str, DecodedImage)>,
) -> (World, EmoPresenter) {
    let assets = load_hit_shell(&lines(rows), (w, h, gradient(w, h, 0x15)), images);
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

#[test]
fn colour_matches_exactly_and_edges_follow_the_image_pixels() {
    let map = canvas(
        300,
        300,
        WHITE,
        &[
            (20, 20, 80, 80, RED),
            (100, 20, 160, 80, DARK_RED),
            (180, 20, 240, 80, GREEN),
        ],
    );
    let (mut world, presenter) = shown_with(
        &["collisionex0,RegRed,region,map_full.png,255,0,0"],
        (300, 300),
        vec![("map_full.png", map)],
    );
    check(
        &mut world,
        &presenter,
        &[
            (50, 50, Some("RegRed")),
            (130, 50, None),
            (210, 50, None),
            (250, 250, None),
            (19, 50, None),
            (20, 50, Some("RegRed")),
            (79, 50, Some("RegRed")),
            (80, 50, None),
            (50, 19, None),
            (50, 20, Some("RegRed")),
            (50, 79, Some("RegRed")),
            (50, 80, None),
        ],
    );
}

#[test]
fn image_smaller_than_the_face_is_placed_at_the_origin_without_stretching() {
    let (mut world, presenter) = shown_with(
        &["collisionex0,RegSmallTl,region,map_small_tl.png,0,0,255"],
        (300, 300),
        vec![(
            "map_small_tl.png",
            canvas(100, 100, BLUE, &[(0, 0, 1, 1, WHITE)]),
        )],
    );
    check(
        &mut world,
        &presenter,
        &[
            (50, 50, Some("RegSmallTl")),
            (1, 0, Some("RegSmallTl")),
            (99, 99, Some("RegSmallTl")),
            (0, 0, None),
            (100, 100, None),
            (150, 150, None),
            (263, 290, None),
        ],
    );
}

#[test]
fn image_of_one_colour_hits_all_of_its_own_area_only() {
    let (mut world, presenter) = shown_with(
        &["collisionex0,RegSmall,region,map_small.png,0,0,255"],
        (300, 300),
        vec![("map_small.png", canvas(100, 100, BLUE, &[]))],
    );
    check(
        &mut world,
        &presenter,
        &[
            (0, 0, Some("RegSmall")),
            (10, 10, Some("RegSmall")),
            (50, 50, Some("RegSmall")),
            (99, 99, Some("RegSmall")),
            (100, 100, None),
            (150, 150, None),
            (250, 250, None),
            (50, 250, None),
            (250, 50, None),
            (263, 290, None),
        ],
    );
}

#[test]
fn red_dot_at_the_top_left_is_a_plain_pixel() {
    let map = canvas(300, 300, WHITE, &[(20, 20, 80, 80, RED), (0, 0, 1, 1, RED)]);
    let (mut world, presenter) = shown_with(
        &["collisionex0,RegRedTl,region,map_red_tl.png,255,0,0"],
        (300, 300),
        vec![("map_red_tl.png", map)],
    );
    check(
        &mut world,
        &presenter,
        &[
            (0, 0, Some("RegRedTl")),
            (1, 0, None),
            (50, 50, Some("RegRedTl")),
            (80, 50, None),
        ],
    );
}

#[test]
fn image_larger_than_the_face_hits_only_inside_the_face() {
    let map = canvas(
        400,
        400,
        WHITE,
        &[(20, 20, 80, 80, YELLOW), (320, 20, 380, 80, YELLOW)],
    );
    let (mut world, presenter) = shown_with(
        &["collisionex0,RegLarge,region,map_large.png,255,255,0"],
        (300, 300),
        vec![("map_large.png", map)],
    );
    check(
        &mut world,
        &presenter,
        &[
            (17, 17, None),
            (19, 19, None),
            (20, 20, Some("RegLarge")),
            (22, 22, Some("RegLarge")),
            (50, 50, Some("RegLarge")),
            (65, 65, Some("RegLarge")),
            (70, 70, Some("RegLarge")),
            (78, 78, Some("RegLarge")),
            (79, 79, Some("RegLarge")),
            (80, 80, None),
            (250, 40, None),
            (263, 40, None),
            (150, 150, None),
            (320, 40, None),
            (350, 40, None),
        ],
    );
}

#[test]
fn part_of_the_image_sticking_out_of_the_face_is_not_a_hit_and_not_a_failure() {
    let map = canvas(200, 200, WHITE, &[(80, 0, 160, 200, RED)]);
    let (mut world, presenter) = shown_with(
        &["collisionex0,Edge,region,wide.png,255,0,0"],
        (100, 100),
        vec![("wide.png", map)],
    );
    check(
        &mut world,
        &presenter,
        &[
            (79, 50, None),
            (80, 50, Some("Edge")),
            (99, 50, Some("Edge")),
            (99, 99, Some("Edge")),
            (100, 50, None),
            (120, 50, None),
            (50, 100, None),
        ],
    );
}

#[test]
fn inversion_hits_other_colours_inside_the_image_and_never_outside_it() {
    let map = canvas(100, 100, WHITE, &[(20, 20, 60, 60, RED)]);
    let (mut world, presenter) = shown_with(
        &["collisionex0,RegInv,region,map_small_red.png,255,0,0,true"],
        (300, 300),
        vec![("map_small_red.png", map)],
    );
    check(
        &mut world,
        &presenter,
        &[
            (30, 30, None),
            (19, 30, Some("RegInv")),
            (20, 30, None),
            (59, 30, None),
            (60, 30, Some("RegInv")),
            (10, 10, Some("RegInv")),
            (80, 80, Some("RegInv")),
            (99, 99, Some("RegInv")),
            (99, 50, Some("RegInv")),
            (100, 100, None),
            (100, 50, None),
            (150, 150, None),
            (250, 250, None),
            (50, 150, None),
        ],
    );
}

#[test]
fn alpha_zero_counts_as_black_and_other_alphas_keep_their_colour() {
    let map = canvas(
        300,
        300,
        CLEAR,
        &[
            (39, 39, 59, 59, [0, 0, 255, 0]),
            (119, 39, 139, 59, [0, 0, 255, 128]),
            (199, 39, 219, 59, [0, 0, 255, 255]),
            (39, 139, 59, 159, [0, 0, 255, 1]),
            (119, 139, 139, 159, [0, 0, 255, 254]),
            (199, 139, 219, 159, [255, 255, 255, 0]),
        ],
    );
    let points = [
        (49, 49),
        (129, 49),
        (209, 49),
        (49, 149),
        (129, 149),
        (209, 149),
        (250, 250),
    ];
    let cases: [(&str, [Option<&str>; 7]); 3] = [
        (
            "collisionex0,RegAlpha,region,map_alpha.png,255,0,0",
            [
                None,
                Some("RegAlpha"),
                Some("RegAlpha"),
                Some("RegAlpha"),
                Some("RegAlpha"),
                None,
                None,
            ],
        ),
        (
            "collisionex0,RegAlphaInv,region,map_alpha.png,255,0,0,true",
            [
                Some("RegAlphaInv"),
                None,
                None,
                None,
                None,
                Some("RegAlphaInv"),
                Some("RegAlphaInv"),
            ],
        ),
        (
            "collisionex0,RegBlack,region,map_alpha.png,0,0,0",
            [
                Some("RegBlack"),
                None,
                None,
                None,
                None,
                Some("RegBlack"),
                Some("RegBlack"),
            ],
        ),
    ];
    for (row, want) in cases {
        let (mut world, presenter) =
            shown_with(&[row], (300, 300), vec![("map_alpha.png", map.clone())]);
        for ((x, y), want) in points.into_iter().zip(want) {
            assert_eq!(
                name_at(&mut world, &presenter, T, x, y),
                want,
                "{row} ({x},{y})"
            );
        }
    }
}

#[test]
fn unreadable_file_hits_nothing_and_does_not_block_the_rect_below() {
    for rows in [
        [
            "collisionex0,Under,rect,20,20,280,280",
            "collisionex1,RegMissing,region,nothing.png,255,0,0",
        ],
        [
            "collisionex0,Under,rect,20,20,280,280",
            "collisionex1,RegMissingInv,region,nothing.png,255,0,0,true",
        ],
        [
            "collisionex0,RegMissing,region,nothing.png,255,0,0",
            "collisionex1,Under,rect,20,20,280,280",
        ],
        [
            "collisionex0,RegMissingInv,region,nothing.png,255,0,0,true",
            "collisionex1,Under,rect,20,20,280,280",
        ],
    ] {
        let (mut world, presenter) = shown_with(&rows, (300, 300), Vec::new());
        for (x, y, want) in [
            (150, 150, Some("Under")),
            (50, 50, Some("Under")),
            (10, 10, None),
        ] {
            assert_eq!(
                name_at(&mut world, &presenter, T, x, y),
                want,
                "{rows:?} ({x},{y})"
            );
        }
    }
}

#[test]
fn region_written_after_a_rect_wins_where_it_hits_and_lets_the_rect_through_elsewhere() {
    let map = canvas(300, 300, WHITE, &[(20, 20, 80, 80, RED)]);
    let (mut world, presenter) = shown_with(
        &[
            "collisionex0,Under,rect,20,20,280,280",
            "collisionex1,RegRed,region,map_full.png,255,0,0",
        ],
        (300, 300),
        vec![("map_full.png", map)],
    );
    check(
        &mut world,
        &presenter,
        &[
            (50, 50, Some("RegRed")),
            (150, 150, Some("Under")),
            (10, 10, None),
        ],
    );
}

#[test]
fn region_imported_from_a_child_is_placed_at_the_shifted_position() {
    let text = lines(&[
        "charset,UTF-8",
        "surface0",
        "{",
        "element0,overlay,p.png,0,0",
        "element1,overlay,10,20,10",
        "collisionex0,Own,region,m.png,255,0,0",
        "}",
        "surface10",
        "{",
        "element0,overlay,c.png,0,0",
        "collisionex0,Child,region,m.png,255,0,0",
        "}",
    ]);
    let dir = temp_path_kit::TempPath::new("presenter-hit-region-child");
    std::fs::write(dir.path().join("surfaces.txt"), text).expect("surfaces.txt 作成");
    std::fs::write(
        dir.path().join("descript.txt"),
        lines(&["charset,UTF-8", "seriko.use_self_alpha,1"]),
    )
    .expect("descript.txt 作成");
    let mut dec = MemoryDecoder::new();
    for (name, w, h) in [("p.png", 100, 100), ("c.png", 10, 10)] {
        std::fs::File::create(dir.path().join(name)).expect("絵の作成");
        dec.insert(
            dir.path().join(name),
            w,
            h,
            w * 4,
            gradient(w, h, 0x15),
            true,
        );
    }
    dec.insert_unmultiplied(
        dir.path().join("m.png"),
        straight_image(10, 10, |x, _| if x < 5 { RED } else { BLUE }),
    );
    let mut target =
        crate::shell_target::load_shell_target(dir.path(), &dec).expect("シェルは読める");
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(
        &mut presenter,
        &mut world,
        T,
        (target.take_world(), target.atlas().clone()),
        0,
        96,
    );
    check(
        &mut world,
        &presenter,
        &[
            (2, 2, Some("Own")),
            (5, 2, None),
            (20, 10, Some("Child")),
            (22, 12, Some("Child")),
            (24, 19, Some("Child")),
            (25, 10, None),
            (29, 19, None),
            (19, 10, None),
            (20, 9, None),
            (20, 20, None),
            (2, 12, None),
        ],
    );
}
