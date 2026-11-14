//! ブレンドの描き方の判定と 1 度だけの記録（装置なしの純粋なテスト・要件 1.1, 1.2, 1.5, 1.7, 7.9）と、
//! ブレンドの答えの画素（WARP・要件 1.1〜1.4, 1.6, 1.7, 7.2〜7.4）。比べる点は境界から 2 画素以上離す。

use super::super::{MixMode, VisualDrawContent};
use super::command::*;
use super::emit::{DrawMethod, draw_method, report_once};
use super::emit_test_support::*;
use crate::numerics::Aabb;
use bevy_ecs::entity::Entity;
use std::sync::atomic::AtomicBool;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::{D2D1_ELLIPSE, ID2D1Geometry};
use windows::core::Interface;
use windows_numerics::{Matrix3x2, Vector2};

#[test]
fn composite_in_range_draws_image_regardless_of_blend() {
    for c in [0, 12] {
        let mode = D2D1_COMPOSITE_MODE(c);
        for available in [true, false] {
            assert_eq!(
                draw_method(MixMode::Composite(mode), available),
                DrawMethod::Image(mode)
            );
        }
    }
}

#[test]
fn composite_out_of_range() {
    for c in [-1, 13] {
        for available in [true, false] {
            assert_eq!(
                draw_method(MixMode::Composite(D2D1_COMPOSITE_MODE(c)), available),
                DrawMethod::OutOfRange
            );
        }
    }
}

#[test]
fn blend_in_range_draws_blend_when_available() {
    for b in [0, 25] {
        let mode = D2D1_BLEND_MODE(b);
        assert_eq!(
            draw_method(MixMode::Blend(mode), true),
            DrawMethod::Blend(mode)
        );
        assert_eq!(
            draw_method(MixMode::Blend(mode), false),
            DrawMethod::BlendUnavailable
        );
    }
}

#[test]
fn blend_out_of_range_is_checked_before_availability() {
    for b in [-1, 26] {
        for available in [true, false] {
            assert_eq!(
                draw_method(MixMode::Blend(D2D1_BLEND_MODE(b)), available),
                DrawMethod::OutOfRange
            );
        }
    }
}

#[test]
fn report_once_is_true_only_the_first_time() {
    let flag = AtomicBool::new(false);
    let other = AtomicBool::new(false);
    assert!(report_once(&flag));
    assert!(!report_once(&flag));
    assert!(!report_once(&flag));
    // 旗が違えば別の種類の誤りとして初めての 1 度になる。
    assert!(report_once(&other));
}

// ---- WARP の画素 ----------------------------------------------------------------
//
// 的 40×20。下の画素は左半分 (0,0)-(20,20) が不透明の B1、右半分 (20,0)-(40,20) が α 128 の B2。
// 絵は (4,4)-(36,16) の不透明の F。見る点は B1 の上 (10,10)・B2 の上 (30,10)。

