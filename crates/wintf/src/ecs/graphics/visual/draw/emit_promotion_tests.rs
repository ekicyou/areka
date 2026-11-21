//! 自動で上げた持ち主の画素の一致（WARP）。
//!
//! 節の部分木を、親の描画面の中で描いた絵と、部分の列で原点を引いて自分の描画面に描き
//! WUC の重ねの写しで整数の位置へ重ねた絵とで比べる。
//!
//! 小数の位置（10.3, 7.6）の候補も拡大 1.0・1.25・1.5 で ±1 に揃う。ただし原点は物理ピクセルで引く
//! （`TreeFrame::Segment` が拡大を掛けてから整数を引く）。DIP で `px / 拡大` を引いた形は、割り切れない原点で
//! 位置が 1e-7 ずれ、被覆の標本にちょうど乗る端数（12.875）で楕円の縁が 1 チャネル 16 ほど違った。
//!
//! 上げ下げのフレーム: 本物の 1 回の判断（`run_round`）を回の列で回し、各回の画面
//! （親の列と、上げた持ち主の描画面を WUC の重ねの写しで重ねた絵）を、本番の合成の道（`compose_planned`
//! と `replay_transform` の根の変換の選び）で描いて、全部を節にした絵と比べる。

use super::super::super::composite::{SurfaceComposite, compose_planned};
use super::super::super::damage::FullReason;
use super::super::super::replay_transform;
use super::super::promotion::Phase;
use super::super::promotion_round::{PromotionData, RoundContext, run_round};
use super::super::promotion_round_tests::ready_surface;
use super::super::promotion_shape::{ShapeInput, check_shape, fractional_origin};
use super::super::world_visual::{ReportedIssues, ScanEntry, TreeScan};
use super::super::*;
use super::builder::TreeFrame;
use super::emit_test_support::*;
use crate::ecs::graphics::components::SurfaceGraphics;
use crate::ecs::layout::GlobalArrangement;
use crate::numerics::Aabb;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::system::SystemState;
use std::collections::HashSet;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{
    D2D1_INTERPOLATION_MODE, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
    D2D1_INTERPOLATION_MODE_LINEAR,
};
use windows_numerics::Matrix3x2;

/// 根の大きさ（DIP）。
const ROOT_DIP: f32 = 40.0;
const SCALES: [f32; 3] = [1.0, 1.25, 1.5];

/// 根の下に、背景（先に描く青）と、小数の位置 (10.3, 7.6) の節 `p`（中身 `content`）と
/// その子（小数の位置 (6.3, 5.2) の赤い 3×3 の円）を置く。返すのは (根, p)。
fn tree(world: &mut World, content: VisualDrawContent) -> (Entity, Entity) {
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, ROOT_DIP, ROOT_DIP)))
        .id();
    world.spawn((
        Visual::default(),
        at(2.0, 2.0, 30.0, 30.0),
        solid(rect(0.0, 0.0, 30.0, 30.0), BLUE),
        ChildOf(root),
    ));
    let p = world
        .spawn((
            Visual::default(),
            at(10.3, 7.6, 8.0, 8.0),
            content,
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Visual::default(),
        at(6.3, 5.2, 3.0, 3.0),
        solid_ellipse(3.0, 3.0, RED),
        ChildOf(p),
    ));
    (root, p)
}

fn nodes(entries: &[ScanEntry]) -> Vec<(Entity, usize)> {
    TreeScan {
        entries: entries.to_vec(),
    }
    .visible_nodes()
    .collect()
}

/// 全部を節にした絵と、`p` を上げた形（親の列＋自分の描画面を整数の位置へ重ねた絵）を返す。
fn full_and_promoted(world: &mut World, root: Entity, p: Entity, s: f32) -> (Pixels, Pixels) {
    let side = (ROOT_DIP * s).ceil() as u32;
    let scale = Matrix3x2::scale(s, s);
    let full = render_tree_to_pixels(world, root, side, side, scale);

    let scan = tree_scan(world, root);
    let idx = scan.entries.iter().position(|e| e.entity == p).unwrap();
    let entry = &scan.entries[idx];
    assert!(
        fractional_origin(entry, s),
        "拡大 {s} で候補の位置に端数がある"
    );
    let input = ShapeInput {
        scan: &scan,
        idx,
        scale: s,
        host_px: (side, side),
    };
    let o = check_shape(&input).expect("上げてよい形");
    let end = entry.end;

    let parent_nodes = nodes(&[&scan.entries[..idx], &scan.entries[end..]].concat());
    let root_frame = TreeFrame::Root {
        root,
        root_name: None,
    };
    let viewport = Aabb::new(0.0, 0.0, ROOT_DIP, ROOT_DIP);
    let parent_cmds = frame_commands(world, &root_frame, viewport, &parent_nodes);
    let parent = replay_to_pixels(&parent_cmds, side, side, scale);

    let seg_frame = TreeFrame::Segment {
        owner: p,
        owner_name: None,
        origin_px: o.px,
        scale: s,
    };
    let (w, h) = o.size_px;
    let seg_viewport = Aabb::new(0.0, 0.0, w as f32, h as f32);
    let seg_cmds = frame_commands(
        world,
        &seg_frame,
        seg_viewport,
        &nodes(&scan.entries[idx..end]),
    );
    // 部分の列は拡大込みの物理ピクセルなので根の変換は恒等。
    let own = replay_to_pixels(&seg_cmds, w, h, Matrix3x2::identity());
    assert_ne!(parent.bytes, full.bytes, "部分木が親の列から抜けている");

    let promoted = composite(side, side, &[(&parent, (0, 0), 1.0), (&own, o.px, 1.0)]);
    (full, promoted)
}

