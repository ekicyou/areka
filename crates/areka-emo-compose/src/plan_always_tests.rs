//! 合成で、動く絵の子と `always` の経過 0 を描く（task 2.5・要件 1.4・3.1・4.1・4.5・4.6・7.1）。
//!
//! 欄が「載っていない」なら経過 0 の絵、「消えている」なら何も、コマならそのコマ。動く絵の子は
//! 下に何も敷かず、コマの絵 1 枚だけを置いた element の描画メソッドで命令にする。

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, AtlasTable,
    DecodedImage, ElementId, LoopCount, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha,
    bake_with_limits,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface, parse};

use crate::bind::BindSet;
use crate::log_capture::capture_logs;
use crate::method::ComposeMethod;
use crate::nesting::{ElementKind, FilmId};
use crate::normalized::SurfaceMaster;
use crate::pattern::{PatternFrame, PatternState};
use crate::plan::{BlitOp, derive_ops};
use crate::world::{EmoWorld, SurfaceIndex};

const BASE: &str = "shell";
/// 静止画（小さな文面の分と検体の分）。
const STILLS: &[&str] = &[
    "body.png",
    "a.png",
    "b.png",
    "part.png",
    "yellow.png",
    "cyan.png",
];
/// 小さな文面の動く絵（コマは [`film_frames`]）。
const FILM: &str = "film.apng";
/// 検体の動く絵（README のコマの待ち時間と繰り返し・中身は 2×2 の単色）。
const FIXTURE_FILMS: [(&str, &[u32], Option<u32>); 4] = [
    ("rgb.apng", &[100, 100], None),
    ("rgb.webp", &[100, 100], None),
    ("alpha.webp", &[100, 0, 70], Some(3)),
    ("surface1.png", &[333, 0, 70, 1], Some(2)),
];

fn image(bgra: Vec<u8>) -> DecodedImage {
    DecodedImage {
        width: 2,
        height: 2,
        stride: 8,
        bgra,
        has_alpha: true,
    }
}

fn solid(c: u8) -> DecodedImage {
    image([c, c, c, 255].repeat(4))
}

/// 動く絵のコマ: 0 は全面不透明・1 は左上の 1 画素だけ不透明・2 は全透明。待ちは 0・100・100 なので
/// 経過 0 はコマ 1（1 枚目の待ちが 0 なら次のコマ）。
fn film_frames() -> Vec<AnimationFrame> {
    let mut one = vec![0u8; 16];
    one[..4].copy_from_slice(&[20, 20, 20, 255]);
    [solid(10), image(one), image(vec![0; 16])]
        .into_iter()
        .zip([0, 100, 100])
        .map(|(image, delay_ms)| AnimationFrame { image, delay_ms })
        .collect()
}

fn insert_film(dec: &mut MemoryDecoder, rel: &str, frames: Vec<AnimationFrame>, laps: Option<u32>) {
    let loop_count = laps.map_or(LoopCount::Infinite, |n| {
        LoopCount::Finite(NonZeroU32::new(n).unwrap())
    });
    dec.insert_animated(
        Path::new(BASE).join(rel),
        AnimationInfo {
            width: 2,
            height: 2,
            frame_count: frames.len() as u32,
        },
        Ok(frames[0].image.clone()),
        Ok(AnimatedImage { frames, loop_count }),
    );
}

/// 文面を面の表にし、静止画と動く絵（どれも 2×2）で焼いて束縛する。`images` は
/// `surface<数字>.png` として置いた絵（番号, 名前）。
fn build_with(text: &str, images: &[(u32, &str)]) -> (EmoWorld, AtlasTable) {
    let mut dec = MemoryDecoder::default();
    for rel in STILLS {
        dec.insert(Path::new(BASE).join(rel), 2, 2, 8, solid(9).bgra, true);
    }
    insert_film(&mut dec, FILM, film_frames(), None);
    for (rel, delays, laps) in FIXTURE_FILMS {
        let frames = delays
            .iter()
            .enumerate()
            .map(|(i, &delay_ms)| AnimationFrame {
                image: solid(10 + i as u8),
                delay_ms,
            })
            .collect();
        insert_film(&mut dec, rel, frames, laps);
    }
    let rels = STILLS
        .iter()
        .copied()
        .chain([FILM])
        .chain(FIXTURE_FILMS.iter().map(|f| f.0));
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
    let images: BTreeMap<u32, String> = images.iter().map(|(i, p)| (*i, p.to_string())).collect();
    let mut world = EmoWorld::build_with_images(&parse(text), &images);
    world.bind_atlas(&baked.table, SetId(0));
    assert!(world.film_skips().is_empty(), "{:?}", world.film_skips());
    (world, baked.table)
}

