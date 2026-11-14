//! 範囲だけの描き直しの答え（WARP・要件 7.3 の写し以外）。範囲だけで合成した入れ物と、全部合成した入れ物が
//! バイトで一致する。

use super::composite::{SurfaceComposite, compose_or_drop, compose_planned};
use super::damage::{ComposePlan, FullReason, PartialPlan, PlanInput, RegionShift, plan};
use super::visual::emit_test_support::*;
use super::visual::*;
use crate::ecs::layout::Arrangement;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
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

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

fn px(left: i32, top: i32, right: i32, bottom: i32) -> PxRect {
    PxRect {
        left,
        top,
        right,
        bottom,
    }
}

fn scale(s: f32) -> Matrix3x2 {
    Matrix3x2 {
        M11: s,
        M22: s,
        ..Matrix3x2::identity()
    }
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
fn partial_with_planned_rects_matches_full_bytes() {
    for s in [1.0f32, 1.25] {
        let mut world = World::new();
        let (root, red) = tree(&mut world);
        let list = root_list();
        let before = tree_commands(&mut world, root, None);
        let (dc, mut comp, prev) = composed(s, Some(&list), &before);
        let size = comp.size();
        let prev_entries = entries(&before, (s, s), size).unwrap();
        world.entity_mut(red).insert(at(5.0, 4.0, 5.0, 5.0));
        let now = tree_commands(&mut world, root, None);
        let now_entries = entries(&now, (s, s), size).unwrap();
        let planned = plan(&PlanInput {
            fresh: None,
            same_root: true,
            same_scale: true,
            root_present: true,
            prev: Some(&prev_entries),
            now: Some(&now_entries),
            presented: true,
            surface: size,
        });
        let ComposePlan::Partial(p) = planned else {
            panic!("拡大 {s}: 範囲だけの計画でない");
        };
        comp.compose_partial(&dc, scale(s), Some(&list), &now, &now_entries, &p)
            .unwrap();
        let (_, _, want) = composed(s, Some(&list), &now);
        assert_ne!(prev, want, "較正: 絵が変わる");
        assert!(
            comp.read_all(warp_device()).unwrap() == want,
            "拡大 {s}: 全部の合成とバイトで違う"
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

#[test]
fn planned_partial_uses_the_previous_record_on_each_call() {
    for s in [1.0f32, 1.25] {
        let mut world = World::new();
        let (root, red) = tree(&mut world);
        let list = root_list();
        let dc = warp_dc();
        let size = (32.0 * s).ceil() as u32;
        let mut slot = Some(SurfaceComposite::new(&dc, (size, size)).unwrap());
        let before = tree_commands(&mut world, root, None);
        planned(
            &dc,
            &mut slot,
            Some(FullReason::First),
            s,
            Some(&list),
            &before,
        );
        slot.as_mut().unwrap().mark_presented();
        // 動かす → 戻す。戻す回が 1 回目の控えを使えば「変わらない」になり画素を誤る。
        for (x, y) in [(5.0, 4.0), (2.0, 2.0), (6.0, 3.0)] {
            world.entity_mut(red).insert(at(x, y, 5.0, 5.0));
            let now = tree_commands(&mut world, root, None);
            let generation = slot.as_ref().unwrap().generation();
            let p = planned(&dc, &mut slot, None, s, Some(&list), &now);
            assert!(
                matches!(p, ComposePlan::Partial(_)),
                "拡大 {s} ({x}, {y}): 範囲だけの計画でない: {p:?}"
            );
            let comp = slot.as_mut().unwrap();
            assert_eq!(
                comp.generation(),
                generation + 1,
                "範囲だけの回に世代が 1 上がる"
            );
            let (_, _, want) = composed(s, Some(&list), &now);
            assert!(
                comp.read_all(warp_device()).unwrap() == want,
                "拡大 {s} ({x}, {y}): 全部の合成とバイトで違う"
            );
            comp.mark_presented();
        }
    }
}

/// 記録の水準を閉じた回と開いた回で、計画と入れ物の画素が同じ（6.3）。
#[test]
fn record_level_does_not_change_plan_or_pixels() {
    let run = |directives: &str| {
        let mut out = Vec::new();
        crate::ecs::test_support::capture_under_filter(directives, || {
            let mut world = World::new();
            let (root, red) = tree(&mut world);
            let list = root_list();
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
                let p = planned(&dc, &mut slot, fresh.take(), 1.0, Some(&list), &now);
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
