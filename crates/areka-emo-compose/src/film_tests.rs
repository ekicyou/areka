//! 動く絵を子へ分解する（`film.rs`・spec: areka-P0-animated-image-playback task 2.3）。
//!
//! 絵は `MemoryDecoder` へ登録して本物の `bake_with_limits` で焼く（上限は既定値を渡し、環境変数に
//! 依らない）。焼く経路が作れない形（コマの並びが約束と違う）だけは `AtlasTable::with_frames` で組む。

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, AtlasEntry,
    AtlasKey, DecodedImage, MemoryDecoder, PackConfig, SetId, Size, SurfaceSet, UseSelfAlpha,
    bake_with_limits,
};
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};

use super::*;
use crate::world::{EmoWorld, SurfaceIndex};

const BASE: &str = "shell";

fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

fn surface(id: u32, elements: Vec<Element>) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

fn shell_of(surfaces: Vec<Surface>) -> Shell {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    }
}

/// 全面が 1 色・不透明の `w`×`h`（色 `c` でコマを見分ける）。
fn solid(w: u32, h: u32, c: u8) -> DecodedImage {
    DecodedImage {
        width: w,
        height: h,
        stride: w * 4,
        bgra: [c, c, c, 255].repeat((w * h) as usize),
        has_alpha: true,
    }
}

/// 焼く絵の登録（相対パスの一覧と読み手）。
#[derive(Default)]
struct Pics {
    dec: MemoryDecoder,
    rels: Vec<String>,
}

impl Pics {
    fn path(rel: &str) -> std::path::PathBuf {
        Path::new(BASE).join(rel)
    }

    fn still(mut self, rel: &str) -> Self {
        self.dec
            .insert(Self::path(rel), 2, 2, 8, solid(2, 2, 9).bgra, true);
        self.rels.push(rel.to_string());
        self
    }

    /// 2×2 の動く絵（コマごとに色が違う）。
    fn film(mut self, rel: &str, delays: &[u32], laps: Option<u32>) -> Self {
        let frames: Vec<AnimationFrame> = delays
            .iter()
            .enumerate()
            .map(|(i, &delay_ms)| AnimationFrame {
                image: solid(2, 2, 10 + i as u8),
                delay_ms,
            })
            .collect();
        let loop_count = match laps {
            None => LoopCount::Infinite,
            Some(n) => LoopCount::Finite(NonZeroU32::new(n).unwrap()),
        };
        self.dec.insert_animated(
            Self::path(rel),
            AnimationInfo {
                width: 2,
                height: 2,
                frame_count: delays.len() as u32,
            },
            Ok(solid(2, 2, 10)),
            Ok(AnimatedImage { frames, loop_count }),
        );
        self.rels.push(rel.to_string());
        self
    }

    /// 全コマを読めず 1 枚へ縮む動く絵（段 1 の 1 枚目が静止画として焼ける）。
    fn shrunk(mut self, rel: &str) -> Self {
        self.dec.insert_animated(
            Self::path(rel),
            AnimationInfo {
                width: 2,
                height: 2,
                frame_count: 2,
            },
            Ok(solid(2, 2, 10)),
            Err("broken frame 1".to_string()),
        );
        self.rels.push(rel.to_string());
        self
    }

