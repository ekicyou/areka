//! 動く絵の子の鍵（要件 1.12）と、`always` の見分け・経過 0 の求め方（要件 4.8・4.9）。
//!
//! `NestReport::from_world` の振り分けは `_` の腕で「無い番号」に落ちるので、`ElementKind::Film`
//! の扱いを書き漏らしてもコンパイラは教えない。ここを檻にする。
//!
//! 後半は見える部品と見える子（task 2.4・要件 1.9・3.7）: `SurfaceParts` の `films`・`always_rest`、
//! `walk` の経過 0 の辺（欄が「載っていない」ときだけ・「消えている」では進まない）、`visible_films`。

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, AtlasTable,
    DecodedImage, LoopCount, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha,
    bake_with_limits,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Interval, Surface, parse};

use super::{
    ElementKind, FilmId, NestTable, PartKey, SurfaceParts, element_kind, is_always_interval,
    rest_index,
};
use crate::bind::BindSet;
use crate::method::ComposeMethod;
use crate::normalized::SurfaceMaster;
use crate::pattern::{PatternFrame, PatternState};
use crate::world::{EmoWorld, SurfaceIndex};

fn other(s: &str) -> Interval {
    Interval::Other(s.into())
}

/// `always` の単独・小文字の完全一致だけが真（要件 4.8・4.9）。
#[test]
fn only_exact_lowercase_always_is_always() {
    assert!(is_always_interval(&other("always")));
    for s in [
        "bind+always",
        "always+bind",
        "Always",
        "always ",
        " always",
        "",
    ] {
        assert!(!is_always_interval(&other(s)), "{s:?} は always ではない");
    }
    assert!(!is_always_interval(&Interval::Bind));
    assert!(!is_always_interval(&Interval::Random { k: 2 }));
    assert!(!is_always_interval(&Interval::BindRandom { k: 2 }));
}

/// 経過 0 のコマ＝待ち時間の累積が 0 の最後の番号。先頭から待つなら無い。
#[test]
fn rest_index_is_last_index_with_zero_cumulative_wait() {
    assert_eq!(rest_index([0, 0, 100].into_iter()), Some(1));
    assert_eq!(rest_index([0, 100, 0].into_iter()), Some(0));
    assert_eq!(rest_index([0, 0, 0].into_iter()), Some(2));
    assert_eq!(rest_index([100, 0].into_iter()), None);
    assert_eq!(rest_index(std::iter::empty()), None);
}

/// 作者の欄の読み分けは動く絵の子を返さない（作者は子を書けない・要件 1.12）。
#[test]
fn element_kind_never_reads_as_film() {
    for s in ["0", "7", "0100", "4294967296", "body.png", "", "-1"] {
        let kind = element_kind(&ElementPath::new(s.to_string()));
        assert!(!matches!(kind, ElementKind::Film(_)), "{s:?} → {kind:?}");
    }
}

/// 鍵の 2 つの種類は、同じ数でも等しくならない（番号が当たらない・要件 1.12）。
#[test]
fn part_key_kinds_never_collide() {
    assert_ne!(PartKey::Surface(3), PartKey::Film(FilmId(3)));
}

/// `ElementKind::Film` の element は「無い番号」にも「循環」にも数えない（子は先を持たない）。
///
/// 子を置く element を作るのは分解の仕事なので、ここでは読み込んだ面の表の element を直に替える。
/// 番号 0 は面の表に在る（surface0 自身）ので、`Film` が `Surface` の辺に混ざれば循環に、
/// `_` の腕へ落ちれば「無い番号」に数えられて赤になる。
#[test]
fn nest_report_counts_film_as_neither_missing_nor_cycle() {
    let mut world = EmoWorld::build(&parse(
        "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,eye.png,0,0\n}\n",
    ));
    let entity = world.world().resource::<SurfaceIndex>().0[&0];
    world
        .world_mut()
        .get_mut::<SurfaceMaster>(entity)
        .expect("surface0 は在る")
        .elements[1]
        .kind = ElementKind::Film(FilmId(0));
    assert_eq!(world.nest_report().issues, vec![]);
}

// ---- 見える部品と見える子（task 2.4） ----

const FIXTURE: &str = include_str!("../tests/fixtures/animated-playback/shell/surfaces.txt");
const BASE: &str = "shell";

