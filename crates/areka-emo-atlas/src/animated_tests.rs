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
    shell_of(&["a.png", "anim.png", "c.png"])
}

/// 名前を並べたシェル（面 0 に 1 層ずつ）。
fn shell_of(names: &[&str]) -> Vec<Surface> {
    let elements = names
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
    let result = bake_in(
        dec,
        &["a.png", "anim.png", "c.png"],
        AnimationLimits::default(),
        PackConfig::default(),
    );
    assert!(result.errors.is_empty(), "no failures: {:?}", result.errors);
    result
}

/// 名前を並べたシェルを上限・設定つきで焼く（失敗の一覧は見ない）。
fn bake_in(
    dec: &MemoryDecoder,
    names: &[&str],
    limits: AnimationLimits,
    cfg: PackConfig,
) -> BakeResult {
    let surfaces = shell_of(names);
    let sets = [SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    }];
    bake_with_limits(&sets, dec, cfg, limits)
}

/// `bake_in` に捕捉したログを添える。
fn bake_logged(
    dec: &MemoryDecoder,
    names: &[&str],
    limits: AnimationLimits,
    cfg: PackConfig,
) -> (BakeResult, String) {
    let mut result = None;
    let log = crate::log_capture::capture_logs(|| {
        result = Some(bake_in(dec, names, limits, cfg));
    });
    (result.unwrap(), log)
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

// ---- 1 枚へ縮める 3 段（要件 3.3・4.3・6.2〜6.8・8.4・8.5） ----

const OVER: &str = "bake: 動く絵が上限を超えたので 1 枚だけ読みます";
const BROKEN: &str = "bake: 動く絵として読めなかったので 1 枚だけ読みます";
const FELL_BACK: &str =
    "bake: 動く絵の 1 枚目を読めなかったので、今までの読み方で 1 枚だけ読みます";
/// 上限で縮むときの全コマの答え。`decode_frames` が呼ばれれば `reason` にこの文が出る。
const NOT_READ: &str = "all frames must not be read";
/// 3 つの `warn!` に共通の欄（設計 Monitoring）。どの絵も組 0 に在る。
const SET: &str = " set=0 ";
const TARGET: &str = "target=areka_emo_atlas ";

/// `rel` を動く絵として登録する（見出しは 2×2・`frame_count` コマ）。
fn insert_anim(
    dec: &mut MemoryDecoder,
    rel: &str,
    frame_count: u32,
    first: Result<DecodedImage, String>,
    frames: Result<AnimatedImage, String>,
) {
    let info = AnimationInfo {
        width: 2,
        height: 2,
        frame_count,
    };
    dec.insert_animated(Path::new(BASE).join(rel), info, first, frames);
}

/// 今までの 1 枚読み（段 2）の答えを登録する。
fn insert_still(dec: &mut MemoryDecoder, rel: &str, img: DecodedImage) {
    let DecodedImage {
        width,
        height,
        stride,
        bgra,
        has_alpha,
    } = img;
    dec.insert(
        Path::new(BASE).join(rel),
        width,
        height,
        stride,
        bgra,
        has_alpha,
    );
}

fn warns(log: &str) -> Vec<&str> {
    log.lines().filter(|l| l.contains("level=WARN")).collect()
}

/// エントリの見え方（寸法・切り詰めの位置と大きさ・原寸へ戻した画素）。ページ上の位置は含めない。
fn view(table: &AtlasTable, id: ElementId) -> String {
    let e = table.entry(id);
    let trim = e
        .placement
        .as_ref()
        .map(|p| (p.trim_offset, p.uv_rect.w, p.uv_rect.h));
    format!("{:?} {:?} {:?}", e.original, trim, full(table, id))
}

/// 透明度を持たない 3 コマの動く絵。0 番の左上の色 K0 を抜くと 0 番は右下の 1 画素だけになる。
fn opaque_frames() -> AnimatedImage {
    animated(
        &[
            ([K0, K0, K0, X], 100),
            ([X, K0, K0, K0], 0),
            ([K1, X, X, X], 70),
        ],
        false,
    )
}

/// 縮める理由 1 つ。
struct Reason {
    name: &'static str,
    limits: AnimationLimits,
    cfg: PackConfig,
    /// `decode_frames` の答え。
    frames: Result<AnimatedImage, String>,
    /// 出るべき `warn!` の文と、その行に在るべき欄。
    warn: &'static str,
    says: &'static str,
}

/// 縮める理由 7 つ（設計の Testing Strategy の順）と、寸法の食い違い。見出しはどれも 2×2・3 コマ。
fn reasons(anim: &AnimatedImage) -> Vec<Reason> {
    let d = AnimationLimits::default();
    let over = |name, limits, cfg, says| Reason {
        name,
        limits,
        cfg,
        frames: Err(NOT_READ.into()),
        warn: OVER,
        says,
    };
    let broken = |name, frames, says| Reason {
        name,
        limits: d,
        cfg: PackConfig::default(),
        frames,
        warn: BROKEN,
        says,
    };
    let mut two = anim.clone();
    two.frames.pop();
    let mut small = anim.clone();
    small.frames[2].image = DecodedImage {
        width: 1,
        height: 1,
        stride: 4,
        bgra: X.to_vec(),
        has_alpha: false,
    };
    vec![
        over(
            "frames",
            AnimationLimits { max_frames: 2, ..d },
            PackConfig::default(),
            "exceeded=\"frames\"",
        ),
        over(
            "pixels",
            AnimationLimits {
                max_pixels: 11,
                ..d
            },
            PackConfig::default(),
            "exceeded=\"pixels\"",
        ),
        over(
            "total pixels",
            AnimationLimits {
                max_total_pixels: 11,
                ..d
            },
            PackConfig::default(),
            "exceeded=\"total_pixels\"",
        ),
        // 2 + 2×1 > 3 で見出しはページに入らないが、切り詰めた 1 画素は入る。
        over(
            "page side",
            d,
            PackConfig {
                page_size: 3,
                padding: 1,
            },
            "exceeded=\"page_side\"",
        ),
        broken("frame count mismatch", Ok(two), "got 2 frames"),
        broken(
            "frame failure",
            Err("frame 2 is broken".into()),
            "frame 2 is broken",
        ),
        broken(
            "unsupported form",
            Err("16-bit APNG is not supported".into()),
            "16-bit APNG is not supported",
        ),
        broken("frame size mismatch", Ok(small), "2x2, 2x2, 1x1"),
    ]
}

/// どの理由でも 1 枚へ縮み、その 1 枚は上限を広げて全コマを焼いたときの 0 番のコマと同じ
/// （透明度なしでも 0 番の左上の色が抜ける）。`warn!` は 1 回・失敗の一覧は空。上限なら全コマを
/// 読まず、段 1 で済むので今までの 1 枚読み（別の絵 Y を登録）は出ない（要件 3.3・4.3・6.2〜6.4・8.4）。
#[test]
fn each_reason_shrinks_to_frame_zero_of_the_full_bake() {
    let anim = opaque_frames();
    let wide = bake(&with_animation(anim.clone())).table;
    let wide_parent = wide.resolve(SetId(0), "anim.png").unwrap();
    assert!(
        wide.animation(wide_parent).is_some(),
        "wide bake keeps all frames"
    );
    let want = view(&wide, wide_parent);

    for r in reasons(&anim) {
        let mut dec = stills();
        insert_anim(
            &mut dec,
            "anim.png",
            3,
            Ok(anim.frames[0].image.clone()),
            r.frames,
        );
        insert_still(&mut dec, "anim.png", image([Y; 4], true));
        let (result, log) = bake_logged(&dec, &["a.png", "anim.png", "c.png"], r.limits, r.cfg);
        let name = r.name;
        assert!(
            result.errors.is_empty(),
            "{name}: no failure: {:?}",
            result.errors
        );
        let table = result.table;
        let parent = table.resolve(SetId(0), "anim.png").unwrap();
        assert!(
            table.animation(parent).is_none(),
            "{name}: shrunk to one picture"
        );
        assert_eq!(view(&table, parent), want, "{name}: same as frame 0");
        assert_eq!(
            full(&table, parent)[0],
            CLEAR,
            "{name}: top-left key color cleared"
        );

        let w = warns(&log);
        assert_eq!(w.len(), 1, "{name}: one warning: {log}");
        for part in [r.warn, r.says, SET, TARGET, "rel_path=\"anim.png\""] {
            assert!(w[0].contains(part), "{name}: warning has {part}: {log}");
        }
        if r.warn == OVER {
            for part in ["frame_count=3", "width=2", "height=2"] {
                assert!(w[0].contains(part), "{name}: warning has {part}: {log}");
            }
        }
        assert!(
            !log.contains(NOT_READ),
            "{name}: all frames not read: {log}"
        );
    }
}

/// 2 コマ・2×2（8 画素）の動く絵を 3 つ登録する。`broken` の絵だけ全コマの読み込みが失敗する。
fn three_animations(broken: Option<&str>) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    for (rel, px) in [("anim1.png", K0), ("anim2.png", K1), ("anim3.png", X)] {
        let anim = animated(&[([px; 4], 100), ([Y; 4], 50)], true);
        let frames = if broken == Some(rel) {
            Err("frame 1 is broken".into())
        } else {
            Ok(anim.clone())
        };
        insert_anim(&mut dec, rel, 2, Ok(anim.frames[0].image.clone()), frames);
    }
    dec
}

