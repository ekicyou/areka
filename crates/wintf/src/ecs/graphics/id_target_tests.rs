//! ID の描き先の塗り（WARP）。ID は許容差なしで比べる。

use super::composite::create_bitmap;
use super::id_target::*;
use super::visual::emit_test_support::*;
use std::num::NonZeroU32;
use std::sync::Arc;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::Matrix3x2;

const SIZE: (u32, u32) = (320, 300);

fn id(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}

fn rect_shape(n: u32, left: f32, top: f32, right: f32, bottom: f32) -> IdShape {
    IdShape {
        id: id(n),
        form: IdForm::Rect {
            left,
            top,
            right,
            bottom,
        },
    }
}

/// 描き先の全画素を CPU へ写した BGRA。
fn pixels(dc: &ID2D1DeviceContext, target: &IdTarget) -> Vec<[u8; 4]> {
    let (w, h) = target.size();
    let staging = create_bitmap(
        dc,
        (w, h),
        D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
    )
    .unwrap();
    let mut out = Vec::with_capacity((w * h) as usize);
    // SAFETY: Windows API 境界。`Map(READ)` の `bits` は少なくとも `pitch * h` バイトを指す。
    unsafe {
        staging.CopyFromBitmap(None, target.bitmap(), None).unwrap();
        let mapped = staging.Map(D2D1_MAP_OPTIONS_READ).unwrap();
        for y in 0..h as usize {
            let row = mapped.bits.add(y * mapped.pitch as usize);
            for x in 0..w as usize {
                out.push(*(row.add(x * 4) as *const [u8; 4]));
            }
        }
        staging.Unmap().unwrap();
    }
    out
}

/// ID `n` の不透明の色（B＝下位・G＝中位・R＝上位）。
fn bgra(n: u32) -> [u8; 4] {
    [n as u8, (n >> 8) as u8, (n >> 16) as u8, 255]
}

#[test]
fn rect_covers_left_top_but_not_right_bottom() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    let n = 0x12_34_56;
    target
        .paint_with(
            &dc,
            &[rect_shape(n, 100.0, 200.0, 200.0, 260.0)],
            &mut IdMaskPictures::new(),
        )
        .unwrap();
    let got = pixels(&dc, &target);
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let inside = (100..=199).contains(&x) && (200..=259).contains(&y);
            let want = if inside { bgra(n) } else { CLEAR };
            assert_eq!(got[(y * SIZE.0 + x) as usize], want, "({x},{y})");
        }
    }
}

#[test]
fn later_shapes_overwrite_and_polygon_and_ellipse_paint() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    let shapes = [
        rect_shape(1, 0.0, 0.0, 100.0, 100.0),
        IdShape {
            id: id(2),
            form: IdForm::Ellipse {
                center: (50.0, 50.0),
                radius: (20.0, 10.0),
            },
        },
        IdShape {
            id: id(MAX_ID),
            form: IdForm::Polygon(vec![(150.0, 0.0), (250.0, 0.0), (250.0, 100.0)]),
        },
    ];
    target
        .paint_with(&dc, &shapes, &mut IdMaskPictures::new())
        .unwrap();
    let got = pixels(&dc, &target);
    let at = |x: u32, y: u32| got[(y * SIZE.0 + x) as usize];
    assert_eq!(at(10, 10), bgra(1));
    assert_eq!(at(50, 50), bgra(2));
    assert_eq!(at(50, 65), bgra(1));
    assert_eq!(at(240, 20), bgra(MAX_ID));
    assert_eq!(at(160, 90), CLEAR);
}

#[test]
fn self_intersecting_polygon_fills_nonzero_winding() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    // 五芒星（中心 (150,150)・外径 100）。巻き数 2 の中心も塗る。
    let star: Vec<(f32, f32)> = (0..5)
        .map(|i| {
            let a = std::f32::consts::PI * (-0.5 + 0.8 * i as f32);
            (150.0 + 100.0 * a.cos(), 150.0 + 100.0 * a.sin())
        })
        .collect();
    target
        .paint_with(
            &dc,
            &[IdShape {
                id: id(7),
                form: IdForm::Polygon(star),
            }],
            &mut IdMaskPictures::new(),
        )
        .unwrap();
    let got = pixels(&dc, &target);
    assert_eq!(got[(150 * SIZE.0 + 150) as usize], bgra(7));
}

