//! emo2 の `overlay` だけの面を WARP で描いた画素の見本と、CPU の合成との差の測り。

use std::path::PathBuf;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, Point, SetId, Size, SurfaceSet, UseSelfAlpha,
    WicDecoderArm, bake,
};
use areka_emo_compose::{ComposeMethod, PatternFrame};
use areka_parsers::shell::{AppendTarget, Shell};

use super::*;
use crate::shell_target::load_shell_target;
use crate::shell_target::test_support::{EMO2, with_com_initialized};

fn emo2_master() -> PathBuf {
    EMO2.folder().join("shell").join("master")
}

fn emo2_shell() -> Shell {
    let path = emo2_master().join("surfaces.txt");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("emo2 の surfaces.txt: {}: {e}", path.display()));
    areka_parsers::shell::parse(&text)
}

/// `dec` の絵だけで emo2 の全部の面を焼く（無い絵は焼きの誤りに残るだけ）。
fn bake_emo2(shell: &Shell, dec: &MemoryDecoder) -> (EmoWorld, AtlasTable) {
    let base = emo2_master();
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: &base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let table = bake(&[set], dec, PackConfig::default()).table;
    let mut world = EmoWorld::build(shell);
    world.bind_atlas(&table, SetId(0));
    (world, table)
}

fn warp_render(
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> ComposedSurface {
    OffscreenRenderer::warp()
        .expect("WARP の描き手")
        .render(world, atlas, surface, binds, pattern)
        .expect("WARP で描く")
}

/// 色の各成分の差の最大と、差のある画素の数。
fn diff(got: &ComposedSurface, want: &ComposedSurface) -> (u8, usize) {
    assert_eq!(
        (got.width(), got.height(), got.stride()),
        (want.width(), want.height(), want.stride()),
        "原寸"
    );
    let pixels = |s: &ComposedSurface| s.bytes().as_chunks::<4>().0.to_vec();
    pixels(got)
        .iter()
        .zip(pixels(want))
        .fold((0, 0), |(max, n), (g, w)| {
            let d = (0..4).map(|c| g[c].abs_diff(w[c])).max().unwrap_or(0);
            (max.max(d), n + usize::from(d > 0))
        })
}

fn assert_bytes_within_one(got: &ComposedSurface, want: &[u8]) {
    for (i, (g, w)) in got.bytes().iter().zip(want).enumerate() {
        assert!(g.abs_diff(*w) <= 1, "{i} バイト目: WARP {g}・見本 {w}");
    }
    assert_eq!(got.bytes().len(), want.len(), "バイトの長さ");
}

fn px(s: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    let i = (y * s.stride() + x * 4) as usize;
    s.bytes()[i..i + 4].try_into().expect("4 バイト")
}

fn alpha_count(s: &ComposedSurface) -> usize {
    s.bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] > 0)
        .count()
}

