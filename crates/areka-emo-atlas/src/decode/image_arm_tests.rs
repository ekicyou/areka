//! `image_arm` の兄弟テスト（spec: areka-P0-animated-image-decode 要件 2.1〜2.5・2.7・2.8・3.2・
//! 3.4・6.4・8.2・8.3・8.6）。
//!
//! 検体（`testdata/animated/`・中身は同じフォルダの README）を本物の `image` で読み、枚数・決め手の
//! 画素・待ち時間・繰り返し回数・透明度の有無を判定する。「重ねる」指定のコマの不透明な画素だけ
//! 色 ±1 を許す（`image-webp` の重ね算の丸め）。α と「重ねない」指定のコマは完全一致。

use std::num::NonZeroU32;
use std::path::PathBuf;

use super::{delay_ms, premultiply, read_first_frame, read_frames};
use crate::decode::{AnimatedImage, AnimationInfo, DecodedImage};
use crate::table::LoopCount;

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/testdata/animated")
        .join(name)
}

fn info(frame_count: u32) -> AnimationInfo {
    AnimationInfo {
        width: 8,
        height: 8,
        frame_count,
    }
}

fn frames(name: &str, frame_count: u32) -> AnimatedImage {
    read_frames(&sample(name), info(frame_count))
        .unwrap_or_else(|e| panic!("{name} should decode: {e}"))
}

fn finite(n: u32) -> LoopCount {
    LoopCount::Finite(NonZeroU32::new(n).unwrap())
}

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// (x, y) の乗算済み BGRA を RGBA の順で返す。
fn rgba(img: &DecodedImage, x: u32, y: u32) -> [u8; 4] {
    let i = (y * img.stride + x * 4) as usize;
    let p = &img.bgra[i..i + 4];
    [p[2], p[1], p[0], p[3]]
}

/// 決め手の画素。`tol` は色だけに効く（α は完全一致）。
fn assert_px(img: &DecodedImage, x: u32, y: u32, want: [u8; 4], tol: u8, what: &str) {
    let got = rgba(img, x, y);
    let ok = got[3] == want[3] && (0..3).all(|c| got[c].abs_diff(want[c]) <= tol);
    assert!(ok, "{what} ({x},{y}): got {got:?}, want {want:?} ±{tol}");
}

fn assert_shape(anim: &AnimatedImage, count: usize, has_alpha: bool) {
    assert_eq!(anim.frames.len(), count);
    for f in &anim.frames {
        let img = &f.image;
        assert_eq!((img.width, img.height, img.stride), (8, 8, 32));
        assert_eq!(img.bgra.len(), (img.stride * img.height) as usize);
        assert_eq!(img.has_alpha, has_alpha, "all frames share has_alpha");
    }
}

fn delays(anim: &AnimatedImage) -> Vec<u32> {
    anim.frames.iter().map(|f| f.delay_ms).collect()
}

// ---- 2 形式の中身（要件 8.2） ----

#[test]
fn basic_apng_frames_delays_loop_and_pixels() {
    let anim = frames("basic.apng", 4);
    assert_shape(&anim, 4, true);
    assert_eq!(delays(&anim), [333, 0, 70, 1]);
    assert_eq!(anim.loop_count, finite(2));

    let f = |n: usize| &anim.frames[n].image;
    // 0 番: 置き換え
    assert_px(f(0), 0, 0, RED, 0, "f0");
    assert_px(f(0), 2, 2, RED, 0, "f0");
    assert_px(f(0), 4, 0, CLEAR, 0, "f0");
    assert_px(f(0), 7, 7, CLEAR, 0, "f0");
    // 1 番: 重ねる
    assert_px(f(1), 0, 0, RED, 1, "f1");
    for (x, y) in [(4, 0), (5, 1), (7, 3)] {
        assert_px(f(1), x, y, GREEN, 1, "f1");
    }
    assert_px(f(1), 4, 4, CLEAR, 0, "f1");
    assert_px(f(1), 7, 7, CLEAR, 0, "f1");
    // 2 番: 重ねる（1 番の緑は背景へ戻った）
    assert_px(f(2), 2, 2, BLUE, 1, "f2");
    assert_px(f(2), 3, 3, BLUE, 1, "f2");
    assert_px(f(2), 0, 0, RED, 1, "f2");
    assert_px(f(2), 4, 0, CLEAR, 0, "f2");
    assert_px(f(2), 5, 1, CLEAR, 0, "f2");
    // 3 番: 置き換えで全部透明
    assert!(f(3).bgra.iter().all(|&b| b == 0), "f3 is fully clear");
}

