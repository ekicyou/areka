//! 焼く入口の動く絵の枝の結合テスト（spec: areka-P0-animated-image-decode 要件 3.1〜3.3・3.5・
//! 4.1〜4.4・4.7・5.1）。
//!
//! 偽の読み手 `MemoryDecoder` で、全コマを読めた動く絵が本番と同じ入口 `bake_with_limits` を
//! 通ってアトラスへ載ることを確かめる。ファイルにも検体にも依存しない。
use crate::decode::{AnimatedImage, AnimationFrame, AnimationInfo, DecodedImage, MemoryDecoder};
use crate::limits::AnimationLimits;
use crate::manifest::SurfaceSet;
use crate::normalize::{AlphaParams, UseSelfAlpha};
use crate::pack::PackConfig;
use crate::table::{AtlasTable, ElementId, LoopCount, SetId};
use crate::{BakeResult, bake_with_limits};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface};
use std::num::NonZeroU32;
use std::path::Path;

type Px = [u8; 4];

const BASE: &str = "shell/master";
const CLEAR: Px = [0, 0, 0, 0];
const K0: Px = [10, 20, 30, 255];
const K1: Px = [40, 50, 60, 255];
const X: Px = [70, 80, 90, 255];
const Y: Px = [100, 110, 120, 255];

/// a.png・anim.png・c.png の 3 つを持つシェル（鍵の並びは a → anim → c）。
fn shell() -> Vec<Surface> {
    let elements = ["a.png", "anim.png", "c.png"]
        .iter()
        .enumerate()
        .map(|(layer, path)| Element {
            layer: layer as u32,
            path: ElementPath::new(path.to_string()),
            x: 0,
            y: 0,
        })
        .collect();
    vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }]
}

/// 2×2 の絵（画素は左上から行順）。
fn image(px: [Px; 4], has_alpha: bool) -> DecodedImage {
    DecodedImage {
        width: 2,
        height: 2,
        stride: 8,
        bgra: px.concat(),
        has_alpha,
    }
}

fn animated(frames: &[([Px; 4], u32)], has_alpha: bool) -> AnimatedImage {
    AnimatedImage {
        frames: frames
            .iter()
            .map(|&(px, delay_ms)| AnimationFrame {
                image: image(px, has_alpha),
                delay_ms,
            })
            .collect(),
        loop_count: LoopCount::Finite(NonZeroU32::new(3).unwrap()),
    }
}

/// 静止画 a.png・c.png を登録した偽の読み手。
fn stills() -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    for (rel, px) in [("a.png", [X; 4]), ("c.png", [Y; 4])] {
        let img = image(px, true);
        dec.insert(Path::new(BASE).join(rel), 2, 2, 8, img.bgra, img.has_alpha);
    }
    dec
}

/// anim.png を動く絵として登録した偽の読み手（全コマの答えだけを使う）。
fn with_animation(anim: AnimatedImage) -> MemoryDecoder {
    let mut dec = stills();
    let info = AnimationInfo {
        width: 2,
        height: 2,
        frame_count: anim.frames.len() as u32,
    };
    dec.insert_animated(
        Path::new(BASE).join("anim.png"),
        info,
        Err("first frame is not used here".into()),
        Ok(anim),
    );
    dec
}

fn bake(dec: &MemoryDecoder) -> BakeResult {
    let surfaces = shell();
    let sets = [SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    }];
    let result = bake_with_limits(
        &sets,
        dec,
        PackConfig::default(),
        AnimationLimits::default(),
    );
    assert!(result.errors.is_empty(), "no failures: {:?}", result.errors);
    result
}

/// エントリをページから原寸の絵へ戻す（切り詰めの外は透明）。
fn full(table: &AtlasTable, id: ElementId) -> Vec<Px> {
    let e = table.entry(id);
    let w = e.original.w;
    let mut out = vec![CLEAR; (w * e.original.h) as usize];
    if let Some(p) = &e.placement {
        let page = table.page(p.page).expect("page exists");
        for y in 0..p.uv_rect.h {
            for x in 0..p.uv_rect.w {
                let src = ((p.uv_rect.y + y) * page.stride + (p.uv_rect.x + x) * 4) as usize;
                let dst = ((p.trim_offset.y as u32 + y) * w + p.trim_offset.x as u32 + x) as usize;
                out[dst] = page.bytes[src..src + 4].try_into().unwrap();
            }
        }
    }
    out
}

/// 表の中身を順序の決まった文字列にする（`HashMap` の並びに依らない）。
fn dump(table: &AtlasTable) -> String {
    let mut s = String::new();
    for i in 0..table.len() {
        let id = ElementId(i as u32);
        s += &format!(
            "{i}: {:?} {:?} {:?}\n",
            table.key(id),
            table.entry(id),
            table.animation(id)
        );
    }
    for page in table.pages() {
        s += &format!(
            "page {}x{} {:?}\n",
            page.width,
            page.height,
            &page.bytes[..]
        );
    }
    s
}

fn three_frames() -> AnimatedImage {
    animated(
        &[
            ([K0, X, X, X], 100),
            ([X, K0, X, X], 0),
            ([X, X, Y, K0], 70),
        ],
        true,
    )
}