#[test]
fn paint_restores_shared_dc_and_counts_generations() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    assert_eq!(target.generation(), 0);
    // SAFETY: Windows API 境界。
    unsafe { dc.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };
    target
        .paint_with(
            &dc,
            &[rect_shape(1, 0.0, 0.0, 10.0, 10.0)],
            &mut IdMaskPictures::new(),
        )
        .unwrap();
    target
        .paint_with(&dc, &[], &mut IdMaskPictures::new())
        .unwrap();
    assert_eq!(target.generation(), 2);
    // SAFETY: Windows API 境界。
    unsafe {
        assert!(dc.GetTarget().is_err());
        let mut m = Matrix3x2::default();
        dc.GetTransform(&mut m);
        assert_eq!(m, Matrix3x2::identity());
        assert_eq!(dc.GetAntialiasMode(), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    }
}

#[test]
fn id_over_the_bound_refuses_the_whole_list() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    target
        .paint_with(
            &dc,
            &[rect_shape(1, 0.0, 0.0, 10.0, 10.0)],
            &mut IdMaskPictures::new(),
        )
        .unwrap();
    let refused = target.paint_with(
        &dc,
        &[
            rect_shape(2, 0.0, 0.0, 50.0, 50.0),
            rect_shape(MAX_ID + 1, 60.0, 60.0, 70.0, 70.0),
        ],
        &mut IdMaskPictures::new(),
    );
    assert!(refused.is_err());
    assert_eq!(target.generation(), 0);
    let got = pixels(&dc, &target);
    assert_eq!(got[(20 * SIZE.0 + 20) as usize], CLEAR);
    assert_eq!(got[5 * SIZE.0 as usize + 5], bgra(1));
}

#[test]
fn size_out_of_range_is_refused() {
    let dc = warp_dc();
    for size in [(0, 1), (1, 0), (16_385, 1), (1, 16_385)] {
        assert!(IdTarget::with_dc(&dc, size).is_err(), "{size:?}");
    }
    assert_eq!(IdTarget::with_dc(&dc, (1, 1)).unwrap().size(), (1, 1));
}

/// 決まった種から 0.5 の割合で立つ画素の表。
fn random_bits(w: u32, h: u32, seed: u64) -> Vec<bool> {
    let mut state = seed;
    (0..w * h)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state >> 63 == 1
        })
        .collect()
}

fn mask_from(bits: &[bool], w: u32, h: u32) -> Arc<IdMask> {
    Arc::new(IdMask::new(w, h, |x, y| bits[(y * w + x) as usize]).unwrap())
}

fn mask_shape(n: u32, mask: &Arc<IdMask>, left: i32, top: i32) -> IdShape {
    IdShape {
        id: id(n),
        form: IdForm::Mask {
            mask: Arc::clone(mask),
            left,
            top,
        },
    }
}

#[test]
fn mask_puts_the_id_only_on_set_pixels_one_to_one_and_is_clipped() {
    let dc = warp_dc();
    let (w, h) = (100, 100);
    let bits = random_bits(w, h, 0x5EED);
    let mask = mask_from(&bits, w, h);
    let mut pictures = IdMaskPictures::new();
    // 面の中・負の位置・面からはみ出す位置。
    for (left, top) in [(20, 30), (-10, 250), (250, -40)] {
        let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
        target
            .paint_with(
                &dc,
                &[
                    rect_shape(1, 0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32),
                    mask_shape(9, &mask, left, top),
                ],
                &mut pictures,
            )
            .unwrap();
        let got = pixels(&dc, &target);
        for y in 0..SIZE.1 as i32 {
            for x in 0..SIZE.0 as i32 {
                let (mx, my) = (x - left, y - top);
                let set = (0..w as i32).contains(&mx)
                    && (0..h as i32).contains(&my)
                    && bits[(my as u32 * w + mx as u32) as usize];
                let want = if set { bgra(9) } else { bgra(1) };
                assert_eq!(
                    got[(y as u32 * SIZE.0 + x as u32) as usize],
                    want,
                    "at ({left},{top}) pixel ({x},{y})"
                );
            }
        }
    }
    assert_eq!(pictures.created(), 1);
}

