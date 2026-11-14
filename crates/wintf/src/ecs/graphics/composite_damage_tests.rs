//! 範囲だけの描き直しの答え（WARP・要件 7.3 の写し以外）。範囲だけで合成した入れ物と、全部合成した入れ物が
//! バイトで一致する。

use super::composite::{SurfaceComposite, compose_or_drop, compose_planned};
use super::damage::{ComposePlan, FullReason, PartialPlan, RegionShift};
use super::visual::emit_test_support::*;
use super::visual::*;
use crate::ecs::ClipShape;
use crate::numerics::Transform2D;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use std::f32::consts::FRAC_PI_2;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::Matrix3x2;

fn scale(s: f32) -> Matrix3x2 {
    Matrix3x2::scale(s, s)
}

/// 根の命令の列（緑の (1,1)-(10,10)）。
fn root_list() -> ID2D1CommandList {
    solid(rect(1.0, 1.0, 10.0, 10.0), [0, 255, 0, 255]).command_list
}

/// 32×32 の根（にじみの余白で全面にならない大きさ）の下に赤（動かす節）と青。動かす節を返す。
fn tree(world: &mut World) -> (Entity, Entity) {
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 32.0, 32.0)))
        .id();
    let red = world
        .spawn((
            Visual::default(),
            at(2.0, 2.0, 5.0, 5.0),
            solid(rect(0.0, 0.0, 5.0, 5.0), RED),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(9.0, 9.0, 5.0, 5.0),
        solid(rect(0.0, 0.0, 5.0, 5.0), BLUE),
        ChildOf(root),
    ));
    (root, red)
}

fn composed(
    s: f32,
    list: Option<&ID2D1CommandList>,
    commands: &[DrawCommand],
) -> (ID2D1DeviceContext, SurfaceComposite, Vec<u8>) {
    let size = (32.0 * s).ceil() as u32;
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (size, size)).unwrap();
    comp.compose(&dc, scale(s), list, commands).unwrap();
    let bytes = comp.read_all(warp_device()).unwrap();
    (dc, comp, bytes)
}

fn partial(rects: Vec<PxRect>, shifts: Vec<RegionShift>) -> PartialPlan {
    PartialPlan {
        area: rects.iter().map(|r| r.area()).sum(),
        shift_area: shifts.iter().map(|s| s.src.area()).sum(),
        rects,
        shifts,
        items_redrawn: 0,
        exposed_area: 0,
        present_rect: PxRect::default(),
    }
}

fn pixel(bytes: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    bytes[i..i + 4].try_into().unwrap()
}

fn inside(r: PxRect, x: u32, y: u32) -> bool {
    (r.left..r.right).contains(&(x as i32)) && (r.top..r.bottom).contains(&(y as i32))
}

#[test]
fn partial_keeps_outside_and_matches_full_inside_at_scale_1_and_1_25() {
    for s in [1.0f32, 1.25] {
        let mut world = World::new();
        let (root, red) = tree(&mut world);
        let list = root_list();
        let before = tree_commands(&mut world, root, None);
        let (dc, mut comp, prev) = composed(s, Some(&list), &before);
        world.entity_mut(red).insert(at(5.0, 4.0, 5.0, 5.0));
        let now = tree_commands(&mut world, root, None);
        let size = comp.size();
        let now_entries = entries(&now, (s, s), size).expect("項の列");
        let (_, _, want) = composed(s, Some(&list), &now);
        // 変化の一部だけを描く矩形にする（外の画素が前のまま残ることが見える）。
        let r = px(3, 3, (8.0 * s) as i32, (9.0 * s) as i32);
        let generation = comp.generation();
        comp.compose_partial(
            &dc,
            scale(s),
            Some(&list),
            &now,
            &now_entries,
            &partial(vec![r], vec![]),
        )
        .unwrap();
        assert_eq!(comp.generation(), generation + 1, "成功で世代が 1 上がる");
        let got = comp.read_all(warp_device()).unwrap();
        let (mut inside_changed, mut outside_changed) = (false, false);
        for y in 0..size.1 {
            for x in 0..size.0 {
                let (g, p, w) = (
                    pixel(&got, size.0, x, y),
                    pixel(&prev, size.0, x, y),
                    pixel(&want, size.0, x, y),
                );
                if inside(r, x, y) {
                    assert_eq!(g, w, "拡大 {s}: 描く矩形の中 ({x}, {y}) が全部の合成と違う");
                    inside_changed |= p != w;
                } else {
                    assert_eq!(g, p, "拡大 {s}: 描く矩形の外 ({x}, {y}) が前の答えでない");
                    outside_changed |= p != w;
                }
            }
        }
        assert!(
            inside_changed && outside_changed,
            "較正: 中と外の両方に変化が在る"
        );
    }
}