/// 1 チャネル ±1 を超えて違う画素（座標・全部を節にした値・上げた値）。
fn mismatches(a: &Pixels, b: &Pixels) -> Vec<(u32, u32, [u8; 4], [u8; 4])> {
    let h = a.bytes.len() as u32 / (a.width * 4);
    (0..h)
        .flat_map(|y| (0..a.width).map(move |x| (x, y)))
        .filter_map(|(x, y)| {
            let (pa, pb) = (a.at(x, y), b.at(x, y));
            let far = pa
                .iter()
                .zip(pb)
                .any(|(&u, v)| (u as i16 - v as i16).abs() > 1);
            far.then_some((x, y, pa, pb))
        })
        .collect()
}

fn assert_promotion_matches(name: &str, content: impl Fn() -> VisualDrawContent) {
    for s in SCALES {
        let mut world = World::new();
        let (root, p) = tree(&mut world, content());
        let (full, promoted) = full_and_promoted(&mut world, root, p, s);
        let bad = mismatches(&full, &promoted);
        assert!(
            bad.is_empty(),
            "{name} 拡大 {s}: ±1 を超える画素 {} 個（先頭 {:?}）",
            bad.len(),
            &bad[..bad.len().min(4)]
        );
    }
}

fn bitmap(mode: D2D1_INTERPOLATION_MODE, side: u32) -> impl Fn() -> VisualDrawContent {
    move || checker(RED, mode, side)
}

#[test]
fn fractional_rect_subtree_matches_when_promoted() {
    assert_promotion_matches("矩形", || solid(rect(0.0, 0.0, 8.0, 8.0), RED));
}

#[test]
fn fractional_bleeding_ellipse_subtree_matches_when_promoted() {
    assert_promotion_matches("楕円", || solid_ellipse(8.0, 6.0, RED));
}

#[test]
fn fractional_bitmap_interpolation_subtree_matches_when_promoted() {
    assert_promotion_matches(
        "ビットマップ（線形）",
        bitmap(D2D1_INTERPOLATION_MODE_LINEAR, 8),
    );
}

#[test]
fn fractional_downscaled_cubic_bitmap_subtree_matches_when_promoted() {
    // 縮める HIGH_QUALITY_CUBIC（diff.rs の `downscaling_hqc`・描く範囲で答えが変わる件）。
    assert_promotion_matches(
        "ビットマップ（縮める CUBIC）",
        bitmap(D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, 32),
    );
}

#[test]
fn composite_places_surface_at_integer_position_with_opacity() {
    let red = draw_to_pixels(2, 2, Matrix3x2::identity(), |dc| unsafe {
        dc.Clear(Some(&D2D1_COLOR_F {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }))
    });
    let out = composite(4, 4, &[(&red, (1, 2), 0.5)]);
    pixel_near(&out, CLEAR, 0, 2);
    pixel_near(&out, [0, 0, 128, 128], 1, 2);
    pixel_near(&out, [0, 0, 128, 128], 2, 3);
    pixel_near(&out, CLEAR, 3, 3);
    pixel_near(&out, CLEAR, 1, 1);
}

/// 本番の合成の道で列を描画面の大きさ `size` の入れ物へ全部描き、読み戻す。根の変換は
/// `replay_transform`（部分の列 `device_px` なら恒等・ほかは拡大 `s`）が選ぶ。
fn compose_px(commands: Vec<DrawCommand>, device_px: bool, s: f32, size: (u32, u32)) -> Pixels {
    let global = GlobalArrangement {
        transform: Matrix3x2::scale(s, s),
        bounds: rect(0.0, 0.0, size.0 as f32, size.1 as f32).into(),
    };
    let tree = DrawTree {
        commands,
        unit_rect: None,
        device_px,
    };
    let dc = warp_dc();
    let mut slot = Some(SurfaceComposite::new(&dc, size).expect("入れ物"));
    let root_transform = replay_transform(&global, Some(&tree));
    compose_planned(
        &mut slot,
        &dc,
        Some(FullReason::First),
        root_transform,
        None,
        &tree.commands,
        size,
    )
    .expect("合成");
    let bytes = slot.unwrap().read_all(warp_device()).expect("読み戻し");
    Pixels {
        width: size.0,
        bytes,
    }
}

