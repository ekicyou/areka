//! 自動で上げた持ち主の画素の一致（WARP・要件 2.7, 3.2, 8.3・研究 R-e）。
//!
//! 節の部分木を、親の描画面の中で描いた絵と、部分の列で原点を引いて自分の描画面に描き
//! WUC の重ねの写しで整数の位置へ重ねた絵とで比べる。
//!
//! R-e の答え: 小数の位置（10.3, 7.6）の候補も拡大 1.0・1.25・1.5 で ±1 に揃う。ただし原点は物理ピクセルで引く
//! （`TreeFrame::Segment` が拡大を掛けてから整数を引く）。DIP で `px / 拡大` を引いた形は、割り切れない原点で
//! 位置が 1e-7 ずれ、被覆の標本にちょうど乗る端数（12.875）で楕円の縁が 1 チャネル 16 ほど違った。

use super::super::promotion_shape::{ShapeInput, check_shape, fractional_origin};
use super::super::world_visual::{ScanEntry, TreeScan};
use super::super::*;
use super::builder::TreeFrame;
use super::emit_test_support::*;
use crate::numerics::Aabb;
use bevy_ecs::hierarchy::ChildOf;
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
    // 縮める HIGH_QUALITY_CUBIC（diff.rs の R1・描く範囲で答えが変わる件）。
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
