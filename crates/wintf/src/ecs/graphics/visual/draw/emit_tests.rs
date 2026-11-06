//! 画素の答え（WARP・要件 1.1, 6.1, 6.2, 6.4）。比べる点は塗りの境界から 2 画素以上離す。

use super::super::*;
use super::emit_test_support::*;
use crate::ecs::graphics::ClipShape;
use crate::ecs::layout::Arrangement;
use crate::numerics::Transform2D;
use bevy_ecs::hierarchy::ChildOf;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows_numerics::Matrix3x2;

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

fn rect8() -> D2D_RECT_F {
    D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 8.0,
        bottom: 8.0,
    }
}

/// 16×16 の根に赤 (0,0)-(8,8) と、その後の兄弟の青 (4,4)-(12,12)。
fn red_then_blue(world: &mut World) -> Entity {
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id();
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 8.0, 8.0),
        solid(rect8(), RED),
        ChildOf(root),
    ));
    world.spawn((
        Visual::default(),
        at(4.0, 4.0, 8.0, 8.0),
        solid(rect8(), BLUE),
        ChildOf(root),
    ));
    root
}

#[test]
fn painter_order_later_sibling_is_on_top() {
    let mut world = World::new();
    let root = red_then_blue(&mut world);
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 2, 2);
    pixel_near(&px, BLUE, 6, 6); // 重なり
    pixel_near(&px, BLUE, 10, 10);
    pixel_near(&px, CLEAR, 14, 14);
    pixel_near(&px, CLEAR, 2, 14);
}

#[test]
fn same_tree_twice_is_bit_identical() {
    let mut world = World::new();
    let root = red_then_blue(&mut world);
    // 図形の切り抜き（層）の道も通す: 回転した矩形の切り抜きの下に青。
    let g = world
        .spawn((
            Visual {
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            at(8.0, 2.0, 6.0, 6.0),
            VisualTransform(Transform2D {
                rotation: 0.3,
                ..Transform2D::identity()
            }),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 8.0, 8.0),
        solid(rect8(), BLUE),
        ChildOf(g),
    ));
    let scale = Matrix3x2 {
        M11: 1.5,
        M22: 1.5,
        ..Matrix3x2::identity()
    };
    let a = render_tree_to_pixels(&mut world, root, 24, 24, scale);
    let b = render_tree_to_pixels(&mut world, root, 24, 24, scale);
    // 較正の前提: 絵が退化していない（塗られた画素と透明な画素が在る）。
    assert!(a.bytes.chunks(4).any(|p| p[3] == 255));
    assert!(a.bytes.chunks(4).any(|p| p[3] == 0));
    assert!(a.bytes == b.bytes, "同じ木を 2 度描いた絵が違う");
}