#[test]
fn shift_copies_previous_pixels_through_work_bitmap() {
    let mut world = World::new();
    let (root, _) = tree(&mut world);
    let commands = tree_commands(&mut world, root, None);
    let (dc, mut comp, prev) = composed(1.0, None, &commands);
    let size = comp.size();
    // 元と先が重なる写し（作業用の絵を通す・D6）。
    let shift = RegionShift {
        src: px(1, 1, 11, 11),
        dst: (4, 3),
    };
    let generation = comp.generation();
    comp.compose_partial(
        &dc,
        scale(1.0),
        None,
        &commands,
        &entries(&commands, (1.0, 1.0), size).unwrap(),
        &partial(vec![], vec![shift]),
    )
    .unwrap();
    assert_eq!(comp.generation(), generation + 1);
    let got = comp.read_all(warp_device()).unwrap();
    let dst = px(4, 3, 14, 13);
    let mut moved = false;
    for y in 0..size.1 {
        for x in 0..size.0 {
            let g = pixel(&got, size.0, x, y);
            if inside(dst, x, y) {
                let p = pixel(&prev, size.0, x - 3, y - 2);
                assert_eq!(g, p, "写しの先 ({x}, {y}) が前の画素をずらしたものでない");
                moved |= p != pixel(&prev, size.0, x, y);
            } else {
                assert_eq!(g, pixel(&prev, size.0, x, y), "写しの外 ({x}, {y})");
            }
        }
    }
    assert!(moved, "較正: ずらすと画素が変わる");
    let mut after = Matrix3x2::default();
    unsafe {
        assert!(dc.GetTarget().is_err(), "合成の後も DC に的が残る");
        dc.GetTransform(&mut after);
    }
    assert_eq!(after, Matrix3x2::identity());
}

/// 本番の係と同じ `compose_planned` を呼ぶ（入れ物を作った回だけ `fresh`）。
fn planned(
    dc: &ID2D1DeviceContext,
    slot: &mut Option<SurfaceComposite>,
    fresh: Option<FullReason>,
    s: f32,
    list: Option<&ID2D1CommandList>,
    commands: &[DrawCommand],
) -> ComposePlan {
    let size = slot.as_ref().unwrap().size();
    let (p, _) = compose_planned(slot, dc, fresh, scale(s), list, commands, size).unwrap();
    p
}

#[test]
fn planned_skip_keeps_generation_and_pixels() {
    let mut world = World::new();
    let (root, _) = tree(&mut world);
    let list = root_list();
    let commands = tree_commands(&mut world, root, None);
    let dc = warp_dc();
    let mut slot = Some(SurfaceComposite::new(&dc, (32, 32)).unwrap());
    let first = planned(
        &dc,
        &mut slot,
        Some(FullReason::First),
        1.0,
        Some(&list),
        &commands,
    );
    assert_eq!(first, ComposePlan::Full(FullReason::First));
    let comp = slot.as_mut().unwrap();
    assert_eq!(comp.generation(), 1);
    assert!(comp.prev_entries().is_some(), "成功で控えを置く");
    // 写し終えていない回は写すだけ（合成しない）。
    let (generation, bytes) = (comp.generation(), comp.read_all(warp_device()).unwrap());
    let p = planned(&dc, &mut slot, None, 1.0, Some(&list), &commands);
    assert_eq!(p, ComposePlan::PresentOnly);
    slot.as_mut().unwrap().mark_presented();
    // 写し終えた後の変わらない回は何もしない。
    let p = planned(&dc, &mut slot, None, 1.0, Some(&list), &commands);
    assert_eq!(p, ComposePlan::Skip);
    let comp = slot.as_ref().unwrap();
    assert_eq!(
        comp.generation(),
        generation,
        "何もしない・写すだけで世代が変わらない"
    );
    assert!(
        comp.read_all(warp_device()).unwrap() == bytes,
        "画素が変わらない"
    );
}

