//! 読み戻しの口の判定の順（要件 5.3, 5.5, 5.9, 7.8）。入れ物と大きさを差し替えた純粋なテストと、
//! WARP で合成した入れ物のテスト。画素の値そのものの答えは `composite_tests.rs` が固定する。

use super::composite::SurfaceComposite;
use super::readback::*;
use super::visual::emit_test_support::*;
use super::{GraphicsCore, SurfaceGraphics};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows_numerics::Matrix3x2;

const SIZE: (u32, u32) = (12, 10);
const GREEN: [u8; 4] = [0, 255, 0, 255];

/// まだ合成していない入れ物（世代 0）。
fn fresh() -> SurfaceComposite {
    SurfaceComposite::new(&warp_dc(), SIZE).unwrap()
}

/// 緑の (1,1)-(6,6) を 1 度合成した入れ物。
fn composed() -> SurfaceComposite {
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, SIZE).unwrap();
    let rect = D2D_RECT_F {
        left: 1.0,
        top: 1.0,
        right: 6.0,
        bottom: 6.0,
    };
    let list = solid(rect, GREEN).command_list;
    comp.compose(&dc, Matrix3x2::identity(), Some(&list), &[])
        .unwrap();
    comp
}

#[test]
fn outside_comes_before_composite_and_does_not_read_gpu() {
    // 入れ物が無くても、世代 0 でも、合成済みでも、外の座標は Outside。
    assert_eq!(pixel_in(SIZE, None, 12, 0).unwrap(), PixelRead::Outside);
    let comp = fresh();
    assert_eq!(
        pixel_in(SIZE, Some(&comp), 0, 10).unwrap(),
        PixelRead::Outside
    );
    let comp = composed();
    for (x, y) in [(12, 0), (0, 10), (u32::MAX, u32::MAX)] {
        assert_eq!(
            pixel_in(SIZE, Some(&comp), x, y).unwrap(),
            PixelRead::Outside
        );
    }
    assert_eq!(comp.gpu_reads(), 0, "外の座標で GPU から読んだ");
}

#[test]
fn not_composed_is_unreadable_without_gpu() {
    assert_eq!(pixel_in(SIZE, None, 0, 0).unwrap(), PixelRead::Unreadable);
    let comp = fresh();
    assert_eq!(
        pixel_in(SIZE, Some(&comp), 0, 0).unwrap(),
        PixelRead::Unreadable
    );
    assert_eq!(comp.gpu_reads(), 0);
}

#[test]
fn composite_of_other_size_is_unreadable() {
    // 描画面の大きさが変わって入れ物がまだ作り直されていない間は、古い絵を読ませない。
    let comp = composed();
    assert_eq!(
        pixel_in((8, 8), Some(&comp), 2, 2).unwrap(),
        PixelRead::Unreadable
    );
    assert!(image_in((8, 8), Some(&comp), warp_device()).unwrap() == SurfaceImage::Unreadable);
}

#[test]
fn composed_point_is_the_pixel_and_transparent_is_not_outside() {
    let comp = composed();
    assert_eq!(
        pixel_in(SIZE, Some(&comp), 3, 3).unwrap(),
        PixelRead::Pixel(GREEN)
    );
    assert_eq!(
        pixel_in(SIZE, Some(&comp), 11, 9).unwrap(),
        PixelRead::Pixel(CLEAR)
    );
}

#[test]
fn read_all_follows_the_same_rules() {
    assert!(image_in(SIZE, None, warp_device()).unwrap() == SurfaceImage::Unreadable);
    let comp = fresh();
    assert!(image_in(SIZE, Some(&comp), warp_device()).unwrap() == SurfaceImage::Unreadable);
    let comp = composed();
    let SurfaceImage::Pixels {
        width,
        height,
        bytes,
    } = image_in(SIZE, Some(&comp), warp_device()).unwrap()
    else {
        panic!("合成済みなのに読めない");
    };
    assert_eq!((width, height), SIZE);
    assert_eq!(bytes.len(), (SIZE.0 * SIZE.1 * 4) as usize);
    let at = |x: u32, y: u32| {
        let i = ((y * SIZE.0 + x) * 4) as usize;
        [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
    };
    assert_eq!(at(3, 3), GREEN);
    assert_eq!(at(11, 9), CLEAR);
}

#[test]
fn ecs_without_device_or_surface_is_unreadable() {
    let mut world = World::new();
    let blank = world.spawn(SurfaceGraphics::default()).id();
    let none = world.spawn_empty().id();
    let mut state = SystemState::<SurfaceReadback>::new(&mut world);
    {
        // 装置が無い（外の座標でも Outside にならない＝装置の確かめが先）。
        let rb = state.get(&world).unwrap();
        assert_eq!(rb.read_pixel(blank, 100, 100), PixelRead::Unreadable);
        assert!(rb.read_all(blank) == SurfaceImage::Unreadable);
    }
    // 無効な装置も同じ。
    let mut core = GraphicsCore::new().expect("GraphicsCore");
    core.invalidate();
    world.insert_resource(core);
    let rb = state.get(&world).unwrap();
    for owner in [blank, none] {
        assert_eq!(rb.read_pixel(owner, 0, 0), PixelRead::Unreadable);
        assert!(rb.read_all(owner) == SurfaceImage::Unreadable);
    }
}

#[test]
fn ecs_with_device_but_without_surface_is_unreadable() {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    // 描画面の無い部品（大きさ 0）と、部品の無い実体。外の座標でも Outside にならない。
    let blank = world.spawn(SurfaceGraphics::default()).id();
    let none = world.spawn_empty().id();
    let mut state = SystemState::<SurfaceReadback>::new(&mut world);
    let rb = state.get(&world).unwrap();
    for owner in [blank, none] {
        assert_eq!(rb.read_pixel(owner, 5, 5), PixelRead::Unreadable);
        assert!(rb.read_all(owner) == SurfaceImage::Unreadable);
    }
}
