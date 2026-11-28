//! 採寸の読み手 `FirstFrameDecoder` の檻。
//!
//! 偽の読み手 `MemoryDecoder` を数える包みで覆い、包んだ相手の全コマ読みが 1 度も呼ばれない
//! ことまで確かめる。ファイルにも COM にも依存しない。
use std::cell::Cell;
use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationInfo, AnimationLimits, AtlasTable, BakeResult,
    DecodeError, DecodedImage, ElementDecoder, MemoryDecoder, PackConfig, SetId, SurfaceSet,
    UseSelfAlpha, bake, bake_quiet, bake_with_limits,
};
use areka_parsers::shell::{
    AppendTarget, DrawMethod, Element, ElementOptions, ElementPath, Surface,
};

use super::first_frame::FirstFrameDecoder;
use crate::test_support::animated_image::{frames, image, info};
use log_capture_kit::capture;

type Px = [u8; 4];

const BASE: &str = "shell/master";
const CLEAR: Px = [0, 0, 0, 0];
const K0: Px = [10, 20, 30, 255];
const K1: Px = [40, 50, 60, 255];
const STILL: Px = [70, 80, 90, 255];

fn path(rel: &str) -> PathBuf {
    Path::new(BASE).join(rel)
}

fn insert(dec: &mut MemoryDecoder, rel: &str, px: Px) {
    let img = image(px);
    dec.insert(path(rel), 2, 2, 8, img.bgra, img.has_alpha);
}

/// 読む口のうち 3 つ（`decode`・`decode_first_frame`・`decode_frames`）の呼ばれた回数を数える包み。
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

/// 動く絵の `decode` は 1 枚目の絵を返し、全コマ読みは 0 回。
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

/// 静止画は包んだ読み手の 1 枚読みをそのまま通す（壊れた絵の失敗も同じ）。
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

/// 焼き方: 包んだ読み手＋出す焼き（対照）・包んだ読み手＋出さない焼き（採寸）・包まない読み手＋出す焼き。
#[derive(Clone, Copy, PartialEq)]
enum Way {
    WrappedBake,
    WrappedQuiet,
    RawBake,
}

/// 1 枚目が全透明・2 枚目が不透明の anim.png を持つシェルを焼き、「全透明」の `warn!` を数える。
fn bake_fade_in_warns(way: Way) -> (BakeResult, usize) {
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
        targets: vec![AppendTarget::Single(0)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new("anim.png".to_string()),
            x: 0,
            y: 0,
            method: DrawMethod::new("overlay".to_string()),
            options: ElementOptions::new(String::new()),
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
    let (result, events) = capture(|| match way {
        Way::WrappedBake => bake(&sets, &FirstFrameDecoder::new(&dec), PackConfig::default()),
        Way::WrappedQuiet => {
            bake_quiet(&sets, &FirstFrameDecoder::new(&dec), PackConfig::default())
        }
        // 上限は既定を焼く（環境変数に左右されない）。
        Way::RawBake => bake_with_limits(
            &sets,
            &dec,
            PackConfig::default(),
            AnimationLimits::default(),
        ),
    });
    let warns = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.message().contains("全透明"))
        .count();
    if way != Way::RawBake {
        assert_eq!(dec.frames.get(), 0, "包んだ焼きは全コマ読みを呼ばない");
    }
    (result, warns)
}

/// 採寸が使う焼き（包んだ読み手＋`bake_quiet`）でも、包まない読み手の焼きでも「全透明」の
/// `warn!` は 0 行。対照として包んだ読み手＋出す焼きで 1 行出ることを確かめ、検体が本当に
/// 1 枚目だけ全透明であることを示す（`bake_quiet` は絵によらず出さないので、対照が無いと
/// 0 行の主張が何も確かめない）。
#[test]
fn fade_in_animation_all_transparent_warns_zero_lines_on_measure_bake_and_raw_bake() {
    let (control, control_warns) = bake_fade_in_warns(Way::WrappedBake);
    let (quiet, quiet_warns) = bake_fade_in_warns(Way::WrappedQuiet);
    let (raw, raw_warns) = bake_fade_in_warns(Way::RawBake);
    assert!(control.errors.is_empty() && quiet.errors.is_empty() && raw.errors.is_empty());
    assert_eq!(control_warns, 1, "対照: 包んだ読み手＋出す焼きは 1 行");
    assert_eq!(
        quiet_warns, 0,
        "採寸の焼き（包んだ読み手＋出さない焼き）は 0 行"
    );
    assert_eq!(raw_warns, 0, "包まない読み手＋出す焼きは 0 行");
}

