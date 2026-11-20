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