/// 回の列: 0〜1.5 秒は 8 Hz で `p` を更新し、その後は落ち着かせる。各回、1 回の判断を回して画面を組み、
/// 全部を節にした絵と ±1 で揃うことを確かめる。3 部品の出入りと描いた印は係の順を写す
/// （判断の回の後に描画面の部品を入れ・外し、用意中の部分を描いた回に印を立てる）。
fn assert_frames_match(s: f32) {
    let mut world = World::new();
    let (root, p) = tree(&mut world, solid_ellipse(8.0, 6.0, RED));
    let side = (ROOT_DIP * s).ceil() as u32;
    let viewport = Aabb::new(0.0, 0.0, ROOT_DIP, ROOT_DIP);
    let root_frame = TreeFrame::Root {
        root,
        root_name: None,
    };
    let full_scan = tree_scan(&mut world, root);
    let all = frame_commands(
        &mut world,
        &root_frame,
        viewport,
        &nodes(&full_scan.entries),
    );
    let full = compose_px(all, false, s, (side, side));

    // WUC の資源は描画面より長く持つ。
    let mut resources = Vec::new();
    let mut state: SystemState<(Commands, Query<PromotionData>)> = SystemState::new(&mut world);
    let mut reported = ReportedIssues::default();
    let mut phases = Vec::new();
    for k in 0..80 {
        let t = k as f64 * 0.125;
        let mut scan = tree_scan(&mut world, root);
        for e in &mut scan.entries {
            e.updated = e.entity == p && t < 1.5;
        }
        let ctx = RoundContext {
            now: Some(t),
            device_ready: true,
            scale: s,
            host_px: (side, side),
        };
        let (mut commands, mut states) = state.get_mut(&mut world).unwrap();
        let plan = run_round(
            &ctx,
            &scan,
            &mut states,
            &mut commands,
            &mut HashSet::new(),
            &mut reported,
        );
        reported.end_round();
        state.apply(&mut world);
        let phase = world.get::<AutoPromotion>(p).unwrap().rule.phase();

        let host = frame_commands(&mut world, &root_frame, viewport, &plan.host_visited);
        let parent = compose_px(host, false, s, (side, side));
        let mut screen = vec![(&parent, (0, 0), 1.0)];
        let mut owns = Vec::new();
        for seg in &plan.segments {
            let (w, h) = seg.origin.size_px;
            let frame = TreeFrame::Segment {
                owner: seg.owner,
                owner_name: None,
                origin_px: seg.origin.px,
                scale: s,
            };
            let vp = Aabb::new(0.0, 0.0, w as f32, h as f32);
            let cmds = frame_commands(&mut world, &frame, vp, &seg.visited);
            owns.push((compose_px(cmds, true, s, (w, h)), seg.origin.px));
        }
        if phase == Phase::Active {
            assert_ne!(
                parent.bytes, full.bytes,
                "回 {k}: 上げた部分木が親の列から抜けている"
            );
            screen.extend(owns.iter().map(|(px, at)| (px, *at, 1.0)));
        } else {
            // 用意中の描画面は WUC の木に繋がない（画面に出ない）。
            assert_eq!(parent.bytes, full.bytes, "回 {k}: 親の列に部分木が在る");
        }
        let got = composite(side, side, &screen);
        let bad = mismatches(&full, &got);
        assert!(
            bad.is_empty(),
            "拡大 {s} 回 {k}（{phase:?}）: ±1 を超える画素 {} 個（先頭 {:?}）",
            bad.len(),
            &bad[..bad.len().min(4)]
        );

        // 係の順の写し: 相が Node でない節は次の回に 3 部品を持ち、Node へ戻れば外れる。
        match (phase, world.get::<SurfaceGraphics>(p).is_some()) {
            (Phase::Node, true) => {
                world.entity_mut(p).remove::<SurfaceGraphics>();
            }
            (Phase::Preparing | Phase::Active, false) => {
                let (resource, surface) = ready_surface();
                resources.push(resource);
                world.entity_mut(p).insert(surface);
            }
            _ => {}
        }
        if phase == Phase::Preparing && !plan.segments.is_empty() {
            world
                .get_mut::<AutoPromotion>(p)
                .unwrap()
                .bypass_change_detection()
                .built = true;
        }
        if phases.last() != Some(&phase) {
            phases.push(phase);
        }
        if phases.len() == 4 {
            break;
        }
    }
    assert_eq!(
        phases,
        [Phase::Node, Phase::Preparing, Phase::Active, Phase::Node],
        "拡大 {s}: 上げて戻すまでを通った"
    );
}

#[test]
fn promote_and_demote_frames_match_all_nodes_at_scale_1_and_1_25() {
    for s in [1.0, 1.25] {
        assert_frames_match(s);
    }
}
