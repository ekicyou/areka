//! 実 GPU で、画面の描画面の下の節が、同じ状態を新しく全部写した節と同じ絵を描くこと。
//!
//! 動く絵の見本でコマ替えを続け、各回に描画面の下の木（原寸のまとめの節と計画の節）を wintf の
//! 画面の外の口で k＝1 のまま描いて読み、同じ面・同じコマを新しい presenter で 1 回だけ出した木の絵と
//! 1 バイト単位で比べる。2 回目からのコマ替えが節を作り直さず中身だけを差し替えた（範囲だけ）ことは
//! 作り直した項の数で確かめる（空の道で緑にしない）。当たりが読む画面の入れ物の α も、画面の外の
//! 描き手（WARP）の見本の α と 0.5 の同じ側に在ることを確かめる。

use super::*;

use windows_numerics::Matrix3x2;
use wintf::ecs::{AlphaRead, render_tree_offscreen, surface_alpha_at};

use super::test_support::{
    bake_world, elem, gradient_with_alpha, make_world_with_gpu, mount_entities, native_golden_with,
    paint_screen, pattern_overlay_at, shell_of, show_with_ok, spawn_window_with_dpi, surface,
};

const TARGET: TargetId = TargetId(0);
const BASE_ID: u32 = 1000;
const CELL_ID: u32 = 3000;
const BASE_SIZE: (u32, u32) = (16, 12);
const CELL_SIZE: (u32, u32) = (4, 3);
const CELL_Y: i64 = 2;

/// 下の面（左半分は不透明・右半分は透明と半透明の混ざり）とコマの面（四隅が透明・縁が半透明）。
fn build_assets() -> (EmoWorld, AtlasTable) {
    let (bw, bh) = BASE_SIZE;
    let (cw, ch) = CELL_SIZE;
    let base = gradient_with_alpha(bw, bh, 11, |x, y| match (x < bw / 2, (x + y) % 3) {
        (true, _) => 255,
        (false, 0) => 0,
        (false, 1) => 96,
        _ => 255,
    });
    let cell = gradient_with_alpha(cw, ch, 0x80, |x, y| {
        let edge_x = x == 0 || x == cw - 1;
        let edge_y = y == 0 || y == ch - 1;
        match (edge_x, edge_y) {
            (true, true) => 0,
            (false, false) => 255,
            _ => 160,
        }
    });
    bake_world(
        shell_of(vec![
            surface(BASE_ID, vec![elem("p.png", 0, 0)]),
            surface(CELL_ID, vec![elem("q.png", 0, 0)]),
        ]),
        vec![("p.png", bw, bh, base), ("q.png", cw, ch, cell)],
    )
}

fn show(presenter: &mut EmoPresenter, world: &mut World, x: i64) {
    show_with_ok(
        presenter,
        world,
        TARGET,
        BASE_ID,
        BindSet::default(),
        pattern_overlay_at(2000, CELL_ID, x, CELL_Y),
    );
}

/// 描画面の下の木を k＝1 の原寸で描いて読む。
fn render_screen(world: &mut World, presenter: &EmoPresenter) -> Vec<u8> {
    let (surface_entity, _) = mount_entities(presenter, TARGET);
    let device = world
        .resource::<GraphicsCore>()
        .d2d_device()
        .cloned()
        .expect("GraphicsCore は装置を持つ");
    render_tree_offscreen(
        &device,
        world,
        surface_entity,
        BASE_SIZE,
        Matrix3x2::identity(),
    )
    .expect("描画面の下の木を画面の外へ描けない")
    .bytes
}

fn rebuilds(presenter: &EmoPresenter) -> u64 {
    presenter.targets[&TARGET].budget.cumulative().node_rebuilds
}

/// コマ替えのたびに、描画面の下の木の絵が同じ状態を新しく全部写した木の絵と同じで、2 回目からは
/// 節を作り直さない。絵は画面の外の描き手（WARP）の見本と色の各成分の差 1 以内。
#[test]
fn frame_changes_update_the_nodes_in_place_and_match_a_fresh_full_tree_on_real_gpu() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = build_assets();
    presenter.attach_target(TARGET, window, emo_world, atlas, 96);
    let (emo_world, atlas) = build_assets();

    // 不透明の側から透明の側へ進み、また戻る（どの回も前の回と違うコマ＝控えに外れる）。
    let xs = [0i64, 3, 6, 9, 12, 10, 7, 4, 1];
    let mut first_rebuilds = None;
    for &x in &xs {
        show(&mut presenter, &mut world, x);
        // 画面の外の口は同じ World の節の世界の行列を書き直すので、画面へ先に描く。
        paint_screen(&mut world);
        let (surface_entity, _) = mount_entities(&presenter, TARGET);
        let (w, h) = BASE_SIZE;
        let screen_alpha: Vec<u8> = (0..h)
            .flat_map(|py| (0..w).map(move |px| (px, py)))
            .map(
                |(px, py)| match surface_alpha_at(&world, surface_entity, px, py) {
                    AlphaRead::Alpha(a) => a,
                    other => panic!("コマ x={x} の画面の入れ物 ({px},{py}) が読めない: {other:?}"),
                },
            )
            .collect();
        let shown = render_screen(&mut world, &presenter);
        match first_rebuilds {
            None => first_rebuilds = Some(rebuilds(&presenter)),
            Some(n) => assert_eq!(
                rebuilds(&presenter),
                n,
                "コマ x={x} の替わりで節を作り直した（中身だけの差し替えになっていない）"
            ),
        }

        let mut fresh = EmoPresenter::new();
        let fresh_window = spawn_window_with_dpi(&mut world, 96);
        let (w, a) = build_assets();
        fresh.attach_target(TARGET, fresh_window, w, a, 96);
        show(&mut fresh, &mut world, x);
        let full = render_screen(&mut world, &fresh);
        assert_eq!(
            shown, full,
            "コマ x={x} の木の絵が新しく全部写した木の絵と違う"
        );

        let (golden, extent) = native_golden_with(
            &emo_world,
            &atlas,
            BASE_ID,
            &BindSet::default(),
            &pattern_overlay_at(2000, CELL_ID, x, CELL_Y),
        );
        assert_eq!(extent, BASE_SIZE, "コマ x={x} の外形");
        let alphas: Vec<u8> = golden.iter().skip(3).step_by(4).copied().collect();
        assert!(
            alphas.contains(&0) && alphas.iter().any(|&a| a != 0 && a != 255),
            "見本の前提: α=0 と中間の α が同居する（x={x}）"
        );
        let worst = shown
            .iter()
            .zip(&golden)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        assert!(worst <= 1, "コマ x={x} の絵が WARP の見本と {worst} 違う");

        for (i, a) in screen_alpha.iter().enumerate() {
            assert_eq!(
                *a >= 128,
                golden[i * 4 + 3] >= 128,
                "コマ x={x} の画面の入れ物の画素 {i} の α {a} が WARP の見本と 0.5 の反対側"
            );
        }
    }
    assert!(
        first_rebuilds.is_some_and(|n| n > 0),
        "最初の表示で節を作っていない"
    );
}
