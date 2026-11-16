//! `build` の部分の列の形（`TreeFrame::Segment`）の純粋なテスト（要件 2.7, 3.2）。
//! 根の中身を出さないこと・自分の描画面の物理ピクセルへ写した（拡大を掛けて原点を引いた）行列と外接矩形・
//! 整数だけ動いた原点で件の鍵が変わらないこと。

use super::super::draw_content_test_support::solid_rect_content;
use super::super::*;
use super::builder::*;
use super::builder_tests::*;
use crate::numerics::*;
use bevy_ecs::system::SystemState;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

/// 中身つきの根の下に、位置 (x, y) の持ち主 `p`（中身つき・`visual`）とその子 `c`（(1, 1)）を置く。
/// 返すのは (p, c, p から始まる部分の列)。
fn segment_tree(
    world: &mut World,
    x: f32,
    y: f32,
    visual: Visual,
) -> (Entity, Entity, Vec<(Entity, usize)>) {
    let rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 4.0,
        bottom: 4.0,
    };
    let root = world
        .spawn((Visual::default(), solid_rect_content(rect)))
        .id();
    let _before = leaf(world, root, 50.0, 50.0);
    let p = leaf(world, root, x, y);
    world.entity_mut(p).insert(visual);
    let c = leaf(world, p, 1.0, 1.0);
    let visited = fill(world, root);
    let head = visited.iter().position(|(e, _)| *e == p).unwrap();
    let d0 = visited[head].1;
    let seg: Vec<_> = std::iter::once(visited[head])
        .chain(
            visited[head + 1..]
                .iter()
                .copied()
                .take_while(|(_, d)| *d > d0),
        )
        .collect();
    (p, c, seg)
}

fn build_segment(
    world: &mut World,
    owner: Entity,
    origin_px: (i32, i32),
    scale: f32,
    viewport: Aabb,
    seg: &[(Entity, usize)],
) -> Vec<DrawCommand> {
    let mut state: SystemState<Query<NodeData>> = SystemState::new(world);
    let nodes = state.get(world).unwrap();
    let mut out = Vec::new();
    build(
        &mut out,
        &factory(),
        &unit_rect(),
        viewport,
        &nodes,
        &TreeFrame::Segment {
            owner,
            owner_name: None,
            origin_px,
            scale,
        },
        seg,
    );
    out
}

#[test]
fn segment_emits_no_root_content_and_subtracts_origin() {
    let mut world = World::new();
    let (p, c, seg) = segment_tree(&mut world, 10.5, 3.25, Visual::default());
    let out = build_segment(&mut world, p, (10, 3), 1.0, INFINITE, &seg);
    // 持ち主自身は恒等の先頭の 1 件ではなく、原点を引いた世界の位置で 1 度だけ出る。
    assert_eq!(
        shape(&out),
        vec![draw(p, 0.5, 0.25, 4.5, 4.25), draw(c, 1.5, 1.25, 5.5, 5.25)]
    );
    let DrawCommand::Draw(d) = &out[0] else {
        panic!("先頭は描く件: {out:?}")
    };
    assert_eq!((d.world_mat.M31, d.world_mat.M32), (0.5, 0.25));
}

#[test]
fn segment_head_emits_opacity_and_clip_with_origin_subtracted() {
    let mut world = World::new();
    let visual = Visual {
        opacity: 0.5,
        ..clipped()
    };
    let (p, c, seg) = segment_tree(&mut world, 10.5, 3.25, visual);
    let out = build_segment(&mut world, p, (10, 3), 1.0, INFINITE, &seg);
    assert_eq!(
        shape(&out),
        vec![
            format!("push_opacity 0.5 {:?}", Aabb::new(0.5, 0.25, 5.5, 5.25)),
            push_rect(0.5, 0.25, 4.5, 4.25),
            draw(p, 0.5, 0.25, 4.5, 4.25),
            draw(c, 1.5, 1.25, 5.5, 5.25),
            "pop_rect".to_string(),
            "pop_opacity".to_string(),
        ]
    );
}

#[test]
fn segment_viewport_is_in_owner_surface_space() {
    let mut world = World::new();
    let (p, _, seg) = segment_tree(&mut world, 10.5, 3.25, Visual::default());
    // 自分の描画面の矩形（原点から）の外へ出る子は間引かれる。
    let out = build_segment(
        &mut world,
        p,
        (10, 3),
        1.0,
        Aabb::new(0.0, 0.0, 1.0, 1.0),
        &seg,
    );
    assert_eq!(shape(&out), vec![draw(p, 0.5, 0.25, 4.5, 4.25)]);
}

#[test]
fn integer_origin_move_keeps_item_keys() {
    let keys = |x: f32, y: f32, origin: (i32, i32)| {
        let mut world = World::new();
        let (p, _, seg) = segment_tree(&mut world, x, y, Visual::default());
        build_segment(&mut world, p, origin, 1.0, INFINITE, &seg)
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Draw(d) => Some((d.hash, d.world_mat)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    // 実体は各ワールドで同じ番号になる（同じ順に作るので）。
    let a = keys(10.5, 3.25, (10, 3));
    let b = keys(12.5, 5.25, (12, 5));
    assert_eq!(a, b);
    // 端数が変われば鍵は変わる。
    let f = keys(10.75, 3.25, (10, 3));
    assert_ne!(a[0].0, f[0].0);
}

#[test]
fn segment_maps_to_surface_pixels_subtracting_integer_origin_exactly() {
    let mut world = World::new();
    let (p, c, seg) = segment_tree(&mut world, 10.3, 7.6, Visual::default());
    let out = build_segment(&mut world, p, (9, 6), 1.25, INFINITE, &seg);
    let mats: Vec<_> = out
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Draw(d) => Some((d.entity, d.world_mat)),
            _ => None,
        })
        .collect();
    // 拡大を掛けた位置（親の描画面の中で描くときの装置の座標）から整数をそのまま引いた値。
    let (px, py) = (10.3f32 * 1.25 - 9.0, 7.6f32 * 1.25 - 6.0);
    assert_eq!((px, py), (3.875, 3.5));
    assert_eq!(mats[0].0, p);
    let m = mats[0].1;
    assert_eq!((m.M11, m.M22, m.M31, m.M32), (1.25, 1.25, px, py));
    assert_eq!(mats[1].0, c);
    assert_eq!(
        (mats[1].1.M31, mats[1].1.M32),
        ((10.3f32 + 1.0) * 1.25 - 9.0, (7.6f32 + 1.0) * 1.25 - 6.0)
    );
}
