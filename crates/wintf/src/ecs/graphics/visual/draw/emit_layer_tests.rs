//! 不透明度の層の答えの画素（WARP・要件 2.1, 2.2, 2.3, 2.5, 7.2, 7.5）と、切り抜きの形ごとの効き先
//! （要件 3.1〜3.4, 7.6・後半）。比べる点は塗りの境界から 2 画素以上離す。
//!
//! 木を組んで（`fill_world_visuals`→`build`）`emit` で再生する本番の道で描く。
//! 期待値は α 掛け済みの SOURCE_OVER（out = src + dst×(1−src.a)）と、層の重ね（層の 1 枚×不透明度）から
//! 手で求めた値（BGRA・255 倍して丸める）。

use super::super::*;
use super::clip_geometry::rounded_rect;
use super::command::*;
use super::emit::emit;
use super::emit_test_support::*;
use crate::ecs::layout::Arrangement;
use crate::numerics::{Aabb, Transform2D};
use bevy_ecs::hierarchy::ChildOf;
use std::mem::ManuallyDrop;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::core::Interface;
use windows_numerics::{Matrix3x2, Vector2};

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

// ---- 切り抜きの形ごとの効き先（要件 3.1〜3.4, 7.6・research.md §10.2 の形） -------------------
//
// 的 40×40。下の画素は左半分 (0,0)-(20,40) が不透明の B1、右半分 (20,0)-(40,40) が α 128 の B2
// （色のある下の画素で、図形の層が背景から始まるか透明から始まるかを見分ける）。絵は (6,6)-(34,34)。
// 切り抜きはどれも中心 (20,20): 矩形 (13,13)-(27,27)・角丸（同じ矩形・半径 4）・45 度回した
// (-5,-5)-(5,5)・半径 7 の楕円。見る点はどの形の縁からも 2 画素以上離す。

/// 絵の色（BGRA・straight）＝ RGB (51,102,204)。不透明と α 128。
const F: [u8; 4] = [204, 102, 51, 255];
const F_HALF: [u8; 4] = [204, 102, 51, 128];
/// 不透明な下の画素 B1 ＝ RGB (153,77,179)。
const B1: [u8; 4] = [179, 77, 153, 255];
/// 半透明な下の画素 B2（straight）＝ RGB (77,230,102)・α 128。的に置くと α 掛け済みの B2_PREMUL。
const B2: [u8; 4] = [102, 230, 77, 128];
const B2_PREMUL: [u8; 4] = [51, 115, 39, 128];

/// どの形でも内側の点（B1 の上・B2 の上）と、絵の中で全部の形の外の点。
const INSIDE_B1: [(u32, u32); 2] = [(17, 18), (17, 21)];
const INSIDE_B2: [(u32, u32); 2] = [(22, 18), (22, 21)];
const OUTSIDE_B1: [(u32, u32); 2] = [(9, 9), (9, 30)];
const OUTSIDE_B2: [(u32, u32); 2] = [(30, 9), (30, 30)];

/// `(名前, 混ぜ方, 絵の色, B1 の上の答え, B2 の上の答え)`（BGRA・α 掛け済み）。
type ModeCase = (&'static str, MixMode, [u8; 4], [u8; 4], [u8; 4]);

/// 混ぜ方 4 つの見本。
///
/// 重ね方は α 128 の絵（Sa = 128/255）で、α 掛け済みの Porter-Duff から手で求めた値:
/// DESTINATION_IN = D×Sa、DESTINATION_OUT = D×(1−Sa)、DESTINATION_OVER = D + S×(1−Da)
/// （D は的に置いた値・S は絵の α 掛け済みの値 (102.4,51.2,25.6,128)）。不透明の下の
/// DESTINATION_OVER は下のまま。MULTIPLY は不透明の絵で `emit_blend_tests.rs` の見本と同じ式の値。
#[rustfmt::skip] // 1 種 1 行の表のまま読む
const CLIP_MODES: [ModeCase; 4] = [
    ("DESTINATION_IN", MixMode::Composite(D2D1_COMPOSITE_MODE_DESTINATION_IN), F_HALF, [90, 39, 77, 128], [26, 58, 20, 64]),
    ("DESTINATION_OUT", MixMode::Composite(D2D1_COMPOSITE_MODE_DESTINATION_OUT), F_HALF, [89, 38, 76, 127], [25, 57, 19, 64]),
    ("DESTINATION_OVER", MixMode::Composite(D2D1_COMPOSITE_MODE_DESTINATION_OVER), F_HALF, B1, [102, 140, 52, 192]),
    ("MULTIPLY", MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY), F, [143, 31, 31, 255], [142, 97, 33, 255]),
];

