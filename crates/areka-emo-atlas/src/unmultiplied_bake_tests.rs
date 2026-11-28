//! `use_self_alpha,0` と `asis` の絵を乗算しない読みから作る焼きのテスト。
//!
//! 偽の読み手には乗算済みの答えと乗算しない答えを別々に登録し、半透明の画素は両者で色が違う
//! ようにしてある。乗算済みの色から作ると、どの期待も赤になる。

use std::path::Path;

use areka_parsers::shell::{DrawMethod, Surface};

use crate::decode::{AnimatedImage, AnimationFrame, AnimationInfo, MemoryDecoder};
use crate::limits::AnimationLimits;
use crate::manifest::SurfaceSet;
use crate::normalize::UseSelfAlpha;
use crate::pack::PackConfig;
use crate::table::{AtlasTable, LoopCount, SetId};
use crate::test_support::{elem, full, image, params, surface};
use crate::{BakeResult, bake_with_limits};

type Px = [u8; 4];

const BASE: &str = "shell";
const CLEAR: Px = [0, 0, 0, 0];
const X: Px = [70, 80, 90, 255];
/// 乗算しない半透明の赤と、その乗算済みの色。
const TL: Px = [0, 0, 200, 128];
const TL_PREMULTIPLIED: Px = [0, 0, 100, 128];
/// 乗算しない半透明の緑と、その乗算済みの色。
const H: Px = [0, 100, 0, 128];
const H_PREMULTIPLIED: Px = [0, 50, 0, 128];
/// 不透明な画素で、色は `TL` の乗算済みの色と同じ（乗算済みの左上で抜くと消える）。
const B: Px = [0, 0, 100, 255];

fn premultiplied(px: Px) -> Px {
    match px {
        TL => TL_PREMULTIPLIED,
        H => H_PREMULTIPLIED,
        other => other,
    }
}

fn opaque(px: Px) -> Px {
    [px[0], px[1], px[2], 255]
}

fn bake_shell(
    dec: &MemoryDecoder,
    surfaces: &[Surface],
    decl: UseSelfAlpha,
    limits: AnimationLimits,
) -> BakeResult {
    let sets = [SurfaceSet {
        surfaces,
        base_dir: Path::new(BASE),
        alpha_params: params(decl),
    }];
    let result = bake_with_limits(&sets, dec, PackConfig::default(), limits);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    result
}

fn one_picture(method: &str) -> Vec<Surface> {
    let mut e = elem(0, "p.png");
    e.method = DrawMethod::new(method.to_string());
    vec![surface(0, vec![e])]
}

fn still_decoder(straight: &[Px], has_alpha: bool) -> MemoryDecoder {
    let w = straight.len() as u32;
    let path = Path::new(BASE).join("p.png");
    let prem: Vec<Px> = straight.iter().copied().map(premultiplied).collect();
    let mut dec = MemoryDecoder::new();
    dec.insert(&path, w, 1, w * 4, prem.concat(), has_alpha);
    dec.insert_unmultiplied(&path, image(w, 1, has_alpha, straight));
    dec
}

fn frames_of(straight: &[[Px; 4]], map: impl Fn(Px) -> Px) -> AnimatedImage {
    AnimatedImage {
        frames: straight
            .iter()
            .map(|f| AnimationFrame {
                image: image(2, 2, true, &f.map(&map)),
                delay_ms: 100,
            })
            .collect(),
        loop_count: LoopCount::Infinite,
    }
}

fn animated_decoder(straight: &[[Px; 4]]) -> MemoryDecoder {
    let path = Path::new(BASE).join("p.png");
    let info = AnimationInfo {
        width: 2,
        height: 2,
        frame_count: straight.len() as u32,
    };
    let prem = frames_of(straight, premultiplied);
    let unmul = frames_of(straight, |px| px);
    let mut dec = MemoryDecoder::new();
    dec.insert_animated(
        &path,
        info,
        Ok(prem.frames[0].image.clone()),
        Ok(prem.clone()),
    );
    dec.insert_animated_unmultiplied(&path, Ok(unmul.frames[0].image.clone()), Ok(unmul));
    dec
}

fn frame_pixels(table: &AtlasTable, parent: crate::table::ElementId) -> Vec<Vec<Px>> {
    let anim = table.animation(parent).expect("all frames kept");
    anim.frames.iter().map(|id| full(table, *id)).collect()
}

const FRAMES: [[Px; 4]; 2] = [[TL, X, H, X], [X, H, TL, B]];

#[test]
fn off_still_turns_translucent_pixels_opaque_in_their_unmultiplied_colour() {
    let dec = still_decoder(&[TL, TL, B, H], true);
    let table = bake_shell(
        &dec,
        &one_picture("overlay"),
        UseSelfAlpha::Off,
        AnimationLimits::default(),
    )
    .table;
    let id = table.resolve(SetId(0), "p.png").unwrap();
    assert_eq!(full(&table, id), vec![CLEAR, CLEAR, B, opaque(H)]);
}

#[test]
fn off_still_without_alpha_keeps_its_answer() {
    let dec = still_decoder(&[B, X, B, X], false);
    let table = bake_shell(
        &dec,
        &one_picture("overlay"),
        UseSelfAlpha::Off,
        AnimationLimits::default(),
    )
    .table;
    let id = table.resolve(SetId(0), "p.png").unwrap();
    assert_eq!(full(&table, id), vec![CLEAR, X, CLEAR, X]);
}

#[test]
fn off_animation_uses_unmultiplied_colours_in_every_frame() {
    let table = bake_shell(
        &animated_decoder(&FRAMES),
        &one_picture("overlay"),
        UseSelfAlpha::Off,
        AnimationLimits::default(),
    )
    .table;
    let id = table.resolve(SetId(0), "p.png").unwrap();
    assert_eq!(
        frame_pixels(&table, id),
        vec![vec![CLEAR, X, opaque(H), X], vec![X, opaque(H), CLEAR, B]]
    );
}

#[test]
fn off_animation_shrunk_to_one_frame_uses_the_unmultiplied_first_frame() {
    let limits = AnimationLimits {
        max_frames: 1,
        ..AnimationLimits::default()
    };
    let table = bake_shell(
        &animated_decoder(&FRAMES),
        &one_picture("overlay"),
        UseSelfAlpha::Off,
        limits,
    )
    .table;
    let id = table.resolve(SetId(0), "p.png").unwrap();
    assert!(table.animation(id).is_none());
    assert_eq!(full(&table, id), vec![CLEAR, X, opaque(H), X]);
}

#[test]
fn asis_animation_keeps_every_frame_unmultiplied_and_opaque() {
    for decl in [UseSelfAlpha::On, UseSelfAlpha::Off] {
        let table = bake_shell(
            &animated_decoder(&FRAMES),
            &one_picture("asis"),
            decl,
            AnimationLimits::default(),
        )
        .table;
        let id = table.resolve_asis(SetId(0), "p.png").unwrap();
        let want: Vec<Vec<Px>> = FRAMES.iter().map(|f| f.map(opaque).to_vec()).collect();
        assert_eq!(frame_pixels(&table, id), want, "{decl:?}");
    }
}
