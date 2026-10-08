//! 外形に動く絵の子と `always` を数える（task 2.6・要件 1.2・1.7・6.2・7.1）。
//!
//! 動く絵の子は画像の element と同じ 1 行で原寸を数え、`always` は全部の pattern の先を着せ替えの
//! pattern0 と同じやり方で数える。外形はコマ（[`PatternState`]）に依らない静的な量のまま。

use std::collections::BTreeMap;
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, AtlasTable,
    DecodedImage, LoopCount, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha,
    bake_with_limits,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface, parse};

use super::compute_extent;
use crate::bind::BindSet;
use crate::method::ComposeMethod;
use crate::nesting::FilmId;
use crate::pattern::{PatternFrame, PatternState};
use crate::plan::{Extent, build_plan};
use crate::world::EmoWorld;

const BASE: &str = "shell";
/// 静止画（名前, 幅, 高さ）。`still.png` は動く絵 `film.apng` と同じ寸法。
const STILLS: &[(&str, u32, u32)] = &[
    ("body.png", 10, 10),
    ("still.png", 30, 20),
    ("wide.png", 40, 5),
    ("tall.png", 5, 50),
    ("big.png", 300, 300),
];
/// 動く絵（名前, 幅, 高さ）。どちらも 2 コマ・待ち 100 ずつ・終わりなし。
const FILMS: &[(&str, u32, u32)] = &[("film.apng", 30, 20), ("surface1.png", 30, 20)];

fn solid(w: u32, h: u32, c: u8) -> DecodedImage {
    DecodedImage {
        width: w,
        height: h,
        stride: w * 4,
        bgra: [c, c, c, 255].repeat((w * h) as usize),
        has_alpha: true,
    }
}

/// 文面を面の表にして焼き、束縛する。`images` は `surface<数字>.png` として置いた絵。
fn build_with(text: &str, images: &[(u32, &str)]) -> (EmoWorld, AtlasTable) {
    let mut dec = MemoryDecoder::default();
    for &(rel, w, h) in STILLS {
        let img = solid(w, h, 9);
        dec.insert(Path::new(BASE).join(rel), w, h, w * 4, img.bgra, true);
    }
    for &(rel, w, h) in FILMS {
        let frames: Vec<AnimationFrame> = [10, 11]
            .into_iter()
            .map(|c| AnimationFrame {
                image: solid(w, h, c),
                delay_ms: 100,
            })
            .collect();
        dec.insert_animated(
            Path::new(BASE).join(rel),
            AnimationInfo {
                width: w,
                height: h,
                frame_count: frames.len() as u32,
            },
            Ok(frames[0].image.clone()),
            Ok(AnimatedImage {
                frames,
                loop_count: LoopCount::Infinite,
            }),
        );
    }
    let elements = STILLS
        .iter()
        .chain(FILMS)
        .map(|(rel, _, _)| Element {
            layer: 0,
            path: ElementPath::new(rel.to_string()),
            x: 0,
            y: 0,
        })
        .collect();
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake_with_limits(
        &[set],
        &dec,
        PackConfig::default(),
        AnimationLimits::default(),
    );
    assert!(baked.errors.is_empty(), "{:?}", baked.errors);
    let images: BTreeMap<u32, String> = images.iter().map(|(i, p)| (*i, p.to_string())).collect();
    let mut world = EmoWorld::build_with_images(&parse(text), &images);
    world.bind_atlas(&baked.table, SetId(0));
    assert!(world.film_skips().is_empty(), "{:?}", world.film_skips());
    (world, baked.table)
}

fn extent(world: &EmoWorld, atlas: &AtlasTable, id: u32) -> Extent {
    compute_extent(&mut Vec::new(), world, atlas, id)
}

/// 要件 1.7・6.2: 動く絵を置いたサーフェスの外形は、同じ寸法の静止画を置いたときと同じ。
#[test]
fn film_element_extent_equals_same_sized_still() {
    let text = |rel: &str| {
        format!("surface0\n{{\nelement0,overlay,body.png,0,0\nelement1,overlay,{rel},5,7\n}}\n")
    };
    let (world, atlas) = build_with(&text("film.apng"), &[]);
    assert_eq!(world.film_sheets().count(), 1, "動く絵は子へ分解されている");
    let (still_world, still_atlas) = build_with(&text("still.png"), &[]);

    let film = extent(&world, &atlas, 0);
    assert_eq!(film, extent(&still_world, &still_atlas, 0));
    assert_eq!(film, Extent { w: 30, h: 20 });
}