fn r(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

/// 描く中身を恒等の世界の行列と混ぜ方 `mix` で置いた 1 件。emit は `world_aabb` を読まない。
fn draw_item(content: &VisualDrawContent, mix: MixMode) -> DrawCommand {
    DrawCommand::Draw(DrawItem {
        hash: 0,
        entity: Entity::PLACEHOLDER,
        version: 0,
        world_mat: Matrix3x2::identity(),
        world_aabb: content.local_aabb,
        command_list: content.command_list.clone(),
        interpolation_mode: content.interpolation_mode,
        mix_mode: mix,
    })
}

fn colored_under() -> Vec<DrawCommand> {
    let normal = MixMode::default();
    vec![
        draw_item(&solid(r(0.0, 0.0, 20.0, 40.0), B1), normal),
        draw_item(&solid(r(20.0, 0.0, 40.0, 40.0), B2), normal),
    ]
}

fn picture(bgra: [u8; 4], mix: MixMode) -> DrawCommand {
    draw_item(&solid(r(6.0, 6.0, 34.0, 34.0), bgra), mix)
}

/// 局所の図形 `geometry`（局所の外接矩形 `local`）を `world_mat` で置いた図形の切り抜き。
fn geometry_clip(geometry: ID2D1Geometry, world_mat: Matrix3x2, local: Aabb) -> ClipGeometry {
    ClipGeometry {
        entity: Entity::PLACEHOLDER,
        version: 0,
        world_mat,
        geometry,
        world_aabb: local.transform(world_mat),
    }
}

/// 図形の切り抜き 3 つ: 角丸・回転した矩形・楕円。
fn geometry_clips() -> Vec<(&'static str, ClipGeometry)> {
    let factory = warp_factory();
    let rounded = rounded_rect(&factory, 14.0, 14.0, 4.0, 4.0, 4.0, 4.0).unwrap();
    let square: ID2D1Geometry =
        unsafe { factory.CreateRectangleGeometry(&r(-5.0, -5.0, 5.0, 5.0)) }
            .unwrap()
            .cast()
            .unwrap();
    let ellipse: ID2D1Geometry = unsafe {
        factory.CreateEllipseGeometry(&D2D1_ELLIPSE {
            point: Vector2 { X: 20.0, Y: 20.0 },
            radiusX: 7.0,
            radiusY: 7.0,
        })
    }
    .unwrap()
    .cast()
    .unwrap();
    vec![
        (
            "rounded",
            geometry_clip(
                rounded,
                Matrix3x2::translation(13.0, 13.0),
                Aabb::new(0.0, 0.0, 14.0, 14.0),
            ),
        ),
        (
            "rotated",
            geometry_clip(
                square,
                Matrix3x2::rotation(45.0) * Matrix3x2::translation(20.0, 20.0),
                Aabb::new(-5.0, -5.0, 5.0, 5.0),
            ),
        ),
        (
            "ellipse",
            geometry_clip(
                ellipse,
                Matrix3x2::identity(),
                Aabb::new(13.0, 13.0, 27.0, 27.0),
            ),
        ),
    ]
}

/// 4 つの切り抜きの push と pop（矩形は軸に沿う切り抜き・ほかは図形の層）。
fn all_clips() -> Vec<(&'static str, DrawCommand, DrawCommand)> {
    let mut clips = vec![(
        "rect",
        DrawCommand::PushClipRect(ClipRect {
            entity: Entity::PLACEHOLDER,
            version: 0,
            world_aabb: Aabb::new(13.0, 13.0, 27.0, 27.0),
        }),
        DrawCommand::PopClipRect,
    )];
    for (name, clip) in geometry_clips() {
        clips.push((
            name,
            DrawCommand::PushClipGeometry(clip),
            DrawCommand::PopClipGeometry,
        ));
    }
    clips
}

fn near(px: &Pixels, what: &str, expected: [u8; 4], x: u32, y: u32) {
    let got = px.at(x, y);
    let ok = got
        .iter()
        .zip(expected)
        .all(|(&a, e)| (a as i16 - e as i16).abs() <= 1);
    assert!(ok, "{what} ({x}, {y}): got {got:?}, expected {expected:?}");
}

fn render40(commands: &[DrawCommand]) -> Pixels {
    replay_to_pixels(commands, 40, 40, Matrix3x2::identity())
}

#[test]
fn modes_without_clip_match_the_formulas() {
    // 較正: 切り抜き無しの答えが表の値。
    for (name, mix, color, on_b1, on_b2) in CLIP_MODES {
        let mut commands = colored_under();
        commands.push(picture(color, mix));
        let px = render40(&commands);
        for (x, y) in INSIDE_B1 {
            near(&px, name, on_b1, x, y);
        }
        for (x, y) in INSIDE_B2 {
            near(&px, name, on_b2, x, y);
        }
    }
}

#[test]
fn every_clip_shape_applies_modes_to_the_under_pixels_and_keeps_outside() {
    // 3.1〜3.3: 内側は切り抜き無しの答え・外側は描く前の値。形どうしで同じ表の値。
    for (name, mix, color, on_b1, on_b2) in CLIP_MODES {
        for (shape, push, pop) in all_clips() {
            let mut commands = colored_under();
            commands.extend([push, picture(color, mix), pop]);
            let px = render40(&commands);
            let what = format!("{name} in {shape}");
            for (x, y) in INSIDE_B1 {
                near(&px, &what, on_b1, x, y);
            }
            for (x, y) in INSIDE_B2 {
                near(&px, &what, on_b2, x, y);
            }
            for (x, y) in OUTSIDE_B1 {
                near(&px, &what, B1, x, y);
            }
            for (x, y) in OUTSIDE_B2 {
                near(&px, &what, B2_PREMUL, x, y);
            }
        }
    }
}

/// 下の画素を描き、`clip` の図形の層を範囲を絞らず（±f32::MAX）に emit と同じ旗で積んで `item` を描く。
fn render_with_unbounded_clip(
    clip: &ClipGeometry,
    item: DrawCommand,
    size: u32,
    root: Matrix3x2,
) -> Pixels {
    draw_to_pixels(size, size, root, |dc| {
        let dc6: ID2D1DeviceContext6 = dc.cast().unwrap();
        emit(dc, Some(&dc6), root, &colored_under());
        // SAFETY: Windows API 境界。足場の `BeginDraw` の中。
        unsafe {
            dc.SetTransform(&Matrix3x2::identity());
            dc.SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_COPY);
            let params = D2D1_LAYER_PARAMETERS1 {
                contentBounds: r(-f32::MAX, -f32::MAX, f32::MAX, f32::MAX),
                geometricMask: ManuallyDrop::new(Some(clip.geometry.clone())),
                maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform: clip.world_mat * root,
                opacity: 1.0,
                opacityBrush: ManuallyDrop::new(None),
                layerOptions: D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND,
            };
            dc.PushLayer(&params, None::<&ID2D1Layer>);
            dc.SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
            drop(ManuallyDrop::into_inner(params.geometricMask));
        }
        emit(dc, Some(&dc6), root, &[item]);
        // SAFETY: 同上。上で積んだ層を降ろす。
        unsafe { dc.PopLayer() };
    })
}

