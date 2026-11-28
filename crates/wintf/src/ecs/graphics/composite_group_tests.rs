//! まとめの節の範囲だけの描き直しの答え（WARP）。範囲だけで作った入れ物と、全部合成した
//! 入れ物がバイトで一致する。拡大 1 で整数の位置のまとめは不透明度 1 の層、拡大 1.25 と
//! 小数の位置のまとめは作業用の絵の道を通る。

use super::composite::SurfaceComposite;
use super::composite_damage_tests::{
    BOTH, Change, GREEN, Want, YELLOW, assert_same, check_frames, full_bytes, item, partial,
    rotated, with_clip,
};
use super::damage::FullReason;
use super::visual::emit_test_support::*;
use super::visual::*;
use crate::ecs::ClipShape;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows_numerics::Matrix3x2;

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

#[test]
fn surface_composite_keeps_one_group_work_bitmap_across_frames() {
    let mut world = World::new();
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 32.0, 32.0)))
        .id();
    source_over_group(&mut world, root, 4.0);
    let commands = tree_commands(&mut world, root, None);
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (40, 40)).unwrap();

    for _ in 0..3 {
        comp.compose(&dc, Matrix3x2::scale(1.25, 1.25), None, &commands)
            .unwrap();
    }

    assert_eq!(comp.group_work().created(), 1);
    assert_eq!(comp.group_work().len(), 1);
}

/// 拡大 1.25 で小数の位置（物理 4.125）のまとめに、離れた 2 つの描く矩形が交わる回。
#[test]
fn two_rects_crossing_a_fractional_group_at_1_25_match_full_bytes() {
    let s = 1.25;
    let mut world = World::new();
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 40.0, 40.0)))
        .id();
    let g = world
        .spawn((
            Visual::default(),
            at(3.3, 3.3, 32.0, 32.0),
            rotated(0.0),
            group(D2D1_COMPOSITE_MODE_SOURCE_OVER),
            ChildOf(root),
        ))
        .id();
    let near = item(&mut world, g, 0.0, 0.0, RED);
    let far = item(&mut world, g, 22.0, 22.0, BLUE);
    let size = 50;
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (size, size)).unwrap();
    let before = tree_commands(&mut world, root, None);
    comp.compose(&dc, Matrix3x2::scale(s, s), None, &before)
        .unwrap();
    let prev = comp.read_all(warp_device()).unwrap();

    world
        .entity_mut(near)
        .insert(solid(rect(0.0, 0.0, 8.0, 8.0), GREEN));
    world
        .entity_mut(far)
        .insert(solid(rect(0.0, 0.0, 8.0, 8.0), YELLOW));
    let now = tree_commands(&mut world, root, None);
    let now_entries = entries(&now, (s, s), (size, size)).expect("項の列");
    let rects = vec![px(3, 3, 16, 16), px(30, 30, 43, 43)];
    comp.compose_partial(
        &dc,
        Matrix3x2::scale(s, s),
        None,
        &now,
        &now_entries,
        &partial(rects, vec![]),
    )
    .unwrap();

    let got = comp.read_all(warp_device()).unwrap();
    let at = |b: &[u8], x: u32, y: u32| {
        let i = ((y * size + x) * 4) as usize;
        b[i..i + 4].to_vec()
    };
    for (x, y) in [(9, 9), (36, 36)] {
        assert_ne!(at(&got, x, y), at(&prev, x, y), "較正: ({x}, {y}) が変わる");
    }
    assert!(comp.group_work().created() > 0, "較正: 作業用の絵の道");
    assert_same(
        &got,
        &full_bytes(size, s, None, &now),
        size,
        "離れた 2 つの矩形",
    );
}