/// 記録の水準を閉じた回と開いた回で、計画と入れ物の画素が同じ（6.3）。
#[test]
fn record_level_does_not_change_plan_or_pixels() {
    let run = |directives: &str| {
        let mut out = Vec::new();
        crate::ecs::test_support::capture_under_filter(directives, || {
            let mut world = World::new();
            let (root, red) = tree(&mut world);
            let dc = warp_dc();
            let mut slot = Some(SurfaceComposite::new(&dc, (32, 32)).unwrap());
            let mut fresh = Some(FullReason::First);
            // 初めて・動かす・何も変えない（同じ命令の列をもう 1 度渡す）
            let mut now = Vec::new();
            for step in [Some((2.0, 2.0)), Some((5.0, 4.0)), None] {
                if let Some((x, y)) = step {
                    world.entity_mut(red).insert(at(x, y, 5.0, 5.0));
                    now = tree_commands(&mut world, root, None);
                }
                let p = planned(&dc, &mut slot, fresh.take(), 1.0, None, &now);
                let comp = slot.as_mut().unwrap();
                out.push((p, comp.read_all(warp_device()).unwrap()));
                comp.mark_presented();
            }
        });
        out
    };
    let closed = run("off");
    let open = run("wintf::damage=debug");
    assert!(
        matches!(closed[1].0, ComposePlan::Partial(_)),
        "{:?}",
        closed[1].0
    );
    assert_eq!(closed[2].0, ComposePlan::Skip);
    for (i, (c, o)) in closed.iter().zip(&open).enumerate() {
        assert_eq!(c.0, o.0, "{i} 回目の計画が記録の水準で変わった");
        assert!(c.1 == o.1, "{i} 回目の画素が記録の水準で変わった");
    }
}

#[test]
fn planned_failure_drops_the_composite_and_its_record() {
    let dc = warp_dc();
    let mut slot = Some(SurfaceComposite::new(&dc, (32, 32)).unwrap());
    let open = unsafe { dc.CreateCommandList().unwrap() };
    let result = compose_planned(
        &mut slot,
        &dc,
        Some(FullReason::First),
        scale(1.0),
        Some(&open),
        &[],
        (32, 32),
    );
    assert!(result.is_err());
    assert!(slot.is_none(), "失敗で入れ物と控えを捨てる");
}

#[test]
fn failed_partial_returns_err_and_drops_the_composite() {
    let (dc, comp, _) = composed(1.0, None, &[]);
    let mut slot = Some(comp);
    // 閉じていない命令の列は EndDraw で D2DERR_WRONG_STATE（描き終わりの失敗を決まって起こす）。
    let open = unsafe { dc.CreateCommandList().unwrap() };
    let generation = slot.as_ref().unwrap().generation();
    let result = compose_or_drop(&mut slot, |c| {
        let r = c.compose_partial(
            &dc,
            scale(1.0),
            Some(&open),
            &[],
            &[],
            &partial(vec![px(0, 0, 4, 4)], vec![]),
        );
        assert_eq!(c.generation(), generation, "失敗で世代を上げない");
        r
    });
    assert!(result.is_err());
    assert!(slot.is_none());
}

// ---- 範囲だけの描き直しと全部の答えのバイトの一致（6.1・R1） ----

/// 6.1 の木の根の大きさ（DIP）。拡大 1.25 でも にじみの余白で全面にならない大きさ。
const SIDE: f32 = 40.0;
pub(super) const BOTH: &[f32] = &[1.0, 1.25];
pub(super) const GREEN: [u8; 4] = [0, 255, 0, 255];
pub(super) const YELLOW: [u8; 4] = [0, 255, 255, 255];

/// `parent` の下に `(x, y)` の 8×8 の単色の件。
pub(super) fn item(world: &mut World, parent: Entity, x: f32, y: f32, bgra: [u8; 4]) -> Entity {
    world
        .spawn((
            Visual::default(),
            at(x, y, 8.0, 8.0),
            solid(rect(0.0, 0.0, 8.0, 8.0), bgra),
            ChildOf(parent),
        ))
        .id()
}

/// 根の下の (4,4)-(16,16) の組（中身なし・変形を持つので節）に、はみ出す赤 (2,2)-(10,10) と青 (10,10)-(18,18)。
/// 返すのは [組, 赤, 青]。
fn group_of_two(world: &mut World, root: Entity) -> Vec<Entity> {
    let g = world
        .spawn((
            Visual::default(),
            at(4.0, 4.0, 12.0, 12.0),
            rotated(0.0),
            ChildOf(root),
        ))
        .id();
    let red = item(world, g, -2.0, -2.0, RED);
    let blue = item(world, g, 6.0, 6.0, BLUE);
    vec![g, red, blue]
}

pub(super) fn with_clip(clip: Option<ClipShape>) -> Visual {
    Visual {
        clip,
        ..Default::default()
    }
}

fn faded(opacity: f32) -> Visual {
    Visual {
        opacity,
        ..Default::default()
    }
}

