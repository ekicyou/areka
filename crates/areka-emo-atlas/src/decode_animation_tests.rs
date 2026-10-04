//! 読み手の口の動く絵の 3 メソッドの単体テスト（spec: areka-P0-animated-image-decode
//! 要件 1.1・2.1〜2.5）。
//!
//! 既定の実装（見出しは無し・読むのは失敗）と、偽の読み手 `MemoryDecoder` が
//! `insert_animated` の登録どおりに 3 つのメソッドへ答えることを確かめる。
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

/// 登録どおり: 見出し・1 枚目・全コマをそれぞれ返す（コマの順・待ち時間・回数も）。
#[test]
fn memory_answers_as_registered() {
    let p = PathBuf::from("shell/anim.png");
    let mut d = MemoryDecoder::new();
    d.insert_animated(p.clone(), info(2), Ok(image(2, 10)), Ok(animated()));

    assert_eq!(d.probe_animation(&p), Some(info(2)));

    let first = d.decode_first_frame(&p, info(2)).expect("first frame");
    assert_eq!(first.bgra, vec![10; 8]);

    let all = d.decode_frames(&p, info(2)).expect("all frames");
    assert_eq!(all.frames.len(), 2);
    assert_eq!(all.frames[0].image.bgra, vec![10; 8]);
    assert_eq!(all.frames[1].image.bgra, vec![20; 8]);
    assert_eq!(
        all.frames.iter().map(|f| f.delay_ms).collect::<Vec<_>>(),
        vec![333, 0]
    );
    assert_eq!(
        all.loop_count,
        LoopCount::Finite(NonZeroU32::new(2).unwrap())
    );

    // 今までの decode の答えは別の登録（insert）で、insert_animated は触らない。
    assert!(matches!(d.decode(&p), Err(DecodeError::NotFound { .. })));
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

/// 全コマの失敗・1 枚目の失敗はそれぞれ `Decode`（登録した文）で返る。
/// 全コマが失敗なら、渡された見出しが何であっても同じ失敗。
#[test]
fn memory_registered_failures_are_decode_errors() {
    let p = PathBuf::from("shell/broken.png");
    let mut d = MemoryDecoder::new();
    d.insert_animated(
        p.clone(),
        info(3),
        Err("first broken".into()),
        Err("frame 2 broken".into()),
    );

    assert_eq!(d.probe_animation(&p), Some(info(3)));
    assert_eq!(
        expect_decode_err(d.decode_first_frame(&p, info(3)), &p),
        "first broken"
    );
    assert_eq!(
        expect_decode_err(d.decode_frames(&p, info(3)), &p),
        "frame 2 broken"
    );
    assert_eq!(
        expect_decode_err(d.decode_frames(&p, info(99)), &p),
        "frame 2 broken"
    );
}

/// 1 枚目だけ読めて全コマは失敗する、を別々に作れる（縮める段 1 の偽の答え）。
#[test]
fn memory_first_ok_frames_err() {
    let p = PathBuf::from("shell/half.png");
    let mut d = MemoryDecoder::new();
    d.insert_animated(p.clone(), info(2), Ok(image(2, 7)), Err("truncated".into()));

    assert_eq!(d.decode_first_frame(&p, info(2)).unwrap().bgra, vec![7; 8]);
    assert_eq!(
        expect_decode_err(d.decode_frames(&p, info(2)), &p),
        "truncated"
    );
}

/// 3 メソッドも trait object 経由で呼べる（口は具体の手段を露出しない）。
#[test]
fn animation_methods_via_dyn_port() {
    let p = PathBuf::from("shell/anim.png");
    let mut d = MemoryDecoder::new();
    d.insert_animated(p.clone(), info(2), Ok(image(2, 10)), Ok(animated()));
    let port: &dyn ElementDecoder = &d;
    let i = port.probe_animation(&p).expect("animated");
    assert_eq!(port.decode_frames(&p, i).unwrap().frames.len(), 2);
    assert_eq!(port.decode_first_frame(&p, i).unwrap().width, 2);
}