#[test]
fn the_same_mask_is_uploaded_once_and_another_instance_again() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    let bits = random_bits(8, 8, 1);
    let mask = mask_from(&bits, 8, 8);
    let mut pictures = IdMaskPictures::new();
    assert!(pictures.is_empty());
    for _ in 0..2 {
        target
            .paint_with(&dc, &[mask_shape(3, &mask, 0, 0)], &mut pictures)
            .unwrap();
    }
    assert_eq!((pictures.len(), pictures.created()), (1, 1));
    let twin = mask_from(&bits, 8, 8);
    target
        .paint_with(
            &dc,
            &[mask_shape(3, &mask, 0, 0), mask_shape(4, &twin, 10, 10)],
            &mut pictures,
        )
        .unwrap();
    assert_eq!((pictures.len(), pictures.created()), (2, 2));
}

#[test]
fn pictures_of_another_device_fail_the_paint() {
    let dc = warp_dc();
    let mask = mask_from(&random_bits(4, 4, 2), 4, 4);
    let mut pictures = IdMaskPictures::new();
    IdTarget::with_dc(&dc, SIZE)
        .unwrap()
        .paint_with(&dc, &[mask_shape(3, &mask, 0, 0)], &mut pictures)
        .unwrap();
    let other = other_device_dc();
    let mut target = IdTarget::with_dc(&other, SIZE).unwrap();
    let refused = target.paint_with(&other, &[mask_shape(3, &mask, 0, 0)], &mut pictures);
    assert!(refused.is_err());
    assert_eq!(target.generation(), 0);
    // SAFETY: Windows API 境界。
    assert!(unsafe { other.GetTarget() }.is_err());
}

#[test]
fn mask_size_out_of_range_is_refused() {
    for (w, h) in [(0, 1), (1, 0), (16_385, 1), (1, 16_385)] {
        assert!(IdMask::new(w, h, |_, _| true).is_none(), "{w}x{h}");
    }
    let edge = IdMask::new(1, 16_384, |_, _| false).unwrap();
    assert_eq!((edge.width(), edge.height()), (1, 16_384));
}

#[test]
fn read_tells_unreadable_outside_empty_and_the_new_id() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    assert_eq!(target.read(15, 15), IdRead::Unreadable);
    let (w, h) = SIZE;
    for (x, y) in [(w, 0), (0, h), (u32::MAX, u32::MAX)] {
        assert_eq!(target.read(x, y), IdRead::Outside, "({x},{y})");
    }
    assert_eq!(target.gpu_reads(), 0);
    let mut pictures = IdMaskPictures::new();
    target
        .paint_with(&dc, &[rect_shape(5, 10.0, 10.0, 20.0, 20.0)], &mut pictures)
        .unwrap();
    assert_eq!(target.read(15, 15), IdRead::Id(id(5)));
    assert_eq!(target.read(0, 0), IdRead::Empty);
    assert_eq!(target.read(w - 1, h - 1), IdRead::Empty);
    assert_eq!(target.read(w, h - 1), IdRead::Outside);
    target
        .paint_with(
            &dc,
            &[rect_shape(MAX_ID, 10.0, 10.0, 20.0, 20.0)],
            &mut pictures,
        )
        .unwrap();
    assert_eq!(target.read(15, 15), IdRead::Id(id(MAX_ID)));
}

#[test]
fn the_same_point_is_not_read_again_until_repainted() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    let mut pictures = IdMaskPictures::new();
    target
        .paint_with(&dc, &[rect_shape(5, 10.0, 10.0, 20.0, 20.0)], &mut pictures)
        .unwrap();
    assert_eq!(target.read(15, 15), IdRead::Id(id(5)));
    assert_eq!(target.read(15, 15), IdRead::Id(id(5)));
    assert_eq!(target.gpu_reads(), 1);
    assert_eq!(target.read(0, 0), IdRead::Empty);
    assert_eq!(target.read(0, 0), IdRead::Empty);
    assert_eq!(target.gpu_reads(), 2);
    target
        .paint_with(&dc, &[rect_shape(6, 0.0, 0.0, 20.0, 20.0)], &mut pictures)
        .unwrap();
    assert_eq!(target.read(0, 0), IdRead::Id(id(6)));
    assert_eq!(target.gpu_reads(), 3);
}