const ANIMS: [&str; 3] = ["anim1.png", "anim2.png", "anim3.png"];

/// 合計の上限 16 画素: 3 つ目だけが縮み、2 回焼いても同じ絵が縮む（合計は呼び出しごとに 0 から）。
/// 縮めた絵は合計に足さないので、1 つ目が読めずに縮むと 2 つ目・3 つ目は両方載る（要件 6.8）。
#[test]
fn total_limit_shrinks_only_the_third_animation() {
    let limits = AnimationLimits {
        max_total_pixels: 16,
        ..AnimationLimits::default()
    };
    let animated_of = |table: &AtlasTable| {
        ANIMS.map(|rel| {
            let id = table.resolve(SetId(0), rel).unwrap();
            table.animation(id).is_some()
        })
    };

    let dec = three_animations(None);
    let (first, log) = bake_logged(&dec, &ANIMS, limits, PackConfig::default());
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    assert_eq!(animated_of(&first.table), [true, true, false]);
    let anim3 = first.table.resolve(SetId(0), "anim3.png").unwrap();
    assert_eq!(full(&first.table, anim3), vec![X; 4], "frame 0 of anim3");
    let w = warns(&log);
    assert_eq!(w.len(), 1, "{log}");
    assert!(w[0].contains("rel_path=\"anim3.png\"") && w[0].contains("exceeded=\"total_pixels\""));
    assert!(w[0].contains(SET) && w[0].contains(TARGET), "{log}");
    let second = bake_in(&dec, &ANIMS, limits, PackConfig::default());
    assert_eq!(
        dump(&first.table),
        dump(&second.table),
        "same picture shrinks"
    );

    let dec = three_animations(Some("anim1.png"));
    let result = bake_in(&dec, &ANIMS, limits, PackConfig::default());
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(animated_of(&result.table), [false, true, true]);
}

