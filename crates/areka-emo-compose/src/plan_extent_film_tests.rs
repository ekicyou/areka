//! 外形に動く絵の子と `always` を数える（要件 1.2・1.7・6.2・7.1）。
//!
//! 動く絵の子は画像の element と同じ 1 行で原寸を数え、`always` は全部の pattern の先を着せ替えの
//! pattern0 と同じやり方で数える。外形はコマ（[`PatternState`]）に依らない静的な量のまま。

use std::path::Path;

use areka_emo_atlas::{AnimationFrame, AtlasTable, LoopCount, MemoryDecoder};

use super::compute_extent;
use crate::bind::BindSet;
use crate::nesting::FilmId;
use crate::pattern::PatternState;
use crate::plan::Extent;
use crate::plan_test_support::{bake_films_and_bind, frame, insert_film, plan_of, solid};
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
        insert_film(
            &mut dec,
            Path::new(BASE).join(rel),
            frames,
            LoopCount::Infinite,
        );
    }
    let rels = STILLS.iter().chain(FILMS).map(|(rel, _, _)| *rel);
    bake_films_and_bind(&dec, Path::new(BASE), rels, text, images)
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

    let film = compute_extent(&world, &atlas, 0);
    assert_eq!(film, compute_extent(&still_world, &still_atlas, 0));
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

    assert_eq!(compute_extent(&world, &atlas, 1), Extent { w: 30, h: 20 });
    let got = plan_of(&world, &atlas, 1, &BindSet::default(), &Default::default()).1;
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
    assert_eq!(compute_extent(&world, &atlas, 0), Extent { w: 42, h: 54 });
}

/// seriko-pattern-calls 6.8・M6: 純 `bind` は `always` と同じく全部の pattern の先を数える（負の番号と
/// 8 語の欄 2 は数えない）。`bind+random` は今のまま番号の最も小さい 1 つだけ（pattern0 の tall を (0,10) に
/// 数え、pattern1 の big は数えない）。
#[test]
fn pure_bind_extent_is_union_of_all_patterns() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\n\
                animation0.interval,bind\n\
                animation0.pattern0,overlay,30,0,2,0\n\
                animation0.pattern1,overlay,31,0,0,4\n\
                animation0.pattern2,overlay,-1,0,0,0\n\
                animation0.pattern3,start,99,0,0,0\n\
                animation0.pattern4,insert,99,0,0,0\n\
                animation0.pattern5,move,99,0,0,0\n\
                animation1.interval,bind+random,2\n\
                animation1.pattern0,overlay,31,0,0,10\n\
                animation1.pattern1,overlay,99,0,0,0\n}\n\
                surface30\n{\nelement0,overlay,wide.png,0,0\n}\n\
                surface31\n{\nelement0,overlay,tall.png,0,0\n}\n\
                surface99\n{\nelement0,overlay,big.png,0,0\n}\n";
    let (world, atlas) = build_with(text, &[]);
    assert_eq!(compute_extent(&world, &atlas, 0), Extent { w: 42, h: 60 });
}

/// 要件 1.7: コマの欄を替えても外形は同じ（`always` のコマ・動く絵の子のコマ）。外形は束縛のときの表を
/// 引くだけでコマを受け取らないので、両方を替えた計画 1 回で足りる（指摘 118）。
#[test]
fn cells_do_not_change_extent() {
    let text = format!(
        "{ALWAYS}surface2\n{{\nelement0,overlay,body.png,0,0\nelement1,overlay,film.apng,0,0\n}}\n"
    );
    let (world, atlas) = build_with(&text, &[]);
    let film = world.film_sheets().next().expect("子が在る");
    let film_id: FilmId = film.id;
    let other_picture = film.frames[1];

    let mut pattern = PatternState::default();
    pattern.set(0, frame(31, 0, 4));
    pattern.set_film(film_id, other_picture);

    for top in [0, 2] {
        let want = compute_extent(&world, &atlas, top);
        let got = plan_of(&world, &atlas, top, &BindSet::default(), &pattern).1;
        assert_eq!(got, Ok(want), "top={top}");
    }
    assert_eq!(compute_extent(&world, &atlas, 0), Extent { w: 42, h: 54 });
    assert_eq!(compute_extent(&world, &atlas, 2), Extent { w: 30, h: 20 });
}
