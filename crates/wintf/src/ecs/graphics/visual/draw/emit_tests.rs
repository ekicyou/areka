//! 画素の答え（WARP・要件 1.1, 1.7, 1.10, 2.1, 2.3〜2.8, 3.1, 6.1〜6.4）。比べる点は塗りの境界から 2 画素以上離す。

use super::super::*;
use super::emit_test_support::*;
use crate::ecs::graphics::ClipShape;
use crate::ecs::layout::Arrangement;
use crate::numerics::{Aabb, Transform2D};
use bevy_ecs::hierarchy::ChildOf;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::{D2D1_ELLIPSE, ID2D1Geometry};
use windows::core::Interface;
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

/// 切り抜き `clip` の節（`at(x, y, w, h)`）に自分の中身として赤 `fill` を置く。
fn clipped(
    world: &mut World,
    parent: Entity,
    clip: ClipShape,
    a: Arrangement,
    fill: D2D_RECT_F,
) -> Entity {
    world
        .spawn((
            Visual {
                clip: Some(clip),
                ..Default::default()
            },
            a,
            solid(fill, RED),
            ChildOf(parent),
        ))
        .id()
}

#[test]
fn rectangle_clip_clears_outside() {
    let mut world = World::new();
    let root = root16(&mut world);
    // 局所 (-4,-4)-(12,12)＝世界 (0,0)-(16,16) の赤を (4,4)-(12,12) で切る。
    clipped(
        &mut world,
        root,
        ClipShape::Rectangle,
        at(4.0, 4.0, 8.0, 8.0),
        rect(-4.0, -4.0, 12.0, 12.0),
    );
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 8, 8);
    pixel_near(&px, CLEAR, 1, 1);
    pixel_near(&px, CLEAR, 14, 8);
    pixel_near(&px, CLEAR, 8, 14);
}

#[test]
fn rounded_clip_clears_corner_outside_and_fills_corner_inside() {
    // 均一 6: 角の弧の中心は各角から (6,6)。(0,0) の画素の中心は弧の外 1.8・(3,3) は内 2.5。
    let mut world = World::new();
    let root = root16(&mut world);
    clipped(
        &mut world,
        root,
        ClipShape::RoundedRectangle { radius: 6.0 },
        at(0.0, 0.0, 16.0, 16.0),
        rect(0.0, 0.0, 16.0, 16.0),
    );
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    for (x, y) in [(0, 0), (15, 0), (0, 15), (15, 15)] {
        pixel_near(&px, CLEAR, x, y);
    }
    for (x, y) in [(3, 3), (12, 3), (3, 12), (12, 12), (8, 2), (2, 8)] {
        pixel_near(&px, RED, x, y);
    }
    // 角ごと（左上 12 だけ）: 24×24 で (2,2) の中心は弧の外 1.4・角張った 3 つの角は塗られる。
    let mut world = World::new();
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 24.0, 24.0)))
        .id();
    let clip = ClipShape::RoundedRectangleIndividual {
        top_left: 12.0,
        top_right: 0.0,
        bottom_left: 0.0,
        bottom_right: 0.0,
    };
    clipped(
        &mut world,
        root,
        clip,
        at(0.0, 0.0, 24.0, 24.0),
        rect(0.0, 0.0, 24.0, 24.0),
    );
    let px = render_tree_to_pixels(&mut world, root, 24, 24, Matrix3x2::identity());
    pixel_near(&px, CLEAR, 2, 2);
    pixel_near(&px, RED, 21, 2);
    pixel_near(&px, RED, 2, 21);
    pixel_near(&px, RED, 21, 21);
    pixel_near(&px, RED, 6, 6);
}

#[test]
fn rotated_rectangle_clip_does_not_grow_to_its_bounds() {
    let mut world = World::new();
    let root = root16(&mut world);
    // 8×8 を (8,2) で 45 度＝ひし形 |x-8|+|y-7.66|≤5.66。外接矩形は (2.3,2)-(13.7,13.3)。
    let n = clipped(
        &mut world,
        root,
        ClipShape::Rectangle,
        at(8.0, 2.0, 8.0, 8.0),
        rect(-10.0, -10.0, 20.0, 20.0),
    );
    world.entity_mut(n).insert(VisualTransform(Transform2D {
        rotation: FRAC_PI_4,
        ..Transform2D::identity()
    }));
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 8, 7);
    pixel_near(&px, RED, 8, 4);
    pixel_near(&px, CLEAR, 3, 3); // 外接矩形の中・ひし形の外 2.1
    pixel_near(&px, CLEAR, 12, 3);
    pixel_near(&px, CLEAR, 12, 12);
}