    fn bake(&self) -> AtlasTable {
        let elements = self.rels.iter().map(|r| elem(0, r, 0, 0)).collect();
        let surfaces = vec![surface(0, elements)];
        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: Path::new(BASE),
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let result = bake_with_limits(
            &[set],
            &self.dec,
            PackConfig::default(),
            AnimationLimits::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        result.table
    }
}

fn bound(shell: &Shell, images: &BTreeMap<u32, String>, atlas: &AtlasTable) -> EmoWorld {
    let mut world = EmoWorld::build_with_images(shell, images);
    world.bind_atlas(atlas, SetId(0));
    world
}

fn binding(world: &EmoWorld, id: u32) -> Vec<Option<ElementId>> {
    let w = world.world();
    let entity = w.resource::<SurfaceIndex>().0[&id];
    w.get::<AtlasBinding>(entity).unwrap().0.clone()
}

fn kinds(world: &EmoWorld, id: u32) -> Vec<ElementKind> {
    world
        .surface(id)
        .unwrap()
        .elements
        .iter()
        .map(|e| e.kind)
        .collect()
}

fn id_of(atlas: &AtlasTable, rel: &str) -> ElementId {
    atlas.resolve(SetId(0), rel).unwrap()
}

/// 事後条件: `Film(id)` の element が在れば `film_sheet(id)` は必ず `Some`。
fn assert_every_film_resolves(world: &EmoWorld) {
    for sid in world.surface_ids() {
        for e in &world.surface(sid).unwrap().elements {
            if let ElementKind::Film(id) = e.kind {
                assert!(world.film_sheet(id).is_some(), "面 {sid} の {id:?}");
            }
        }
    }
}

/// 要件 1.1・1.4: 動く絵の element は種類だけが替わり、番号・X,Y・描画メソッドは静止画のときと同じ。
/// 束縛は空になり、子の定義がファイルの値を運ぶ。
#[test]
fn film_element_keeps_layer_xy_and_method() {
    let shell = shell_of(vec![surface(
        0,
        vec![
            elem(0, "body.png", 0, 0),
            elem(1, "anim.png", 3, 4),
            elem(2, "top.png", 1, 1),
        ],
    )]);
    let pics =
        Pics::default()
            .still("body.png")
            .still("top.png")
            .film("anim.png", &[100, 0, 70], Some(3));
    let atlas = pics.bake();
    let parent = id_of(&atlas, "anim.png");
    let world = bound(&shell, &BTreeMap::new(), &atlas);

    // 対照: 同じ名前を静止画で焼いた面の表。
    let still_atlas = Pics::default()
        .still("body.png")
        .still("top.png")
        .still("anim.png")
        .bake();
    let still = bound(&shell, &BTreeMap::new(), &still_atlas);

    let film = FilmId(parent.0);
    assert_eq!(
        kinds(&world, 0),
        vec![
            ElementKind::Image,
            ElementKind::Film(film),
            ElementKind::Image
        ]
    );
    for (a, b) in world
        .surface(0)
        .unwrap()
        .elements
        .iter()
        .zip(&still.surface(0).unwrap().elements)
    {
        assert_eq!(
            (a.layer, &a.path, a.transform, &a.method),
            (b.layer, &b.path, b.transform, &b.method)
        );
    }
    assert_eq!(
        binding(&world, 0),
        vec![
            Some(id_of(&atlas, "body.png")),
            None,
            Some(id_of(&atlas, "top.png"))
        ]
    );

    let frames: Vec<u32> = atlas
        .animation(parent)
        .unwrap()
        .frames
        .iter()
        .map(|f| f.0)
        .collect();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0], parent.0);
    assert_eq!(
        world.film_sheet(film),
        Some(&FilmSheet {
            id: film,
            path: "anim.png".to_string(),
            frames,
            delays_ms: vec![100, 0, 70],
            laps: NonZeroU32::new(3),
            original: (2, 2),
            rest: 0,
        })
    );
    assert_eq!(world.film_sheets().count(), 1);
    assert!(world.film_skips().is_empty());
    assert_every_film_resolves(&world);
}

/// 要件 1.2: 面の画像（`surface<数字>.png`）が動く絵なら、土台の element として分解される。
#[test]
fn animated_base_image_is_decomposed() {
    let shell = shell_of(vec![surface(0, vec![elem(0, "body.png", 0, 0)])]);
    let atlas = Pics::default()
        .still("body.png")
        .film("surface1.png", &[333, 0, 70, 1], Some(2))
        .bake();
    let images = BTreeMap::from([(1, "surface1.png".to_string())]);
    let world = bound(&shell, &images, &atlas);

    let film = FilmId(id_of(&atlas, "surface1.png").0);
    assert_eq!(kinds(&world, 1), vec![ElementKind::Film(film)]);
    assert_eq!(kinds(&world, 0), vec![ElementKind::Image]);
    let sheet = world.film_sheet(film).unwrap();
    assert_eq!(sheet.laps, NonZeroU32::new(2));
    assert_eq!(sheet.delays_ms, vec![333, 0, 70, 1]);
    assert_every_film_resolves(&world);
}

