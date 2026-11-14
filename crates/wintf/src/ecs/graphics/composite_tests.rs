//! 入れ物の答え（WARP・要件 4.1, 4.2, 4.4, 4.8, 7.7）。入れ物へ合成して別の的の整数の offset へ
//! 写した絵が、offset を変換に入れて直接描いた絵（`render_surface` の今の道）とビットで一致する。

use super::composite::{SurfaceComposite, compose_or_drop};
use super::visual::emit_test_support::*;
use super::visual::*;
use crate::ecs::layout::Arrangement;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::core::Interface;
use windows_numerics::Matrix3x2;

/// 写し先の大きさと、入れ物を置く整数の offset（Surface Atlas の中の位置を模す）。
const DST: u32 = 40;
const OFFSET: POINT = POINT { x: 7, y: 5 };

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

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

fn scale(s: f32) -> Matrix3x2 {
    Matrix3x2 {
        M11: s,
        M22: s,
        ..Matrix3x2::identity()
    }
}

/// 根の命令の列（緑の (1,1)-(10,10)）。
fn root_list() -> ID2D1CommandList {
    solid(rect(1.0, 1.0, 10.0, 10.0), [0, 255, 0, 255]).command_list
}

/// 16×16 の根の下に赤と、重なる半透明でない青、回転した図形の切り抜きの下の青。
fn tree(world: &mut World) -> Entity {
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id();
    world.spawn((
        Visual::default(),
        at(3.0, 3.0, 8.0, 8.0),
        solid(rect(0.0, 0.0, 8.0, 8.0), RED),
        ChildOf(root),
    ));
    world.spawn((
        Visual::default(),
        at(7.0, 7.0, 8.0, 8.0),
        solid(rect(0.0, 0.0, 8.0, 8.0), BLUE),
        ChildOf(root),
    ));
    let g = world
        .spawn((
            Visual {
                clip: Some(crate::ecs::graphics::ClipShape::Rectangle),
                ..Default::default()
            },
            at(9.0, 1.0, 6.0, 6.0),
            VisualTransform(crate::numerics::Transform2D {
                rotation: 0.3,
                ..crate::numerics::Transform2D::identity()
            }),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 8.0, 8.0),
        solid(rect(0.0, 0.0, 8.0, 8.0), BLUE),
        ChildOf(g),
    ));
    root
}

/// 今の道: offset を変換に入れ、根の列を `DrawImage`（`LINEAR`・`SOURCE_OVER`）し、木を再生する。
fn direct(s: f32, list: Option<&ID2D1CommandList>, commands: &[DrawCommand]) -> Pixels {
    let transform = Matrix3x2 {
        M31: OFFSET.x as f32,
        M32: OFFSET.y as f32,
        ..scale(s)
    };
    draw_to_pixels(DST, DST, transform, |dc| unsafe {
        if let Some(list) = list {
            dc.DrawImage(
                list,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            );
        }
        if !commands.is_empty() {
            let dc6 = dc.cast::<ID2D1DeviceContext6>().ok();
            emit(dc, dc6.as_ref(), transform, commands);
        }
    })
}

/// 入れ物の道: 描画面の大きさの入れ物へ原点で合成し、別の的の offset へ写す。
fn via_composite(
    s: f32,
    list: Option<&ID2D1CommandList>,
    commands: &[DrawCommand],
) -> (SurfaceComposite, Pixels) {
    let size = (16.0 * s).ceil() as u32;
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (size, size)).expect("入れ物");
    comp.compose(&dc, scale(s), list, commands).expect("合成");
    let px = copy(&comp);
    (comp, px)
}

fn copy(comp: &SurfaceComposite) -> Pixels {
    draw_to_pixels(DST, DST, scale(3.0), |dst| comp.copy_to(dst, OFFSET))
}

/// 較正の前提: 絵が退化していない（不透明の画素と透明な画素が在る）。
fn assert_not_degenerate(px: &Pixels) {
    assert!(
        px.bytes.chunks(4).any(|p| p[3] == 255),
        "不透明の画素が無い"
    );
    assert!(px.bytes.chunks(4).any(|p| p[3] == 0), "透明な画素が無い");
}