#[test]
fn a_failed_paint_reads_unreadable_without_the_gpu() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    let mut pictures = IdMaskPictures::new();
    target
        .paint_with(&dc, &[rect_shape(5, 10.0, 10.0, 20.0, 20.0)], &mut pictures)
        .unwrap();
    assert_eq!(target.read(15, 15), IdRead::Id(id(5)));
    let refused = target.paint_with(
        &dc,
        &[rect_shape(MAX_ID + 1, 0.0, 0.0, 1.0, 1.0)],
        &mut pictures,
    );
    assert!(refused.is_err());
    assert_eq!(target.read(15, 15), IdRead::Unreadable);
    assert_eq!(target.gpu_reads(), 1);
}

#[test]
fn a_failed_read_is_logged_once_and_stays_unreadable_until_repainted() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    let mut pictures = IdMaskPictures::new();
    target
        .paint_with(&dc, &[rect_shape(5, 10.0, 10.0, 20.0, 20.0)], &mut pictures)
        .unwrap();
    assert_eq!(target.read(15, 15), IdRead::Id(id(5)));
    target.break_staging(&dc);
    let mut answers = Vec::new();
    let log = crate::ecs::test_support::capture_under_filter("error", || {
        answers.push(target.read(0, 0));
        answers.push(target.read(0, 0));
        answers.push(target.read(15, 15));
    });
    assert_eq!(answers, [IdRead::Unreadable; 3]);
    assert_eq!(log.matches("[id_target] read failed").count(), 1, "{log}");
    assert_eq!(target.gpu_reads(), 2);
    target
        .paint_with(&dc, &[rect_shape(5, 10.0, 10.0, 20.0, 20.0)], &mut pictures)
        .unwrap();
    assert_eq!(target.read(15, 15), IdRead::Unreadable);
    assert_eq!(target.gpu_reads(), 3);
}

#[test]
fn a_pixel_neither_clear_nor_opaque_is_logged_and_unreadable() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    target
        .paint_with(&dc, &[], &mut IdMaskPictures::new())
        .unwrap();
    // SAFETY: Windows API 境界。
    unsafe {
        dc.SetTarget(target.bitmap());
        dc.BeginDraw();
        dc.Clear(Some(
            &windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                r: 0.25,
                g: 0.25,
                b: 0.25,
                a: 0.5,
            },
        ));
        dc.EndDraw(None, None).unwrap();
        dc.SetTarget(None);
    }
    let mut answer = IdRead::Empty;
    let log = crate::ecs::test_support::capture_under_filter("error", || {
        answer = target.read(3, 3);
    });
    assert_eq!(answer, IdRead::Unreadable);
    assert_eq!(
        log.matches("[id_target] unexpected pixel").count(),
        1,
        "{log}"
    );
}

fn painted(dc: &ID2D1DeviceContext, shapes: &[IdShape]) -> IdTarget {
    let mut target = IdTarget::with_dc(dc, SIZE).unwrap();
    target
        .paint_with(dc, shapes, &mut IdMaskPictures::new())
        .unwrap();
    target
}

fn ellipse(n: u32, center: (f32, f32), radius: (f32, f32)) -> IdShape {
    IdShape {
        id: id(n),
        form: IdForm::Ellipse { center, radius },
    }
}

fn polygon(n: u32, points: Vec<(f32, f32)>) -> IdShape {
    IdShape {
        id: id(n),
        form: IdForm::Polygon(points),
    }
}

/// `read` で点ごとに答えを比べる。`inside` が真の点だけ `Id(n)`、それ以外は `Empty`。
fn assert_reads(target: &IdTarget, n: u32, points: &[((u32, u32), bool)]) {
    for &((x, y), inside) in points {
        let want = if inside {
            IdRead::Id(id(n))
        } else {
            IdRead::Empty
        };
        assert_eq!(target.read(x, y), want, "({x},{y})");
    }
}

#[test]
fn rect_edges_read_exactly() {
    let dc = warp_dc();
    let target = painted(&dc, &[rect_shape(3, 100.0, 200.0, 200.0, 260.0)]);
    assert_reads(
        &target,
        3,
        &[
            ((99, 200), false),
            ((100, 200), true),
            ((100, 199), false),
            ((199, 259), true),
            ((200, 259), false),
            ((199, 260), false),
            ((150, 230), true),
        ],
    );
}