/// 同じ描く中身で補間と混ぜ方だけを変える。
fn content_with(
    bgra: [u8; 4],
    interpolation_mode: D2D1_INTERPOLATION_MODE,
    mix_mode: MixMode,
) -> VisualDrawContent {
    VisualDrawContent {
        interpolation_mode,
        mix_mode,
        ..solid(rect(0.0, 0.0, 8.0, 8.0), bgra)
    }
}

pub(super) fn rotated(rad: f32) -> VisualTransform {
    VisualTransform(Transform2D {
        rotation: rad,
        anchor: windows_numerics::Vector2 { X: 4.0, Y: 4.0 },
        ..Transform2D::identity()
    })
}

/// `size`×`size` の入れ物へ全部で合成した答え。
fn full_bytes(
    size: u32,
    s: f32,
    list: Option<&ID2D1CommandList>,
    commands: &[DrawCommand],
) -> Vec<u8> {
    let dc = warp_dc();
    let mut comp = SurfaceComposite::new(&dc, (size, size)).unwrap();
    comp.compose(&dc, scale(s), list, commands).unwrap();
    comp.read_all(warp_device()).unwrap()
}

/// 2 つの絵がバイトで同じ（違えば最初の画素の座標と値を出す）。
fn assert_same(got: &[u8], want: &[u8], width: u32, at: &str) {
    if let Some(i) = got.chunks(4).zip(want.chunks(4)).position(|(g, w)| g != w) {
        let (x, y) = (i as u32 % width, i as u32 / width);
        panic!(
            "{at}: 全部の合成とバイトで違う ({x}, {y}): {:?} != {:?}",
            pixel(got, width, x, y),
            pixel(want, width, x, y)
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Want {
    /// 範囲だけ（写し無し）。
    Partial,
    /// 範囲だけで写しが `n` 個。
    Shift(usize),
    /// 範囲だけで描くが絵は変わらない（重ね方で消される件の変化）。
    Hidden,
    Skip,
    Full(FullReason),
}

/// 1 フレームの変え方（`nodes[0]` は根・続きは組み立ての返した節）。
pub(super) type Change = fn(&mut World, &[Entity]);

/// `setup` で組んだ木を全部で合成し、`steps` の各回に木を変えて本番と同じ `compose_planned` で合成する。
/// 毎回、計画が `Want` の形で、入れ物が今の列で全部合成した答えとバイトで一致する。範囲だけの回は世代が
/// 1 上がり・絵が変わり・描く矩形と写しの先の外が前の答えのまま（2.3）。何もしない回は世代と画素が
/// 変わらない（3.2）。拡大は `scales` の各値。変化の覚えを持ち続けるので、変えない節の版は上がらない。
/// 根の命令の列も持つ（`with_root`）と、範囲だけの回は `Full(Undecidable)` になる（R1）。
pub(super) fn check_frames(
    scales: &[f32],
    with_root: bool,
    setup: &dyn Fn(&mut World, Entity) -> Vec<Entity>,
    steps: &[(&str, Want, Change)],
) {
    for &s in scales {
        let mut world = World::new();
        let root = world
            .spawn((Visual::default(), at(0.0, 0.0, SIDE, SIDE)))
            .id();
        let mut nodes = vec![root];
        nodes.extend(setup(&mut world, root));
        let list = with_root.then(root_list);
        let list = list.as_ref();
        let mut frames = TreeFrames::new(&mut world);
        let dc = warp_dc();
        let size = (SIDE * s).ceil() as u32;
        let mut slot = Some(SurfaceComposite::new(&dc, (size, size)).unwrap());
        let commands = frames.commands(&mut world, root, None);
        planned(&dc, &mut slot, Some(FullReason::First), s, list, &commands);
        slot.as_mut().unwrap().mark_presented();
        for &(what, want, change) in steps {
            let at = format!("拡大 {s}・根の列 {with_root}・{what}");
            change(&mut world, &nodes);
            let commands = frames.commands(&mut world, root, None);
            let comp = slot.as_ref().unwrap();
            let (generation, prev) = (comp.generation(), comp.read_all(warp_device()).unwrap());
            let p = planned(&dc, &mut slot, None, s, list, &commands);
            let comp = slot.as_mut().unwrap();
            let got = comp.read_all(warp_device()).unwrap();
            let want = match want {
                Want::Partial | Want::Hidden | Want::Shift(_) if with_root => {
                    Want::Full(FullReason::Undecidable)
                }
                w => w,
            };
            match (&p, want) {
                (ComposePlan::Partial(p), Want::Partial | Want::Hidden | Want::Shift(_)) => {
                    assert_eq!(comp.generation(), generation + 1, "{at}: 世代");
                    let n = if let Want::Shift(n) = want { n } else { 0 };
                    assert_eq!(p.shifts.len(), n, "{at}: 写しの数: {p:?}");
                    assert_eq!(
                        got != prev,
                        !matches!(want, Want::Hidden),
                        "{at}: 較正: 絵が変わるか"
                    );
                    let drawn: Vec<PxRect> = p
                        .rects
                        .iter()
                        .copied()
                        .chain(p.shifts.iter().map(|sh| {
                            let (dx, dy) =
                                (sh.dst.0 as i32 - sh.src.left, sh.dst.1 as i32 - sh.src.top);
                            sh.src.offset(dx, dy)
                        }))
                        .collect();
                    for y in 0..size {
                        for x in 0..size {
                            if !drawn.iter().any(|&r| inside(r, x, y)) {
                                assert_eq!(
                                    pixel(&got, size, x, y),
                                    pixel(&prev, size, x, y),
                                    "{at}: 描く矩形の外 ({x}, {y}) が前の答えでない"
                                );
                            }
                        }
                    }
                }
                (ComposePlan::Full(r), Want::Full(w)) if *r == w => {
                    assert_eq!(comp.generation(), generation + 1, "{at}: 世代");
                }
                (ComposePlan::Skip, Want::Skip) => {
                    assert_eq!(comp.generation(), generation, "{at}: 世代");
                    assert!(got == prev, "{at}: 何もしない回に画素が変わった");
                }
                _ => panic!("{at}: 計画が {want:?} でない: {p:?}"),
            }
            assert_same(&got, &full_bytes(size, s, list, &commands), size, &at);
            comp.mark_presented();
        }
    }
}

/// 赤 (2,2)・青 (6,6)（赤と重なる）・黄 (26,26)。返すのは [赤, 青, 黄]。
fn three_items(world: &mut World, root: Entity) -> Vec<Entity> {
    vec![
        item(world, root, 2.0, 2.0, RED),
        item(world, root, 6.0, 6.0, BLUE),
        item(world, root, 26.0, 26.0, YELLOW),
    ]
}

const ITEM_STEPS: &[(&str, Want, Change)] = &[
    ("足す", Want::Partial, |w, n| {
        item(w, n[0], 20.5, 3.25, GREEN);
    }),
    ("小数の位置へ動かす", Want::Partial, |w, n| {
        w.entity_mut(n[1]).insert(at(4.5, 3.25, 8.0, 8.0));
    }),
    ("描く中身の差し替え", Want::Partial, |w, n| {
        w.entity_mut(n[2])
            .insert(solid(rect(0.0, 0.0, 8.0, 8.0), GREEN));
    }),
    (
        "並べ替え（赤を青の上へ）",
        Want::Partial,
        |w, n| {
            w.entity_mut(n[1]).remove::<ChildOf>().insert(ChildOf(n[0]));
        },
    ),
    ("消す", Want::Partial, |w, n| {
        w.despawn(n[3]);
    }),
];

#[test]
fn items_added_removed_moved_replaced_reordered_match_full_bytes() {
    check_frames(BOTH, false, &three_items, ITEM_STEPS);
    // 根の命令の列と木を両方持つ（範囲だけにしない・R1）。
    check_frames(BOTH, true, &three_items, ITEM_STEPS);
}

#[test]
fn rect_clip_added_reshaped_removed_matches_full_bytes() {
    check_frames(
        BOTH,
        false,
        &group_of_two,
        &[
            ("矩形の切り抜きを足す", Want::Partial, |w, n| {
                w.entity_mut(n[1])
                    .insert(with_clip(Some(ClipShape::Rectangle)));
            }),
            ("切り抜きの形を変える", Want::Partial, |w, n| {
                w.entity_mut(n[1]).insert(at(4.0, 4.0, 10.0, 13.0));
            }),
            (
                "切り抜きの中の件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[3]).insert(at(5.5, 4.75, 8.0, 8.0));
                },
            ),
            ("矩形の切り抜きを外す", Want::Partial, |w, n| {
                w.entity_mut(n[1]).insert(with_clip(None));
            }),
        ],
    );
}

#[test]
fn geometry_clip_added_reshaped_removed_matches_full_bytes() {
    check_frames(
        BOTH,
        false,
        &group_of_two,
        &[
            ("角丸の切り抜きを足す", Want::Partial, |w, n| {
                w.entity_mut(n[1])
                    .insert(with_clip(Some(ClipShape::rounded_rectangle(3.0))));
            }),
            ("角丸の形を変える", Want::Partial, |w, n| {
                w.entity_mut(n[1])
                    .insert(with_clip(Some(ClipShape::rounded_rectangle(5.5))));
            }),
            (
                "図形の切り抜きの中の件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[3]).insert(at(5.5, 4.75, 8.0, 8.0));
                },
            ),
            (
                "任意の図形（楕円）に変える",
                Want::Partial,
                |w, n| {
                    let ellipse: ID2D1Geometry = unsafe {
                        warp_factory().CreateEllipseGeometry(&D2D1_ELLIPSE {
                            point: windows_numerics::Vector2 { X: 6.0, Y: 6.0 },
                            radiusX: 6.5,
                            radiusY: 4.5,
                        })
                    }
                    .unwrap()
                    .into();
                    w.entity_mut(n[1])
                        .insert(with_clip(Some(ClipShape::Geometry(ellipse))));
                },
            ),
            ("図形の切り抜きを外す", Want::Partial, |w, n| {
                w.entity_mut(n[1]).insert(with_clip(None));
            }),
        ],
    );
}

#[test]
fn opacity_layer_added_changed_removed_matches_full_bytes() {
    check_frames(
        BOTH,
        false,
        &group_of_two,
        &[
            ("不透明度の層を足す", Want::Partial, |w, n| {
                w.entity_mut(n[1]).insert(faded(0.5));
            }),
            // 描く矩形は動いた青とかかる赤を丸ごと含み、層の中の件を切らない（R1）。
            (
                "層の中の件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[3]).insert(at(7.5, 6.25, 8.0, 8.0));
                },
            ),
            ("層の値を変える", Want::Partial, |w, n| {
                w.entity_mut(n[1]).insert(faded(0.75));
            }),
            ("不透明度の層を外す", Want::Partial, |w, n| {
                w.entity_mut(n[1]).insert(faded(1.0));
            }),
        ],
    );
}