/// 本物の emo2 の絵で、surface0・surface1000 の着せ替え・まばたきのコマを CPU と WARP で描き、
/// 差の最大を出す（`--nocapture` で読む）。`overlay` だけなので各成分の差は 1 以内。
#[test]
fn emo2_overlay_only_warp_differs_from_the_cpu_compose_by_at_most_one() {
    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("WIC");
        let target = load_shell_target(&emo2_master(), &dec).expect("emo2 のシェル");
        assert!(
            target.bake_errors().is_empty(),
            "{:?}",
            target.bake_errors()
        );
        let world = target.build_world();
        let atlas = target.atlas();

        let dress = [1100, 1207, 1302, 1500, 1800];
        let blinking = BindSet::from_ids(dress.into_iter().chain([1400]));
        let blink = |surface_id| {
            let mut p = PatternState::default();
            p.set(
                1400,
                PatternFrame {
                    surface_id,
                    method: ComposeMethod::Overlay,
                    x: 0,
                    y: 0,
                },
            );
            p
        };
        let cases = [
            ("surface0", 0, BindSet::default(), PatternState::default()),
            (
                "surface1000 着せ替え（既定）",
                1000,
                BindSet::from_ids(dress),
                PatternState::default(),
            ),
            (
                "surface1000 着せ替え（1101・1200・1305・1501）",
                1000,
                BindSet::from_ids([1101, 1200, 1305, 1501]),
                PatternState::default(),
            ),
            ("まばたき 1412", 1000, blinking.clone(), blink(1412)),
            ("まばたき 1411", 1000, blinking.clone(), blink(1411)),
            ("まばたき 1410", 1000, blinking.clone(), blink(1410)),
        ];
        let mut answers = Vec::new();
        for (name, surface, binds, pattern) in &cases {
            let cpu = Composer::new()
                .compose(&world, atlas, *surface, binds, pattern)
                .expect("CPU の合成");
            let warp = warp_render(&world, atlas, *surface, binds, pattern);
            let (max, n) = diff(&warp, &cpu);
            let half = cpu
                .bytes()
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] != 0 && p[3] != 255)
                .count();
            eprintln!(
                "R2 {name}: {}x{} 差の最大 {max}・差のある画素 {n}/{}・α>0 {}・半透明 {half}",
                cpu.width(),
                cpu.height(),
                cpu.width() * cpu.height(),
                alpha_count(&cpu)
            );
            assert!(alpha_count(&cpu) > 0, "{name}: 描くものがある");
            assert!(max <= 1, "{name}: 差の最大 {max}");
            answers.push(cpu.bytes().to_vec());
        }
        assert!(
            answers[3] != answers[4] && answers[4] != answers[5],
            "まばたきのコマごとに絵が替わる"
        );
    });
}

/// 座標から作る不透明の傾斜（位置のずれも見える）。
fn distinctive_opaque(w: u32, h: u32) -> Vec<u8> {
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            bgra.extend_from_slice(&[
                ((x * 7 + y * 3) % 251) as u8,
                ((x * 3 + y * 11) % 241) as u8,
                ((x * 13 + y * 5) % 233) as u8,
                255,
            ]);
        }
    }
    bgra
}

fn insert(dec: &mut MemoryDecoder, rel: &str, w: u32, h: u32, bgra: Vec<u8>) {
    dec.insert(emo2_master().join(rel), w, h, w * 4, bgra, true);
}

#[test]
fn surface0_element0_single_layer_equals_inserted_image() {
    let shell = emo2_shell();
    let s0 = shell
        .surfaces
        .iter()
        .find(|s| s.targets == [AppendTarget::Single(0)])
        .expect("surface0");
    assert_eq!(s0.elements.len(), 1, "element0 の 1 枚だけ");
    assert_eq!(s0.elements[0].path.as_str(), "surface0.png");
    assert_eq!((s0.elements[0].x, s0.elements[0].y), (0, 0));
    assert!(s0.animations.is_empty(), "bind も animation も無い");

    let (w, h) = (37, 29);
    let inserted = distinctive_opaque(w, h);
    let mut dec = MemoryDecoder::new();
    insert(&mut dec, "surface0.png", w, h, inserted.clone());
    let (world, atlas) = bake_emo2(&shell, &dec);

    let out = warp_render(
        &world,
        &atlas,
        0,
        &BindSet::default(),
        &PatternState::default(),
    );
    assert_eq!((out.width(), out.height(), out.stride()), (w, h, w * 4));
    assert_bytes_within_one(&out, &inserted);
}