#[test]
fn root_list_only_matches_direct_at_scale_1_and_1_25() {
    for s in [1.0, 1.25] {
        let list = root_list();
        let want = direct(s, Some(&list), &[]);
        assert_not_degenerate(&want);
        let (_, got) = via_composite(s, Some(&list), &[]);
        assert!(got.bytes == want.bytes, "拡大 {s}: 根の列だけの絵が違う");
    }
}

#[test]
fn root_list_and_tree_match_direct_at_scale_1_and_1_25() {
    let mut world = World::new();
    let root = tree(&mut world);
    let commands = tree_commands(&mut world, root, None);
    assert!(!commands.is_empty());
    for s in [1.0, 1.25] {
        let list = root_list();
        let want = direct(s, Some(&list), &commands);
        assert_not_degenerate(&want);
        let (_, got) = via_composite(s, Some(&list), &commands);
        assert!(got.bytes == want.bytes, "拡大 {s}: 木を持つ絵が違う");
        // 根の列の後に木（根の緑の上に赤が乗る）。
        let at = |x: f32| (x * s) as u32;
        pixel_near(
            &got,
            RED,
            OFFSET.x as u32 + at(5.0),
            OFFSET.y as u32 + at(5.0),
        );
    }
}

#[test]
fn contents_stay_after_copy_and_second_copy_is_identical() {
    let mut world = World::new();
    let root = tree(&mut world);
    let commands = tree_commands(&mut world, root, None);
    let list = root_list();
    let (comp, first) = via_composite(1.25, Some(&list), &commands);
    assert_not_degenerate(&first);
    let second = copy(&comp);
    assert!(first.bytes == second.bytes, "2 度目の写しが違う");
}

/// 部分の写し（2.6・2.9）: 写す矩形の外は的の前の画素のまま、中は全面の写しと同じ画素になる。
#[test]
fn copy_rect_to_keeps_outside_and_matches_full_copy_inside() {
    let mut world = World::new();
    let root = tree(&mut world);
    let commands = tree_commands(&mut world, root, None);
    let list = root_list();
    let (comp, full) = via_composite(1.25, Some(&list), &commands);
    assert_not_degenerate(&full);
    // 入れ物の (1,2)-(14,20)（左下に透明な画素を含む）を、その左上が全面の写しと同じ位置に来る
    // offset へ写す。
    let r = PxRect {
        left: 1,
        top: 2,
        right: 14,
        bottom: 20,
    };
    let offset = POINT {
        x: OFFSET.x + r.left,
        y: OFFSET.y + r.top,
    };
    let got = draw_to_pixels(DST, DST, scale(3.0), |dst| unsafe {
        dst.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        }));
        comp.copy_rect_to(dst, r, offset);
    });
    let inside = |x: u32, y: u32| {
        (offset.x as u32..offset.x as u32 + (r.right - r.left) as u32).contains(&x)
            && (offset.y as u32..offset.y as u32 + (r.bottom - r.top) as u32).contains(&y)
    };
    for y in 0..DST {
        for x in 0..DST {
            let want = if inside(x, y) { full.at(x, y) } else { BLUE };
            assert_eq!(got.at(x, y), want, "({x},{y})");
        }
    }
    // 較正: 矩形の中に透明な画素と絵の画素の両方が在る（塗りと写しの両方を見ている）。
    let px: Vec<_> = (0..DST)
        .flat_map(|y| (0..DST).map(move |x| (x, y)))
        .filter(|&(x, y)| inside(x, y))
        .map(|(x, y)| got.at(x, y))
        .collect();
    assert!(px.contains(&CLEAR) && px.iter().any(|p| p[3] == 255));
}

#[test]
fn compose_leaves_dc_without_target_and_identity_transform() {
    let mut world = World::new();
    let root = tree(&mut world);
    let commands = tree_commands(&mut world, root, None);
    let list = root_list();
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (20, 20)).unwrap();
    comp.compose(&dc, scale(1.25), Some(&list), &commands)
        .unwrap();
    let mut after = Matrix3x2::default();
    unsafe {
        assert!(dc.GetTarget().is_err(), "合成の後も DC に的が残る");
        dc.GetTransform(&mut after);
    }
    assert_eq!(after, Matrix3x2::identity());
}