/// 要件 1.2: 動く `surface<数字>.png` だけで建つサーフェスの外形は 0×0 にならず、合成できる。
#[test]
fn animated_base_image_only_surface_is_not_empty() {
    let (world, atlas) = build_with(
        "surface1\n{\ncollision0,0,0,8,8,Body\n}\n",
        &[(1, "surface1.png")],
    );
    assert_eq!(
        world.film_sheets().count(),
        1,
        "surface1.png は子へ分解されている"
    );

    assert_eq!(extent(&world, &atlas, 1), Extent { w: 30, h: 20 });
    let got = build_plan(
        &mut Vec::new(),
        &mut Vec::new(),
        &world,
        &atlas,
        1,
        &BindSet::default(),
        &Default::default(),
    );
    assert_eq!(got, Ok(Extent { w: 30, h: 20 }));
}

/// 手書きの `always` の 2 つの pattern の先（wide を (2,0)・tall を (0,4)）と、数えない 2 つ
/// （止める `-1`・欄 2 が animation の番号になる `start` の 99＝面 99 は 300×300）と、サーフェスの
/// 番号を無視する `move` の 99（ukadoc「サーフェスIDは無視される」）。
const ALWAYS: &str = "surface0\n{\nelement0,overlay,body.png,0,0\n\
                      animation0.interval,always\n\
                      animation0.pattern0,overlay,30,100,2,0\n\
                      animation0.pattern1,overlay,31,100,0,4\n\
                      animation0.pattern2,overlay,-1,100,0,0\n\
                      animation0.pattern3,start,99,100,0,0\n\
                      animation0.pattern4,move,99,100,0,0\n}\n\
                      surface30\n{\nelement0,overlay,wide.png,0,0\n}\n\
                      surface31\n{\nelement0,overlay,tall.png,0,0\n}\n\
                      surface99\n{\nelement0,overlay,big.png,0,0\n}\n";

/// design b-2・議題の裁定 1: 手書きの `always` の外形は全部の pattern の先の和集合。負の番号と、
/// 欄 2 が animation の番号になる語の先は数えない。
#[test]
fn hand_written_always_extent_is_union_of_all_patterns() {
    let (world, atlas) = build_with(ALWAYS, &[]);
    // wide (2+40)×5・tall 5×(4+50)・body 10×10 の和集合。
    assert_eq!(extent(&world, &atlas, 0), Extent { w: 42, h: 54 });
}

/// 要件 1.7: コマの欄を替えても外形は同じ（`always` のコマ・消えている・動く絵の子のコマ）。
#[test]
fn cells_do_not_change_extent() {
    let text = format!(
        "{ALWAYS}surface2\n{{\nelement0,overlay,body.png,0,0\nelement1,overlay,film.apng,0,0\n}}\n"
    );
    let (world, atlas) = build_with(&text, &[]);
    let film = world.film_sheets().next().expect("子が在る");
    let film_id: FilmId = film.id;
    let other_picture = film.frames[1];

    let mut cells = vec![PatternState::default()];
    let mut p = PatternState::default();
    p.set(
        0,
        PatternFrame {
            surface_id: 31,
            method: ComposeMethod::Overlay,
            x: 0,
            y: 4,
        },
    );
    cells.push(p);
    let mut p = PatternState::default();
    p.set_blank(0);
    cells.push(p);
    let mut p = PatternState::default();
    p.set_film(film_id, other_picture);
    cells.push(p);

    for top in [0, 2] {
        let want = extent(&world, &atlas, top);
        for pattern in &cells {
            let got = build_plan(
                &mut Vec::new(),
                &mut Vec::new(),
                &world,
                &atlas,
                top,
                &BindSet::default(),
                pattern,
            );
            assert_eq!(got, Ok(want), "top={top} pattern={pattern:?}");
        }
    }
    assert_eq!(extent(&world, &atlas, 0), Extent { w: 42, h: 54 });
    assert_eq!(extent(&world, &atlas, 2), Extent { w: 30, h: 20 });
}