/// 乗算しない半透明の緑と、その乗算済みの色。
const HALF: Px = [0, 100, 0, 128];
const HALF_PREMULTIPLIED: Px = [0, 50, 0, 128];

/// 2×2 の絵（左上が `K0`・残り 3 画素が `rest`）。`Off` は左上の色を抜く。
fn keyed(rest: Px) -> DecodedImage {
    DecodedImage {
        bgra: [K0, rest, rest, rest].concat(),
        ..image(K0)
    }
}

/// 焼いた絵を原寸の画素へ戻す。
fn pixels(table: &AtlasTable, rel: &str) -> Vec<Px> {
    let e = table.entry(table.resolve(SetId(0), rel).expect("baked"));
    let mut out = vec![CLEAR; (e.original.w * e.original.h) as usize];
    if let Some(p) = &e.placement {
        let page = table.page(p.page).expect("page");
        for y in 0..p.uv_rect.h {
            for x in 0..p.uv_rect.w {
                let src = ((p.uv_rect.y + y) * page.stride + (p.uv_rect.x + x) * 4) as usize;
                let dst = ((p.trim_offset.y as u32 + y) * e.original.w + p.trim_offset.x as u32 + x)
                    as usize;
                out[dst] = page.bytes[src..src + 4].try_into().unwrap();
            }
        }
    }
    out
}

/// `use_self_alpha,0` の採寸の焼きは、動く絵を乗算しない 1 枚目（読めなければ乗算しない
/// 1 枚読み）から、静止画を乗算しない 1 枚読みから作る。乗算済みの答えと乗算しない 1 枚読みは
/// どれも色が違うので、読み分けを誤ると画素が合わない。
#[test]
fn off_bake_reads_unmultiplied_first_frame_then_single_read_then_still() {
    let mut dec = MemoryDecoder::new();
    for rel in ["anim.png", "broken.png"] {
        dec.insert_animated(
            path(rel),
            info(),
            Ok(keyed(HALF_PREMULTIPLIED)),
            Ok(frames(K0, K1)),
        );
        dec.insert(path(rel), 2, 2, 8, keyed(STILL).bgra, true);
        dec.insert_unmultiplied(path(rel), keyed(K1));
    }
    dec.insert_animated_unmultiplied(path("anim.png"), Ok(keyed(HALF)), Ok(frames(K0, K1)));
    dec.insert_animated_unmultiplied(path("broken.png"), Err("broken".into()), Ok(frames(K0, K1)));
    dec.insert(
        path("still.png"),
        2,
        2,
        8,
        keyed(HALF_PREMULTIPLIED).bgra,
        true,
    );
    dec.insert_unmultiplied(path("still.png"), keyed(HALF));

    let element = |layer, rel: &str| Element {
        layer,
        path: ElementPath::new(rel.to_string()),
        x: 0,
        y: 0,
        method: DrawMethod::new("overlay".to_string()),
        options: ElementOptions::new(String::new()),
    };
    let surfaces = vec![Surface {
        targets: vec![AppendTarget::Single(0)],
        elements: vec![
            element(0, "anim.png"),
            element(1, "broken.png"),
            element(2, "still.png"),
        ],
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let sets = [SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::Off,
        },
    }];
    let result = bake_quiet(&sets, &FirstFrameDecoder::new(&dec), PackConfig::default());
    assert!(result.errors.is_empty(), "{:?}", result.errors);

    let opaque_half = [HALF[0], HALF[1], HALF[2], 255];
    assert_eq!(
        pixels(&result.table, "anim.png"),
        [CLEAR, opaque_half, opaque_half, opaque_half]
    );
    assert_eq!(pixels(&result.table, "broken.png"), [CLEAR, K1, K1, K1]);
    assert_eq!(
        pixels(&result.table, "still.png"),
        [CLEAR, opaque_half, opaque_half, opaque_half]
    );
}