/// 段 1 が `Err` なら段 2（今までの 1 枚読み）の絵が出て `warn!` は 2 回（2 回目に段 1 の理由）。
/// 段 2 も `Err` なら段 3: 失敗の一覧に 1 件で、ほかの絵は載る（要件 6.4〜6.6）。
#[test]
fn first_frame_failure_falls_back_to_the_old_reader_then_to_the_failure_list() {
    let broken = |legacy: bool| {
        let mut dec = stills();
        insert_anim(
            &mut dec,
            "anim.png",
            3,
            Err("16-bit first frame".into()),
            Err("16-bit APNG is not supported".into()),
        );
        if legacy {
            insert_still(&mut dec, "anim.png", image([Y; 4], true));
        } else {
            dec.insert_corrupt(Path::new(BASE).join("anim.png"), "old reader fails too");
        }
        bake_logged(
            &dec,
            &["a.png", "anim.png", "c.png"],
            AnimationLimits::default(),
            PackConfig::default(),
        )
    };

    // 段 2。
    let (result, log) = broken(true);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let parent = result.table.resolve(SetId(0), "anim.png").unwrap();
    assert!(result.table.animation(parent).is_none());
    assert_eq!(
        full(&result.table, parent),
        vec![Y; 4],
        "old reader's picture"
    );
    let w = warns(&log);
    assert_eq!(w.len(), 2, "{log}");
    assert!(w[0].contains(BROKEN), "{log}");
    assert!(
        w[1].contains(FELL_BACK) && w[1].contains("16-bit first frame"),
        "{log}"
    );
    assert!(w[1].contains("rel_path=\"anim.png\""), "{log}");
    assert!(w[1].contains(SET) && w[1].contains(TARGET), "{log}");

    // 段 3。
    let (result, log) = broken(false);
    assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
    assert!(
        result.errors[0]
            .to_string()
            .contains("old reader fails too"),
        "{:?}",
        result.errors
    );
    assert_eq!(warns(&log).len(), 2, "{log}");
    let table = result.table;
    assert!(
        table.resolve(SetId(0), "anim.png").is_none(),
        "not in the table"
    );
    for (rel, px) in [("a.png", X), ("c.png", Y)] {
        let id = table.resolve(SetId(0), rel).expect("other pictures stay");
        assert_eq!(full(&table, id), vec![px; 4], "{rel}");
    }
}

/// 上限を渡さない入口 `bake` は既定の上限で焼く（環境変数は触らない）。
#[test]
fn bake_without_limits_uses_the_default_limits() {
    let dec = with_animation(three_frames());
    let surfaces = shell();
    let sets = [SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    }];
    let table = crate::bake(&sets, &dec, PackConfig::default()).table;
    let parent = table.resolve(SetId(0), "anim.png").unwrap();
    assert_eq!(table.animation(parent).unwrap().frames.len(), 3);
    assert_eq!(dump(&table), dump(&bake(&dec).table));
}