/// 絵の色 F（BGRA・straight）＝ RGB (51,102,204)。
const F: [u8; 4] = [204, 102, 51, 255];
/// 不透明な下の画素 B1 ＝ RGB (153,77,179)。
const B1: [u8; 4] = [179, 77, 153, 255];
/// 半透明な下の画素 B2（straight）＝ RGB (77,230,102)・α 128。
const B2: [u8; 4] = [102, 230, 77, 128];
/// B2 を的に置いた値（α 掛け済み・各チャネル round(c×128/255)）。
const B2_PREMUL: [u8; 4] = [51, 115, 39, 128];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn r(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

/// 描く中身を世界の行列 `world_mat` と混ぜ方 `mix` で置いた 1 件。emit は `world_aabb` を読まない。
fn draw(content: &VisualDrawContent, world_mat: Matrix3x2, mix: MixMode) -> DrawCommand {
    DrawCommand::Draw(DrawItem {
        hash: 0,
        entity: Entity::PLACEHOLDER,
        version: 0,
        world_mat,
        world_aabb: content.local_aabb,
        command_list: content.command_list.clone(),
        interpolation_mode: content.interpolation_mode,
        mix_mode: mix,
    })
}

fn under() -> Vec<DrawCommand> {
    let id = Matrix3x2::identity();
    vec![
        draw(&solid(r(0.0, 0.0, 20.0, 20.0), B1), id, MixMode::default()),
        draw(&solid(r(20.0, 0.0, 40.0, 20.0), B2), id, MixMode::default()),
    ]
}

/// 絵 F を `mix` で描く 1 件。
fn source(mix: MixMode) -> DrawCommand {
    draw(
        &solid(r(4.0, 4.0, 36.0, 16.0), F),
        Matrix3x2::identity(),
        mix,
    )
}

/// 下の画素の上に F を `mix` で描いた列。
fn over_under(mix: MixMode) -> Vec<DrawCommand> {
    let mut commands = under();
    commands.push(source(mix));
    commands
}

/// 絵の外形の外（どれも絵の縁から 2.5 画素以上）。
const OUTSIDE: [(u32, u32); 4] = [(1, 1), (38, 18), (10, 18), (30, 1)];

/// 26 種の見本（BGRA・α 掛け済み）: `(種類, B1 の上 (10,10), B2 の上 (30,10))`。
///
/// D2D の Blend effect の文書（learn.microsoft.com/windows/win32/direct2d/blend）の式で手計算した値。
/// 出力 = f(F, B)·FA·BA + F·FA·(1−BA) + B·BA·(1−FA)（F・B は straight・FA=1）を 255 倍して丸める。
/// B2 の straight は的に置いた値を α で割り戻した値（例: 赤 39/128）。f は W3C Compositing と同じ形:
/// 色の比べ（DARKER/LIGHTER_COLOR）と HUE〜LUMINOSITY の Lum は 0.30R+0.59G+0.11B、SetLum／SetSat／
/// ClipColor は W3C の定義、SOFT_LIGHT は W3C の D(B)、HARD_MIX は F+B≥1 で 1、SUBTRACT は B−F、
/// DIVISION は B/F（どれも [0,1] へ切る）。DISSOLVE は絵の α が 1 の点なので F そのもの。
/// 閾値（0.5・F+B=1・Lum の大小）から離れた色を選んでいる。
#[rustfmt::skip] // 1 種 1 行の表のまま読む
const BLENDS: [(D2D1_BLEND_MODE, [u8; 4], [u8; 4]); 26] = [
    (D2D1_BLEND_MODE_MULTIPLY, [143, 31, 31, 255], [142, 97, 33, 255]),
    (D2D1_BLEND_MODE_SCREEN, [240, 148, 173, 255], [214, 171, 82, 255]),
    (D2D1_BLEND_MODE_DARKEN, [179, 77, 51, 255], [153, 102, 51, 255]),
    (D2D1_BLEND_MODE_LIGHTEN, [204, 102, 153, 255], [204, 166, 64, 255]),
    (D2D1_BLEND_MODE_DISSOLVE, [204, 102, 51, 255], [204, 102, 51, 255]),
    (D2D1_BLEND_MODE_COLOR_BURN, [160, 0, 0, 255], [133, 146, 25, 255]),
    (D2D1_BLEND_MODE_LINEAR_BURN, [128, 0, 0, 255], [127, 89, 25, 255]),
    (D2D1_BLEND_MODE_DARKER_COLOR, [204, 102, 51, 255], [204, 102, 51, 255]),
    (D2D1_BLEND_MODE_LIGHTER_COLOR, [179, 77, 153, 255], [153, 166, 64, 255]),
    (D2D1_BLEND_MODE_COLOR_DODGE, [255, 128, 191, 255], [230, 179, 74, 255]),
    (D2D1_BLEND_MODE_LINEAR_DODGE, [255, 179, 204, 255], [230, 179, 90, 255]),
    (D2D1_BLEND_MODE_OVERLAY, [225, 62, 92, 255], [183, 163, 41, 255]),
    (D2D1_BLEND_MODE_SOFT_LIGHT, [200, 66, 116, 255], [170, 163, 48, 255]),
    (D2D1_BLEND_MODE_HARD_LIGHT, [225, 62, 61, 255], [199, 143, 41, 255]),
    (D2D1_BLEND_MODE_VIVID_LIGHT, [255, 33, 0, 255], [229, 163, 25, 255]),
    (D2D1_BLEND_MODE_LINEAR_LIGHT, [255, 26, 0, 255], [229, 140, 25, 255]),
    (D2D1_BLEND_MODE_PIN_LIGHT, [179, 77, 102, 255], [178, 153, 64, 255]),
    (D2D1_BLEND_MODE_HARD_MIX, [255, 0, 0, 255], [230, 179, 25, 255]),
    (D2D1_BLEND_MODE_DIFFERENCE, [25, 25, 102, 255], [153, 115, 39, 255]),
    (D2D1_BLEND_MODE_EXCLUSION, [97, 117, 143, 255], [173, 125, 74, 255]),
    (D2D1_BLEND_MODE_HUE, [182, 114, 80, 255], [230, 138, 92, 255]),
    (D2D1_BLEND_MODE_SATURATION, [213, 60, 174, 255], [152, 166, 64, 255]),
    (D2D1_BLEND_MODE_COLOR, [217, 115, 64, 255], [230, 138, 92, 255]),
    (D2D1_BLEND_MODE_LUMINOSITY, [166, 64, 140, 255], [117, 130, 28, 255]),
    (D2D1_BLEND_MODE_SUBTRACT, [0, 0, 102, 255], [102, 115, 39, 255]),
    (D2D1_BLEND_MODE_DIVISION, [224, 192, 255, 255], [165, 179, 153, 255]),
];

fn near(px: &Pixels, what: &str, expected: [u8; 4], x: u32, y: u32) {
    let got = px.at(x, y);
    let ok = got
        .iter()
        .zip(expected)
        .all(|(&a, e)| (a as i16 - e as i16).abs() <= 1);
    assert!(ok, "{what} ({x}, {y}): got {got:?}, expected {expected:?}");
}

#[test]
fn under_pixels_are_as_assumed() {
    // 較正: 見本の前提の下の画素。
    let px = replay_to_pixels(&under(), 40, 20, Matrix3x2::identity());
    pixel_near(&px, B1, 10, 10);
    pixel_near(&px, B2_PREMUL, 30, 10);
}

#[test]
fn all_26_blends_match_the_formulas_and_keep_outside() {
    // 26 種が 0〜25 を 1 度ずつ埋める。
    let mut values: Vec<i32> = BLENDS.iter().map(|(m, _, _)| m.0).collect();
    values.sort();
    assert_eq!(values, (0..26).collect::<Vec<_>>());

    let before = replay_to_pixels(&under(), 40, 20, Matrix3x2::identity());
    for (mode, on_opaque, on_half) in BLENDS {
        let px = replay_to_pixels(
            &over_under(MixMode::Blend(mode)),
            40,
            20,
            Matrix3x2::identity(),
        );
        let what = format!("{mode:?}");
        near(&px, &what, on_opaque, 10, 10);
        near(&px, &what, on_half, 30, 10);
        for (x, y) in OUTSIDE {
            near(&px, &what, before.at(x, y), x, y); // 1.3
        }
    }
}

#[test]
fn composite_mix_is_unchanged() {
    // 1.4: 既定（SOURCE_OVER）は F が上に乗る。
    let id = Matrix3x2::identity();
    let px = replay_to_pixels(&over_under(MixMode::default()), 40, 20, id);
    pixel_near(&px, F, 10, 10);
    pixel_near(&px, F, 30, 10);
    // DESTINATION_OVER: 不透明の下はそのまま、半透明の下は B2 + (1−128/255)·F。
    let mix = MixMode::Composite(D2D1_COMPOSITE_MODE_DESTINATION_OVER);
    let px = replay_to_pixels(&over_under(mix), 40, 20, id);
    pixel_near(&px, B1, 10, 10);
    pixel_near(&px, [153, 166, 64, 255], 30, 10);
    pixel_near(&px, B2_PREMUL, 38, 18);
}

#[test]
fn blend_without_dc6_or_out_of_range_draws_source_over() {
    let id = Matrix3x2::identity();
    // 1.7: ブレンドの描き方が無い DC。描き終わり（EndDraw）の成功は足場が確かめる。
    let mix = MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY);
    let px = replay_to_pixels_without_blend(&over_under(mix), 40, 20, id);
    pixel_near(&px, F, 10, 10);
    pixel_near(&px, F, 30, 10);
    pixel_near(&px, B1, 1, 1);
    // 範囲の外の値は D2D へ渡さず SOURCE_OVER。
    for mix in [
        MixMode::Blend(D2D1_BLEND_MODE(26)),
        MixMode::Blend(D2D1_BLEND_MODE(-1)),
        MixMode::Composite(D2D1_COMPOSITE_MODE(13)),
    ] {
        let px = replay_to_pixels(&over_under(mix), 40, 20, id);
        pixel_near(&px, F, 10, 10);
        pixel_near(&px, F, 30, 10);
        pixel_near(&px, B2_PREMUL, 38, 18);
    }
}

