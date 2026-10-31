//! 実 GPU で、矩形の重ね直しの絵がいちからの合成と同じこと（`animated-surface-log-flood`
//! 要件 3.2・3.7・8.4）。
//!
//! 動く絵の見本でコマ替えを続け、各回に窓の面へ載った表示の記録（閉じたコマンドリスト）を
//! `display_gpu_tests.rs` の [`replay_and_read_back`] で実 GPU に再生して読み戻し、同じ面・同じコマを
//! 新しい合成器でいちから合成したバイトと 1 バイト単位で比べる。当たりのマスクは、いちから合成した
//! 絵から作ったマスクと比べる。
//!
//! 見本は透明・半透明・不透明が同居し、動く絵は下の面の透明な側へも重なる。ゆえにコマごとにマスクが
//! 替わり、premultiplied の重ね方の違いも画素に出る。2 回目からの外れが実際に矩形（`partial`）で
//! 合成されたことは、重ね直しの範囲の行の件数で確かめる（空の道で緑にしない）。

use super::*;

use std::path::Path;
use std::time::Duration;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use wintf::ecs::GraphicsCommandList;
use wintf::ecs::widget::bitmap_source::AlphaMask;

use crate::display::gpu_tests::replay_and_read_back;

use super::test_support::{
    capture, elem, make_world_with_gpu, mount_entities, native_golden_with, pattern_overlay_at,
    shell_of, spawn_window_with_dpi, surface,
};

const RANGE_LINE_MESSAGE: &str = "apply(ShowSurface): 重ね直しの範囲";

const TARGET: TargetId = TargetId(0);
const BASE_ID: u32 = 1000;
const CELL_ID: u32 = 3000;
const BASE_SIZE: (u32, u32) = (16, 12);
const CELL_SIZE: (u32, u32) = (4, 3);
const CELL_Y: i64 = 2;

/// premultiplied BGRA（成分は straight 色に α を掛けるので α 以下）。
fn image((w, h): (u32, u32), salt: u8, alpha: impl Fn(u32, u32) -> u8) -> Vec<u8> {
    let mut img = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let a = alpha(x, y);
            let pm = |v: u8| ((v as u16 * a as u16) / 255) as u8;
            img.push(pm((x as u8).wrapping_mul(37).wrapping_add(salt)));
            img.push(pm((y as u8).wrapping_mul(53).wrapping_add(salt)));
            img.push(pm(((x + y) as u8).wrapping_mul(71).wrapping_add(salt)));
            img.push(a);
        }
    }
    img
}

/// 下の面（左半分は不透明・右半分は透明と半透明の混ざり）とコマの面（四隅が透明・縁が半透明）。
fn build_assets() -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![
        surface(BASE_ID, vec![elem("p.png", 0, 0)]),
        surface(CELL_ID, vec![elem("q.png", 0, 0)]),
    ];
    let mut dec = MemoryDecoder::new();
    let (bw, bh) = BASE_SIZE;
    let (cw, ch) = CELL_SIZE;
    dec.insert(
        base.join("p.png"),
        bw,
        bh,
        bw * 4,
        image(BASE_SIZE, 11, |x, y| match (x < bw / 2, (x + y) % 3) {
            (true, _) => 255,
            (false, 0) => 0,
            (false, 1) => 96,
            _ => 255,
        }),
        true,
    );
    dec.insert(
        base.join("q.png"),
        cw,
        ch,
        cw * 4,
        image(CELL_SIZE, 0x80, |x, y| {
            let edge_x = x == 0 || x == cw - 1;
            let edge_y = y == 0 || y == ch - 1;
            match (edge_x, edge_y) {
                (true, true) => 0,
                (false, false) => 255,
                _ => 160,
            }
        }),
        true,
    );
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake: {:?}", baked.errors);
    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

fn show(presenter: &mut EmoPresenter, world: &mut World, x: i64) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TARGET,
            surface_id: BASE_ID,
            binds: BindSet::default(),
            pattern: pattern_overlay_at(2000, CELL_ID, x, CELL_Y),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "コマ x={x} の ShowSurface が Ok でない"
    );
}

/// 要件 3.2・3.7・8.4: コマ替えのたびに、窓の面へ載った表示の記録を実 GPU で読み戻したバイトが
/// いちからの合成と同じで、当たりのマスクもいちから合成した絵から作ったマスクと同じ。
#[test]
fn partial_recompose_display_matches_from_scratch_on_real_gpu() {
    let mut world = make_world_with_gpu();
    // 宛先矩形は論理 px ゆえ、96 でだけ 1 論理 px = 1 画素（記録と再生で同じ DC を使う）。
    let dc = world
        .resource::<GraphicsCore>()
        .device_context()
        .expect("GraphicsCore::device_context が None")
        .clone();
    // SAFETY: Windows API 境界。このテストのスレッドだけが使う DC の設定変更。
    unsafe { dc.SetDpi(96.0, 96.0) };

    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = build_assets();
    presenter
        .attach_target(&mut world, TARGET, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    let (emo_world, atlas) = build_assets();

    // 不透明の側から透明の側へ進み、また戻る（どの回も前の回と違うコマ＝控えに外れる）。
    let xs = [0i64, 3, 6, 9, 12, 10, 7, 4, 1];
    let (masks, events) = capture(|| {
        let mut masks = Vec::new();
        for &x in &xs {
            show(&mut presenter, &mut world, x);
            let (surface_entity, _) = mount_entities(&presenter, TARGET);
            let list = world
                .get::<GraphicsCommandList>(surface_entity)
                .expect("表示成立後は面に表示の記録が載る")
                .clone();
            let (w, h) = BASE_SIZE;
            let shown = replay_and_read_back(&dc, &list, w, h);

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
            assert_eq!(shown, golden, "コマ x={x} の読み戻しがいちからの合成と違う");

            let want = AlphaMask::from_pbgra32(&golden, w, h, w * 4);
            let supplied = world
                .get::<AlphaMaskResource>(surface_entity)
                .expect("表示成立後は面に AlphaMaskResource が載る")
                .mask()
                .expect("表示成立後はマスクが供給済み");
            assert_eq!(
                *supplied, want,
                "コマ x={x} のマスクがいちから合成した絵のマスクと違う"
            );
            masks.push(want);
        }
        masks
    });

    // 空の道で緑にしない: 2 回目からの外れは矩形で合成され、マスクはコマで替わる。
    let partial = events
        .iter()
        .filter(|e| {
            e.message() == RANGE_LINE_MESSAGE && e.field_str("recompose") == Some("partial")
        })
        .count();
    assert_eq!(partial, xs.len() - 1, "2 回目からはどれも矩形の重ね直し");
    assert!(
        masks.windows(2).any(|m| m[0] != m[1]),
        "コマでマスクが替わらない（見本が透明の側へ重なっていない）"
    );
}