fn build(text: &str) -> (EmoWorld, AtlasTable) {
    build_with(text, &[])
}

fn film_id(atlas: &AtlasTable) -> FilmId {
    FilmId(atlas.resolve(SetId(0), FILM).expect("焼いてある").0)
}

/// 命令の絵の名前（静止画はファイル名、動く絵のコマは `<ファイル名>#<コマの番号>`）。
fn name(world: &EmoWorld, atlas: &AtlasTable, id: ElementId) -> String {
    if let Some(rel) = STILLS
        .iter()
        .find(|rel| atlas.resolve(SetId(0), rel) == Some(id))
    {
        return rel.to_string();
    }
    world
        .film_sheets()
        .find_map(|s| {
            let k = s.frames.iter().position(|&f| f == id.0)?;
            Some(format!("{}#{k}", s.path))
        })
        .expect("命令の絵は既知")
}

/// `top` を合成した命令列（(絵の名前, X, Y)）と、命令そのもの・記録。
fn compose(
    world: &EmoWorld,
    atlas: &AtlasTable,
    top: u32,
    pattern: &PatternState,
) -> (Vec<(String, i64, i64)>, Vec<BlitOp>, String) {
    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let logs = capture_logs(|| {
        derive_ops(
            &mut ops,
            &mut visited,
            world,
            atlas,
            top,
            &BindSet::default(),
            pattern,
        )
    });
    let named = ops
        .iter()
        .map(|op| {
            let (x, y) = op.transform.offset();
            (name(world, atlas, op.element), x, y)
        })
        .collect();
    (named, ops, logs)
}

fn ops_of(text: &str, top: u32, pattern: &PatternState) -> Vec<(String, i64, i64)> {
    let (world, atlas) = build(text);
    compose(&world, &atlas, top, pattern).0
}

fn op(name: &str, x: i64, y: i64) -> (String, i64, i64) {
    (name.to_string(), x, y)
}

