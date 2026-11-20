//! ID の描き先の塗り（WARP）。ID は許容差なしで比べる。

use super::composite::create_bitmap;
use super::id_target::*;
use super::visual::emit_test_support::*;
use std::num::NonZeroU32;
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
        .paint_with(&dc, &[rect_shape(n, 100.0, 200.0, 200.0, 260.0)])
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
    target.paint_with(&dc, &shapes).unwrap();
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
        .paint_with(&dc, &[rect_shape(1, 0.0, 0.0, 10.0, 10.0)])
        .unwrap();
    target.paint_with(&dc, &[]).unwrap();
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
        .paint_with(&dc, &[rect_shape(1, 0.0, 0.0, 10.0, 10.0)])
        .unwrap();
    let refused = target.paint_with(
        &dc,
        &[
            rect_shape(2, 0.0, 0.0, 50.0, 50.0),
            rect_shape(MAX_ID + 1, 60.0, 60.0, 70.0, 70.0),
        ],
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