/// 的の左 (0,0)-(20,h) を不透明の白に塗る（右は透明）。白の上の乗算も透明の上のブレンドも絵の色になる。
fn white_left(h: f32) -> DrawCommand {
    draw(
        &solid(r(0.0, 0.0, 20.0, h), WHITE),
        Matrix3x2::identity(),
        MixMode::default(),
    )
}

#[test]
fn blend_outline_under_transforms_matches_source_over() {
    // 1.6・research.md §10.3 の形: 局所 (-10,-6)-(10,6) を 40×40 の的の中央 (20,20) へ置く。
    let content = solid(r(-10.0, -6.0, 10.0, 6.0), F);
    let center = Matrix3x2::translation(20.0, 20.0);
    let cases = [
        ("rotation", Matrix3x2::rotation(30.0) * center),
        ("negative scale", Matrix3x2::scale(-1.5, 1.2) * center),
        ("scale", Matrix3x2::scale(1.7, 1.7) * center),
        (
            "all",
            Matrix3x2::scale(-1.5, 1.2) * Matrix3x2::rotation(30.0) * center,
        ),
    ];
    let id = Matrix3x2::identity();
    let background = replay_to_pixels(&[white_left(40.0)], 40, 40, id);
    for (name, mat) in cases {
        let render =
            |mix| replay_to_pixels(&[white_left(40.0), draw(&content, mat, mix)], 40, 40, id);
        let blend = render(MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY));
        let over = render(MixMode::default());
        // 較正: 絵が描かれている。
        let changed = (0..40 * 40)
            .filter(|i| over.at(i % 40, i / 40) != background.at(i % 40, i / 40))
            .count();
        assert!(changed > 200, "{name}: 絵が描かれていない（{changed}）");
        for i in 0..40 * 40 {
            let (x, y) = (i % 40, i / 40);
            near(&blend, name, over.at(x, y), x, y);
        }
    }
}

