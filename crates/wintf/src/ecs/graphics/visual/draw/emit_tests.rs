//! 画素の答え（WARP・要件 1.1, 1.10, 2.1, 2.3, 2.8, 3.1, 6.1〜6.4）。比べる点は塗りの境界から 2 画素以上離す。

use super::super::*;
use super::emit_test_support::*;
use crate::ecs::graphics::ClipShape;
use crate::ecs::layout::Arrangement;
use crate::numerics::{Aabb, Transform2D};
use bevy_ecs::hierarchy::ChildOf;
use std::f32::consts::FRAC_PI_2;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows_numerics::Matrix3x2;
use windows_numerics::Vector2;

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

fn rect8() -> D2D_RECT_F {
    rect(0.0, 0.0, 8.0, 8.0)
}

fn root16(world: &mut World) -> Entity {
    world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id()
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

#[test]
fn destination_over_puts_later_sibling_below() {
    let mut world = World::new();
    let root = red_then_blue(&mut world);
    let over = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    let blue = *world.get::<Children>(root).unwrap().last().unwrap();
    world
        .get_mut::<VisualDrawContent>(blue)
        .unwrap()
        .composite_mode = D2D1_COMPOSITE_MODE_DESTINATION_OVER;
    let under = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&over, BLUE, 6, 6);
    pixel_near(&under, RED, 6, 6); // 重なりでは後の兄弟が下に入る
    pixel_near(&under, RED, 2, 2);
    pixel_near(&under, BLUE, 10, 10); // 透明の上には描かれる
    pixel_near(&under, CLEAR, 14, 14);
}

#[test]
fn rotation_90_turns_the_rectangle() {
    let mut world = World::new();
    let root = root16(&mut world);
    // 横長 12×4 を 90 度: 局所 (x, y) → (10 - y, 2 + x)＝縦長 (6,2)-(10,14)。
    world.spawn((
        Visual::default(),
        at(10.0, 2.0, 12.0, 4.0),
        VisualTransform(Transform2D {
            rotation: FRAC_PI_2,
            ..Transform2D::identity()
        }),
        solid(rect(0.0, 0.0, 12.0, 4.0), RED),
        ChildOf(root),
    ));
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 8, 4);
    pixel_near(&px, RED, 8, 12);
    pixel_near(&px, CLEAR, 13, 4); // 回さなければ塗られる点
    pixel_near(&px, CLEAR, 3, 8);
}

#[test]
fn scale_2_fills_beyond_the_original_rect() {
    let mut world = World::new();
    let root = root16(&mut world);
    // 4×4 を 2 倍: (2,2)-(10,10)。
    world.spawn((
        Visual::default(),
        at(2.0, 2.0, 4.0, 4.0),
        VisualTransform(Transform2D {
            scale: Vector2 { X: 2.0, Y: 2.0 },
            ..Transform2D::identity()
        }),
        solid(rect(0.0, 0.0, 4.0, 4.0), RED),
        ChildOf(root),
    ));
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 4, 4);
    pixel_near(&px, RED, 8, 8); // 元の矩形からはみ出した点
    pixel_near(&px, CLEAR, 12, 12);
}

#[test]
fn negative_scale_swaps_left_and_right() {
    let mut world = World::new();
    let root = root16(&mut world);
    // 赤 (0..8)・青 (8..16) の組を x → 16 - x で反転＝青が左・赤が右。
    let g = world
        .spawn((
            Visual::default(),
            at(16.0, 4.0, 16.0, 8.0),
            VisualTransform(Transform2D {
                scale: Vector2 { X: -1.0, Y: 1.0 },
                ..Transform2D::identity()
            }),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 8.0, 8.0),
        solid(rect8(), RED),
        ChildOf(g),
    ));
    world.spawn((
        Visual::default(),
        at(8.0, 0.0, 8.0, 8.0),
        solid(rect8(), BLUE),
        ChildOf(g),
    ));
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, BLUE, 4, 8);
    pixel_near(&px, RED, 12, 8);
    pixel_near(&px, CLEAR, 8, 2);
}

#[test]
fn single_node_matches_surface_without_tree() {
    // 同じ命令の列を、木を持たない描画面の道（根の変換で 1 度 DrawImage）と、節 1 つに置いた道で。
    let content = solid(rect(3.0, 5.0, 11.0, 9.0), RED);
    let list = content.command_list.clone();
    let mut world = World::new();
    let root = root16(&mut world);
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 16.0, 16.0),
        content,
        ChildOf(root),
    ));
    // DPI の拡大＋BeginDraw の offset の形。
    let root_transform = Matrix3x2 {
        M11: 1.5,
        M22: 1.5,
        M31: 1.0,
        M32: 2.0,
        ..Matrix3x2::identity()
    };
    let tree = render_tree_to_pixels(&mut world, root, 26, 26, root_transform);
    let plain = render_list_to_pixels(&list, 26, 26, root_transform);
    assert!(plain.bytes.chunks(4).any(|p| p[3] == 255));
    assert!(tree.bytes == plain.bytes, "節 1 つの絵が木なしの絵と違う");
}

#[test]
fn culling_does_not_change_the_picture() {
    let mut world = World::new();
    let root = red_then_blue(&mut world);
    // 縁をまたぐ節（間引かない）と、viewport の外の節（間引く）。
    world.spawn((
        Visual::default(),
        at(12.0, 12.0, 8.0, 8.0),
        solid(rect8(), RED),
        ChildOf(root),
    ));
    world.spawn((
        Visual::default(),
        at(20.0, 20.0, 8.0, 8.0),
        solid(rect8(), BLUE),
        ChildOf(root),
    ));
    let inf = f32::INFINITY;
    let culled = tree_commands(&mut world, root, None);
    let all = tree_commands(&mut world, root, Some(Aabb::new(-inf, -inf, inf, inf)));
    assert_eq!(all.len(), 4);
    assert_eq!(culled.len(), 3, "viewport の外の節が間引かれていない");
    let id = Matrix3x2::identity();
    let a = replay_to_pixels(&culled, 16, 16, id);
    let b = replay_to_pixels(&all, 16, 16, id);
    pixel_near(&a, RED, 14, 14);
    assert!(a.bytes == b.bytes, "間引きの有無で絵が違う");
}