/// 青 (4,4) の上のブレンド（乗算）の赤 (8,8) と、矩形の切り抜きの組 (20,4)-(36,20) の中の緑・
/// 黄（`DESTINATION_IN` に消される左下の隅）と `DESTINATION_IN` の件。
/// 返すのは [ブレンドの件, 緑, DESTINATION_IN の件, 黄]。
fn mixed_items(world: &mut World, root: Entity) -> Vec<Entity> {
    item(world, root, 4.0, 4.0, BLUE);
    let blend = world
        .spawn((
            Visual::default(),
            at(8.0, 8.0, 8.0, 8.0),
            content_with(
                RED,
                D2D1_INTERPOLATION_MODE_LINEAR,
                MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY),
            ),
            ChildOf(root),
        ))
        .id();
    let g = world
        .spawn((
            with_clip(Some(ClipShape::Rectangle)),
            at(20.0, 4.0, 16.0, 16.0),
            rotated(0.0),
            ChildOf(root),
        ))
        .id();
    let green = item(world, g, 1.0, 1.0, GREEN);
    let hidden = item(world, g, -4.0, 12.0, YELLOW);
    let dst_in = world
        .spawn((
            Visual::default(),
            at(4.0, 4.0, 8.0, 8.0),
            content_with(
                RED,
                D2D1_INTERPOLATION_MODE_LINEAR,
                MixMode::Composite(D2D1_COMPOSITE_MODE_DESTINATION_IN),
            ),
            ChildOf(g),
        ))
        .id();
    vec![blend, green, dst_in, hidden]
}