#[test]
fn generation_goes_up_per_compose() {
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (16, 16)).unwrap();
    assert_eq!(comp.size(), (16, 16));
    assert_eq!(comp.generation(), 0);
    let list = root_list();
    comp.compose(&dc, scale(1.0), Some(&list), &[]).unwrap();
    assert_eq!(comp.generation(), 1);
    comp.compose(&dc, scale(1.0), None, &[]).unwrap();
    assert_eq!(comp.generation(), 2);
    assert_eq!(
        format!("{comp:?}"),
        "SurfaceComposite { size: (16, 16), generation: 2 }"
    );
}

// ---- 読みの手順（要件 5.1, 5.2, 5.4, 5.9, 7.8）----

/// 根の列と木を拡大 `s` で合成した入れ物と、同じ絵を原点に直接描いた答えの画素。
fn composed_with_answer(s: f32) -> (SurfaceComposite, Pixels) {
    let mut world = World::new();
    let root = tree(&mut world);
    let commands = tree_commands(&mut world, root, None);
    let list = root_list();
    let size = (16.0 * s).ceil() as u32;
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (size, size)).unwrap();
    comp.compose(&dc, scale(s), Some(&list), &commands).unwrap();
    let want = draw_to_pixels(size, size, scale(s), |dc| unsafe {
        dc.DrawImage(
            &list,
            None,
            None,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
        );
        let dc6 = dc.cast::<ID2D1DeviceContext6>().ok();
        emit(dc, dc6.as_ref(), scale(s), &commands);
    });
    assert_not_degenerate(&want);
    (comp, want)
}

#[test]
fn read_pixel_matches_answer_pixel_at_every_point() {
    let (comp, want) = composed_with_answer(1.25);
    let (w, h) = comp.size();
    for y in 0..h {
        for x in 0..w {
            assert_eq!(comp.read_pixel(x, y).unwrap(), want.at(x, y), "({x}, {y})");
        }
    }
    // 較正: 色の違う点を読み分けている（緑・赤・青・透明）。
    assert_eq!(comp.read_pixel(2, 2).unwrap(), [0, 255, 0, 255]);
    pixel_near(&want, RED, 6, 6);
    assert_eq!(comp.read_pixel(19, 19).unwrap(), CLEAR);
}

#[test]
fn read_pixel_returns_new_answer_right_after_recompose() {
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (16, 16)).unwrap();
    let list = root_list();
    comp.compose(&dc, scale(1.0), Some(&list), &[]).unwrap();
    assert_eq!(comp.read_pixel(5, 5).unwrap(), [0, 255, 0, 255]);
    comp.compose(&dc, scale(1.0), None, &[]).unwrap();
    assert_eq!(comp.read_pixel(5, 5).unwrap(), CLEAR, "合成し直す前の値");
    comp.compose(&dc, scale(1.0), Some(&list), &[]).unwrap();
    assert_eq!(comp.read_pixel(5, 5).unwrap(), [0, 255, 0, 255]);
    assert_eq!(comp.gpu_reads(), 3, "世代が変われば GPU から読み直す");
}

#[test]
fn same_point_without_recompose_does_not_read_gpu() {
    let (comp, want) = composed_with_answer(1.0);
    assert_eq!(comp.gpu_reads(), 0);
    assert_eq!(comp.read_pixel(5, 5).unwrap(), want.at(5, 5));
    assert_eq!(comp.gpu_reads(), 1);
    assert_eq!(comp.read_pixel(5, 5).unwrap(), want.at(5, 5));
    assert_eq!(comp.gpu_reads(), 1, "同じ座標で GPU から読んだ");
    assert_eq!(comp.read_pixel(6, 5).unwrap(), want.at(6, 5));
    assert_eq!(comp.gpu_reads(), 2, "座標を変えても GPU から読まない");
    assert_eq!(comp.read_pixel(5, 5).unwrap(), want.at(5, 5));
    assert_eq!(comp.gpu_reads(), 3, "控えは最後の 1 点だけ");
}

#[test]
fn read_all_returns_packed_rows_equal_to_answer() {
    for s in [1.0, 1.25] {
        let (comp, want) = composed_with_answer(s);
        let (w, h) = comp.size();
        let all = comp.read_all(warp_device()).unwrap();
        assert_eq!(
            all.len(),
            (w * h * 4) as usize,
            "1 行＝幅×4 バイトに詰めていない"
        );
        let got = Pixels {
            width: w,
            bytes: all,
        };
        for y in 0..h {
            for x in 0..w {
                assert_eq!(got.at(x, y), want.at(x, y), "拡大 {s}: ({x}, {y})");
            }
        }
        assert_eq!(comp.gpu_reads(), 0, "全体の読みは 1 画素の控えに触らない");
    }
}

