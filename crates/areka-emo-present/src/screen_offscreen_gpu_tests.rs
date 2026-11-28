//! 実 GPU で、画面の入れ物（描画面の合成し終えた絵）が画面の外の答えと一致すること。
//!
//! 入れ物は wintf の `SurfaceReadback::read_all` で全体を読む。比べる相手は 3 つ:
//! - k＝1: 最後に出した絵の書き出し（`last_shown`）とバイト一致。
//! - k＝1.25: 書き出しの原寸の答えを、同じ装置で 1 枚の絵として k 倍・`LINEAR`・`SOURCE_OVER` で
//!   透明の的へ描いた絵とバイト一致（原寸のまとめの節が中を原寸で描いてから拡大していること）。
//!   隣り合う 2 つの element の境（物理で画素の格子に乗らない）に α の落ち込みが無い。
//! - コマ替え: 同じ面・同じコマを新しい presenter で 1 回だけ出した入れ物とバイト一致。

use super::*;

use areka_parsers::shell::Element;
use bevy_ecs::system::SystemState;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COMPOSITE_MODE_SOURCE_OVER;
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_OPTIONS_TARGET, D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
    D2D1_INTERPOLATION_MODE_LINEAR,
};
use windows_numerics::Matrix3x2;
use wintf::ecs::{AlphaRead, SurfaceImage, SurfaceReadback, surface_alpha_at};

use crate::display::gpu_tests::{make_bitmap, read_target};

use super::test_support::{
    bake_world, elem, gradient, gradient_with_alpha, make_world_with_gpu, mount_entities,
    native_golden_with, paint_screen, pattern_overlay_at, shell_of, show_ok, show_with_ok,
    spawn_window_with_dpi, surface,
};

const TARGET: TargetId = TargetId(0);
const BASE_ID: u32 = 1000;
const CELL_ID: u32 = 3000;
const SEAM_ID: u32 = 4000;
const BASE_SIZE: (u32, u32) = (16, 12);
const CELL_SIZE: (u32, u32) = (4, 3);
const CELL_Y: i64 = 2;
/// 左の element の幅＝右の element の左端。k＝1.25 で物理 8.75 に当たる（格子に乗らない）。
const SEAM_X: u32 = 7;

/// 下の面（左半分は不透明・右半分は透明と半透明の混ざり）とコマの面（四隅が透明・縁が半透明）、
/// 隣り合う 2 つの element の面（外形は element の位置を足さないので、全透明の原寸の絵を最初に敷く。
/// 左は不透明・右は左寄りが不透明で右寄りに半透明と透明が混ざる）。
fn build_assets() -> (EmoWorld, AtlasTable) {
    let (bw, bh) = BASE_SIZE;
    let (cw, ch) = CELL_SIZE;
    let mixed = |opaque_below: u32| {
        move |x: u32, y: u32| match (x < opaque_below, (x + y) % 3) {
            (true, _) => 255,
            (false, 0) => 0,
            (false, 1) => 96,
            _ => 255,
        }
    };
    let base = gradient_with_alpha(bw, bh, 11, mixed(bw / 2));
    let cell = gradient_with_alpha(cw, ch, 0x80, |x, y| {
        let edge_x = x == 0 || x == cw - 1;
        let edge_y = y == 0 || y == ch - 1;
        match (edge_x, edge_y) {
            (true, true) => 0,
            (false, false) => 255,
            _ => 160,
        }
    });
    let left = gradient(SEAM_X, bh, 0x30);
    let right_w = bw - SEAM_X;
    let right = gradient_with_alpha(right_w, bh, 0x60, mixed(4));
    bake_world(
        shell_of(vec![
            surface(BASE_ID, vec![elem("p.png", 0, 0)]),
            surface(CELL_ID, vec![elem("q.png", 0, 0)]),
            surface(
                SEAM_ID,
                vec![
                    elem("c.png", 0, 0),
                    Element {
                        layer: 1,
                        ..elem("l.png", 0, 0)
                    },
                    Element {
                        layer: 2,
                        ..elem("r.png", i64::from(SEAM_X), 0)
                    },
                ],
            ),
        ]),
        vec![
            ("p.png", bw, bh, base),
            ("q.png", cw, ch, cell),
            ("c.png", bw, bh, vec![0; (bw * bh * 4) as usize]),
            ("l.png", SEAM_X, bh, left),
            ("r.png", right_w, bh, right),
        ],
    )
}