#[test]
fn blend_and_non_source_over_items_match_full_bytes() {
    check_frames(
        BOTH,
        false,
        &mixed_items,
        &[
            (
                "ブレンドの件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[1]).insert(at(9.5, 7.25, 8.0, 8.0));
                },
            ),
            (
                "SOURCE_OVER でない件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[3]).insert(at(8.25, 0.0, 8.0, 8.0));
                },
            ),
            // 消す件は切り抜きの中の全部を変えうる（D4）ので、描く矩形はかかるその件の範囲（切り抜き）まで
            // 広がる（R1）。絵は変わらない。
            (
                "SOURCE_OVER でない件に消される件を動かす",
                Want::Hidden,
                |w, n| {
                    w.entity_mut(n[4]).insert(at(-3.5, 12.25, 8.0, 8.0));
                },
            ),
            (
                "SOURCE_OVER でない件の下の件の中身を変える",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[2])
                        .insert(solid(rect(0.0, 0.0, 8.0, 8.0), YELLOW));
                },
            ),
            (
                "ブレンドの件の中身を差し替える",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[1]).insert(content_with(
                        GREEN,
                        D2D1_INTERPOLATION_MODE_LINEAR,
                        MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY),
                    ));
                },
            ),
        ],
    );
}

/// 補間の違う件（LINEAR・HIGH_QUALITY_CUBIC・回転した HIGH_QUALITY_CUBIC）。返すのは [LINEAR, CUBIC, 回転]。
fn interpolated_items(world: &mut World, root: Entity) -> Vec<Entity> {
    let mut spawn = |x: f32, y: f32, bgra, mode| {
        world
            .spawn((
                Visual::default(),
                at(x, y, 8.0, 8.0),
                content_with(bgra, mode, MixMode::default()),
                ChildOf(root),
            ))
            .id()
    };
    let linear = spawn(3.5, 3.5, RED, D2D1_INTERPOLATION_MODE_LINEAR);
    let cubic = spawn(15.25, 4.5, BLUE, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC);
    let turned = spawn(
        24.0,
        20.0,
        GREEN,
        D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
    );
    world.entity_mut(turned).insert(rotated(0.5));
    vec![linear, cubic, turned]
}