#[test]
fn ellipse_and_circle_cover_their_axis_ends() {
    let dc = warp_dc();
    let target = painted(&dc, &[ellipse(4, (150.0, 100.0), (50.0, 30.0))]);
    assert_reads(
        &target,
        4,
        &[
            ((99, 100), false),
            ((100, 100), true),
            ((199, 100), true),
            ((200, 100), false),
            ((149, 69), false),
            ((149, 70), true),
            ((149, 129), true),
            ((149, 130), false),
        ],
    );
    let target = painted(&dc, &[ellipse(5, (150.0, 150.0), (40.0, 40.0))]);
    assert_reads(
        &target,
        5,
        &[
            ((109, 150), false),
            ((110, 150), true),
            ((189, 150), true),
            ((190, 150), false),
            ((150, 109), false),
            ((150, 110), true),
            ((150, 189), true),
            ((150, 190), false),
        ],
    );
}

#[test]
fn square_polygon_covers_20_to_79() {
    let dc = warp_dc();
    let square = vec![(20.0, 20.0), (80.0, 20.0), (80.0, 80.0), (20.0, 80.0)];
    let target = painted(&dc, &[polygon(6, square)]);
    assert_reads(
        &target,
        6,
        &[
            ((19, 50), false),
            ((20, 50), true),
            ((79, 50), true),
            ((80, 50), false),
            ((50, 19), false),
            ((50, 20), true),
            ((50, 79), true),
            ((50, 80), false),
        ],
    );
}

#[test]
fn pentagram_fills_the_centre_but_not_the_notches() {
    let dc = warp_dc();
    let star: Vec<(f32, f32)> = (0..5)
        .map(|i| {
            let a = std::f32::consts::PI * (-0.5 + 0.8 * i as f32);
            (150.0 + 100.0 * a.cos(), 150.0 + 100.0 * a.sin())
        })
        .collect();
    let target = painted(&dc, &[polygon(7, star)]);
    // 隣り合う頂点の間の方向で、凸包の中・星の外の点（中心から 70）。
    let notches: Vec<_> = (0..5)
        .map(|i| {
            let a = std::f32::consts::PI * (-0.3 + 0.4 * i as f32);
            (
                (150.0 + 70.0 * a.cos()).floor() as u32,
                (150.0 + 70.0 * a.sin()).floor() as u32,
            )
        })
        .collect();
    let mut points = vec![((150, 150), true), ((149, 70), true)];
    points.extend(notches.into_iter().map(|p| (p, false)));
    assert_reads(&target, 7, &points);
}

#[test]
fn three_overlapping_shapes_keep_the_later_id_and_never_mix() {
    let dc = warp_dc();
    let shapes = [
        rect_shape(1, 0.0, 0.0, 100.0, 100.0),
        rect_shape(2, 50.0, 50.0, 150.0, 150.0),
        rect_shape(3, 80.0, 20.0, 120.0, 120.0),
    ];
    let target = painted(&dc, &shapes);
    let want_at = |x: u32, y: u32| -> u32 {
        let hit = |l: u32, t: u32, r: u32, b: u32| (l..r).contains(&x) && (t..b).contains(&y);
        if hit(80, 20, 120, 120) {
            3
        } else if hit(50, 50, 150, 150) {
            2
        } else if hit(0, 0, 100, 100) {
            1
        } else {
            0
        }
    };
    let got = pixels(&dc, &target);
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let want = match want_at(x, y) {
                0 => CLEAR,
                n => bgra(n),
            };
            assert_eq!(got[(y * SIZE.0 + x) as usize], want, "({x},{y})");
        }
    }
    for (x, y, want) in [
        (90, 30, 3),
        (85, 60, 3),
        (55, 55, 2),
        (140, 140, 2),
        (10, 10, 1),
    ] {
        assert_eq!(target.read(x, y), IdRead::Id(id(want)), "({x},{y})");
    }
    assert_eq!(target.read(160, 160), IdRead::Empty);

    let reversed = [shapes[2].clone(), shapes[1].clone(), shapes[0].clone()];
    let target = painted(&dc, &reversed);
    assert_eq!(target.read(90, 30), IdRead::Id(id(1)));
}

