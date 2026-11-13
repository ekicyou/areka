//! 不透明度の層の答えの画素（WARP・要件 2.1, 2.2, 2.3, 2.5, 7.2, 7.5）。比べる点は塗りの境界から 2 画素以上離す。
//!
//! 木を組んで（`fill_world_visuals`→`build`）`emit` で再生する本番の道で描く。
//! 期待値は α 掛け済みの SOURCE_OVER（out = src + dst×(1−src.a)）と、層の重ね（層の 1 枚×不透明度）から
//! 手で求めた値（BGRA・255 倍して丸める）。

use super::super::*;
use super::command::DrawCommand;
use super::emit_test_support::*;
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

/// 中身なしの節の印（`WorldVisual` を連れてくる・無いと描画面の持ち主とみなされ訪れられない）。
fn ident() -> VisualTransform {
    VisualTransform(Transform2D::identity())
}

fn faded(opacity: f32) -> Visual {
    let mut v = Visual::default();
    v.set_opacity(opacity);
    v
}

/// 16×16 の根の下に不透明度 `group` の節を置き、その子に赤 (0,0)-(8,8) と後の兄弟の青 (4,4)-(12,12)。
/// 子の不透明度は `child`（1.0 なら層なし）。
fn group_of_red_blue(world: &mut World, group: f32, child: f32) -> Entity {
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id();
    let g = world
        .spawn((
            faded(group),
            at(0.0, 0.0, 16.0, 16.0),
            ident(),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        faded(child),
        at(0.0, 0.0, 8.0, 8.0),
        solid(rect8(), RED),
        ChildOf(g),
    ));
    world.spawn((
        faded(child),
        at(4.0, 4.0, 8.0, 8.0),
        solid(rect8(), BLUE),
        ChildOf(g),
    ));
    root
}

fn render(world: &mut World, root: Entity) -> Pixels {
    render_tree_to_pixels(world, root, 16, 16, Matrix3x2::identity())
}

/// 赤だけの点・重なり・青だけの点・外。
const RED_ONLY: (u32, u32) = (2, 2);
const OVERLAP: (u32, u32) = (6, 6);
const BLUE_ONLY: (u32, u32) = (10, 10);
const OUTSIDE: (u32, u32) = (14, 2);

#[test]
fn opacity_half_fades_the_group_as_one_overlap_does_not_see_through() {
    // 層の中: 重なりは不透明な青が赤を覆い (255,0,0,255)。層×0.5 を透明へ重ねる → 各チャネル
    // 127.5 を丸めて 128。赤だけは (0,0,128,128)。互いに透けるなら重なりは下の値（別のテスト）。
    let mut world = World::new();
    let root = group_of_red_blue(&mut world, 0.5, 1.0);
    let px = render(&mut world, root);
    pixel_near(&px, [0, 0, 128, 128], RED_ONLY.0, RED_ONLY.1);
    pixel_near(&px, [128, 0, 0, 128], OVERLAP.0, OVERLAP.1);
    pixel_near(&px, [128, 0, 0, 128], BLUE_ONLY.0, BLUE_ONLY.1);
    pixel_near(&px, CLEAR, OUTSIDE.0, OUTSIDE.1);
}

#[test]
fn children_faded_one_by_one_see_through_each_other() {
    // 対照: 子をそれぞれ 0.5 にすると重なりは赤 0.5 (0,0,127.5,127.5) の上に青 0.5 (127.5,0,0,127.5) を
    // SOURCE_OVER → B=127.5、R=127.5×0.5=63.75、A=127.5+127.5×0.5=191.25 → (128,0,64,191)。
    let mut world = World::new();
    let root = group_of_red_blue(&mut world, 1.0, 0.5);
    let px = render(&mut world, root);
    pixel_near(&px, [0, 0, 128, 128], RED_ONLY.0, RED_ONLY.1);
    pixel_near(&px, [128, 0, 64, 191], OVERLAP.0, OVERLAP.1);
    pixel_near(&px, [128, 0, 0, 128], BLUE_ONLY.0, BLUE_ONLY.1);
}

#[test]
fn opacity_zero_draws_nothing() {
    let mut world = World::new();
    let root = group_of_red_blue(&mut world, 0.0, 1.0);
    let px = render(&mut world, root);
    for (x, y) in [RED_ONLY, OVERLAP, BLUE_ONLY, OUTSIDE] {
        pixel_near(&px, CLEAR, x, y);
    }
}

#[test]
fn opacity_one_is_the_same_as_no_layer() {
    let mut world = World::new();
    let root = group_of_red_blue(&mut world, 1.0, 1.0);
    let px = render(&mut world, root);
    pixel_near(&px, RED, RED_ONLY.0, RED_ONLY.1);
    pixel_near(&px, BLUE, OVERLAP.0, OVERLAP.1);
    pixel_near(&px, BLUE, BLUE_ONLY.0, BLUE_ONLY.1);
    pixel_near(&px, CLEAR, OUTSIDE.0, OUTSIDE.1);
}

#[test]
fn layer_played_at_opacity_one_matches_the_list_without_layers() {
    // build は 1.0 で層を出さないので、emit の層そのものを 1.0 で再生して層なしの列と全画素で比べる。
    let mut world = World::new();
    let root = group_of_red_blue(&mut world, 0.5, 1.0);
    let commands = tree_commands(&mut world, root, None);
    let with_layer: Vec<DrawCommand> = commands
        .iter()
        .cloned()
        .map(|c| match c {
            DrawCommand::PushOpacity(mut layer) => {
                layer.opacity = 1.0;
                DrawCommand::PushOpacity(layer)
            }
            c => c,
        })
        .collect();
    assert!(
        with_layer
            .iter()
            .any(|c| matches!(c, DrawCommand::PushOpacity(_))),
        "列に層がある"
    );
    let without: Vec<DrawCommand> = commands
        .into_iter()
        .filter(|c| !matches!(c, DrawCommand::PushOpacity(_) | DrawCommand::PopOpacity))
        .collect();
    let id = Matrix3x2::identity();
    let a = replay_to_pixels(&with_layer, 16, 16, id);
    let b = replay_to_pixels(&without, 16, 16, id);
    for y in 0..16 {
        for x in 0..16 {
            pixel_near(&a, b.at(x, y), x, y);
        }
    }
    pixel_near(&a, RED, RED_ONLY.0, RED_ONLY.1);
    pixel_near(&a, BLUE, OVERLAP.0, OVERLAP.1);
}

#[test]
fn nested_half_in_half_is_a_quarter() {
    // 内の層: 赤×0.5 → 外の層の中で (0,0,127.5,127.5)。外×0.5 → 63.75 → 丸めて (0,0,64,64)。
    let mut world = World::new();
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id();
    let outer = world
        .spawn((faded(0.5), at(0.0, 0.0, 16.0, 16.0), ident(), ChildOf(root)))
        .id();
    let inner = world
        .spawn((
            faded(0.5),
            at(0.0, 0.0, 16.0, 16.0),
            ident(),
            ChildOf(outer),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(0.0, 0.0, 8.0, 8.0),
        solid(rect8(), RED),
        ChildOf(inner),
    ));
    let px = render(&mut world, root);
    pixel_near(&px, [0, 0, 64, 64], 4, 4);
    pixel_near(&px, CLEAR, 12, 12);
}