fn frame(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

/// a.png の 5・b.png の 6（手書きの `always` のコマが指す先）。
const TARGETS: &str = "surface5\n{\nelement0,overlay,a.png,0,0\n}\n\
                       surface6\n{\nelement0,overlay,b.png,0,0\n}\n";

/// 要件 3.1・4.1: 一番上の手書きの `always` は、欄が空なら経過 0 の pattern（待ち 0 の pattern0）を
/// pattern の X,Y で描く。「消えている」なら何も描かず（4.5）、コマならコマが置き換える。
#[test]
fn top_always_draws_rest_pattern_when_cell_is_empty() {
    let text = format!(
        "surface0\n{{\nelement0,overlay,body.png,0,0\nanimation0.interval,always\n\
         animation0.pattern0,overlay,5,0,3,4\nanimation0.pattern1,overlay,6,100,0,0\n}}\n{TARGETS}"
    );
    let (world, atlas) = build(&text);
    let ops = |p: &PatternState| compose(&world, &atlas, 0, p).0;

    assert_eq!(
        ops(&PatternState::default()),
        vec![op("body.png", 0, 0), op("a.png", 3, 4)]
    );

    let mut blank = PatternState::default();
    blank.set_blank(0);
    assert_eq!(ops(&blank), vec![op("body.png", 0, 0)], "消えている");

    let mut cell = PatternState::default();
    cell.set(0, frame(6));
    assert_eq!(
        ops(&cell),
        vec![op("body.png", 0, 0), op("b.png", 0, 0)],
        "コマが経過 0 を置き換える"
    );
}

/// 要件 3.1・4.1: 部品（element定義で置いたサーフェス）の手書きの `always` も、部品の欄が空なら経過 0
/// を描く。部品の欄の「消えている」で消え、一番上の同じ番号の欄は効かない。
#[test]
fn part_always_draws_rest_pattern_when_part_cell_is_empty() {
    let text = format!(
        "surface0\n{{\nelement0,overlay,body.png,0,0\nelement1,overlay,20,10,10\n}}\n\
         surface20\n{{\nelement0,overlay,part.png,0,0\nanimation0.interval,always\n\
         animation0.pattern0,overlay,5,0,1,1\nanimation0.pattern1,overlay,6,100,0,0\n}}\n{TARGETS}"
    );
    let (world, atlas) = build(&text);
    let ops = |p: &PatternState| compose(&world, &atlas, 0, p).0;
    let at_rest = vec![
        op("body.png", 0, 0),
        op("part.png", 10, 10),
        op("a.png", 11, 11),
    ];

    assert_eq!(ops(&PatternState::default()), at_rest);

    let mut top_blank = PatternState::default();
    top_blank.set_blank(0);
    assert_eq!(ops(&top_blank), at_rest, "一番上の欄は部品に効かない");

    let mut blank = PatternState::default();
    blank.set_part_blank(20, 0);
    assert_eq!(
        ops(&blank),
        vec![op("body.png", 0, 0), op("part.png", 10, 10)],
        "部品の欄が消えている"
    );
}

/// 要件 4.6: 待ち時間の合計が 0 の手書きの `always` は、1 周だけ評価した絵（最後のコマ）を描く。
#[test]
fn zero_total_always_draws_the_picture_after_one_lap() {
    let text = format!(
        "surface0\n{{\nelement0,overlay,body.png,0,0\nanimation0.interval,always\n\
         animation0.pattern0,overlay,5,0,0,0\nanimation0.pattern1,overlay,6,0,0,0\n}}\n{TARGETS}"
    );
    assert_eq!(
        ops_of(&text, 0, &PatternState::default()),
        vec![op("body.png", 0, 0), op("b.png", 0, 0)]
    );
}

/// 要件 1.4・3.1: 動く絵の子は、欄が空なら経過 0 のコマ（1 枚目の待ちが 0 なのでコマ 1）を、置いた
/// element の位置・描画メソッドで 1 枚だけ描く（下に 1 枚目を敷かない＝透明な所から透けない）。
/// コマの欄が在ればそのコマ、全透明のコマは命令にならない。
#[test]
fn film_child_draws_one_frame_with_the_placing_method() {
    let (mut world, atlas) = build(
        "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,film.apng,4,6\n\
         element2,overlay,part.png,0,0\n}\n",
    );
    // element定義の描画メソッドは読み込みで overlay に揃うので、運ぶことを見るために直に替える。
    let entity = world.world().resource::<SurfaceIndex>().0[&0];
    world
        .world_mut()
        .get_mut::<SurfaceMaster>(entity)
        .expect("surface0 は在る")
        .elements[1]
        .method = ComposeMethod::Asis;
    let film = film_id(&atlas);
    assert_eq!(world.film_sheet(film).map(|s| s.rest), Some(1));

    let (named, ops, _) = compose(&world, &atlas, 0, &PatternState::default());
    assert_eq!(
        named,
        vec![
            op("body.png", 0, 0),
            op("film.apng#1", 4, 6),
            op("part.png", 0, 0)
        ],
        "経過 0 のコマ 1 枚だけが element の順の位置に出る"
    );
    assert_eq!(
        ops[1].method,
        ComposeMethod::Asis,
        "置いた element の描画メソッド"
    );

    let mut first = PatternState::default();
    first.set_film(film, world.film_sheet(film).unwrap().frames[0]);
    assert_eq!(
        compose(&world, &atlas, 0, &first).0,
        vec![
            op("body.png", 0, 0),
            op("film.apng#0", 4, 6),
            op("part.png", 0, 0)
        ]
    );

    let mut clear = PatternState::default();
    clear.set_film(film, world.film_sheet(film).unwrap().frames[2]);
    assert_eq!(
        compose(&world, &atlas, 0, &clear).0,
        vec![op("body.png", 0, 0), op("part.png", 0, 0)],
        "全透明のコマは命令にならない"
    );
}

/// 欄の絵の番号が子のコマに無い（差し替えの継ぎ目の古い指令）なら経過 0 のコマを描いて `debug!`。
#[test]
fn unknown_picture_falls_back_to_rest_with_debug() {
    let (world, atlas) = build("surface0\n{\nelement0,overlay,film.apng,0,0\n}\n");
    let mut stale = PatternState::default();
    stale.set_film(film_id(&atlas), 9_999);
    let (named, _, logs) = compose(&world, &atlas, 0, &stale);
    assert_eq!(named, vec![op("film.apng#1", 0, 0)]);
    assert!(
        logs.lines()
            .any(|l| l.contains("level=DEBUG") && l.contains("picture=9999")),
        "{logs}"
    );
}

/// 子の定義が無い `Film`（起きないはず）は描かずに `error!` を 1 行。ほかの element はそのまま。
#[test]
fn film_without_sheet_is_not_drawn_and_logs_error() {
    let (mut world, atlas) =
        build("surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,part.png,0,0\n}\n");
    let entity = world.world().resource::<SurfaceIndex>().0[&0];
    world
        .world_mut()
        .get_mut::<SurfaceMaster>(entity)
        .expect("surface0 は在る")
        .elements[0]
        .kind = ElementKind::Film(FilmId(123_456));
    let (named, _, logs) = compose(&world, &atlas, 0, &PatternState::default());
    assert_eq!(named, vec![op("part.png", 0, 0)]);
    assert_eq!(
        logs.lines().filter(|l| l.contains("level=ERROR")).count(),
        1,
        "{logs}"
    );
}

/// 要件 7.1: `always` でない animation（組み合わせ・大文字・ほかの語）は欄が空でも何も足さない。
/// 命令列は animation の行を消した文面と同じ。
#[test]
fn surfaces_without_always_keep_their_ops() {
    let plain = format!("surface0\n{{\nelement0,overlay,body.png,0,0\n}}\n{TARGETS}");
    let expected = ops_of(&plain, 0, &PatternState::default());
    assert_eq!(expected, vec![op("body.png", 0, 0)]);
    for interval in [
        "bind+always",
        "always+bind",
        "Always",
        "sometimes",
        "runonce",
    ] {
        let text = format!(
            "surface0\n{{\nelement0,overlay,body.png,0,0\nanimation0.interval,{interval}\n\
             animation0.pattern0,overlay,5,0,0,0\nanimation0.pattern1,overlay,6,0,0,0\n}}\n{TARGETS}"
        );
        assert_eq!(
            ops_of(&text, 0, &PatternState::default()),
            expected,
            "{interval}"
        );
    }
}

/// 完了の姿: 検体のシェルをコマの欄が空の `PatternState` で合成すると、経過 0 の絵が出る。
/// 動く絵の子は経過 0 のコマ（どれも 1 枚目の待ちが 1 以上なのでコマ 0）を 1 枚だけ、合計 0 の
/// 手書きの `always`（10）は 1 周だけ評価した絵（31 の cyan）、先頭から待つ `always`（0・20）と
/// `bind+always`（11）は何も足さない。
#[test]
fn fixture_surfaces_at_rest_show_rest_pictures() {
    let text = include_str!("../tests/fixtures/animated-playback/shell/surfaces.txt");
    let (world, atlas) = build_with(text, &[(1, "surface1.png")]);
    let ops = |top: u32| compose(&world, &atlas, top, &PatternState::default()).0;
    assert_eq!(
        ops(0),
        vec![
            op("body.png", 0, 0),
            op("rgb.apng#0", 0, 0),
            op("rgb.apng#0", 8, 0),
            op("alpha.webp#0", 0, 8),
            op("part.png", 8, 8),
            op("rgb.webp#0", 8, 8),
        ]
    );
    assert_eq!(ops(1), vec![op("surface1.png#0", 0, 0)]);
    assert_eq!(ops(3), vec![op("body.png", 0, 0), op("rgb.apng#0", 4, 4)]);
    assert_eq!(ops(10), vec![op("body.png", 0, 0), op("cyan.png", 0, 0)]);
    assert_eq!(ops(11), vec![op("body.png", 0, 0)]);
}
