//! 本物の読み手で検体のフォルダを焼く結合テスト（spec: areka-P0-animated-image-decode
//! 要件 1.2・1.3・1.4・3.3・4.3・8.3〜8.6）。
//!
//! `WicDecoderArm` で `testdata/animated/`（中身は同じフォルダの README）の検体を 1 つのシェルに
//! 並べて焼く。動く絵の全コマと縮めた 1 枚は `image` が読むので、結果は Windows の WebP の拡張機能の
//! 有無に依らない。そのため WIC の WebP の読み手が要る `single.webp` と、段 2（WIC）へ落ちる
//! `deep16.apng` はここに入れない（どちらも偽の読み手・兄弟のテストが判定する）。

use std::path::PathBuf;

use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use crate::{
    AlphaParams, AnimationLimits, AtlasTable, BakeResult, ElementId, PackConfig, SetId, SurfaceSet,
    UseSelfAlpha, WicDecoderArm, bake_with_limits,
};

type Px = [u8; 4];

const CLEAR: Px = [0, 0, 0, 0];
/// 乗算済み BGRA の赤。
const RED: Px = [0, 0, 255, 255];

/// 焼く検体（拡張子が `.png` の WebP・1 コマの APNG・GIF・途中で切れた APNG を含む）。
const SAMPLES: &[&str] = &[
    "basic.apng",
    "alpha.webp",
    "webp_named.png",
    "rgb.webp",
    "default_image.apng",
    "rgb.apng",
    "trns.apng",
    "truncated.apng",
    "single.apng",
    "two_frames.gif",
];

/// 全コマが載るはずの動く絵とコマの枚数。
const ANIMATED: &[(&str, usize)] = &[
    ("basic.apng", 4),
    ("alpha.webp", 3),
    ("webp_named.png", 3),
    ("rgb.webp", 2),
    ("default_image.apng", 2),
    ("rgb.apng", 2),
    ("trns.apng", 2),
];

const OVER: &str = "bake: 動く絵が上限を超えたので 1 枚だけ読みます";
const BROKEN: &str = "bake: 動く絵として読めなかったので 1 枚だけ読みます";
const STAGE2: &str = "bake: 動く絵の 1 枚目を読めなかったので";

/// 検体を面 0 に 1 層ずつ並べ、本物の読み手で焼く（COM を張った下で）。ログも返す。
fn bake_samples(limits: AnimationLimits) -> (BakeResult, String) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/testdata/animated");
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements: SAMPLES
            .iter()
            .enumerate()
            .map(|(layer, name)| Element {
                layer: layer as u32,
                path: ElementPath::new(name.to_string()),
                x: 0,
                y: 0,
            })
            .collect(),
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let sets = [SurfaceSet {
        surfaces: &surfaces,
        base_dir: &base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    }];
    let mut result = None;
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let log = crate::log_capture::capture_logs(|| {
        let arm = WicDecoderArm::new().expect("WIC factory creates under COM init");
        result = Some(bake_with_limits(&sets, &arm, PackConfig::default(), limits));
    });
    unsafe {
        CoUninitialize();
    }
    let result = result.unwrap();
    assert!(result.errors.is_empty(), "no failures: {:?}", result.errors);
    (result, log)
}

fn id(table: &AtlasTable, name: &str) -> ElementId {
    table
        .resolve(SetId(0), name)
        .unwrap_or_else(|| panic!("{name} is in the table"))
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

/// エントリの見え方（寸法・切り詰めの位置と大きさ・原寸へ戻した画素）。ページ上の位置は含めない。
fn view(table: &AtlasTable, id: ElementId) -> String {
    let e = table.entry(id);
    let trim = e
        .placement
        .as_ref()
        .map(|p| (p.trim_offset, p.uv_rect.w, p.uv_rect.h));
    format!("{:?} {:?} {:?}", e.original, trim, full(table, id))
}

/// `name` について出た `warn!` の行。
fn warns_for<'a>(log: &'a str, name: &str) -> Vec<&'a str> {
    let rel = format!("rel_path=\"{name}\"");
    log.lines()
        .filter(|l| l.contains("level=WARN") && l.contains(&rel))
        .collect()
}

