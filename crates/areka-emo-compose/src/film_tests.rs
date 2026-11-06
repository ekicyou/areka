//! 動く絵を子へ分解する（`film.rs`・spec: areka-P0-animated-image-playback）。
//!
//! 絵は `MemoryDecoder` へ登録して本物の `bake_with_limits` で焼く（`plan_test_support::bake_decoded`・上限は
//! 既定値を渡し、環境変数に依らない）。焼く経路が作れない形（コマの並びが約束と違う）だけは `AtlasTable::with_frames`
//! で組むか、検査へ並びを手で渡す（`check_rejects_six_ways`）。

use std::num::NonZeroU32;
use std::path::Path;

use areka_emo_atlas::{AnimationInfo, AtlasEntry, AtlasKey, MemoryDecoder, SetId, Size};

use super::*;
use crate::plan_test_support::{
    bake_decoded, bound_world, component_of, elem, insert_film_2x2, shell_of, solid, surface,
};
use crate::world::EmoWorld;

const BASE: &str = "shell";

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
        insert_film_2x2(&mut self.dec, Self::path(rel), delays, laps);
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
        bake_decoded(
            &self.dec,
            Path::new(BASE),
            self.rels.iter().map(String::as_str),
        )
    }
}

fn binding(world: &EmoWorld, id: u32) -> Vec<Option<ElementId>> {
    component_of::<AtlasBinding>(world.world(), id)
        .unwrap()
        .0
        .clone()
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
    let world = bound_world(&shell, &[], &atlas);

    // 対照: 同じ名前を静止画で焼いた面の表。
    let still_atlas = Pics::default()
        .still("body.png")
        .still("top.png")
        .still("anim.png")
        .bake();
    let still = bound_world(&shell, &[], &still_atlas);

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
    let world = bound_world(&shell, &[(1, "surface1.png")], &atlas);

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
    let world = bound_world(&shell, &[(2, "surface2.png")], &atlas);

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

/// 要件 2.5・8.2: 検査で落ちる 6 通り（①0 番が親でない ②コマと待ち時間の数が違う ③原寸が揃わない
/// ④待ち時間の合計が 0 ⑤コマが 2 枚未満 ⑥コマの番号が表の外）。②⑤⑥は焼く経路も `with_frames` も作れない
/// ので、並びを手で渡す（⑥は表を引く前に断らないと `AtlasTable::entry` が panic する）。
#[test]
fn check_rejects_six_ways() {
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

    let mut single = good.clone();
    single.frames.truncate(1);
    single.delays_ms.truncate(1);
    assert_eq!(
        check(a, &single, &atlas),
        Err(FilmSkipReason::BrokenFrames),
        "⑤ コマが 2 枚未満"
    );

    let mut outside = good.clone();
    outside.frames[1] = ElementId(atlas.len() as u32);
    assert_eq!(
        check(a, &outside, &atlas),
        Err(FilmSkipReason::BrokenFrames),
        "⑥ コマの番号が表の外"
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
    let world = bound_world(&shell, &[], &atlas);

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

/// 要件 1.11: 1 枚へ縮んだ動く絵は分解されない。`animation` が `None` なので対象にならず、面の表に
/// 何も載らない（動く GIF を 1 枚の絵として読むのは読み手の側・`areka-emo-atlas` の試験が見る）。
#[test]
fn shrunk_picture_is_not_decomposed() {
    let shell = shell_of(vec![surface(0, vec![elem(1, "shrunk.png", 0, 0)])]);
    let atlas = Pics::default().shrunk("shrunk.png").bake();
    assert!(
        atlas.animation(id_of(&atlas, "shrunk.png")).is_none(),
        "前提"
    );
    let world = bound_world(&shell, &[], &atlas);

    assert_eq!(kinds(&world, 0), vec![ElementKind::Image]);
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
    let world = bound_world(&shell, &[], &atlas);

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
    let world = bound_world(&shell, &[], &atlas);

    assert!(world.world().get_resource::<FilmSheets>().is_none());
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
/// 1 枚目の待ち時間が 0 なら次のコマ。待ち時間が全部 0 の絵（d.png）は子にならず落ちる。
#[test]
fn rest_frame_skips_zero_first_delays() {
    let shell = shell_of(vec![surface(
        0,
        vec![
            elem(0, "a.png", 0, 0),
            elem(1, "b.png", 0, 0),
            elem(2, "c.png", 0, 0),
            elem(3, "d.png", 0, 0),
        ],
    )]);
    let atlas = Pics::default()
        .film("a.png", &[100, 0, 70], None)
        .film("b.png", &[0, 50, 70], None)
        .film("c.png", &[0, 0, 70], None)
        .film("d.png", &[0, 0], None)
        .bake();
    let world = bound_world(&shell, &[], &atlas);
    let rest = |rel| world.film_sheet(FilmId(id_of(&atlas, rel).0)).unwrap().rest;
    assert_eq!((rest("a.png"), rest("b.png"), rest("c.png")), (0, 1, 2));
    assert_eq!(
        world.film_skips(),
        &[FilmSkip {
            path: "d.png".to_string(),
            reason: FilmSkipReason::ZeroTotalDelay
        }]
    );
}

/// 同じ面の表に `bind_atlas` を 2 度呼ぶのは誤り（子の定義が古いアトラスのまま残る）。
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "1 度だけ")]
fn binding_twice_is_caught() {
    let shell = shell_of(vec![surface(0, vec![elem(0, "anim.png", 0, 0)])]);
    let atlas = Pics::default().film("anim.png", &[100, 100], None).bake();
    let mut world = bound_world(&shell, &[], &atlas);
    world.bind_atlas(&atlas, SetId(0));
}