/// 静止画の番号は動く絵を静止画に替えた場合と同じ・コマは末尾・鍵で 0 番・各コマが引ける。
#[test]
fn frames_follow_the_key_entries_and_the_key_resolves_to_frame_zero() {
    let anim = bake(&with_animation(three_frames())).table;
    let mut still_dec = stills();
    still_dec.insert(
        Path::new(BASE).join("anim.png"),
        2,
        2,
        8,
        [K0, X, X, X].concat(),
        true,
    );
    let still = bake(&still_dec).table;

    for rel in ["a.png", "anim.png", "c.png"] {
        assert_eq!(
            anim.resolve(SetId(0), rel),
            still.resolve(SetId(0), rel),
            "{rel}: same number as when the picture is still"
        );
    }
    assert_eq!(anim.len(), still.len() + 2, "two more frames at the tail");

    let parent = anim.resolve(SetId(0), "anim.png").unwrap();
    let a = anim.animation(parent).expect("parent answers its frames");
    assert_eq!(a.frames, vec![parent, ElementId(3), ElementId(4)]);
    assert_eq!(a.delays_ms, vec![100, 0, 70]);
    assert_eq!(a.loop_count, LoopCount::Finite(NonZeroU32::new(3).unwrap()));

    let want = [[K0, X, X, X], [X, K0, X, X], [X, X, Y, K0]];
    for (n, id) in a.frames.iter().enumerate() {
        assert_eq!(full(&anim, *id), want[n].to_vec(), "frame {n} pixels");
        assert_eq!(
            anim.key(*id),
            anim.key(parent),
            "frame {n} carries the parent key"
        );
    }
    assert_eq!(
        full(&anim, parent),
        full(&still, parent),
        "key shows frame 0"
    );
    for id in [
        anim.resolve(SetId(0), "a.png").unwrap(),
        ElementId(3),
        ElementId(4),
    ] {
        assert!(anim.animation(id).is_none(), "{id:?}: not a parent");
    }
}

/// 同じ入力から 2 回焼いて同じ表（要件 4.7）。
#[test]
fn baking_twice_gives_the_same_table() {
    let dec = with_animation(three_frames());
    assert_eq!(dump(&bake(&dec).table), dump(&bake(&dec).table));
}

/// 透明度の無い動く絵は、0 番のコマの左上の色だけを全コマから消す（要件 3.3）。
#[test]
fn opaque_animation_clears_only_frame_zero_top_left_from_every_frame() {
    let dec = with_animation(animated(
        &[
            ([K0, X, X, K0], 100),
            ([K1, K0, X, K0], 100),
            ([K0, K1, K0, X], 100),
        ],
        false,
    ));
    let table = bake(&dec).table;
    let parent = table.resolve(SetId(0), "anim.png").unwrap();
    let frames = table.animation(parent).unwrap().frames.clone();
    let want = [
        [CLEAR, X, X, CLEAR],
        [K1, CLEAR, X, CLEAR],
        [CLEAR, K1, CLEAR, X],
    ];
    for (n, id) in frames.iter().enumerate() {
        assert_eq!(full(&table, *id), want[n].to_vec(), "frame {n}");
    }
}

/// 全透明のコマは位置無しで番号と待ち時間を残し、全透明の `warn!` は出さない（要件 3.5）。
#[test]
fn transparent_frame_keeps_its_number_and_delay_without_warning() {
    let dec = with_animation(animated(
        &[([K0; 4], 100), ([CLEAR; 4], 40), ([X; 4], 70)],
        true,
    ));
    let mut result = None;
    let log = crate::log_capture::capture_logs(|| result = Some(bake(&dec)));
    let table = result.unwrap().table;
    let parent = table.resolve(SetId(0), "anim.png").unwrap();
    let a = table.animation(parent).unwrap();
    assert_eq!(a.frames.len(), 3, "the transparent frame is not dropped");
    assert_eq!(a.delays_ms, vec![100, 40, 70]);
    let empty = table.entry(a.frames[1]);
    assert!(
        empty.placement.is_none(),
        "transparent frame has no placement"
    );
    assert_eq!((empty.original.w, empty.original.h), (2, 2));
    assert!(!log.contains("level=WARN"), "no warning: {log}");
}

/// 0 番のコマだけが全透明（見えない所から始まる動く絵）でも、鍵のエントリは位置無しで残り、
/// 全透明の `warn!` は出さない（要件 3.5）。静止画なら警告する唯一の分かれ目。
#[test]
fn transparent_frame_zero_keeps_the_key_entry_without_warning() {
    let dec = with_animation(animated(
        &[([CLEAR; 4], 100), ([K0; 4], 40), ([X; 4], 70)],
        true,
    ));
    let mut result = None;
    let log = crate::log_capture::capture_logs(|| result = Some(bake(&dec)));
    let table = result.unwrap().table;
    let parent = table.resolve(SetId(0), "anim.png").unwrap();
    let a = table.animation(parent).unwrap();
    assert_eq!(a.delays_ms, vec![100, 40, 70]);
    let key = table.entry(parent);
    assert!(key.placement.is_none(), "frame 0 has no placement");
    assert_eq!((key.original.w, key.original.h), (2, 2));
    assert!(!log.contains("全透明"), "no all-transparent warning: {log}");
}

/// 全部のコマが透明な動く絵にだけ、全透明の `warn!` を 1 回出す。
#[test]
fn all_transparent_animation_warns_once() {
    let dec = with_animation(animated(&[([CLEAR; 4], 100), ([CLEAR; 4], 40)], true));
    let mut result = None;
    let log = crate::log_capture::capture_logs(|| result = Some(bake(&dec)));
    let table = result.unwrap().table;
    let parent = table.resolve(SetId(0), "anim.png").unwrap();
    assert_eq!(table.animation(parent).unwrap().frames.len(), 2);
    assert_eq!(log.matches("全透明").count(), 1, "one warning: {log}");
    assert!(log.contains("anim.png"), "names the picture: {log}");
}