#[test]
fn alpha_webp_frames_delays_loop_and_pixels() {
    let anim = frames("alpha.webp", 3);
    assert_shape(&anim, 3, true);
    assert_eq!(delays(&anim), [100, 0, 70]);
    assert_eq!(anim.loop_count, finite(3));

    let f = |n: usize| &anim.frames[n].image;
    // 0 番: 重ねない
    assert_px(f(0), 0, 0, RED, 0, "f0");
    assert_px(f(0), 2, 2, RED, 0, "f0");
    assert_px(f(0), 4, 0, CLEAR, 0, "f0");
    assert_px(f(0), 7, 7, CLEAR, 0, "f0");
    // 1 番: 左半分が全部透明（0 番が背景へ戻った。公開版 0.2.4 ではここが赤）
    for y in 0..8 {
        for x in 0..4 {
            assert_px(f(1), x, y, CLEAR, 0, "f1 left half");
        }
    }
    assert_px(f(1), 4, 0, CLEAR, 0, "f1");
    for (x, y) in [(4, 1), (5, 1), (7, 7)] {
        assert_px(f(1), x, y, GREEN, 1, "f1");
    }
    // 2 番
    assert_px(f(2), 2, 2, BLUE, 1, "f2");
    assert_px(f(2), 3, 3, BLUE, 1, "f2");
    assert_px(f(2), 0, 0, CLEAR, 0, "f2");
    assert_px(f(2), 4, 0, CLEAR, 0, "f2");
    assert_px(f(2), 4, 1, GREEN, 1, "f2");
    assert_px(f(2), 7, 7, GREEN, 1, "f2");
}

#[test]
fn default_image_apng_starts_with_first_animation_frame() {
    let anim = frames("default_image.apng", 2);
    assert_shape(&anim, 2, true);
    assert_eq!(delays(&anim), [100, 100]);
    assert_eq!(anim.loop_count, LoopCount::Infinite);
    assert_px(
        &anim.frames[0].image,
        0,
        0,
        RED,
        0,
        "f0 (not the black default image)",
    );
    assert_px(&anim.frames[1].image, 0, 0, GREEN, 0, "f1");
}

// ---- 透明度の有無（要件 3.2・3.4） ----

#[test]
fn rgb_samples_have_no_alpha() {
    for name in ["rgb.apng", "rgb.webp"] {
        let anim = frames(name, 2);
        assert_shape(&anim, 2, false);
        assert_eq!(delays(&anim), [100, 100], "{name}");
        assert_eq!(anim.loop_count, LoopCount::Infinite, "{name}");
        assert_px(&anim.frames[0].image, 0, 0, WHITE, 0, name);
        assert_px(&anim.frames[0].image, 7, 7, RED, 0, name);
        assert_px(&anim.frames[1].image, 0, 0, WHITE, 0, name);
        assert_px(&anim.frames[1].image, 7, 7, BLUE, 0, name);
    }
}

#[test]
fn trns_apng_has_alpha_and_premultiplies_the_key() {
    let anim = frames("trns.apng", 2);
    assert_shape(&anim, 2, true);
    // (255,255,255,0) は乗算すると (0,0,0,0)
    assert_px(&anim.frames[0].image, 0, 0, CLEAR, 0, "f0");
    assert_px(&anim.frames[0].image, 7, 7, RED, 0, "f0");
    assert_px(&anim.frames[1].image, 7, 7, BLUE, 0, "f1");
}

// ---- 失敗（要件 6.4・8.3） ----