#[test]
fn ellipse_geometry_clip() {
    let mut world = World::new();
    let root = root16(&mut world);
    // 中心 (8,8)・半径 8×4 の楕円。(8,2) は矩形の中・楕円の外。
    let ellipse: ID2D1Geometry = unsafe {
        warp_factory().CreateEllipseGeometry(&D2D1_ELLIPSE {
            point: windows_numerics::Vector2 { X: 8.0, Y: 8.0 },
            radiusX: 8.0,
            radiusY: 4.0,
        })
    }
    .unwrap()
    .cast()
    .unwrap();
    clipped(
        &mut world,
        root,
        ClipShape::Geometry(ellipse),
        at(0.0, 0.0, 16.0, 16.0),
        rect(0.0, 0.0, 16.0, 16.0),
    );
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 8, 8);
    pixel_near(&px, RED, 2, 8);
    pixel_near(&px, CLEAR, 8, 2);
    pixel_near(&px, CLEAR, 8, 13);
    pixel_near(&px, CLEAR, 1, 1);
}

#[test]
fn nested_clips_intersect() {
    let mut world = World::new();
    let root = root16(&mut world);
    // 外 (0,0)-(12,12)・内 (4,4)-(16,16)＝交わり (4,4)-(12,12)。赤は内の局所で世界の全面。
    let outer = world
        .spawn((
            Visual {
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            at(0.0, 0.0, 12.0, 12.0),
            VisualTransform(Transform2D::identity()), // 中身の無い節の目印（WorldVisual）
            ChildOf(root),
        ))
        .id();
    clipped(
        &mut world,
        outer,
        ClipShape::Rectangle,
        at(4.0, 4.0, 12.0, 12.0),
        rect(-4.0, -4.0, 12.0, 12.0),
    );
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 8, 8);
    pixel_near(&px, CLEAR, 2, 2); // 外だけ
    pixel_near(&px, CLEAR, 14, 14); // 内だけ
    pixel_near(&px, CLEAR, 2, 14);
    pixel_near(&px, CLEAR, 14, 2);
}

#[test]
fn invisible_node_is_skipped() {
    let mut world = World::new();
    let root = red_then_blue(&mut world);
    let blue = *world.get::<Children>(root).unwrap().last().unwrap();
    world.get_mut::<Visual>(blue).unwrap().is_visible = false;
    // 見えない親の下の見える子も塗られない。
    let g = world
        .spawn((
            Visual {
                is_visible: false,
                ..Default::default()
            },
            at(8.0, 8.0, 8.0, 8.0),
            VisualTransform(Transform2D::identity()), // 中身の無い節の目印（WorldVisual）
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 8.0, 8.0),
        solid(rect8(), RED),
        ChildOf(g),
    ));
    let px = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    pixel_near(&px, RED, 2, 2);
    pixel_near(&px, RED, 6, 6); // 青が描かれていない
    pixel_near(&px, CLEAR, 10, 10);
    pixel_near(&px, CLEAR, 13, 13);
}

#[test]
fn promoted_node_draws_same_picture_as_node() {
    // 節 n（赤）とその子（青）を根の下で描いた絵と、n を根にして（昇格）描画面の位置へ置いた絵。
    let mut world = World::new();
    let root = root16(&mut world);
    let n = world
        .spawn((
            Visual::default(),
            at(4.0, 4.0, 8.0, 8.0),
            solid(rect8(), RED),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(2.0, 2.0, 4.0, 4.0),
        solid(rect(0.0, 0.0, 4.0, 4.0), BLUE),
        ChildOf(n),
    ));
    let as_node = render_tree_to_pixels(&mut world, root, 16, 16, Matrix3x2::identity());
    // 持ち主の位置は WUC の見た目のオフセット＝根の変換の平行移動。
    let place = Matrix3x2::translation(4.0, 4.0);
    let as_root = render_tree_to_pixels(&mut world, n, 16, 16, place);
    pixel_near(&as_root, RED, 5, 5); // 根の自分の中身
    pixel_near(&as_root, BLUE, 7, 7); // 子が上
    pixel_near(&as_root, CLEAR, 14, 14);
    assert!(
        as_node.bytes == as_root.bytes,
        "昇格した節の絵が節のときと違う"
    );
}
