//! `WicDecoderArm` のパレットの絵の兄弟テスト（spec: emo-atlas-palette-alpha 要件 1・2・5）。
//!
//! 検体（`testdata/palette/`・中身は同じフォルダの README）を本番の読み手で読み、
//! `has_alpha` の判定と、`Normalizer` に通した後の画素（設計の Data Models の 2 つの表）を見る。

use std::path::PathBuf;

use super::test_support::with_arm;
use crate::decode::ElementDecoder;
use crate::normalize::{AlphaParams, Normalizer, UseSelfAlpha};

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/testdata/palette")
        .join(name)
}

/// 2×2 の乗算済み BGRA を (0,0) (1,0) (0,1) (1,1) の順に並べる。
fn pixels(bgra: &[u8], stride: u32) -> [[u8; 4]; 4] {
    let px = |x: u32, y: u32| {
        let i = (y * stride + x * 4) as usize;
        <[u8; 4]>::try_from(&bgra[i..i + 4]).unwrap()
    };
    [px(0, 0), px(1, 0), px(0, 1), px(1, 1)]
}

fn assert_has_alpha(name: &str, expected: bool) {
    with_arm(|arm| {
        let img = arm.decode(&sample(name)).expect("palette sample decodes");
        assert_eq!(img.has_alpha, expected, "has_alpha of {name}");
    });
}

fn assert_baked(name: &str, decl: UseSelfAlpha, expected: [[u8; 4]; 4]) {
    with_arm(|arm| {
        let img = arm.decode(&sample(name)).expect("palette sample decodes");
        assert_eq!((img.width, img.height), (2, 2));
        let out = Normalizer.normalize(
            img,
            AlphaParams {
                use_self_alpha: decl,
            },
        );
        assert_eq!(
            pixels(&out.pbgra, out.stride),
            expected,
            "{name} baked with {decl:?}"
        );
    });
}

const RED: [u8; 4] = [0, 0, 255, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
const SEMI_BLUE: [u8; 4] = [128, 0, 0, 128];
/// 番号 0 赤・1 透明・2 半透明の青（`idx8_mixed`・`idx4_mixed`）。
const MIXED: [[u8; 4]; 4] = [RED, RED, CLEAR, SEMI_BLUE];
/// 透明度を持たない絵を `On` で通した後（左上の赤が抜ける・下の行は緑）。
const PLAIN_KEYED: [[u8; 4]; 4] = [CLEAR, CLEAR, GREEN, GREEN];

// 判定の表（要件 1.1・1.5・1.6・2.1・2.2）。

#[test]
fn has_alpha_idx8_mixed() {
    assert_has_alpha("idx8_mixed.png", true);
}

#[test]
fn has_alpha_idx4_mixed() {
    assert_has_alpha("idx4_mixed.png", true);
}

#[test]
fn has_alpha_gray8_trns() {
    assert_has_alpha("gray8_trns.png", true);
}

#[test]
fn has_alpha_trans_gif() {
    assert_has_alpha("trans.gif", true);
}

#[test]
fn has_alpha_idx8_plain() {
    assert_has_alpha("idx8_plain.png", false);
}

#[test]
fn has_alpha_idx8_opaque_trns() {
    assert_has_alpha("idx8_opaque_trns.png", false);
}

// `On`（`1`／`true`）は出る画素のまま（要件 1.2・1.4・5.1・5.4・5.8）。

#[test]
fn on_keeps_idx8_mixed() {
    assert_baked("idx8_mixed.png", UseSelfAlpha::On, MIXED);
}

#[test]
fn on_keeps_idx4_mixed() {
    assert_baked("idx4_mixed.png", UseSelfAlpha::On, MIXED);
}

#[test]
fn on_keeps_gray8_trns() {
    let white = [255, 255, 255, 255];
    let gray = [128, 128, 128, 255];
    assert_baked(
        "gray8_trns.png",
        UseSelfAlpha::On,
        [white, white, CLEAR, gray],
    );
}

#[test]
fn on_keeps_trans_gif() {
    let blue = [255, 0, 0, 255];
    assert_baked("trans.gif", UseSelfAlpha::On, [RED, RED, CLEAR, blue]);
}

// `Full` は透明と半透明の番号を不透明にしない（要件 1.3・1.4・5.2）。

#[test]
fn full_keeps_idx8_mixed() {
    assert_baked("idx8_mixed.png", UseSelfAlpha::Full, MIXED);
}

#[test]
fn full_keeps_idx4_mixed() {
    assert_baked("idx4_mixed.png", UseSelfAlpha::Full, MIXED);
}

// 透明度を持たないパレットの絵は今のまま左上の色を抜く（要件 2.1・2.2・5.3）。

#[test]
fn on_keys_idx8_plain() {
    assert_baked("idx8_plain.png", UseSelfAlpha::On, PLAIN_KEYED);
}

#[test]
fn on_keys_idx8_opaque_trns() {
    assert_baked("idx8_opaque_trns.png", UseSelfAlpha::On, PLAIN_KEYED);
}

// 失敗の道（要件 4.1）: 番号つきでない絵でパレットを読むと偽を推測せず WIC のエラーを返す。

#[test]
fn palette_has_alpha_fails_on_non_indexed_image() {
    use windows::Win32::Foundation::{GENERIC_READ, WINCODEC_ERR_PALETTEUNAVAILABLE};
    use windows::Win32::Graphics::Imaging::{IWICBitmapSource, WICDecodeMetadataCacheOnDemand};
    use windows::core::{HSTRING, Interface};

    with_arm(|arm| {
        let png = crate::sample_test_support::emo2_balloon_root().join("online0.png");
        assert!(png.exists(), "fixture must exist: {}", png.display());
        let frame: IWICBitmapSource = unsafe {
            arm.factory
                .CreateDecoderFromFilename(
                    &HSTRING::from(png.to_string_lossy().as_ref()),
                    None,
                    GENERIC_READ,
                    WICDecodeMetadataCacheOnDemand,
                )
                .and_then(|d| d.GetFrame(0))
                .and_then(|f| f.cast())
                .expect("online0.png frame 0")
        };
        let err = super::palette_has_alpha(&arm.factory, &frame)
            .expect_err("non-indexed image has no palette");
        assert_eq!(err.code(), WINCODEC_ERR_PALETTEUNAVAILABLE);
    });
}