#[test]
fn truncated_apng_fails_all_frames_but_reads_first() {
    let err = read_frames(&sample("truncated.apng"), info(4)).unwrap_err();
    assert!(err.contains("frame 1"), "{err}");
    // 1 枚目だけの口は 2 枚目以降を解かないので読める
    let first = read_first_frame(&sample("truncated.apng"), info(4)).unwrap();
    assert_px(&first, 0, 0, RED, 0, "first");
}

#[test]
fn deep16_apng_fails_both_entrances() {
    // image の APNG の読み手は 16 ビットを 1 枚目で断る
    for err in [
        read_frames(&sample("deep16.apng"), info(2)).unwrap_err(),
        read_first_frame(&sample("deep16.apng"), info(2)).unwrap_err(),
    ] {
        assert!(err.contains("frame 0") && err.contains("Rgba16"), "{err}");
    }
}

#[test]
fn header_mismatch_fails() {
    let basic = sample("basic.apng");
    // 見出しより多い（4 枚目が出た）・少ない
    let more = read_frames(&basic, info(3)).unwrap_err();
    assert!(more.contains("more frames"), "{more}");
    let fewer = read_frames(&basic, info(5)).unwrap_err();
    assert!(fewer.contains("4 frames"), "{fewer}");
    // 寸法が違う
    let wide = AnimationInfo {
        width: 9,
        ..info(4)
    };
    for err in [
        read_frames(&basic, wide).unwrap_err(),
        read_first_frame(&basic, wide).unwrap_err(),
    ] {
        assert!(err.contains("8x8"), "{err}");
    }
}

#[test]
fn non_apng_webp_and_missing_files_fail() {
    for err in [
        read_frames(&sample("two_frames.gif"), info(2)).unwrap_err(),
        read_first_frame(&sample("two_frames.gif"), info(2)).unwrap_err(),
    ] {
        assert!(err.contains("not an APNG or WebP"), "{err}");
    }
    assert!(read_frames(&sample("no_such_file.apng"), info(2)).is_err());
}

// ---- 同じ答え（要件 2.8・8.6） ----

fn same(a: &DecodedImage, b: &DecodedImage) -> bool {
    (a.width, a.height, a.stride, a.has_alpha, &a.bgra)
        == (b.width, b.height, b.stride, b.has_alpha, &b.bgra)
}

#[test]
fn reading_twice_gives_the_same_answer() {
    for (name, n) in [("basic.apng", 4), ("alpha.webp", 3)] {
        let (a, b) = (frames(name, n), frames(name, n));
        assert_eq!(a.loop_count, b.loop_count, "{name}");
        assert_eq!(delays(&a), delays(&b), "{name}");
        for (fa, fb) in a.frames.iter().zip(&b.frames) {
            assert!(same(&fa.image, &fb.image), "{name}");
        }
    }
}

#[test]
fn first_frame_matches_frame_zero() {
    let cases = [
        ("basic.apng", 4),
        ("alpha.webp", 3),
        ("default_image.apng", 2),
        ("rgb.apng", 2),
        ("rgb.webp", 2),
        ("trns.apng", 2),
    ];
    for (name, n) in cases {
        let all = frames(name, n);
        let first = read_first_frame(&sample(name), info(n)).unwrap();
        assert!(same(&first, &all.frames[0].image), "{name}");
    }
}

// ---- 換算 ----

#[test]
fn delay_rounds_half_up_and_never_panics() {
    assert_eq!(delay_ms(1000, 3), 333);
    assert_eq!(delay_ms(2000, 3), 667);
    assert_eq!(delay_ms(1, 2), 1);
    assert_eq!(delay_ms(0, 1), 0);
    assert_eq!(delay_ms(70, 1), 70);
    assert_eq!(delay_ms(5, 0), 0);
    assert_eq!(delay_ms(u32::MAX, 1), u32::MAX);
}

#[test]
fn premultiply_uses_wic_rounding() {
    assert_eq!(premultiply(255, 255), 255);
    assert_eq!(premultiply(255, 0), 0);
    assert_eq!(premultiply(255, 128), 128);
    assert_eq!(premultiply(1, 127), 0);
    assert_eq!(premultiply(1, 128), 1);
    assert_eq!(premultiply(200, 100), 78);
}