/// 既定の上限で焼くと、動く 2 形式は全コマが載り、今までの鍵は 0 番のコマを引く。GIF と 1 コマの
/// APNG は動く絵の答えが無しで 1 枚。途中で切れた APNG は `warn!` 1 回で 0 番のコマ 1 枚へ縮む。
#[test]
fn samples_folder_bakes_every_frame_and_single_pictures() {
    let (result, log) = bake_samples(AnimationLimits::default());
    let table = &result.table;

    for &(name, count) in ANIMATED {
        let parent = id(table, name);
        let anim = table
            .animation(parent)
            .unwrap_or_else(|| panic!("{name} is animated"));
        assert_eq!(anim.frames.len(), count, "{name} frames");
        assert_eq!(anim.frames[0], parent, "{name}: key resolves to frame 0");
        for (n, &f) in anim.frames.iter().enumerate() {
            let e = table.entry(f);
            assert_eq!((e.original.w, e.original.h), (8, 8), "{name} frame {n}");
        }
        assert!(
            warns_for(&log, name).is_empty(),
            "{name}: no warning: {log}"
        );
    }
    // 拡張子が `.png` の WebP は `alpha.webp` と同じ絵になる（要件 1.2）。
    let (webp, named) = (id(table, "alpha.webp"), id(table, "webp_named.png"));
    let (wf, nf) = (
        &table.animation(webp).unwrap().frames,
        &table.animation(named).unwrap().frames,
    );
    for (n, (&a, &b)) in wf.iter().zip(nf).enumerate() {
        assert_eq!(view(table, a), view(table, b), "webp_named.png frame {n}");
    }
    // 1 番のコマは 0 番が背景へ戻って左上が透明（固定コミットの `image-webp` の分かれ目）。
    assert_eq!(full(table, wf[1])[0], CLEAR, "alpha.webp frame 1 top-left");

    // GIF と 1 コマの APNG は 1 枚の絵（要件 1.3・1.4）。
    let gif = id(table, "two_frames.gif");
    assert!(table.animation(gif).is_none(), "gif is one picture");
    // GIF は透明度を持たないので、0 番のコマ（全面が赤）の左上の色が抜けて全部が透明になる。
    // 読んだのが 0 番（赤）であることは抜いた色の記録で見る（1 番は緑）。
    let e = table.entry(gif);
    assert_eq!((e.original.w, e.original.h), (2, 2), "gif size");
    assert_eq!(
        full(table, gif),
        vec![CLEAR; 4],
        "gif: red key color cleared"
    );
    assert!(
        log.lines().any(
            |l| l.contains("rel_path=\"two_frames.gif\"") && l.contains(" b=0 g=0 r=255 a=255")
        ),
        "gif: frame 0 (red) was read: {log}"
    );
    let single = id(table, "single.apng");
    assert!(
        table.animation(single).is_none(),
        "single.apng is one picture"
    );
    assert_eq!(full(table, single), vec![RED; 64], "single.apng is red");
    // 1 枚の絵は動く絵の道を通らない（全透明になった GIF の今までの `warn!` は別物）。
    for name in ["two_frames.gif", "single.apng"] {
        assert!(
            warns_for(&log, name)
                .iter()
                .all(|l| !l.contains(OVER) && !l.contains(BROKEN)),
            "{name}: no animation warning: {log}"
        );
    }

    // 途中で切れた APNG は全コマの読み込みに失敗し、0 番のコマ（`basic.apng` の 0 番）1 枚へ縮む。
    let truncated = id(table, "truncated.apng");
    assert!(table.animation(truncated).is_none(), "truncated shrinks");
    assert_eq!(
        view(table, truncated),
        view(table, id(table, "basic.apng")),
        "truncated shows frame 0 of basic.apng"
    );
    let w = warns_for(&log, "truncated.apng");
    assert_eq!(w.len(), 1, "truncated: one warning: {log}");
    assert!(w[0].contains(BROKEN), "truncated: broken warning: {log}");
    assert!(!log.contains(STAGE2), "no fallback to WIC: {log}");
}

/// 上限を 1 コマにして焼くと、どの動く絵も 1 枚へ縮み、その 1 枚は上限を広げて焼いたときの
/// 0 番のコマと同じ（`default_image.apng` は既定の絵の黒でなく赤・`rgb.webp` は左上の色が抜ける）。
/// 縮めた 1 枚は段 1（`image`）で読むので、WIC へは落ちない。
#[test]
fn one_frame_limit_shrinks_to_frame_zero_of_the_wide_bake() {
    let (wide, _) = bake_samples(AnimationLimits::default());
    let (narrow, log) = bake_samples(AnimationLimits {
        max_frames: 1,
        ..AnimationLimits::default()
    });
    let (wide, narrow) = (&wide.table, &narrow.table);

    for name in ANIMATED
        .iter()
        .map(|&(name, _)| name)
        .chain(["truncated.apng"])
    {
        let parent = id(narrow, name);
        assert!(narrow.animation(parent).is_none(), "{name} shrinks");
        // 途中で切れた APNG は広げても縮むので、比べる先は `basic.apng` の 0 番のコマ。
        let wide_name = if name == "truncated.apng" {
            "basic.apng"
        } else {
            name
        };
        let wide_parent = id(wide, wide_name);
        assert!(
            wide.animation(wide_parent).is_some(),
            "{wide_name}: wide bake keeps all frames"
        );
        assert_eq!(
            view(narrow, parent),
            view(wide, wide_parent),
            "{name}: same as frame 0 of the wide bake"
        );
        let w = warns_for(&log, name);
        assert_eq!(w.len(), 1, "{name}: one warning: {log}");
        assert!(w[0].contains(OVER), "{name}: limit warning: {log}");
        assert!(w[0].contains("exceeded=\"frames\""), "{name}: {log}");
    }

    let default_image = full(narrow, id(narrow, "default_image.apng"));
    assert_eq!(
        default_image,
        vec![RED; 64],
        "default_image.apng is red, not black"
    );
    let rgb = full(narrow, id(narrow, "rgb.webp"));
    assert_eq!(rgb[0], CLEAR, "rgb.webp: top-left color cleared");
    assert_eq!(rgb[63], RED, "rgb.webp: right half stays red");
    assert!(!log.contains(STAGE2), "no fallback to WIC: {log}");
}
