//! 平行移動の写しの答え（WARP・要件 7.3 の平行移動・5.1）。写しと帯の描き直しで作った入れ物と、全部合成した
//! 入れ物がバイトで一致する。

use super::composite_damage_tests::{
    BOTH, Change, GREEN, Want, YELLOW, check_frames, item, rotated, with_clip,
};
use super::visual::emit_test_support::*;
use super::visual::*;
use crate::ecs::ClipShape;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;

/// 中身なしの節（変形を持つので節になる）。
fn group(
    world: &mut World,
    parent: Entity,
    visual: Visual,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Entity {
    let parts = (visual, at(x, y, w, h), rotated(0.0), ChildOf(parent));
    world.spawn(parts).id()
}

/// 動かす中身の節 `content` の下に、赤 (2,2)・小数の位置の青・下の外の緑（入る）・上へはみ出す黄（出る）と、
/// 入れ子の矩形の切り抜き (14,24)-(24,34)（はみ出す赤を持つ）。黄は上へ 4 で余白ごと切り抜きの外へ出る位置
/// （-5 だと前の余白がずらした先で切り抜きにかかり S7 で写さない）。
fn fill_content(world: &mut World, content: Entity) {
    item(world, content, 2.0, 2.0, RED);
    item(world, content, 13.5, 11.25, BLUE);
    item(world, content, 2.0, 27.0, GREEN);
    item(world, content, 14.0, -6.5, YELLOW);
    let inner = group(
        world,
        content,
        with_clip(Some(ClipShape::Rectangle)),
        14.0,
        24.0,
        10.0,
        10.0,
    );
    item(world, inner, 5.5, -3.25, RED);
}

/// 根の下の矩形の切り抜き (8,8)-(32,32) の中に動かす中身。返すのは [切り抜き, 中身]。
fn scroll_box(world: &mut World, root: Entity) -> Vec<Entity> {
    let clip = group(
        world,
        root,
        with_clip(Some(ClipShape::Rectangle)),
        8.0,
        8.0,
        24.0,
        24.0,
    );
    let content = group(world, clip, Visual::default(), 0.0, 0.0, 24.0, 40.0);
    fill_content(world, content);
    vec![clip, content]
}

/// 切り抜きの下に重なる絵（根の青 (4,4)-(12,12)）の在る `scroll_box`。
fn scroll_box_over_item(world: &mut World, root: Entity) -> Vec<Entity> {
    item(world, root, 4.0, 4.0, BLUE);
    scroll_box(world, root)
}

/// 根の下の動かす中身（描画面の全面の候補）。返すのは [中身, 中身]（`n[2]` を中身にそろえる）。
fn whole_surface(world: &mut World, root: Entity) -> Vec<Entity> {
    let content = group(world, root, Visual::default(), 0.0, 0.0, 24.0, 40.0);
    fill_content(world, content);
    vec![content, content]
}

fn scroll(w: &mut World, n: &[Entity], x: f32, y: f32) {
    w.entity_mut(n[2]).insert(at(x, y, 24.0, 40.0));
}

/// 拡大 1.0 と 1.25 の両方で物理ピクセルの整数になる量（4 DIP＝4 px・5 px）。
const INTEGER_STEPS: &[(&str, Want, Change)] = &[
    (
        "上へ 4（下から入る・上へ出る）",
        Want::Shift(1),
        |w, n| scroll(w, n, 0.0, -4.0),
    ),
    ("左へ 4", Want::Shift(1), |w, n| scroll(w, n, -4.0, -4.0)),
    ("斜め（右下へ 4・4）", Want::Shift(1), |w, n| {
        scroll(w, n, 0.0, 0.0)
    }),
    ("下へ 8", Want::Shift(1), |w, n| scroll(w, n, 0.0, 8.0)),
];

/// 切り抜きの中の後の 2 歩は、帯を交わる件で閉じると写しの先を覆うので写しを落として描き直す（R1）。
const RECT_CLIP_STEPS: &[(&str, Want, Change)] = &[
    INTEGER_STEPS[0],
    INTEGER_STEPS[1],
    ("斜め（右下へ 4・4）", Want::Partial, |w, n| {
        scroll(w, n, 0.0, 0.0)
    }),
    ("下へ 8", Want::Partial, |w, n| scroll(w, n, 0.0, 8.0)),
];

#[test]
fn integer_scroll_inside_rect_clip_copies_and_matches_full_bytes() {
    check_frames(BOTH, false, &scroll_box, RECT_CLIP_STEPS);
    // 根の命令の列を持つ描画面では写さない（S1）
    check_frames(BOTH, true, &scroll_box, RECT_CLIP_STEPS);
}

#[test]
fn integer_scroll_of_whole_surface_copies_and_matches_full_bytes() {
    check_frames(BOTH, false, &whole_surface, INTEGER_STEPS);
}

/// 拡大 1.0 でも 1.25 でも整数にならない量（2.5 DIP＝2.5 px・3.125 px）は写さない（S4・5.2）。
const FRACTIONAL_STEPS: &[(&str, Want, Change)] = &[
    ("上へ 2.5", Want::Partial, |w, n| scroll(w, n, 0.0, -2.5)),
    ("左へ 2.5", Want::Partial, |w, n| scroll(w, n, -2.5, -2.5)),
];

#[test]
fn fractional_scroll_is_redrawn_without_copy_and_matches_full_bytes() {
    check_frames(BOTH, false, &scroll_box, FRACTIONAL_STEPS);
    check_frames(BOTH, false, &whole_surface, FRACTIONAL_STEPS);
}

/// 切り抜きの下に重なる絵が在れば写さない（S8・5.2）。
#[test]
fn scroll_over_an_item_below_is_redrawn_without_copy_and_matches_full_bytes() {
    let steps: &[(&str, Want, Change)] = &[
        ("上へ 4", Want::Partial, |w, n| scroll(w, n, 0.0, -4.0)),
        ("左へ 4", Want::Partial, |w, n| scroll(w, n, -4.0, -4.0)),
    ];
    check_frames(BOTH, false, &scroll_box_over_item, steps);
}