/// 要件 1.3: `element0` が在るサーフェスの動く `surface<数字>.png` は土台に使われず、分解もされない。
#[test]
fn animated_base_image_under_element0_is_not_decomposed() {
    let shell = shell_of(vec![surface(2, vec![elem(0, "body.png", 0, 0)])]);
    let atlas = Pics::default()
        .still("body.png")
        .film("surface2.png", &[100, 100], None)
        .bake();
    let images = BTreeMap::from([(2, "surface2.png".to_string())]);
    let world = bound(&shell, &images, &atlas);

    assert_eq!(
        world
            .base_images()
            .shadowed
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(kinds(&world, 2), vec![ElementKind::Image]);
    assert_eq!(world.film_sheets().count(), 0);
    assert!(world.film_skips().is_empty());
}

/// `with_frames` で組む表: `a.png`（良い・2 コマ）・`b.png`（2 コマ目の原寸が違う）・
/// `c.png`（待ち時間がすべて 0）。番号は a=0,1・b=2,3・c=4,5 で、各 2 コマ目の鍵は親と同じ。
fn crafted_atlas() -> AtlasTable {
    let key = |rel: &str| AtlasKey {
        set: SetId(0),
        rel_path: rel.to_string(),
    };
    let entry = |w, h| AtlasEntry {
        original: Size { w, h },
        placement: None,
    };
    let anim = |frames: [u32; 2], delays: [u32; 2]| Animation {
        frames: frames.map(ElementId).to_vec(),
        delays_ms: delays.to_vec(),
        loop_count: LoopCount::Infinite,
    };
    AtlasTable::with_frames(
        vec![
            key("a.png"),
            key("a.png"),
            key("b.png"),
            key("b.png"),
            key("c.png"),
            key("c.png"),
        ],
        vec![
            entry(2, 2),
            entry(2, 2),
            entry(2, 2),
            entry(3, 2),
            entry(2, 2),
            entry(2, 2),
        ],
        Vec::new(),
        vec![
            (ElementId(0), anim([0, 1], [100, 100])),
            (ElementId(2), anim([2, 3], [100, 100])),
            (ElementId(4), anim([4, 5], [0, 0])),
        ],
    )
}

/// 要件 2.5・8.2: 検査で落ちる 4 通り（①0 番が親でない ②コマと待ち時間の数が違う ③原寸が揃わない
/// ④待ち時間の合計が 0）。②は焼く経路も `with_frames` も作れないので、並びを手で渡す。
#[test]
fn check_rejects_four_ways() {
    let atlas = crafted_atlas();
    let a = ElementId(0);
    let good = atlas.animation(a).unwrap().clone();
    assert_eq!(check(a, &good, &atlas), Ok(()), "対照: 良い並びは通る");

    let mut not_parent = good.clone();
    not_parent.frames.swap(0, 1);
    assert_eq!(
        check(a, &not_parent, &atlas),
        Err(FilmSkipReason::BrokenFrames),
        "① 0 番が親でない"
    );

    let mut counts = good.clone();
    counts.delays_ms.pop();
    assert_eq!(
        check(a, &counts, &atlas),
        Err(FilmSkipReason::BrokenFrames),
        "② コマと待ち時間の数が違う"
    );

    let b = ElementId(2);
    assert_eq!(
        check(b, atlas.animation(b).unwrap(), &atlas),
        Err(FilmSkipReason::BrokenFrames),
        "③ 原寸が揃わない"
    );

    let c = ElementId(4);
    assert_eq!(
        check(c, atlas.animation(c).unwrap(), &atlas),
        Err(FilmSkipReason::ZeroTotalDelay),
        "④ 待ち時間の合計が 0"
    );
}

/// 要件 2.5・8.2: 落ちた絵は画像の element のまま（束縛も残る＝ 1 枚目の静止画）で、理由つきで
/// 相対パスの昇順に `film_skips` へ載る。通った絵だけが子になる。
#[test]
fn rejected_pictures_stay_images_and_are_listed() {
    let atlas = crafted_atlas();
    let shell = shell_of(vec![surface(
        0,
        vec![
            elem(0, "c.png", 0, 0),
            elem(1, "b.png", 0, 0),
            elem(2, "a.png", 0, 0),
        ],
    )]);
    let world = bound(&shell, &BTreeMap::new(), &atlas);

    assert_eq!(
        kinds(&world, 0),
        vec![
            ElementKind::Image,
            ElementKind::Image,
            ElementKind::Film(FilmId(0))
        ]
    );
    assert_eq!(
        binding(&world, 0),
        vec![Some(ElementId(4)), Some(ElementId(2)), None]
    );
    assert_eq!(
        world.film_skips(),
        &[
            FilmSkip {
                path: "b.png".to_string(),
                reason: FilmSkipReason::BrokenFrames
            },
            FilmSkip {
                path: "c.png".to_string(),
                reason: FilmSkipReason::ZeroTotalDelay
            },
        ]
    );
    assert_eq!(
        world.film_sheets().map(|s| s.id).collect::<Vec<_>>(),
        vec![FilmId(0)]
    );
    assert_every_film_resolves(&world);
}

/// 要件 1.11: 動く GIF（読み手は 1 枚の絵として読む）と、1 枚へ縮んだ動く絵は分解されない。
/// どちらも `animation` が `None` なので対象にならず、面の表に何も載らない。
#[test]
fn gif_and_shrunk_pictures_are_not_decomposed() {
    let shell = shell_of(vec![surface(
        0,
        vec![elem(0, "anim.gif", 0, 0), elem(1, "shrunk.png", 0, 0)],
    )]);
    let atlas = Pics::default()
        .still("anim.gif")
        .shrunk("shrunk.png")
        .bake();
    for rel in ["anim.gif", "shrunk.png"] {
        assert!(atlas.animation(id_of(&atlas, rel)).is_none(), "前提: {rel}");
    }
    let world = bound(&shell, &BTreeMap::new(), &atlas);

    assert_eq!(kinds(&world, 0), vec![ElementKind::Image; 2]);
    assert!(binding(&world, 0).iter().all(Option::is_some));
    assert!(world.world().get_resource::<FilmSheets>().is_none());
}

/// 要件 3.4: 同じファイルは、置いた場所・サーフェスに依らず同じ子（子の定義は 1 つ）。
#[test]
fn same_picture_is_the_same_child() {
    let shell = shell_of(vec![
        surface(
            0,
            vec![elem(1, "anim.png", 0, 0), elem(2, "anim.png", 8, 0)],
        ),
        surface(3, vec![elem(1, "anim.png", 4, 4)]),
    ]);
    let atlas = Pics::default().film("anim.png", &[100, 100], None).bake();
    let world = bound(&shell, &BTreeMap::new(), &atlas);

    let film = ElementKind::Film(FilmId(id_of(&atlas, "anim.png").0));
    assert_eq!(kinds(&world, 0), vec![film, film]);
    assert_eq!(kinds(&world, 3), vec![film]);
    assert_eq!(world.film_sheets().count(), 1);
    assert_eq!(world.film_sheets().next().unwrap().laps, None);
}

/// 要件 7.1: 動く絵が 0 の面の表には何も載らず、element と束縛は分解の前と同じ。
#[test]
fn no_films_stores_nothing() {
    let shell = shell_of(vec![surface(
        0,
        vec![elem(0, "body.png", 0, 0), elem(1, "top.png", 2, 2)],
    )]);
    let atlas = Pics::default().still("body.png").still("top.png").bake();
    let world = bound(&shell, &BTreeMap::new(), &atlas);

    assert!(world.world().get_resource::<FilmSheets>().is_none());
    assert_eq!(world.film_sheets().count(), 0);
    assert!(world.film_skips().is_empty());
    assert_eq!(kinds(&world, 0), vec![ElementKind::Image; 2]);
    assert_eq!(
        binding(&world, 0),
        vec![
            Some(id_of(&atlas, "body.png")),
            Some(id_of(&atlas, "top.png"))
        ]
    );
}

/// 経過 0 のコマ＝出す前の待ち（0, コマ 0 の待ち, コマ 1 の待ち, …）の累積が 0 の最後の番号。
/// 1 枚目の待ち時間が 0 なら次のコマ。
#[test]
fn rest_frame_skips_zero_first_delays() {
    let shell = shell_of(vec![surface(
        0,
        vec![
            elem(0, "a.png", 0, 0),
            elem(1, "b.png", 0, 0),
            elem(2, "c.png", 0, 0),
        ],
    )]);
    let atlas = Pics::default()
        .film("a.png", &[100, 0, 70], None)
        .film("b.png", &[0, 50, 70], None)
        .film("c.png", &[0, 0, 70], None)
        .bake();
    let world = bound(&shell, &BTreeMap::new(), &atlas);
    let rest = |rel| world.film_sheet(FilmId(id_of(&atlas, rel).0)).unwrap().rest;
    assert_eq!((rest("a.png"), rest("b.png"), rest("c.png")), (0, 1, 2));
}

/// 要件 2.6 の前提: 同じ入力から同じ子の定義・同じ element・同じ束縛。
#[test]
fn same_input_same_result() {
    let shell = shell_of(vec![
        surface(0, vec![elem(0, "b.png", 0, 0), elem(1, "a.png", 1, 1)]),
        surface(5, vec![elem(0, "a.png", 0, 0), elem(3, "c.png", 2, 0)]),
    ]);
    let pics = Pics::default()
        .film("a.png", &[0, 40, 40], Some(1))
        .film("b.png", &[0, 0], None)
        .still("c.png");
    let atlas = pics.bake();
    let one = bound(&shell, &BTreeMap::new(), &atlas);
    let two = bound(&shell, &BTreeMap::new(), &atlas);

    assert_eq!(
        one.film_sheets().collect::<Vec<_>>(),
        two.film_sheets().collect::<Vec<_>>()
    );
    assert_eq!(one.film_skips(), two.film_skips());
    assert_eq!(one.film_skips().len(), 1, "b.png は合計 0 で落ちる");
    for id in [0, 5] {
        assert_eq!(one.surface(id), two.surface(id));
        assert_eq!(binding(&one, id), binding(&two, id));
    }
}

/// 同じ面の表に `bind_atlas` を 2 度呼ぶのは誤り（子の定義が古いアトラスのまま残る）。
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "1 度だけ")]
fn binding_twice_is_caught() {
    let shell = shell_of(vec![surface(0, vec![elem(0, "anim.png", 0, 0)])]);
    let atlas = Pics::default().film("anim.png", &[100, 100], None).bake();
    let mut world = bound(&shell, &BTreeMap::new(), &atlas);
    world.bind_atlas(&atlas, SetId(0));
}
