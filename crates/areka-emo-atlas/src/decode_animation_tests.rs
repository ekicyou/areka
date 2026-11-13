//! 読み手の口の動く絵の 3 メソッドの単体テスト（spec: areka-P0-animated-image-decode
//! 要件 1.1・2.1〜2.5）。
//!
//! 本番の既定の実装（見出しは無し・読むのは失敗）と、焼きが頼る偽の読み手 `MemoryDecoder` の
//! 約束（見出しと中身を照らし合わせず登録どおりに返す・未登録は既定と同じ失敗）だけを確かめる。
use super::*;
use crate::table::LoopCount;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

fn image(width: u32, fill: u8) -> DecodedImage {
    DecodedImage {
        width,
        height: 1,
        stride: width * 4,
        bgra: vec![fill; (width * 4) as usize],
        has_alpha: true,
    }
}

fn info(frame_count: u32) -> AnimationInfo {
    AnimationInfo {
        width: 2,
        height: 1,
        frame_count,
    }
}

fn animated() -> AnimatedImage {
    AnimatedImage {
        frames: vec![
            AnimationFrame {
                image: image(2, 10),
                delay_ms: 333,
            },
            AnimationFrame {
                image: image(2, 20),
                delay_ms: 0,
            },
        ],
        loop_count: LoopCount::Finite(NonZeroU32::new(2).unwrap()),
    }
}

fn expect_decode_err<T: std::fmt::Debug>(r: Result<T, DecodeError>, want_path: &Path) -> String {
    match r {
        Err(DecodeError::Decode { path, source }) => {
            assert_eq!(path, want_path);
            source
        }
        other => panic!("expected Decode, got {other:?}"),
    }
}

/// 静止画しか読まない読み手（既存の 2 メソッドだけを実装）。
struct StillOnly;

impl ElementDecoder for StillOnly {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        Err(DecodeError::NotFound {
            path: path.to_path_buf(),
        })
    }
}

/// 既定の実装: 見出しは無し・全コマも 1 枚目も読めない（`Decode` の失敗・パス付き）。
#[test]
fn default_methods_say_not_animated_and_fail_to_read() {
    let p = Path::new("shell/a.png");
    let d = StillOnly;
    assert_eq!(d.probe_animation(p), None);
    expect_decode_err(d.decode_frames(p, info(2)), p);
    expect_decode_err(d.decode_first_frame(p, info(2)), p);
}

/// 登録の無いパスでは、偽の読み手も既定と同じに答える。
#[test]
fn memory_unregistered_path_is_not_animated() {
    let p = Path::new("shell/none.png");
    let d = MemoryDecoder::new();
    assert_eq!(d.probe_animation(p), None);
    expect_decode_err(d.decode_frames(p, info(2)), p);
    expect_decode_err(d.decode_first_frame(p, info(2)), p);
}

/// 見出しと中身は別々: 見出しが 2,000 枚と言っても、全コマの答えは登録したもの。
#[test]
fn memory_header_and_contents_are_independent() {
    let p = PathBuf::from("shell/huge.png");
    let mut d = MemoryDecoder::new();
    d.insert_animated(p.clone(), info(2000), Ok(image(2, 10)), Ok(animated()));

    assert_eq!(d.probe_animation(&p).map(|i| i.frame_count), Some(2000));
    assert_eq!(d.decode_frames(&p, info(2000)).unwrap().frames.len(), 2);
}