#[test]
fn interpolated_items_moved_to_fractional_positions_match_full_bytes() {
    check_frames(
        BOTH,
        false,
        &interpolated_items,
        &[
            (
                "LINEAR の件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[1]).insert(at(4.75, 2.25, 8.0, 8.0));
                },
            ),
            (
                "HIGH_QUALITY_CUBIC の件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[2]).insert(at(16.5, 5.75, 8.0, 8.0));
                },
            ),
            // にじみの余白（2.1）は回転で核の広がりが列の長さを越えうるので、ここで確かめる。
            (
                "回転した HIGH_QUALITY_CUBIC の件を小数の位置へ動かす",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[3]).insert(at(22.25, 21.5, 8.0, 8.0));
                },
            ),
            (
                "回転した HIGH_QUALITY_CUBIC の件の角度を変える",
                Want::Partial,
                |w, n| {
                    w.entity_mut(n[3]).insert(rotated(0.9));
                },
            ),
        ],
    );
}

/// 続けて木を変える（控えが古いと誤りが次の全部の合成まで残る＝1 回きりの比べでは見えない）。
#[test]
fn six_frames_of_changes_match_full_bytes_each_frame() {
    check_frames(
        BOTH,
        false,
        &group_of_two,
        &[
            ("足す", Want::Partial, |w, n| {
                item(w, n[0], 24.5, 22.25, GREEN);
            }),
            ("動かす", Want::Partial, |w, n| {
                w.entity_mut(n[2]).insert(at(1.5, -1.25, 8.0, 8.0));
            }),
            ("何も変えない", Want::Skip, |_, _| {}),
            ("層を足す", Want::Partial, |w, n| {
                w.entity_mut(n[1]).insert(faded(0.5));
            }),
            ("動かしたのを戻す", Want::Partial, |w, n| {
                w.entity_mut(n[2]).insert(at(-2.0, -2.0, 8.0, 8.0));
            }),
            ("何も変えない", Want::Skip, |_, _| {}),
        ],
    );
}

/// 全部へ倒す各回（3.1・3.3）も入れ物は全部の答えとバイトで一致する。項にできない回は控えを空にし、
/// 次の回が `Full(NoPrev)` になる。
#[test]
fn planned_full_reasons_match_full_bytes() {
    let mut world = World::new();
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, SIDE, SIDE)))
        .id();
    let red = three_items(&mut world, root)[0];
    let mut frames = TreeFrames::new(&mut world);
    let dc = warp_dc();
    let size = SIDE as u32;
    let mut slot = Some(SurfaceComposite::new(&dc, (size, size)).unwrap());
    let (first, second) = (root_list(), root_list());
    let nothing: Change = |_, _| {};
    let steps: [(ComposePlan, f32, &ID2D1CommandList, Change); 8] = [
        (ComposePlan::Full(FullReason::First), 1.0, &first, nothing),
        (ComposePlan::Full(FullReason::Scale), 1.25, &first, nothing),
        (
            ComposePlan::Full(FullReason::RootList),
            1.25,
            &second,
            nothing,
        ),
        (
            ComposePlan::Full(FullReason::CoversSurface),
            1.25,
            &second,
            |w, n| {
                w.spawn((
                    Visual::default(),
                    at(0.0, 0.0, SIDE, SIDE),
                    solid(rect(0.0, 0.0, SIDE, SIDE), [0, 0, 128, 128]),
                    ChildOf(n[0]),
                ));
            },
        ),
        // 物理の値が i32 に収まらない件（項にできない・3.3）。
        (
            ComposePlan::Full(FullReason::Undecidable),
            1.25,
            &second,
            |w, n| {
                w.entity_mut(n[1]).insert(VisualTransform(Transform2D {
                    scale: windows_numerics::Vector2 { X: 1e10, Y: 1e10 },
                    ..Transform2D::identity()
                }));
            },
        ),
        (
            ComposePlan::Full(FullReason::NoPrev),
            1.25,
            &second,
            |w, n| {
                w.entity_mut(n[1]).remove::<VisualTransform>();
            },
        ),
        (ComposePlan::Skip, 1.25, &second, nothing),
        (
            ComposePlan::Full(FullReason::Recreated),
            1.25,
            &second,
            nothing,
        ),
    ];
    for (i, (want, s, list, change)) in steps.into_iter().enumerate() {
        change(&mut world, &[root, red]);
        let commands = frames.commands(&mut world, root, None);
        let fresh = match want {
            ComposePlan::Full(r @ (FullReason::First | FullReason::Recreated)) => Some(r),
            _ => None,
        };
        let p = planned(&dc, &mut slot, fresh, s, Some(list), &commands);
        assert_eq!(p, want, "{i} 回目");
        let undecidable = want == ComposePlan::Full(FullReason::Undecidable);
        let comp = slot.as_mut().unwrap();
        assert_eq!(
            comp.prev_entries().is_none(),
            undecidable,
            "{i} 回目: 項にできない回だけ控えが空"
        );
        let got = comp.read_all(warp_device()).unwrap();
        let want_bytes = full_bytes(size, s, Some(list), &commands);
        assert_same(&got, &want_bytes, size, &format!("{i} 回目"));
        comp.mark_presented();
    }
}