/// 透明の縁を切り詰めた絵が、切り詰めない原画像の置き場所に描かれる。
#[test]
fn surface0_trimmed_composite_equals_untrimmed_placement() {
    const ORIG: u32 = 40;
    const INNER: (u32, u32) = (12, 15);
    const CORE: u32 = 10;
    let mut original = vec![0u8; (ORIG * ORIG * 4) as usize];
    for y in INNER.1..INNER.1 + CORE {
        for x in INNER.0..INNER.0 + CORE {
            let i = ((y * ORIG + x) * 4) as usize;
            original[i..i + 4].copy_from_slice(&[30, 170, 220, 255]);
        }
    }
    let shell = emo2_shell();
    let mut dec = MemoryDecoder::new();
    insert(&mut dec, "surface0.png", ORIG, ORIG, original.clone());
    let (world, atlas) = bake_emo2(&shell, &dec);

    let entry = atlas.entry(atlas.resolve(SetId(0), "surface0.png").expect("焼いた"));
    assert_eq!(entry.original, Size { w: ORIG, h: ORIG }, "原寸");
    let placement = entry.placement.as_ref().expect("不透明の芯がある");
    assert_eq!(
        placement.trim_offset,
        Point {
            x: INNER.0 as i32,
            y: INNER.1 as i32
        },
        "切り詰めた左上"
    );
    assert_eq!((placement.uv_rect.w, placement.uv_rect.h), (CORE, CORE));

    let out = warp_render(
        &world,
        &atlas,
        0,
        &BindSet::default(),
        &PatternState::default(),
    );
    assert_eq!((out.width(), out.height()), (ORIG, ORIG), "外形は原寸");
    assert_bytes_within_one(&out, &original);
}

/// 着せ替えの部品ごとに色と大きさを替え、その部品だけが覆う点が、その bind が有効なときだけ
/// 不透明になる。部品の element0 はどれも (0,0)。
#[test]
fn surface1000_bind_count_overlap_point_sampling() {
    // (animation, 部品の element0, 幅, 高さ, BGR)
    let parts = [
        (1100, "purple/0/base1.png", 80, 20, [255, 0, 0]),
        (1200, "purple/2/a.png", 30, 30, [0, 255, 0]),
        (1302, "purple/4/normal.png", 20, 90, [0, 0, 255]),
    ];
    let shell = emo2_shell();
    let mut dec = MemoryDecoder::new();
    for (_, rel, w, h, [b, g, r]) in parts {
        insert(&mut dec, rel, w, h, [b, g, r, 255].repeat((w * h) as usize));
    }
    let (world, atlas) = bake_emo2(&shell, &dec);
    let draw = |ids: &[u32]| {
        warp_render(
            &world,
            &atlas,
            1000,
            &BindSet::from_ids(ids.iter().copied()),
            &PatternState::default(),
        )
    };
    const ARM_ONLY: (u32, u32) = (70, 5);
    const EYE_ONLY: (u32, u32) = (5, 80);

    let arm = draw(&[1100]);
    assert_eq!(px(&arm, ARM_ONLY.0, ARM_ONLY.1), [255, 0, 0, 255], "腕");
    assert_eq!(px(&arm, EYE_ONLY.0, EYE_ONLY.1), [0; 4], "目は無効");

    let eye = draw(&[1302]);
    assert_eq!(px(&eye, EYE_ONLY.0, EYE_ONLY.1), [0, 0, 255, 255], "目");
    assert_eq!(px(&eye, ARM_ONLY.0, ARM_ONLY.1), [0; 4], "腕は無効");

    let both = draw(&[1100, 1302]);
    assert_eq!(px(&both, ARM_ONLY.0, ARM_ONLY.1)[3], 255);
    assert_eq!(px(&both, EYE_ONLY.0, EYE_ONLY.1)[3], 255);
    let three = draw(&[1100, 1200, 1302]);
    let (n_arm, n_eye, n_both, n_three) = (
        alpha_count(&arm),
        alpha_count(&eye),
        alpha_count(&both),
        alpha_count(&three),
    );
    assert!(n_both > n_arm && n_both > n_eye, "腕＋目は片方より多い");
    assert!(
        n_three > n_both,
        "口の 30×30 が腕にも目にも覆われない所を足す"
    );
}
