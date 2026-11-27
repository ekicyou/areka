//! 描画メソッドの語ごとの画素を、WARP の画面の外の描き手で固める檻。

use std::collections::BTreeMap;
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, DecodedImage, MemoryDecoder, PackConfig, SetId, SurfaceSet,
    UseSelfAlpha, bake,
};
use areka_emo_compose::{
    BindSet, ComposedSurface, ElementKind, EmoWorld, PatternState, element_kind,
};
use areka_parsers::shell::parse;

use crate::offscreen::OffscreenRenderer;
use crate::presenter::test_support::{elem, surface};

const BASE: &str = "shell/master";
const W: u32 = 30;
const H: u32 = 10;

type Px = [u8; 4];

const RED: Px = [0, 0, 255, 255];
const HALF: Px = [0, 0, 128, 128];
const CLEAR: Px = [0; 4];

/// (語, 面 A の絵, 面 B の絵)。A は 30 幅で全域を覆い、B は 10 幅で x=10 から置く。
const WORDS: [(&str, &str, &str); 6] = [
    ("overlay", "blue60.png", "blue60_10.png"),
    ("overlay-fast", "blue60.png", "blue60_10.png"),
    ("interpolate", "blue60.png", "blue60_10.png"),
    ("replace", "blue60.png", "blue60_10.png"),
    ("reduce", "blue60.png", "blue60_10.png"),
    ("asis", "bluekey.png", "blue10.png"),
];

/// 面 A・B の絵を持つ面の番号（`surface{N}.png` の慣習）。
const FACES_A: [u32; 6] = [100, 100, 100, 100, 100, 101];
const FACES_B: [u32; 6] = [102, 102, 102, 102, 102, 103];

fn solid(w: u32, h: u32, f: impl Fn(u32) -> Px) -> Vec<u8> {
    (0..h).flat_map(|_| (0..w).flat_map(&f)).collect()
}

