//! 採寸の読み手 `FirstFrameDecoder` の檻（spec: placement-measure-bake-once 要件 1.1・1.4・2.2・
//! 3.7、design「FirstFrameDecoder」・D4 の全透明の例外）。
//!
//! 偽の読み手 `MemoryDecoder` を数える包みで覆い、包んだ相手の全コマ読みが 1 度も呼ばれない
//! ことまで確かめる。ファイルにも COM にも依存しない。
use std::cell::Cell;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, BakeResult, DecodeError,
    DecodedImage, ElementDecoder, LoopCount, MemoryDecoder, PackConfig, SurfaceSet, UseSelfAlpha,
    bake,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface};

use super::first_frame::FirstFrameDecoder;
use crate::placement::test_support::capture_logs;

type Px = [u8; 4];

const BASE: &str = "shell/master";
const CLEAR: Px = [0, 0, 0, 0];
const K0: Px = [10, 20, 30, 255];
const K1: Px = [40, 50, 60, 255];
const STILL: Px = [70, 80, 90, 255];

fn path(rel: &str) -> PathBuf {
    Path::new(BASE).join(rel)
}

/// 2×2 の絵。
fn image(px: Px) -> DecodedImage {
    DecodedImage {
        width: 2,
        height: 2,
        stride: 8,
        bgra: [px; 4].concat(),
        has_alpha: true,
    }
}

fn info() -> AnimationInfo {
    AnimationInfo {
        width: 2,
        height: 2,
        frame_count: 2,
    }
}

/// 2 コマの動く絵の全コマ（コマの絵は `px0`・`px1`）。
fn frames(px0: Px, px1: Px) -> AnimatedImage {
    AnimatedImage {
        frames: [px0, px1]
            .iter()
            .map(|&px| AnimationFrame {
                image: image(px),
                delay_ms: 100,
            })
            .collect(),
        loop_count: LoopCount::Finite(NonZeroU32::new(3).unwrap()),
    }
}

fn insert(dec: &mut MemoryDecoder, rel: &str, px: Px) {
    let img = image(px);
    dec.insert(path(rel), 2, 2, 8, img.bgra, img.has_alpha);
}

/// 5 つの口の呼ばれた回数を数える包み（このファイルの中だけで使う）。
#[derive(Default)]
struct Counting {
    inner: MemoryDecoder,
    decode: Cell<u32>,
    first: Cell<u32>,
    frames: Cell<u32>,
}

impl ElementDecoder for Counting {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        self.decode.set(self.decode.get() + 1);
        self.inner.decode(path)
    }
    fn probe_pna(&self, path: &Path) -> bool {
        self.inner.probe_pna(path)
    }
    fn probe_animation(&self, path: &Path) -> Option<AnimationInfo> {
        self.inner.probe_animation(path)
    }
    fn decode_frames(
        &self,
        path: &Path,
        info: AnimationInfo,
    ) -> Result<AnimatedImage, DecodeError> {
        self.frames.set(self.frames.get() + 1);
        self.inner.decode_frames(path, info)
    }
    fn decode_first_frame(
        &self,
        path: &Path,
        info: AnimationInfo,
    ) -> Result<DecodedImage, DecodeError> {
        self.first.set(self.first.get() + 1);
        self.inner.decode_first_frame(path, info)
    }
}

/// 動く絵 anim.png（1 枚目 K0・1 枚読み STILL）と静止画 still.png を持つ読み手。
fn animated_and_still() -> Counting {
    let mut inner = MemoryDecoder::new();
    inner.insert_animated(path("anim.png"), info(), Ok(image(K0)), Ok(frames(K0, K1)));
    insert(&mut inner, "anim.png", STILL);
    insert(&mut inner, "still.png", STILL);
    inner.insert_pna(path("still.png"));
    Counting {
        inner,
        ..Default::default()
    }
}

/// 要件 1.1・2.2: 動く絵の `decode` は 1 枚目の絵を返し、全コマ読みは 0 回。
#[test]
fn animated_decode_returns_the_first_frame() {
    let dec = animated_and_still();
    let got = FirstFrameDecoder::new(&dec)
        .decode(&path("anim.png"))
        .expect("first frame");
    assert_eq!(got.bgra, image(K0).bgra);
    assert_eq!((dec.first.get(), dec.decode.get()), (1, 0));
    assert_eq!(dec.frames.get(), 0, "全コマ読みは呼ばない");
}