#[test]
fn blend_under_rectangle_clip() {
    // 矩形の切り抜き (12,2)-(28,18) の中は下の画素と混ざり、外は下の画素のまま。
    let mut commands = under();
    commands.push(DrawCommand::PushClipRect(ClipRect {
        entity: Entity::PLACEHOLDER,
        version: 0,
        world_aabb: Aabb::new(12.0, 2.0, 28.0, 18.0),
    }));
    commands.push(source(MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY)));
    commands.push(DrawCommand::PopClipRect);
    let px = replay_to_pixels(&commands, 40, 20, Matrix3x2::identity());
    let (_, on_opaque, on_half) = BLENDS[0];
    pixel_near(&px, on_opaque, 15, 10);
    pixel_near(&px, on_half, 25, 10);
    pixel_near(&px, B1, 8, 10); // 絵の中・切り抜きの外
    pixel_near(&px, B2_PREMUL, 32, 10);
}

/// 楕円の切り抜き（中心 (20,10)・半径 12×7）の push。
fn push_ellipse_clip() -> DrawCommand {
    let ellipse: ID2D1Geometry = unsafe {
        warp_factory().CreateEllipseGeometry(&D2D1_ELLIPSE {
            point: Vector2 { X: 20.0, Y: 10.0 },
            radiusX: 12.0,
            radiusY: 7.0,
        })
    }
    .unwrap()
    .cast()
    .unwrap();
    DrawCommand::PushClipGeometry(ClipGeometry {
        entity: Entity::PLACEHOLDER,
        version: 0,
        world_mat: Matrix3x2::identity(),
        geometry: ellipse,
        world_aabb: Aabb::new(8.0, 3.0, 32.0, 17.0),
    })
}

#[test]
fn blend_under_geometry_clip() {
    // 下の画素は白と透明で、図形の層の旗に依らず答えが絵の色になる形。
    let commands = [
        white_left(20.0),
        push_ellipse_clip(),
        source(MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY)),
        DrawCommand::PopClipGeometry,
    ];
    let px = replay_to_pixels(&commands, 40, 20, Matrix3x2::identity());
    pixel_near(&px, F, 15, 10);
    pixel_near(&px, F, 25, 10);
    pixel_near(&px, WHITE, 5, 10); // 絵の中・楕円の外
    pixel_near(&px, CLEAR, 34, 10);
}

#[test]
fn blend_under_geometry_clip_mixes_with_colored_under_pixels() {
    // 図形の切り抜きの中も矩形の切り抜きと同じく下の画素に効く（3.1）。外は下の画素のまま（3.2）。
    let mut commands = under();
    commands.push(push_ellipse_clip());
    commands.push(source(MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY)));
    commands.push(DrawCommand::PopClipGeometry);
    let px = replay_to_pixels(&commands, 40, 20, Matrix3x2::identity());
    let (_, on_opaque, on_half) = BLENDS[0];
    pixel_near(&px, on_opaque, 15, 10);
    pixel_near(&px, on_half, 25, 10);
    pixel_near(&px, B1, 5, 10); // 絵の中・楕円の外
    pixel_near(&px, B2_PREMUL, 35, 10);
}