#[test]
fn curved_and_diagonal_edges_leave_no_mixed_pixels() {
    let dc = warp_dc();
    let shapes = [
        rect_shape(1, 0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32),
        ellipse(2, (100.3, 100.7), (63.4, 41.9)),
        polygon(3, vec![(150.0, 20.0), (300.0, 130.0), (190.0, 280.0)]),
    ];
    let target = painted(&dc, &shapes);
    let got = pixels(&dc, &target);
    let (mut twos, mut threes) = (0, 0);
    for (i, px) in got.iter().enumerate() {
        assert!(
            [bgra(1), bgra(2), bgra(3)].contains(px),
            "pixel ({},{}) = {px:?}",
            i as u32 % SIZE.0,
            i as u32 / SIZE.0
        );
        twos += (*px == bgra(2)) as u32;
        threes += (*px == bgra(3)) as u32;
    }
    assert!(twos > 0 && threes > 0);
}

#[test]
fn mask_reads_only_set_pixels_overflowing_and_negative() {
    let dc = warp_dc();
    let mask = Arc::new(IdMask::new(4, 4, |x, y| (x + y) % 2 == 0).unwrap());
    let set = |x: i32, y: i32, left: i32, top: i32| {
        let (mx, my) = (x - left, y - top);
        (0..4).contains(&mx) && (0..4).contains(&my) && (mx + my) % 2 == 0
    };
    for (left, top) in [(10, 10), (-2, -2), (SIZE.0 as i32 - 2, SIZE.1 as i32 - 2)] {
        let target = painted(&dc, &[mask_shape(8, &mask, left, top)]);
        for y in (top - 1)..(top + 5) {
            for x in (left - 1)..(left + 5) {
                if x < 0 || y < 0 || x >= SIZE.0 as i32 || y >= SIZE.1 as i32 {
                    continue;
                }
                let want = if set(x, y, left, top) {
                    IdRead::Id(id(8))
                } else {
                    IdRead::Empty
                };
                assert_eq!(
                    target.read(x as u32, y as u32),
                    want,
                    "at ({left},{top}) ({x},{y})"
                );
            }
        }
    }
}

#[test]
fn a_refused_first_list_reads_unreadable_without_the_gpu() {
    let dc = warp_dc();
    let mut target = IdTarget::with_dc(&dc, SIZE).unwrap();
    let refused = target.paint_with(
        &dc,
        &[
            rect_shape(1, 0.0, 0.0, 10.0, 10.0),
            rect_shape(MAX_ID + 1, 0.0, 0.0, 10.0, 10.0),
        ],
        &mut IdMaskPictures::new(),
    );
    assert!(refused.is_err());
    assert_eq!(target.read(5, 5), IdRead::Unreadable);
    assert_eq!(target.read(SIZE.0, 0), IdRead::Outside);
    assert_eq!(target.gpu_reads(), 0);
}

#[test]
fn ids_round_trip_for_component_edges_and_above_255() {
    let dc = warp_dc();
    let mut ids: Vec<u32> = Vec::new();
    for c in [0u32, 1, 127, 128, 254, 255] {
        for shift in [0, 8, 16] {
            ids.push(c << shift);
        }
        ids.push(c | 0x01_00_00 | 0x80_00);
        ids.push(c << 16 | 0x7F << 8 | 0xFE);
    }
    ids.extend([
        256,
        257,
        511,
        512,
        0x01_00_00,
        0x01_00_01,
        MAX_ID - 1,
        MAX_ID,
    ]);
    ids.retain(|&n| n != 0);
    ids.sort_unstable();
    ids.dedup();
    let width = ids.len() as u32 * 2;
    let mut target = IdTarget::with_dc(&dc, (width, 4)).unwrap();
    let shapes: Vec<_> = ids
        .iter()
        .enumerate()
        .map(|(i, &n)| rect_shape(n, 2.0 * i as f32, 0.0, 2.0 * i as f32 + 2.0, 4.0))
        .collect();
    target
        .paint_with(&dc, &shapes, &mut IdMaskPictures::new())
        .unwrap();
    for (i, &n) in ids.iter().enumerate() {
        assert_eq!(target.read(2 * i as u32, 1), IdRead::Id(id(n)), "{n:#08X}");
        assert_eq!(
            target.read(2 * i as u32 + 1, 3),
            IdRead::Id(id(n)),
            "{n:#08X}"
        );
    }
}
