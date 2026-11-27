//! 画面の外へ描く口の答え（WARP）。同じ装置・同じ木なら、描く係の入れ物へ合成して読み戻した絵とバイトで一致する。

use super::composite::SurfaceComposite;
use super::visual::emit_test_support::*;
use super::visual::*;
use super::{OffscreenImage, render_tree_offscreen};
use crate::ecs::test_support::capture_under_filter;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows_numerics::Matrix3x2;

const SCALE: f32 = 1.5;
const SIZE: (u32, u32) = (24, 24);

/// 16×16 の根の下に赤、半透明の節の下に重なる青、楕円。
fn tree(world: &mut World) -> Entity {
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id();
    world.spawn((
        Visual::default(),
        at(1.0, 1.0, 8.0, 8.0),
        solid(rect(0.0, 0.0, 8.0, 8.0), RED),
        ChildOf(root),
    ));
    let faded = world
        .spawn((
            Visual {
                opacity: 0.5,
                ..Default::default()
            },
            at(5.0, 5.0, 10.0, 10.0),
            VisualTransform(crate::numerics::Transform2D::identity()),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 6.0, 6.0),
        solid(rect(0.0, 0.0, 6.0, 6.0), BLUE),
        ChildOf(faded),
    ));
    world.spawn((
        Visual::default(),
        at(9.0, 2.0, 6.0, 4.0),
        solid_ellipse(6.0, 4.0, RED),
        ChildOf(root),
    ));
    root
}

fn scale() -> Matrix3x2 {
    Matrix3x2 {
        M11: SCALE,
        M22: SCALE,
        ..Matrix3x2::identity()
    }
}

/// 描く係の入れ物（`SurfaceComposite::compose`）へ合成して読み戻した絵。
fn composite_bytes(world: &mut World, root: Entity) -> Vec<u8> {
    let commands = tree_commands(world, root, None);
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, SIZE).unwrap();
    comp.compose(&dc, scale(), None, &commands).unwrap();
    comp.read_all(warp_device()).unwrap()
}

#[test]
fn offscreen_matches_the_surface_composite_byte_for_byte() {
    let mut world = World::new();
    let root = tree(&mut world);
    let expected = composite_bytes(&mut world, root);

    let got = render_tree_offscreen(warp_device(), &mut world, root, SIZE, scale()).unwrap();

    let OffscreenImage {
        width,
        height,
        bytes,
    } = got;
    assert_eq!((width, height), SIZE);
    assert_eq!(bytes.len(), (SIZE.0 * SIZE.1 * 4) as usize);
    let image = Pixels {
        width,
        bytes: bytes.clone(),
    };
    pixel_near(&image, RED, 3, 3);
    pixel_near(&image, CLEAR, 23, 23);
    assert!(bytes == expected, "入れ物の読み戻しとバイトで一致する");
}

#[test]
fn offscreen_without_a_window_or_wuc_on_a_throwaway_world() {
    let mut world = World::new();
    let root = tree(&mut world);
    let got = render_tree_offscreen(
        warp_device(),
        &mut world,
        root,
        (16, 16),
        Matrix3x2::identity(),
    )
    .unwrap();
    let image = Pixels {
        width: got.width,
        bytes: got.bytes,
    };
    pixel_near(&image, RED, 2, 2);
    // 半透明の節の下の青は不透明度 0.5 で赤の上に重なる（乗算済み）。
    let mixed = image.at(6, 6);
    assert!(
        mixed[0] > 100 && mixed[2] > 100 && mixed[3] == 255,
        "{mixed:?}"
    );
}

#[test]
fn offscreen_failure_is_logged_and_returned() {
    let mut world = World::new();
    let root = tree(&mut world);
    let mut result = None;
    // SAFETY: Windows API 境界。
    let max = unsafe { warp_dc().GetMaximumBitmapSize() };
    let log = capture_under_filter("error", || {
        result = Some(render_tree_offscreen(
            warp_device(),
            &mut world,
            root,
            (max + 1, 1),
            Matrix3x2::identity(),
        ));
    });
    assert!(result.unwrap().is_err());
    assert!(log.contains("[render_tree_offscreen]"), "{log}");
}
