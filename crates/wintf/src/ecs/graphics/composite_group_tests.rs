//! まとめの節の範囲だけの描き直しの答え（WARP）。範囲だけで作った入れ物と、全部合成した
//! 入れ物がバイトで一致する。拡大 1 で整数の位置のまとめは不透明度 1 の層、拡大 1.25 と
//! 小数の位置のまとめは作業用の絵の道を通る。

use super::composite_damage_tests::{
    BOTH, Change, GREEN, Want, YELLOW, check_frames, item, rotated, with_clip,
};
use super::damage::FullReason;
use super::visual::emit_test_support::*;
use super::visual::*;
use crate::ecs::ClipShape;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::*;

fn group(mix: D2D1_COMPOSITE_MODE) -> VisualGroup {
    VisualGroup {
        mix: MixMode::Composite(mix),
    }
}

/// `parent` の下の `(x, y)` のまとめ（16×16）に、重なる赤 (0,0)・青 (6,6)。
/// 返すのは [まとめ, 赤, 青]。
fn group_at(
    world: &mut World,
    parent: Entity,
    x: f32,
    y: f32,
    mix: D2D1_COMPOSITE_MODE,
) -> Vec<Entity> {
    let g = world
        .spawn((
            Visual::default(),
            at(x, y, 16.0, 16.0),
            rotated(0.0),
            group(mix),
            ChildOf(parent),
        ))
        .id();
    let red = item(world, g, 0.0, 0.0, RED);
    let blue = item(world, g, 6.0, 6.0, BLUE);
    vec![g, red, blue]
}

/// 黄 (26,26) と、その手前の `SOURCE_OVER` のまとめ (4,4)。返すのは [黄, まとめ, 赤, 青]。
fn source_over_group(world: &mut World, root: Entity, x: f32) -> Vec<Entity> {
    let mut out = vec![item(world, root, 26.0, 26.0, YELLOW)];
    out.extend(group_at(
        world,
        root,
        x,
        4.0,
        D2D1_COMPOSITE_MODE_SOURCE_OVER,
    ));
    out
}

const SOURCE_OVER_STEPS: &[(&str, Want, Change)] = &[
    (
        "まとめの中の 1 つの子の中身を替える",
        Want::Partial,
        |w, n| {
            w.entity_mut(n[4])
                .insert(solid(rect(0.0, 0.0, 8.0, 8.0), GREEN));
        },
    ),
    (
        "まとめの中の子を小数の位置へ動かす",
        Want::Partial,
        |w, n| {
            w.entity_mut(n[3]).insert(at(1.5, 0.25, 8.0, 8.0));
        },
    ),
    (
        "まとめの外の件を動かす",
        Want::Partial,
        |w, n| {
            w.entity_mut(n[1]).insert(at(27.0, 25.0, 8.0, 8.0));
        },
    ),
];

#[test]
fn child_change_inside_a_source_over_group_matches_full_bytes() {
    check_frames(
        BOTH,
        false,
        &|w, root| source_over_group(w, root, 4.0),
        SOURCE_OVER_STEPS,
    );
    check_frames(
        &[1.0],
        false,
        &|w, root| source_over_group(w, root, 4.5),
        SOURCE_OVER_STEPS,
    );
}

/// 黄 (2,26) と、矩形の切り抜き (8,8)-(32,32) の中の `SOURCE_ATOP` のまとめ (4,4)（切り抜きの中に
/// 赤い下地）。返すのは [黄, 切り抜き, まとめ, 赤, 青]。
fn clipped_atop_group(world: &mut World, root: Entity) -> Vec<Entity> {
    let yellow = item(world, root, 2.0, 30.0, YELLOW);
    let clip = world
        .spawn((
            with_clip(Some(ClipShape::Rectangle)),
            at(8.0, 8.0, 24.0, 24.0),
            rotated(0.0),
            ChildOf(root),
        ))
        .id();
    item(world, clip, 8.0, 8.0, GREEN);
    let mut out = vec![yellow, clip];
    out.extend(group_at(
        world,
        clip,
        4.0,
        4.0,
        D2D1_COMPOSITE_MODE_SOURCE_ATOP,
    ));
    out
}

#[test]
fn source_atop_group_redraws_its_rect_clip_and_matches_full_bytes() {
    check_frames(
        BOTH,
        false,
        &clipped_atop_group,
        &[
            (
                "まとめの中の 1 つの子の中身を替える",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[5])
                        .insert(solid(rect(0.0, 0.0, 8.0, 8.0), YELLOW));
                },
            ),
            (
                "切り抜きの外の件を動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[1]).insert(at(1.0, 31.0, 8.0, 8.0));
                },
            ),
            ("まとめを外す", Want::Partial, |w, n| {
                w.entity_mut(n[3]).remove::<VisualGroup>();
            }),
        ],
    );
}

/// 切り抜きの無い `SOURCE_OVER` でないまとめは描画面の全面を変えうる。
#[test]
fn unclipped_destination_over_group_redraws_the_whole_surface() {
    check_frames(
        BOTH,
        false,
        &|w, root| {
            let mut out = vec![item(w, root, 26.0, 26.0, YELLOW)];
            out.extend(group_at(
                w,
                root,
                4.0,
                4.0,
                D2D1_COMPOSITE_MODE_DESTINATION_OVER,
            ));
            out
        },
        &[(
            "まとめの中の 1 つの子の中身を替える",
            Want::Full(FullReason::CoversSurface),
            |w, n| {
                w.entity_mut(n[4])
                    .insert(solid(rect(0.0, 0.0, 8.0, 8.0), GREEN));
            },
        )],
    );
}