#[test]
fn narrowing_the_layer_bounds_does_not_change_the_edge_pixels() {
    // 縁の半端な覆いの画素を含む全画素が、範囲を絞らない層の答えと同じ（拡大 1 と 1.25）。
    for (scale, size) in [(1.0, 40), (1.25, 50)] {
        let root = Matrix3x2::scale(scale, scale);
        let before = replay_to_pixels(&colored_under(), size, size, root);
        for (name, mix, color, _, _) in CLIP_MODES {
            for (shape, clip) in geometry_clips() {
                let mut commands = colored_under();
                commands.extend([
                    DrawCommand::PushClipGeometry(clip.clone()),
                    picture(color, mix),
                    DrawCommand::PopClipGeometry,
                ]);
                let bounded = replay_to_pixels(&commands, size, size, root);
                let unbounded = render_with_unbounded_clip(&clip, picture(color, mix), size, root);
                let what = format!("{name} in {shape} x{scale}");
                let mut changed = 0;
                for y in 0..size {
                    for x in 0..size {
                        near(&bounded, &what, unbounded.at(x, y), x, y);
                        changed += (unbounded.at(x, y) != before.at(x, y)) as u32;
                    }
                }
                // 較正: 比べた絵に切り抜きの中の答えが描かれている（不透明の下の DESTINATION_OVER は
                // 左半分を変えないので右半分だけで数える下限）。
                assert!(changed > 30, "{what}: 描かれていない（{changed}）");
            }
        }
    }
}