fn frame(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

/// 見える部品と見える子（着せ替えは無し）。
fn visible(table: &NestTable, top: u32, pattern: &PatternState) -> (Vec<u32>, Vec<FilmId>) {
    let mut parts = Vec::new();
    table.visible_parts(top, &BindSet::default(), pattern, &mut parts);
    let mut films = vec![FilmId(u32::MAX)]; // 先に空にすることも確かめる。
    table.visible_films(top, &parts, &mut films);
    (parts, films)
}

/// 全面が 1 色・不透明の 2×2。
fn solid(c: u8) -> DecodedImage {
    DecodedImage {
        width: 2,
        height: 2,
        stride: 8,
        bgra: [c, c, c, 255].repeat(4),
        has_alpha: true,
    }
}

/// 検体のシェルを、検体の README のコマ・待ち時間・繰り返しの動く絵（中身は 2×2 の単色）で焼いて
/// 束縛する（本物の分解を通す）。`surface2.png` は element0 が在るので土台にしない（本番と同じ）。
fn fixture_world() -> (EmoWorld, AtlasTable) {
    let mut dec = MemoryDecoder::default();
    let stills = ["body.png", "part.png", "yellow.png", "cyan.png"];
    for rel in stills {
        dec.insert(Path::new(BASE).join(rel), 2, 2, 8, solid(9).bgra, true);
    }
    let films: [(&str, &[u32], Option<u32>); 4] = [
        ("rgb.apng", &[100, 100], None),
        ("rgb.webp", &[100, 100], None),
        ("alpha.webp", &[100, 0, 70], Some(3)),
        ("surface1.png", &[333, 0, 70, 1], Some(2)),
    ];
    for (rel, delays, laps) in films {
        let frames = delays
            .iter()
            .enumerate()
            .map(|(i, &delay_ms)| AnimationFrame {
                image: solid(10 + i as u8),
                delay_ms,
            })
            .collect();
        let loop_count = laps.map_or(LoopCount::Infinite, |n| {
            LoopCount::Finite(NonZeroU32::new(n).unwrap())
        });
        dec.insert_animated(
            Path::new(BASE).join(rel),
            AnimationInfo {
                width: 2,
                height: 2,
                frame_count: delays.len() as u32,
            },
            Ok(solid(10)),
            Ok(AnimatedImage { frames, loop_count }),
        );
    }
    let rels = stills.into_iter().chain(films.iter().map(|f| f.0));
    let elements = rels
        .map(|rel| Element {
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
    let images = BTreeMap::from([(1, "surface1.png".to_string())]);
    let mut world = EmoWorld::build_with_images(&parse(FIXTURE), &images);
    world.bind_atlas(&baked.table, SetId(0));
    assert!(world.film_skips().is_empty(), "{:?}", world.film_skips());
    (world, baked.table)
}

fn film_of(atlas: &AtlasTable, rel: &str) -> FilmId {
    FilmId(atlas.resolve(SetId(0), rel).unwrap().0)
}

fn films_of(atlas: &AtlasTable, rels: &[&str]) -> Vec<FilmId> {
    let mut v: Vec<FilmId> = rels.iter().map(|r| film_of(atlas, r)).collect();
    v.sort_unstable();
    v
}

/// 要件 1.9・3.7: 検体の各サーフェスを一番上にしたとき（コマの欄は空＝全部が経過 0）の見える部品と
/// 見える子。子は一番上（0・1・3）にも部品（0 の element4 が置く 20 の rgb.webp）にも数える。
/// 経過 0 の先は、待ち 0 で始まる `always` の 10 だけ（0・20 の `always` は先頭から待つので無い・
/// 11 の `bind+always` は `always` ではない）。
#[test]
fn fixture_visible_parts_and_films_at_rest() {
    let (world, atlas) = fixture_world();
    let table = world.nest_table();
    let empty = PatternState::default();
    let cases: [(u32, &[u32], &[&str]); 9] = [
        (0, &[20], &["rgb.apng", "alpha.webp", "rgb.webp"]),
        (1, &[], &["surface1.png"]),
        (2, &[], &[]),
        (3, &[], &["rgb.apng"]),
        (10, &[31], &[]),
        (11, &[], &[]),
        (20, &[], &["rgb.webp"]),
        (30, &[], &[]),
        (31, &[], &[]),
    ];
    for (top, parts, films) in cases {
        assert_eq!(
            visible(&table, top, &empty),
            (parts.to_vec(), films_of(&atlas, films)),
            "一番上 {top}"
        );
    }
}

/// 要件 1.9: pattern の先に置かれた子も見える。コマが在れば経過 0 の先は数えず、コマの先を数える。
#[test]
fn fixture_films_behind_pattern_targets_are_visible() {
    let (world, atlas) = fixture_world();
    let table = world.nest_table();

    // 10 の always のコマが 20 を指す → 経過 0 の先 31 は消え、20 と 20 の子が見える。
    let mut pattern = PatternState::default();
    pattern.set(0, frame(20));
    assert_eq!(
        visible(&table, 10, &pattern),
        (vec![20], films_of(&atlas, &["rgb.webp"]))
    );

    // 0 の always のコマが 3 を指す → 3 の子（0 と同じ rgb.apng）は重ねて数えない。
    let mut pattern = PatternState::default();
    pattern.set(0, frame(3));
    assert_eq!(
        visible(&table, 0, &pattern),
        (
            vec![3, 20],
            films_of(&atlas, &["rgb.apng", "alpha.webp", "rgb.webp"])
        )
    );

    // 部品 20 の always の部品の欄のコマが 1 を指す → 1 の土台の子が見える。
    let mut pattern = PatternState::default();
    pattern.set_part(20, 0, frame(1));
    assert_eq!(
        visible(&table, 0, &pattern),
        (
            vec![1, 20],
            films_of(
                &atlas,
                &["rgb.apng", "alpha.webp", "rgb.webp", "surface1.png"]
            )
        )
    );
}

/// 動く絵だけを置いたサーフェス（土台の `surface1.png` だけ）の行が載る。経過 0 の先だけの 10 も載る。
#[test]
fn fixture_rows_for_films_only_and_always_rest_only() {
    let (world, atlas) = fixture_world();
    let table = world.nest_table();
    assert_eq!(
        table.parts(1),
        Some(&SurfaceParts {
            films: vec![film_of(&atlas, "surface1.png")],
            ..Default::default()
        })
    );
    assert_eq!(
        table.parts(10),
        Some(&SurfaceParts {
            always_rest: vec![(0, 31)],
            ..Default::default()
        })
    );
    // 同じ絵を 2 か所に置いても子は 1 つ（昇順・重複なし）。
    assert_eq!(
        table.parts(0).map(|p| p.films.clone()),
        Some(films_of(&atlas, &["rgb.apng", "alpha.webp"]))
    );
    // 動く絵も入れ子も always の経過 0 の先も無い番号は載らない。
    for id in [2, 11, 30, 31] {
        assert_eq!(table.parts(id), None, "面 {id}");
    }
}

/// 経過 0 の辺は一番上の欄・部品の欄のそれぞれで「載っていない」ときだけ進む。「消えている」なら
/// 進まず、別の場所の欄（一番上と部品）は混ざらない。経過 0 の pattern の番号が負・描画メソッドが
/// 動かない・先頭から待つ・`always` でないなら辺は無い。
#[test]
fn always_rest_edge_follows_only_rest_cells() {
    let table = EmoWorld::build(&parse(
        "surface0\n{\nelement0,overlay,5,0,0\nanimation0.interval,always\n\
         animation0.pattern0,overlay,8,0,0,0\nanimation0.pattern1,overlay,9,100,0,0\n}\n\
         surface5\n{\nanimation0.interval,always\n\
         animation0.pattern1,overlay,7,0,0,0\nanimation0.pattern0,overlay,6,0,0,0\n}\n\
         surface6\n{\n}\nsurface7\n{\n}\nsurface8\n{\n}\nsurface9\n{\n}\n\
         surface40\n{\nanimation0.interval,always\nanimation0.pattern0,overlay,-1,0,0,0\n\
         animation1.interval,always\nanimation1.pattern0,start,6,0,0,0\n\
         animation2.interval,always\nanimation2.pattern0,overlay,6,10,0,0\n\
         animation3.interval,sometimes\nanimation3.pattern0,overlay,6,0,0,0\n}\n",
    ))
    .nest_table();
    let parts = |pattern: &PatternState| visible(&table, 0, pattern).0;

    // 一番上の経過 0 は pattern0 の 8、部品 5 の経過 0 は番号の順で最後の待ち 0 の pattern1 の 7。
    assert_eq!(
        table.parts(5).map(|p| p.always_rest.clone()),
        Some(vec![(0, 7)])
    );
    assert_eq!(parts(&PatternState::default()), vec![5, 7, 8]);

    let mut p = PatternState::default();
    p.set_blank(0);
    assert_eq!(parts(&p), vec![5, 7], "一番上が消えている");

    let mut p = PatternState::default();
    p.set_part_blank(5, 0);
    assert_eq!(parts(&p), vec![5, 8], "部品 5 が消えている");

    let mut p = PatternState::default();
    p.set(0, frame(9));
    p.set_part(5, 0, frame(6));
    assert_eq!(parts(&p), vec![5, 6, 9], "コマが経過 0 を置き換える");

    // 部品の欄の「消えている」は一番上の同じ番号の欄に効かない（逆も）。
    let mut p = PatternState::default();
    p.set_part_blank(0, 0);
    p.set_blank(5);
    assert_eq!(parts(&p), vec![5, 7, 8]);

    // 負の番号・動かない描画メソッド・先頭から待つ・always でない → 辺が無く、行も載らない。
    assert_eq!(table.parts(40), None);
}

/// 動く絵も `always` も無い表では、足した 2 欄はどの行でも空（答えは本 spec の前と同じ）。
#[test]
fn tables_without_films_or_always_keep_new_columns_empty() {
    let fixture = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");
    let images = BTreeMap::from([
        (2, "surface2.png".to_string()),
        (11, "surface11.png".to_string()),
    ]);
    let world = EmoWorld::build_with_images(&parse(fixture), &images);
    let table = world.nest_table();
    assert!(!table.is_empty());
    for id in world.surface_ids() {
        if let Some(p) = table.parts(id) {
            assert!(p.films.is_empty() && p.always_rest.is_empty(), "面 {id}");
        }
        assert_eq!(visible(&table, id, &PatternState::default()).1, vec![]);
    }
}