fn pictures() -> Vec<(&'static str, u32, u32, Vec<u8>, bool)> {
    let base = |x: u32| match x {
        0..=9 => RED,
        10..=19 => HALF,
        _ => CLEAR,
    };
    let key = |x: u32| {
        if x < 10 {
            [255, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        }
    };
    vec![
        ("base.png", W, H, solid(W, H, base), true),
        ("blue60.png", W, H, solid(W, H, |_| [153, 0, 0, 153]), true),
        ("bluekey.png", W, H, solid(W, H, key), false),
        (
            "blue60_10.png",
            10,
            H,
            solid(10, H, |_| [153, 0, 0, 153]),
            true,
        ),
        (
            "blue10.png",
            10,
            H,
            solid(10, H, |_| [255, 0, 0, 255]),
            true,
        ),
        (
            "green_r.png",
            W,
            H,
            solid(W, H, |x| if x >= 20 { [0, 255, 0, 255] } else { CLEAR }),
            true,
        ),
    ]
}

fn shell_text() -> String {
    let mut s = String::from("charset,UTF-8\n");
    for (i, (word, a, b)) in WORDS.iter().enumerate() {
        let id = 100 + i as u32 * 10;
        let (fa, fb) = (FACES_A[i], FACES_B[i]);
        let n = i as u32;
        s += &format!(
            "surface{}\n{{\nelement0,overlay,base.png,0,0\nelement1,{word},{a},0,0\n}}\n\
             surface{}\n{{\nelement0,overlay,base.png,0,0\nelement1,{word},{b},10,0\n}}\n\
             surface{}\n{{\nanimation{}.interval,always\nanimation{}.pattern0,overlay,104,0,0,0\n\
             animation{}.interval,always\nanimation{}.pattern0,{word},{fa},0,0,0\n}}\n\
             surface{}\n{{\nanimation{}.interval,always\nanimation{}.pattern0,overlay,104,0,0,0\n\
             animation{}.interval,always\nanimation{}.pattern0,{word},{fb},0,10,0\n}}\n\
             surface{}\n{{\nelement0,overlay,base.png,0,0\nanimation{}.interval,bind\n\
             animation{}.pattern0,{word},{fa},0,0,0\n}}\n\
             surface{}\n{{\nelement0,overlay,base.png,0,0\nanimation{}.interval,bind\n\
             animation{}.pattern0,{word},{fb},0,10,0\n}}\n\
             surface{}\n{{\nelement0,{word},{a},0,0\n}}\n",
            1000 + n,
            1100 + n,
            2000 + n,
            id + 1,
            id + 1,
            id + 2,
            id + 2,
            2100 + n,
            id + 3,
            id + 3,
            id + 4,
            id + 4,
            3000 + n,
            id + 5,
            id + 5,
            3100 + n,
            id + 6,
            id + 6,
            4000 + n,
        );
    }
    s += "surface5000\n{\nelement0,overlay,blue60.png,0,0\nelement1,asis,80,0,0\n}\n\
          surface80\n{\nelement0,overlay,base.png,0,0\n}\n\
          surface6000\n{\nelement0,overlay,base.png,0,0\nanimation1.interval,always\n\
          animation1.pattern0,overlay,105,0,0,0\nanimation2.interval,always\n\
          animation2.pattern0,reduce,100,0,0,0\n}\n\
          surface6001\n{\nelement0,overlay,base.png,0,0\nanimation3.interval,always\n\
          animation3.pattern0,reduce,100,0,0,0\nanimation4.interval,always\n\
          animation4.pattern0,overlay,105,0,0,0\n}\n\
          surface7000\n{\nelement0,overlay,base.png,0,0\nanimation70.interval,always\n\
          animation70.pattern0,base,100,0,7,3\n}\n\
          surface7001\n{\nelement0,overlay,base.png,0,0\nanimation71.interval,always\n\
          animation71.pattern0,base,103,0,10,0\n}\n\
          surface7002\n{\nelement0,overlay,base.png,0,0\nanimation72.interval,bind\n\
          animation72.pattern0,base,100,0,7,3\n}\n";
    s
}

struct Scene {
    world: EmoWorld,
    atlas: AtlasTable,
    renderer: OffscreenRenderer,
}

impl Scene {
    fn new() -> Scene {
        let shell = parse(&shell_text());
        let base = Path::new(BASE);
        let mut dec = MemoryDecoder::new();
        for (name, w, h, bgra, has_alpha) in pictures() {
            let image = DecodedImage {
                width: w,
                height: h,
                stride: w * 4,
                bgra: bgra.clone(),
                has_alpha,
            };
            dec.insert_unmultiplied(base.join(name), image);
            dec.insert(base.join(name), w, h, w * 4, bgra, has_alpha);
        }
        let mut surfaces = shell.surfaces.clone();
        for s in &mut surfaces {
            s.elements
                .retain(|e| element_kind(&e.path) == ElementKind::Image);
        }
        let faces = [
            (100, "blue60.png"),
            (101, "bluekey.png"),
            (102, "blue60_10.png"),
            (103, "blue10.png"),
            (104, "base.png"),
            (105, "green_r.png"),
        ];
        for (id, file) in faces {
            surfaces.push(surface(id, vec![elem(file, 0, 0)]));
        }
        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "{:?}", baked.errors);
        let images: BTreeMap<u32, String> =
            faces.iter().map(|(id, f)| (*id, f.to_string())).collect();
        let mut world = EmoWorld::build_with_images(&shell, &images);
        world.bind_atlas(&baked.table, SetId(0));
        Scene {
            world,
            atlas: baked.table,
            renderer: OffscreenRenderer::warp().expect("WARP の描き手"),
        }
    }

    fn draw(&mut self, surface: u32, binds: &BindSet) -> ComposedSurface {
        self.renderer
            .render(
                &self.world,
                &self.atlas,
                surface,
                binds,
                &PatternState::default(),
            )
            .expect("画面の外へ描く")
    }
}

fn px(s: &ComposedSurface, x: u32) -> Px {
    let at = (5 * s.stride() + x * 4) as usize;
    s.bytes()[at..at + 4].try_into().unwrap()
}

fn assert_px(what: &str, got: Px, want: Px) {
    assert!(
        got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 1),
        "{what}: 描いた {got:?}・期待 {want:?}（BGRA 乗算済み）"
    );
}

/// 点 x=5（ベース不透明）・15（α128）・25（全透明）の期待値。新規レイヤは α153 の青（`asis` は
/// 不透明の青で、A は左の 3 分の 1 が透過色のマゼンタ）。
fn want_a(word: &str) -> [Px; 3] {
    match word {
        "overlay" => [[153, 0, 102, 255], [153, 0, 51, 204], [153, 0, 0, 153]],
        "overlay-fast" => [[153, 0, 102, 255], [77, 0, 51, 128], CLEAR],
        "interpolate" => [RED, [76, 0, 128, 204], [153, 0, 0, 153]],
        "replace" => [[153, 0, 0, 153]; 3],
        "reduce" => [[0, 0, 153, 153], [0, 0, 77, 77], CLEAR],
        _ => [[255, 0, 255, 255], [255, 0, 0, 255], [255, 0, 0, 255]],
    }
}