/// 絵を変形して描く件は描く範囲で D2D の答えが 1 段変わりうる（R1）ので、描く矩形は交わる件を丸ごと含む。
/// 矩形が件の縁を切る（A を動かす）・件の中に在る（B の上の件を動かす）の両方を、変形と補間の組ごとに。
#[test]
fn bitmap_items_transformed_and_moved_match_full_bytes() {
    const MOVES: [(&str, Change); 2] = [
        ("絵 A を小数の位置へ動かす", |w, n| {
            w.entity_mut(n[1]).insert(at(7.75, 6.5, 8.0, 8.0));
        }),
        ("絵 B の上の小さい件を動かす", |w, n| {
            w.entity_mut(n[3]).insert(at(19.5, 17.25, 3.0, 3.0));
        }),
    ];
    let t = |sx: f32, sy: f32, rotation: f32| Transform2D {
        scale: windows_numerics::Vector2 { X: sx, Y: sy },
        rotation,
        anchor: windows_numerics::Vector2 { X: 4.0, Y: 4.0 },
        ..Transform2D::identity()
    };
    let hqc = D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC;
    let transforms = [
        t(1.25, 1.25, 0.0),
        t(1.0, 1.0, 0.5),
        t(1.0, 1.0, FRAC_PI_2),
        t(0.5, 1.6, 0.0),
    ];
    for (k, tr) in transforms.into_iter().enumerate() {
        for mode in [
            D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_INTERPOLATION_MODE_CUBIC,
            D2D1_INTERPOLATION_MODE_MULTI_SAMPLE_LINEAR,
            D2D1_INTERPOLATION_MODE_ANISOTROPIC,
            hqc,
        ] {
            // 縮める HIGH_QUALITY_CUBIC は WARP では全部の合成どうしも揃わない（下の 0.6 倍で全部へ倒す）
            if k == 3 && mode == hqc {
                continue;
            }
            let names = MOVES.map(|(m, _)| format!("{k} 番目の変形・{mode:?}・{m}"));
            let steps: Vec<_> = (0..2)
                .map(|i| (&*names[i], Want::Partial, MOVES[i].1))
                .collect();
            check_frames(BOTH, false, &|w, r| checker_pair(w, r, tr, mode, 8), &steps);
        }
    }
    // 縮める HIGH_QUALITY_CUBIC の件は全面まで広がる（2.1・R1）
    let shrink = t(0.6, 0.6, 0.0);
    let steps = MOVES.map(|(m, c)| (m, Want::Full(FullReason::CoversSurface), c));
    check_frames(
        BOTH,
        false,
        &|w, r| checker_pair(w, r, shrink, hqc, 8),
        &steps,
    );
    // 中身の中で縮める HIGH_QUALITY_CUBIC（16×16 の絵を 8×8 へ・外の変形は恒等）
    let one = t(1.0, 1.0, 0.0);
    let steps = MOVES.map(|(m, c)| (m, Want::Partial, c));
    check_frames(
        BOTH,
        false,
        &|w, r| checker_pair(w, r, one, hqc, 16),
        &steps,
    );
}
