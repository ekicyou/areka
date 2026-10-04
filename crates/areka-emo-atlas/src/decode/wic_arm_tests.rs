//! `WicDecoderArm` の動く絵の 3 メソッドの兄弟テスト（spec: areka-P0-animated-image-decode
//! 要件 1.5・5.5）。
//!
//! 本番の読み手で検体（`testdata/animated/`・中身は同じフォルダの README）の見出し・全コマ・
//! 動きの 1 枚目が引けること、読めないときは `DecodeError::Decode`（パス付き）になることを判定する。
//! 画素の細部は `image_arm` の兄弟テストが持つので、ここでは決め手の 1 画素だけを見る。

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use super::WicDecoderArm;
use crate::decode::{AnimationInfo, DecodeError, ElementDecoder};

/// COM を張った下で本番の読み手を組んで渡す（`new` は COM が要る）。
fn with_arm<F: FnOnce(&WicDecoderArm)>(f: F) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    {
        let arm = WicDecoderArm::new().expect("WIC factory creates under COM init");
        f(&arm);
    }
    unsafe {
        CoUninitialize();
    }
}

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

/// 乗算済み BGRA の (x, y) の 1 画素。
fn pixel(bgra: &[u8], stride: u32, x: u32, y: u32) -> [u8; 4] {
    let i = (y * stride + x * 4) as usize;
    bgra[i..i + 4].try_into().unwrap()
}

const RED_BGRA: [u8; 4] = [0, 0, 255, 255];

fn assert_decode_error(result: Result<impl std::fmt::Debug, DecodeError>, path: &Path) {
    match result {
        Err(DecodeError::Decode { path: p, source }) => {
            assert_eq!(p, path);
            assert!(!source.is_empty(), "decode error carries a source string");
        }
        other => panic!("expected Decode for {}, got {other:?}", path.display()),
    }
}

#[test]
fn probe_answers_header_of_animated_samples() {
    with_arm(|arm| {
        assert_eq!(arm.probe_animation(&sample("basic.apng")), Some(info(4)));
        assert_eq!(arm.probe_animation(&sample("alpha.webp")), Some(info(3)));
        // 拡張子でなく中身で見分ける。
        assert_eq!(
            arm.probe_animation(&sample("webp_named.png")),
            Some(info(3))
        );
        // 見出しだけで答えるので、途中で切れた絵・読めない 16 ビットも見出しは引ける。
        assert_eq!(
            arm.probe_animation(&sample("truncated.apng")),
            Some(info(4))
        );
        assert_eq!(arm.probe_animation(&sample("deep16.apng")), Some(info(2)));
    });
}

#[test]
fn probe_is_none_for_single_gif_and_missing() {
    with_arm(|arm| {
        for name in ["single.apng", "single.webp", "two_frames.gif"] {
            assert_eq!(arm.probe_animation(&sample(name)), None, "{name}");
        }
        let missing = sample("does_not_exist_xyz.apng");
        assert!(!missing.exists());
        assert_eq!(arm.probe_animation(&missing), None);
    });
}

#[test]
fn decode_frames_reads_all_frames_with_probed_header() {
    with_arm(|arm| {
        for (name, count) in [("basic.apng", 4), ("alpha.webp", 3)] {
            let path = sample(name);
            let header = arm.probe_animation(&path).expect("animated");
            let anim = arm
                .decode_frames(&path, header)
                .unwrap_or_else(|e| panic!("{name} should decode: {e}"));
            assert_eq!(anim.frames.len(), count, "{name}");
            let first = &anim.frames[0].image;
            assert_eq!((first.width, first.height), (8, 8), "{name}");
            assert_eq!(pixel(&first.bgra, first.stride, 0, 0), RED_BGRA, "{name}");
        }
    });
}

#[test]
fn decode_first_frame_matches_frame_zero() {
    with_arm(|arm| {
        for name in ["basic.apng", "alpha.webp"] {
            let path = sample(name);
            let header = arm.probe_animation(&path).expect("animated");
            let first = arm
                .decode_first_frame(&path, header)
                .unwrap_or_else(|e| panic!("{name} first frame: {e}"));
            let all = arm.decode_frames(&path, header).expect("all frames");
            assert_eq!(first.bgra, all.frames[0].image.bgra, "{name}");
            assert_eq!(first.has_alpha, all.frames[0].image.has_alpha, "{name}");
        }
    });
}

#[test]
fn truncated_fails_all_frames_but_reads_first() {
    with_arm(|arm| {
        let path = sample("truncated.apng");
        assert_decode_error(arm.decode_frames(&path, info(4)), &path);
        let first = arm
            .decode_first_frame(&path, info(4))
            .expect("first frame of truncated.apng reads");
        assert_eq!(pixel(&first.bgra, first.stride, 0, 0), RED_BGRA);
    });
}

#[test]
fn deep16_fails_both_as_decode_error_with_path() {
    with_arm(|arm| {
        let path = sample("deep16.apng");
        assert_decode_error(arm.decode_frames(&path, info(2)), &path);
        assert_decode_error(arm.decode_first_frame(&path, info(2)), &path);
    });
}

/// 足した 3 メソッドは今の 1 枚読みと `.pna` の問い合わせを変えない。
#[test]
fn still_decode_and_probe_pna_unchanged_on_animated_file() {
    with_arm(|arm| {
        let path = sample("basic.apng");
        let still = arm.decode(&path).expect("WIC reads APNG as a still image");
        assert_eq!((still.width, still.height), (8, 8));
        assert!(!arm.probe_pna(&path));
    });
}