/// B は新規レイヤが x=10..19 だけ。x=5・25 は範囲の外、x=15 は範囲の中（ベース α128）。
fn want_b(word: &str) -> [Px; 3] {
    let inside = want_a(word)[1];
    match word {
        "reduce" => [CLEAR, inside, CLEAR],
        "asis" => [RED, [255, 0, 0, 255], CLEAR],
        _ => [RED, inside, CLEAR],
    }
}

/// `bind_offset` が `Some` なら、面の着せ替え（番号は `100 + 10 * 語 + offset`）を有効にして描く。
fn check_words(first_surface: u32, second_surface: u32, bind_offset: Option<u32>) {
    let mut scene = Scene::new();
    for (i, (word, _, _)) in WORDS.iter().enumerate() {
        for (surface, want, offset) in [
            (first_surface + i as u32, want_a(word), 0),
            (second_surface + i as u32, want_b(word), 1),
        ] {
            let binds = match bind_offset {
                Some(o) => BindSet::from_ids([100 + i as u32 * 10 + o + offset]),
                None => BindSet::default(),
            };
            let got = scene.draw(surface, &binds);
            for (x, w) in [5, 15, 25].into_iter().zip(want) {
                assert_px(&format!("{word} 面 {surface} x={x}"), px(&got, x), w);
            }
        }
    }
}

#[test]
fn draw_methods_element_words_pixels_on_warp() {
    check_words(1000, 1100, None);
}

#[test]
fn draw_methods_two_simultaneous_animations_give_the_same_pixels_on_warp() {
    check_words(2000, 2100, None);
}

#[test]
fn draw_methods_dress_up_gives_the_same_pixels_on_warp() {
    check_words(3000, 3100, Some(5));
}

#[test]
fn draw_methods_first_element_is_the_new_layer_itself_on_warp() {
    let mut scene = Scene::new();
    for (i, (word, _, _)) in WORDS.iter().enumerate() {
        let got = scene.draw(4000 + i as u32, &BindSet::default());
        let want = match *word {
            "asis" => [[255, 0, 255, 255], [255, 0, 0, 255], [255, 0, 0, 255]],
            _ => [[153, 0, 0, 153]; 3],
        };
        for (x, w) in [5, 15, 25].into_iter().zip(want) {
            assert_px(&format!("最初の {word} x={x}"), px(&got, x), w);
        }
    }
}

#[test]
fn draw_methods_nested_asis_face_shows_black_where_transparent_on_warp() {
    let mut scene = Scene::new();
    let got = scene.draw(5000, &BindSet::default());
    assert_px("x=5", px(&got, 5), RED);
    assert_px("x=15", px(&got, 15), [0, 0, 128, 255]);
    assert_px("x=25", px(&got, 25), [0, 0, 0, 255]);
}

#[test]
fn draw_methods_simultaneous_reduce_takes_the_earlier_animation_as_its_base_on_warp() {
    let mut scene = Scene::new();
    let got = scene.draw(6000, &BindSet::default());
    assert_px("overlay→reduce x=5", px(&got, 5), [0, 0, 153, 153]);
    assert_px("overlay→reduce x=15", px(&got, 15), [0, 0, 77, 77]);
    assert_px("overlay→reduce x=25", px(&got, 25), [0, 153, 0, 153]);
    let got = scene.draw(6001, &BindSet::default());
    assert_px("reduce→overlay x=5", px(&got, 5), [0, 0, 153, 153]);
    assert_px("reduce→overlay x=25", px(&got, 25), [0, 255, 0, 255]);
}

#[test]
fn draw_methods_pattern_base_replaces_the_layer_and_ignores_xy_on_warp() {
    let mut scene = Scene::new();
    let blue60 = [153, 0, 0, 153];
    let got = scene.draw(7000, &BindSet::default());
    for x in [5, 15, 25] {
        assert_px(&format!("全域の面 x={x}"), px(&got, x), blue60);
    }
    let got = scene.draw(7001, &BindSet::default());
    assert_px("範囲の中 x=5", px(&got, 5), [255, 0, 0, 255]);
    assert_px("範囲の外 x=15", px(&got, 15), CLEAR);
    assert_px("範囲の外 x=25", px(&got, 25), CLEAR);
    let got = scene.draw(7002, &BindSet::from_ids([72]));
    for x in [5, 15, 25] {
        assert_px(&format!("着せ替え x={x}"), px(&got, x), blue60);
    }
}