#[test]
fn two_threads_reading_different_points_get_their_own_values() {
    let (comp, want) = composed_with_answer(1.0);
    let points = [(2u32, 2u32), (13u32, 13u32)];
    assert_ne!(want.at(2, 2), want.at(13, 13), "較正: 2 点の値が同じ");
    std::thread::scope(|scope| {
        for (x, y) in points {
            let (comp, expected) = (&comp, want.at(x, y));
            scope.spawn(move || {
                for _ in 0..200 {
                    assert_eq!(comp.read_pixel(x, y).unwrap(), expected, "({x}, {y})");
                }
            });
        }
    });
    assert!(comp.gpu_reads() >= 2);
}

#[test]
fn failed_compose_drops_the_composite_and_its_record() {
    let dc = warp_dc();
    let mut slot = Some(SurfaceComposite::new(&dc, (16, 16)).unwrap());
    let list = root_list();
    compose_or_drop(&mut slot, |c| c.compose(&dc, scale(1.0), Some(&list), &[])).unwrap();
    assert_eq!(slot.as_ref().map(|c| c.generation()), Some(1));
    slot.as_mut()
        .unwrap()
        .remember(Some(vec![entry(1)]), Some(&list), (1.0, 1.0), true);
    // 閉じていない命令の列は EndDraw で D2DERR_WRONG_STATE（合成の失敗を決まって起こす）。
    let open = unsafe { dc.CreateCommandList().unwrap() };
    assert!(compose_or_drop(&mut slot, |c| c.compose(&dc, scale(1.0), Some(&open), &[])).is_err());
    assert!(
        slot.is_none(),
        "途中まで描いた入れ物と控えを読ませない: {slot:?}"
    );
}

fn entry(id: u64) -> DamageEntry {
    DamageEntry {
        id,
        shape: id,
        kind: EntryKind::Draw { unbounded: false },
        parent: None,
        end: 1,
        bounds: PxRect {
            left: 1,
            top: 1,
            right: 4,
            bottom: 4,
        },
        exact: [1.0, 1.0, 4.0, 4.0],
        linear: [1.0, 0.0, 0.0, 1.0],
        origin: [1.0, 1.0],
    }
}

#[test]
fn new_composite_has_no_record_and_is_not_presented() {
    let comp = SurfaceComposite::new(&warp_dc(), (16, 16)).unwrap();
    assert_eq!(comp.prev_entries(), None);
    assert!(!comp.presented());
}

#[test]
fn remember_replaces_the_record_and_compose_clears_presented() {
    let mut comp = SurfaceComposite::new(&warp_dc(), (16, 16)).unwrap();
    let (list, other) = (root_list(), root_list());
    comp.remember(
        Some(vec![entry(1), entry(2)]),
        Some(&list),
        (1.25, 1.25),
        true,
    );
    assert_eq!(comp.prev_entries(), Some(&[entry(1), entry(2)][..]));
    // 根の命令の列は参照の同一性で比べる（同じ中身の別の列は違う）。
    assert!(comp.same_root(Some(&list.clone())));
    assert!(!comp.same_root(Some(&other)));
    assert!(!comp.same_root(None));
    // 拡大はビットで比べる（0.0 と -0.0 は違う）。
    assert!(comp.same_scale((1.25, 1.25)));
    assert!(!comp.same_scale((1.25, 1.0)));
    assert!(!comp.presented(), "合成した回は写し終えていない");

    comp.mark_presented();
    assert!(comp.presented());
    // 合成しなかった回（何もしない・写すだけ）は写し終えた印を残す。
    comp.remember(Some(vec![entry(3)]), None, (0.0, 1.0), false);
    assert!(comp.presented());
    assert_eq!(comp.prev_entries(), Some(&[entry(3)][..]));
    assert!(comp.same_root(None));
    assert!(!comp.same_scale((-0.0, 1.0)));

    comp.remember(None, None, (1.0, 1.0), true);
    assert_eq!(comp.prev_entries(), None);
    assert!(!comp.presented());
}