/// 1 枚目が `Err` なら包んだ読み手の 1 枚読みへ落ちる（資産の縮め方の段 2 と同じ）。
#[test]
fn first_frame_error_falls_back_to_decode() {
    let mut inner = MemoryDecoder::new();
    inner.insert_animated(
        path("anim.png"),
        info(),
        Err("broken".into()),
        Ok(frames(K0, K1)),
    );
    insert(&mut inner, "anim.png", STILL);
    let dec = Counting {
        inner,
        ..Default::default()
    };
    let got = FirstFrameDecoder::new(&dec)
        .decode(&path("anim.png"))
        .expect("fallback");
    assert_eq!(got.bgra, image(STILL).bgra);
    assert_eq!((dec.first.get(), dec.decode.get()), (1, 1));
    assert_eq!(dec.frames.get(), 0, "全コマ読みは呼ばない");
}

/// 要件 1.4: 静止画は包んだ読み手の 1 枚読みをそのまま通す（壊れた絵の失敗も同じ）。
#[test]
fn still_image_passes_through() {
    let mut dec = animated_and_still();
    dec.inner.insert_corrupt(path("bad.png"), "corrupt");
    let ffd = FirstFrameDecoder::new(&dec);
    assert_eq!(
        ffd.decode(&path("still.png")).unwrap().bgra,
        image(STILL).bgra
    );
    assert_eq!(
        format!("{:?}", ffd.decode(&path("bad.png")).unwrap_err()),
        format!("{:?}", dec.inner.decode(&path("bad.png")).unwrap_err())
    );
    assert_eq!((dec.first.get(), dec.frames.get()), (0, 0));
}

/// 動く絵の見出しは焼く段に見せず、`.pna` の問い合わせは素通し。
#[test]
fn hides_animation_and_passes_pna_through() {
    let dec = animated_and_still();
    let ffd = FirstFrameDecoder::new(&dec);
    assert!(
        dec.probe_animation(&path("anim.png")).is_some(),
        "前提: 動く絵"
    );
    assert_eq!(ffd.probe_animation(&path("anim.png")), None);
    assert!(ffd.probe_pna(&path("still.png")));
    assert!(!ffd.probe_pna(&path("anim.png")));
}

/// 1 枚目が全透明・2 枚目が不透明の anim.png を持つシェルを焼き、「全透明」の `warn!` を数える。
fn bake_fade_in_warns(wrap: bool) -> (BakeResult, usize) {
    let mut inner = MemoryDecoder::new();
    inner.insert_animated(
        path("anim.png"),
        info(),
        Ok(image(CLEAR)),
        Ok(frames(CLEAR, K1)),
    );
    let dec = Counting {
        inner,
        ..Default::default()
    };
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new("anim.png".to_string()),
            x: 0,
            y: 0,
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let sets = [SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    }];
    let (result, events) = capture_logs(|| {
        if wrap {
            bake(&sets, &FirstFrameDecoder::new(&dec), PackConfig::default())
        } else {
            bake(&sets, &dec, PackConfig::default())
        }
    });
    let warns = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.message().contains("全透明"))
        .count();
    if wrap {
        assert_eq!(dec.frames.get(), 0, "包んだ焼きは全コマ読みを呼ばない");
    }
    (result, warns)
}

/// 要件 3.7 の例外（design D4）: 包んだ読み手では 1 枚目だけの静止画として見えるので
/// 「全透明」の `warn!` がちょうど 1 行、包まない読み手（動く絵の道）では 0 行。
#[test]
fn fade_in_animation_warns_all_transparent_only_when_wrapped() {
    let (wrapped, wrapped_warns) = bake_fade_in_warns(true);
    let (raw, raw_warns) = bake_fade_in_warns(false);
    assert!(wrapped.errors.is_empty() && raw.errors.is_empty());
    assert_eq!(wrapped_warns, 1, "包んだ読み手は 1 行");
    assert_eq!(raw_warns, 0, "包まない読み手は 0 行");
}