/// 不透明度 0.5 の層の中に青 (8,8)-(32,32) を描き、`clip`（push と pop）の中で絵 F を `mix` で描く。
fn faded_blue_then(mix: MixMode, clip: Option<(DrawCommand, DrawCommand)>) -> Vec<DrawCommand> {
    let mut commands = colored_under();
    commands.push(DrawCommand::PushOpacity(OpacityLayer {
        entity: Entity::PLACEHOLDER,
        version: 0,
        opacity: 0.5,
        world_aabb: Aabb::new(0.0, 0.0, 40.0, 40.0),
    }));
    commands.push(draw_item(
        &solid(r(8.0, 8.0, 32.0, 32.0), BLUE),
        MixMode::default(),
    ));
    match clip {
        Some((push, pop)) => commands.extend([push, picture(F, mix), pop]),
        None => commands.push(picture(F, mix)),
    }
    commands.push(DrawCommand::PopOpacity);
    commands
}

#[test]
fn modes_in_an_opacity_layer_apply_to_what_the_layer_drew_first() {
    // 3.4: 層の中の DESTINATION_OUT は先に描いた青を消し（下の画素は層の不透明度でだけ変わる＝そのまま）、
    // MULTIPLY は青に掛かる（F×青 = BGRA (204,0,0) の層×0.5 を下へ重ねる）。切り抜きの外の青は青×0.5 を
    // 重ねた値: over B1 = (127.5,0,0,127.5)+B1×0.5、over B2 = (127.5,0,0,127.5)+B2_PREMUL×0.5。
    let blue_over_b1 = [217, 39, 77, 255];
    let blue_over_b2 = [153, 58, 20, 192];
    let cases = [
        (
            "DESTINATION_OUT",
            MixMode::Composite(D2D1_COMPOSITE_MODE_DESTINATION_OUT),
            B1,
            B2_PREMUL,
        ),
        (
            "MULTIPLY",
            MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY),
            [192, 39, 77, 255],
            [128, 58, 20, 192],
        ),
    ];
    for (name, mix, on_b1, on_b2) in cases {
        let mut clips: Vec<(&str, Option<(DrawCommand, DrawCommand)>)> = vec![("none", None)];
        for (shape, push, pop) in all_clips() {
            clips.push((shape, Some((push, pop))));
        }
        for (shape, clip) in clips {
            let has_clip = clip.is_some();
            let px = render40(&faded_blue_then(mix, clip));
            let what = format!("{name} in {shape}");
            for (x, y) in INSIDE_B1 {
                near(&px, &what, on_b1, x, y);
            }
            for (x, y) in INSIDE_B2 {
                near(&px, &what, on_b2, x, y);
            }
            if has_clip {
                // 青の中・どの切り抜きの外（青の縁と切り抜きの縁から 2.5 画素）。
                near(&px, &what, blue_over_b1, 10, 10);
                near(&px, &what, blue_over_b2, 29, 29);
            }
            near(&px, &what, B1, 3, 3); // 青の外・絵の外
        }
    }
}