fn attach(world: &mut World, presenter: &mut EmoPresenter, window_dpi: u16) {
    let window = spawn_window_with_dpi(world, window_dpi);
    let (emo_world, atlas) = build_assets();
    presenter.attach_target(TARGET, window, emo_world, atlas, 96);
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

/// 画面の入れ物の全体（物理の大きさと乗算済み BGRA）。
fn read_container(world: &mut World, presenter: &EmoPresenter) -> (u32, u32, Vec<u8>) {
    let (surface_entity, _) = mount_entities(presenter, TARGET);
    let mut state = SystemState::<SurfaceReadback>::new(world);
    match state
        .get(world)
        .expect("SurfaceReadback")
        .read_all(surface_entity)
    {
        SurfaceImage::Pixels {
            width,
            height,
            bytes,
        } => (width, height, bytes),
        SurfaceImage::Unreadable => panic!("画面の入れ物が読めない"),
    }
}

/// 最後に出した絵の書き出し（原寸）。
fn last_shown(presenter: &EmoPresenter) -> (u32, u32, Vec<u8>) {
    let (_, picture) = presenter.last_shown(TARGET).expect("表示済み");
    let p = picture.expect("書き出しが描ける");
    (p.width(), p.height(), p.bytes().to_vec())
}

/// 原寸の答えを画面と同じ装置で 1 枚の絵にし、透明の `size` の的へ `scale(k)`・`LINEAR`・`SOURCE_OVER`
/// で描いて読む（原寸のまとめの節が作業用の絵を親へ重ねるのと同じ描き方）。
fn scaled_native(world: &World, native: &(u32, u32, Vec<u8>), k: f32, size: (u32, u32)) -> Vec<u8> {
    let (nw, nh, bytes) = native;
    let device = world
        .resource::<GraphicsCore>()
        .d2d_device()
        .cloned()
        .expect("GraphicsCore は装置を持つ");
    // SAFETY: Windows API 境界。生きている装置から DC を作るだけ。
    let dc = unsafe { device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }
        .expect("DC を作れない");
    let source = make_bitmap(&dc, *nw, *nh, D2D1_BITMAP_OPTIONS_NONE);
    let target = make_bitmap(&dc, size.0, size.1, D2D1_BITMAP_OPTIONS_TARGET);
    // SAFETY: Windows API 境界。`bytes` は `nw*4` バイトの行が `nh` 行。`BeginDraw`／`EndDraw` はこの中で対になる。
    unsafe {
        source
            .CopyFromMemory(None, bytes.as_ptr().cast(), nw * 4)
            .expect("原寸の絵を上げられない");
        dc.SetTarget(&target);
        dc.BeginDraw();
        dc.Clear(None);
        dc.SetTransform(&Matrix3x2 {
            M11: k,
            M22: k,
            ..Matrix3x2::identity()
        });
        dc.DrawImage(
            &source,
            None,
            None,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
        );
        dc.EndDraw(None, None).expect("EndDraw");
    }
    read_target(&dc, &target, size.0, size.1)
}

fn rebuilds(presenter: &EmoPresenter) -> u64 {
    presenter.targets[&TARGET].budget.cumulative().node_rebuilds
}

/// k＝1 で、コマ替えのどの回も画面の入れ物の全体が最後に出した絵の書き出しとバイト一致する。
#[test]
fn container_equals_last_shown_export_at_k1_on_real_gpu() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut world, &mut presenter, 96);
    for x in [0i64, 6, 12] {
        show(&mut presenter, &mut world, x);
        paint_screen(&mut world);
        let container = read_container(&mut world, &presenter);
        assert_eq!(
            (container.0, container.1),
            BASE_SIZE,
            "コマ x={x} の入れ物の大きさ"
        );
        assert!(
            container
                .2
                .iter()
                .skip(3)
                .step_by(4)
                .any(|&a| a != 0 && a != 255),
            "前提: 入れ物に中間の α が在る（x={x}）"
        );
        assert_eq!(
            container,
            last_shown(&presenter),
            "コマ x={x} の入れ物が書き出しと違う"
        );
    }
}

/// k＝1.25 で、画面の入れ物が原寸の答えを k 倍した絵とバイト一致し、隣り合う element の境で α が落ちない。
#[test]
fn container_equals_scaled_native_answer_without_seams_at_k125_on_real_gpu() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut world, &mut presenter, 120);
    show_ok(&mut presenter, &mut world, TARGET, SEAM_ID);
    paint_screen(&mut world);

    let native = last_shown(&presenter);
    assert_eq!((native.0, native.1), BASE_SIZE, "書き出しは原寸");
    let (w, h, container) = read_container(&mut world, &presenter);
    assert_eq!((w, h), (20, 15), "入れ物は原寸 16×12 の 1.25 倍");
    assert_eq!(
        container,
        scaled_native(&world, &native, 1.25, (w, h)),
        "入れ物が原寸の答えを k 倍した絵と違う（まとめの中が原寸で描かれていない）"
    );

    // 境の前後の物理の列（7..=10）は両側の不透明な画素だけから補う。外の縁に掛かる最上・最下の行は除く。
    for y in 1..h - 1 {
        for x in 7..=10 {
            let a = container[((y * w + x) * 4 + 3) as usize];
            assert_eq!(a, 255, "境の近く ({x},{y}) の α が {a} に落ちた");
        }
    }
}

/// コマ替えのたびに、画面の入れ物が同じ状態を新しく全部描いた入れ物とバイト一致し、2 回目からは
/// 節を作り直さない（中身だけの差し替え）。入れ物は画面の外の描き手（WARP）の見本と色の各成分の差 1 以内。
#[test]
fn frame_changes_update_the_nodes_in_place_and_match_a_fresh_full_container_on_real_gpu() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut world, &mut presenter, 96);
    let (emo_world, atlas) = build_assets();

    // 不透明の側から透明の側へ進み、また戻る（どの回も前の回と違うコマ＝控えに外れる）。
    let xs = [0i64, 3, 6, 9, 12, 10, 7, 4, 1];
    let mut first_rebuilds = None;
    for &x in &xs {
        show(&mut presenter, &mut world, x);
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
        let (_, _, shown) = read_container(&mut world, &presenter);
        match first_rebuilds {
            None => first_rebuilds = Some(rebuilds(&presenter)),
            Some(n) => assert_eq!(
                rebuilds(&presenter),
                n,
                "コマ x={x} の替わりで節を作り直した（中身だけの差し替えになっていない）"
            ),
        }

        let mut fresh = EmoPresenter::new();
        attach(&mut world, &mut fresh, 96);
        show(&mut fresh, &mut world, x);
        paint_screen(&mut world);
        let (_, _, full) = read_container(&mut world, &fresh);
        assert_eq!(
            shown, full,
            "コマ x={x} の入れ物が新しく全部描いた入れ物と違う"
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
        assert!(
            worst <= 1,
            "コマ x={x} の入れ物が WARP の見本と {worst} 違う"
        );

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
