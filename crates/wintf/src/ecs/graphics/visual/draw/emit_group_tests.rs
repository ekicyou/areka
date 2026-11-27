//! まとめの節（`VisualGroup`）の答えの画素（WARP）。比べる点は塗りの境界から 2 画素以上離す。
//!
//! 木を組んで画面の外の口（`render_tree_offscreen`）で描く本番の道で確かめる。期待値は α 掛け済みの
//! 重ね方の式（S＝まとめの 1 枚・D＝親の画素）から手で求めた値（BGRA）。

use super::super::*;
use super::emit_test_support::*;
use crate::ecs::graphics::clip::ClipShape;
use crate::numerics::Transform2D;
use bevy_ecs::hierarchy::ChildOf;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows_numerics::{Matrix3x2, Vector2};

fn group(mix: D2D1_COMPOSITE_MODE) -> VisualGroup {
    VisualGroup {
        mix: MixMode::Composite(mix),
    }
}

/// 16×16 の根。
fn root16(world: &mut World) -> Entity {
    world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id()
}

/// 親 `parent` の下に、`(l, t)-(r, b)` を色 `bgra` で塗った節を置く。
fn paint(world: &mut World, parent: Entity, (l, t, r, b): (f32, f32, f32, f32), bgra: [u8; 4]) {
    world.spawn((
        Visual::default(),
        at(l, t, r - l, b - t),
        solid(rect(0.0, 0.0, r - l, b - t), bgra),
        ChildOf(parent),
    ));
}

fn render16(world: &mut World, root: Entity) -> Pixels {
    render_tree_to_pixels(world, root, 16, 16, Matrix3x2::identity())
}

#[test]
fn destination_in_inside_a_group_does_not_reach_the_parent_pixels() {
    let mut world = World::new();
    let root = root16(&mut world);
    paint(&mut world, root, (0.0, 0.0, 16.0, 16.0), RED);
    let g = world
        .spawn((
            Visual::default(),
            at(0.0, 0.0, 16.0, 16.0),
            group(D2D1_COMPOSITE_MODE_SOURCE_OVER),
            ChildOf(root),
        ))
        .id();
    paint(&mut world, g, (0.0, 0.0, 8.0, 8.0), BLUE);
    world.spawn((
        Visual::default(),
        at(4.0, 4.0, 8.0, 8.0),
        VisualDrawContent {
            mix_mode: MixMode::Composite(D2D1_COMPOSITE_MODE_DESTINATION_IN),
            ..solid(rect(0.0, 0.0, 8.0, 8.0), RED)
        },
        ChildOf(g),
    ));

    let p = render16(&mut world, root);

    pixel_near(&p, BLUE, 6, 6);
    // まとめの中では青の (0,0)-(4,4) が削られ、親の赤が見える。
    pixel_near(&p, RED, 2, 2);
    pixel_near(&p, RED, 10, 10);
    pixel_near(&p, RED, 14, 14);
}

#[test]
fn closing_mix_composes_the_group_as_one_image_onto_the_parent() {
    // 親: 左半分 (0,0)-(8,16) が赤。まとめ: 青の (4,4)-(12,8) と (4,10)-(12,12)。
    // 点: 親だけ (2,2)・両方 (6,6)・まとめだけ (10,6)・まとめの外接矩形の中の透明と親 (6,9)・どちらも無い (14,14)。
    let points = [(2, 2), (6, 6), (10, 6), (6, 9), (14, 14)];
    let cases = [
        (
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
            [RED, BLUE, BLUE, RED, CLEAR],
        ),
        (
            D2D1_COMPOSITE_MODE_SOURCE_ATOP,
            [RED, BLUE, CLEAR, RED, CLEAR],
        ),
        (
            D2D1_COMPOSITE_MODE_DESTINATION_OVER,
            [RED, RED, BLUE, RED, CLEAR],
        ),
        (
            D2D1_COMPOSITE_MODE_DESTINATION_IN,
            [CLEAR, RED, CLEAR, CLEAR, CLEAR],
        ),
        (
            D2D1_COMPOSITE_MODE_SOURCE_COPY,
            [CLEAR, BLUE, BLUE, CLEAR, CLEAR],
        ),
    ];
    for (mix, expected) in cases {
        let mut world = World::new();
        let root = root16(&mut world);
        paint(&mut world, root, (0.0, 0.0, 8.0, 16.0), RED);
        let g = world
            .spawn((
                Visual::default(),
                at(0.0, 0.0, 16.0, 16.0),
                group(mix),
                ChildOf(root),
            ))
            .id();
        paint(&mut world, g, (4.0, 4.0, 12.0, 8.0), BLUE);
        paint(&mut world, g, (4.0, 10.0, 12.0, 12.0), BLUE);

        let p = render16(&mut world, root);

        for ((x, y), e) in points.into_iter().zip(expected) {
            let got = p.at(x, y);
            let near = got
                .iter()
                .zip(e)
                .all(|(&a, e)| (a as i16 - e as i16).abs() <= 1);
            assert!(near, "{mix:?} ({x}, {y}): got {got:?}, expected {e:?}");
        }
    }
}

#[test]
fn opacity_layer_wraps_the_group_so_the_closing_mix_sees_only_the_layer() {
    let mut world = World::new();
    let root = root16(&mut world);
    paint(&mut world, root, (0.0, 0.0, 16.0, 16.0), RED);
    let mut faded = Visual::default();
    faded.set_opacity(0.5);
    let g = world
        .spawn((
            faded,
            at(0.0, 0.0, 16.0, 16.0),
            group(D2D1_COMPOSITE_MODE_DESTINATION_IN),
            ChildOf(root),
        ))
        .id();
    paint(&mut world, g, (4.0, 4.0, 12.0, 12.0), BLUE);

    let p = render16(&mut world, root);

    // まとめは透明から始まる不透明度の層へ `DESTINATION_IN` で重なる（層は透明のまま）ので、親は変わらない。
    // 順が逆（まとめ → 不透明度）なら (6,6) は半分の赤、(2,2) は透明になる。
    pixel_near(&p, RED, 6, 6);
    pixel_near(&p, RED, 2, 2);
}

#[test]
fn clip_of_the_group_node_cuts_the_inside_not_the_closing_mix() {
    let mut world = World::new();
    let root = root16(&mut world);
    paint(&mut world, root, (0.0, 0.0, 16.0, 16.0), RED);
    let g = world
        .spawn((
            Visual {
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            at(4.0, 4.0, 8.0, 8.0),
            group(D2D1_COMPOSITE_MODE_DESTINATION_IN),
            ChildOf(root),
        ))
        .id();
    paint(&mut world, g, (-4.0, -4.0, 12.0, 12.0), BLUE);

    let p = render16(&mut world, root);

    pixel_near(&p, RED, 6, 6);
    // 切り抜きはまとめの内側。1 枚は (4,4)-(12,12) だけが不透明で、その外の親は削られる。
    pixel_near(&p, CLEAR, 2, 2);
    pixel_near(&p, CLEAR, 14, 14);
}

#[test]
fn inside_is_drawn_at_local_resolution_and_then_scaled() {
    let mut world = World::new();
    let root = root16(&mut world);
    let g = world
        .spawn((
            Visual::default(),
            at(0.0, 0.0, 4.0, 4.0),
            VisualTransform(Transform2D {
                scale: Vector2 { X: 4.0, Y: 4.0 },
                ..Transform2D::identity()
            }),
            group(D2D1_COMPOSITE_MODE_SOURCE_OVER),
            ChildOf(root),
        ))
        .id();
    // 局所の x＝1.5 の縁: 原寸では列 0 が赤・列 1 が被覆 0.5。
    paint(&mut world, g, (0.0, 0.0, 1.5, 4.0), RED);

    let p = render16(&mut world, root);

    // 4 倍の `LINEAR`: 物理 x の中心 (x+0.5)/4 を列 0（中心 0.5・α 1）と列 1（中心 1.5・α 0.5）で補間する。
    // x＝2 → 0.625: 0.875×1＋0.125×0.5＝0.9375。x＝5 → 1.375: 0.125×1＋0.875×0.5＝0.5625。
    // 拡大してから描けば縁は物理 x＝6 で、どちらも α 255 になる。
    pixel_near(&p, [0, 0, 239, 239], 2, 8);
    pixel_near(&p, [0, 0, 143, 143], 5, 8);
}
