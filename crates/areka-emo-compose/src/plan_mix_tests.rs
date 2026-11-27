//! 重ね方ごとの計画の規則（最初の項・`replace` の範囲・`asis`・まとめ）を、element定義・通常の
//! アニメーション・着せ替えの 3 つの場面で計画の値に固める檻。

use std::path::Path;

use areka_emo_atlas::{
    AnimatedImage, AnimationFrame, AtlasTable, DecodedImage, ElementId, LoopCount, MemoryDecoder,
    SetId,
};
use areka_parsers::shell::parse;

use crate::bind::BindSet;
use crate::draw_plan::{DrawPlan, IRect, Mix, PlanItem};
use crate::method::ComposeMethod;
use crate::normalized::Transform;
use crate::pattern::{PatternFrame, PatternState};
use crate::plan_test_support::{
    bake_surfaces, bound_world, draw_plan_of, elem, frame, insert_film_2x2, opaque_wxh, solid,
    surface, transparent_wxh,
};
use crate::world::EmoWorld;

const BASE: &str = "shell/master";

/// 2×2 の不透明の絵の名前（`r.png` は 4×4 の真ん中 2×2 だけ不透明・`t.png` は全透明）。
const OPAQUE: &[&str] = &[
    "a.png", "b.png", "c.png", "d.png", "e.png", "k.png", "p100.png",
];

/// 面 100 は `p100.png`・面 101 は全透明の `t101.png` だけをファイル名の慣習で置いた面（element定義を
/// 持たない）。`f.png` は 2×2 の動く絵で、経過 0 のコマは 1 番。
struct Scene {
    world: EmoWorld,
    atlas: AtlasTable,
}

impl Scene {
    fn new(text: &str) -> Scene {
        let shell = parse(text);
        let mut dec = MemoryDecoder::new();
        let base = Path::new(BASE);
        let mut put = |rel: &str, (width, height, stride, bgra, has_alpha)| {
            let image = DecodedImage {
                width,
                height,
                stride,
                bgra,
                has_alpha,
            };
            dec.insert_unmultiplied(base.join(rel), image.clone());
            dec.insert(base.join(rel), width, height, stride, image.bgra, has_alpha);
        };
        for rel in OPAQUE {
            put(rel, opaque_wxh(2, 2, 10, 20, 30));
        }
        let mut framed = transparent_wxh(4, 4);
        for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
            let i = (y * 4 + x) * 4;
            framed.3[i..i + 4].copy_from_slice(&[10, 20, 30, 255]);
        }
        put("r.png", framed);
        put("t.png", transparent_wxh(2, 2));
        put("t101.png", transparent_wxh(2, 2));
        let film = base.join("f.png");
        insert_film_2x2(&mut dec, film.clone(), &[0, 50], None);
        let frames: Vec<AnimationFrame> = (0..2)
            .map(|i| AnimationFrame {
                image: solid(2, 2, 10 + i),
                delay_ms: [0, 50][usize::from(i)],
            })
            .collect();
        let first = Ok(frames[0].image.clone());
        let loop_count = LoopCount::Infinite;
        dec.insert_animated_unmultiplied(film, first, Ok(AnimatedImage { frames, loop_count }));
        let mut surfaces = shell.surfaces.clone();
        surfaces.push(surface(100, vec![elem(0, "p100.png", 0, 0)]));
        surfaces.push(surface(101, vec![elem(0, "t101.png", 0, 0)]));
        let atlas = bake_surfaces(&surfaces, base, &dec).table;
        let world = bound_world(&shell, &[(100, "p100.png"), (101, "t101.png")], &atlas);
        Scene { world, atlas }
    }

    fn id(&self, rel: &str) -> ElementId {
        self.atlas.resolve(SetId(0), rel).expect(rel)
    }

    fn asis(&self, rel: &str) -> ElementId {
        self.atlas.resolve_asis(SetId(0), rel).expect(rel)
    }

    fn plan(&self, surface: u32, binds: &BindSet, pattern: &PatternState) -> DrawPlan {
        draw_plan_of(&self.world, &self.atlas, surface, binds, pattern).expect("計画を組める")
    }

    fn still(&self, surface: u32) -> Vec<PlanItem> {
        self.plan(surface, &BindSet::default(), &PatternState::default())
            .items
    }

    fn img(&self, rel: &str, x: i64, y: i64, mix: Mix) -> PlanItem {
        image(self.id(rel), x, y, mix, None)
    }
}

fn image(element: ElementId, x: i64, y: i64, mix: Mix, range: Option<IRect>) -> PlanItem {
    PlanItem::Image {
        element,
        transform: Transform::translate(x, y),
        mix,
        range,
    }
}

fn group(x: i64, y: i64, mix: Mix) -> PlanItem {
    PlanItem::GroupBegin {
        transform: Transform::translate(x, y),
        mix,
        range: None,
    }
}

fn rect(x: i64, y: i64, w: u32, h: u32) -> IRect {
    IRect { x, y, w, h }
}

fn with(method: ComposeMethod, surface: u32, x: i64, y: i64) -> PatternFrame {
    PatternFrame {
        method,
        ..frame(surface, x, y)
    }
}

const ELEMENTS: &str = "charset,UTF-8\n\
    surface0\n{\n\
    element0,overlay-fast,a.png,0,0\n\
    element1,overlay-fast,b.png,1,0\n\
    element2,interpolate,c.png,2,0\n\
    element3,replace,r.png,5,6\n\
    element4,reduce,d.png,0,1\n\
    element5,asis,k.png,1,1\n\
    element6,overlay,e.png,2,2\n\
    element7,base,a.png,3,3\n\
    }\n\
    surface1\n{\n\
    element0,overlay,missing.png,0,0\n\
    element1,reduce,d.png,0,0\n\
    element2,reduce,d.png,1,1\n\
    }\n\
    surface2\n{\n\
    element0,overlay,t.png,0,0\n\
    element1,replace,r.png,0,0\n\
    element2,replace,r.png,1,1\n\
    }\n\
    surface3\n{\n\
    element0,asis,k.png,0,0\n\
    element1,overlay,a.png,0,0\n\
    }\n";

/// element定義: 語ごとの重ね方、最初の項だけ `SourceOver`、`replace` の範囲は位置と縁を切る前の原寸、
/// `asis` は `asis` 用の絵を `SourceOver` で置く。
#[test]
fn element_words_map_to_mixes_with_first_as_source_over() {
    let s = Scene::new(ELEMENTS);
    assert_eq!(
        s.still(0),
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            s.img("b.png", 1, 0, Mix::SourceAtop),
            s.img("c.png", 2, 0, Mix::DestinationOver),
            image(s.id("r.png"), 5, 6, Mix::Copy, Some(rect(5, 6, 4, 4))),
            s.img("d.png", 0, 1, Mix::DestinationIn),
            image(s.asis("k.png"), 1, 1, Mix::SourceOver, None),
            s.img("e.png", 2, 2, Mix::SourceOver),
            s.img("a.png", 3, 3, Mix::SourceOver),
        ]
    );
}

/// 無い絵・全透明の絵の項は「最初」に数えず、描くものが在る最初の項を `SourceOver` にする（範囲も外す）。
#[test]
fn first_item_skips_missing_and_transparent_pictures() {
    let s = Scene::new(ELEMENTS);
    assert_eq!(
        s.still(1),
        [
            s.img("d.png", 0, 0, Mix::SourceOver),
            s.img("d.png", 1, 1, Mix::DestinationIn),
        ]
    );
    assert_eq!(
        s.still(2),
        [
            s.img("r.png", 0, 0, Mix::SourceOver),
            image(s.id("r.png"), 1, 1, Mix::Copy, Some(rect(1, 1, 4, 4))),
        ]
    );
}

/// 最初の `asis` は通常の絵へ戻さず、`asis` 用の絵のまま置く。
#[test]
fn first_asis_keeps_the_asis_picture() {
    let s = Scene::new(ELEMENTS);
    assert_eq!(
        s.still(3),
        [
            image(s.asis("k.png"), 0, 0, Mix::SourceOver, None),
            s.img("a.png", 0, 0, Mix::SourceOver),
        ]
    );
}

const ANIMATIONS: &str = "charset,UTF-8\n\
    surface0\n{\n\
    element0,overlay,a.png,0,0\n\
    animation12.interval,random,2\n\
    animation12.pattern0,asis,100,0,5,5\n\
    }\n\
    surface10\n{\nelement0,overlay,c.png,0,0\n}\n\
    surface11\n{\nelement0,overlay,c.png,0,0\nelement1,reduce,d.png,1,1\n}\n\
    surface13\n{\nelement0,overlay,e.png,1,1\n}\n\
    surface20\n{\nelement0,overlay,t.png,0,0\n}\n";

/// 通常のアニメーション: コマは ID の順に重なり、入れ子の面は中に `SourceOver` でない項が在るか
/// 指す語が `SourceOver` でないときだけまとめで囲む。`asis` は element定義を持つ面なら外形の黒の上の
/// まとめ、持たない面なら `asis` 用の絵 1 枚。
#[test]
fn animation_frames_group_by_word_and_contents_in_id_order() {
    let s = Scene::new(ANIMATIONS);
    let mut pattern = PatternState::default();
    pattern.set(12, with(ComposeMethod::Asis, 100, 5, 5));
    pattern.set(9, with(ComposeMethod::Asis, 13, 4, 4));
    pattern.set(8, with(ComposeMethod::OverlayFast, 10, 0, 0));
    pattern.set(7, frame(11, 2, 2));
    pattern.set(5, with(ComposeMethod::Reduce, 10, 3, 4));
    pattern.set(3, frame(10, 1, 2));
    let plan = s.plan(0, &BindSet::default(), &pattern);
    let outline = s.world.extent(13);
    assert_eq!(
        plan.items,
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            s.img("c.png", 1, 2, Mix::SourceOver),
            group(3, 4, Mix::DestinationIn),
            s.img("c.png", 0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
            group(2, 2, Mix::SourceOver),
            s.img("c.png", 0, 0, Mix::SourceOver),
            s.img("d.png", 1, 1, Mix::DestinationIn),
            PlanItem::GroupEnd,
            group(0, 0, Mix::SourceAtop),
            s.img("c.png", 0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
            PlanItem::Fill {
                rect: rect(4, 4, outline.w, outline.h)
            },
            group(4, 4, Mix::SourceOver),
            s.img("e.png", 1, 1, Mix::SourceOver),
            PlanItem::GroupEnd,
            image(s.asis("p100.png"), 5, 5, Mix::SourceOver, None),
        ]
    );
}

/// 土台の絵の無い面では、最初に描くコマの重ね方を `SourceOver` にし、ほかの項は中に `SourceOver` でない
/// ものが無いので平らに並べる。描くもののないコマは「最初」に数えない。
#[test]
fn first_frame_on_an_empty_surface_is_source_over() {
    let s = Scene::new(ANIMATIONS);
    let mut pattern = PatternState::default();
    pattern.set(1, with(ComposeMethod::Reduce, 10, 1, 1));
    pattern.set(2, with(ComposeMethod::Reduce, 10, 2, 2));
    let plan = s.plan(20, &BindSet::default(), &pattern);
    assert_eq!(
        plan.items,
        [
            s.img("c.png", 1, 1, Mix::SourceOver),
            group(2, 2, Mix::DestinationIn),
            s.img("c.png", 0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
        ]
    );
}

const NESTED: &str = "charset,UTF-8\n\
    surface0\n{\nelement0,overlay,a.png,0,0\n}\n\
    surface30\n{\nelement0,overlay,c.png,0,0\nelement1,interpolate,31,2,2\n}\n\
    surface31\n{\nelement0,overlay,d.png,0,0\nelement1,overlay,e.png,1,0\n}\n\
    surface50\n{\nelement0,overlay,51,1,1\n}\n\
    surface51\n{\nelement0,overlay,e.png,0,0\nelement1,reduce,d.png,1,1\n}\n";

/// まとめは入れ子の深さどおりに開いて閉じ、中の位置はまとめの左上から。平らに並べた面の中のまとめは
/// その面の位置を足した所に開く。
#[test]
fn groups_nest_and_positions_are_group_relative() {
    let s = Scene::new(NESTED);
    let mut pattern = PatternState::default();
    pattern.set(1, with(ComposeMethod::Reduce, 30, 3, 3));
    pattern.set(2, frame(50, 2, 0));
    let plan = s.plan(0, &BindSet::default(), &pattern);
    assert_eq!(
        plan.items,
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            group(3, 3, Mix::DestinationIn),
            s.img("c.png", 0, 0, Mix::SourceOver),
            group(2, 2, Mix::DestinationOver),
            s.img("d.png", 0, 0, Mix::SourceOver),
            s.img("e.png", 1, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
            PlanItem::GroupEnd,
            group(3, 1, Mix::SourceOver),
            s.img("e.png", 0, 0, Mix::SourceOver),
            s.img("d.png", 1, 1, Mix::DestinationIn),
            PlanItem::GroupEnd,
        ]
    );
}

const DRESS: &str = "charset,UTF-8\n\
    surface0\n{\n\
    element0,overlay,a.png,0,0\n\
    animation10.interval,bind\n\
    animation10.pattern0,reduce,20,0,1,1\n\
    animation10.pattern1,replace,21,0,2,2\n\
    animation10.pattern2,asis,100,0,3,3\n\
    animation10.pattern3,overlay-fast,20,0,0,0\n\
    animation10.pattern4,overlay,20,0,4,4\n\
    }\n\
    surface1\n{\n\
    animation10.interval,bind\n\
    animation10.pattern0,interpolate,20,0,0,0\n\
    animation10.pattern1,interpolate,20,0,1,1\n\
    }\n\
    surface20\n{\nelement0,overlay,c.png,0,0\n}\n\
    surface21\n{\nelement0,overlay,r.png,0,0\n}\n";

/// 着せ替え: pattern は順に重なり、指す語ごとのまとめ・`replace` のまとめの範囲（入れ子の面の外形）・
/// `asis` の絵は通常のアニメーションと同じ規則。
#[test]
fn dressup_patterns_follow_the_same_rules() {
    let s = Scene::new(DRESS);
    let plan = s.plan(0, &BindSet::from_ids([10]), &PatternState::default());
    assert_eq!(
        plan.items,
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            group(1, 1, Mix::DestinationIn),
            s.img("c.png", 0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
            PlanItem::GroupBegin {
                transform: Transform::translate(2, 2),
                mix: Mix::Copy,
                range: Some(rect(2, 2, 4, 4)),
            },
            s.img("r.png", 0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
            image(s.asis("p100.png"), 3, 3, Mix::SourceOver, None),
            group(0, 0, Mix::SourceAtop),
            s.img("c.png", 0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
            s.img("c.png", 4, 4, Mix::SourceOver),
        ]
    );
}

/// 着せ替えだけの面: 最初の pattern は土台の面へ写し（`SourceOver`・平ら）、以後は重ね方どおり。
#[test]
fn first_dressup_pattern_is_copied_onto_the_base() {
    let s = Scene::new(DRESS);
    let plan = s.plan(1, &BindSet::from_ids([10]), &PatternState::default());
    assert_eq!(
        plan.items,
        [
            s.img("c.png", 0, 0, Mix::SourceOver),
            group(1, 1, Mix::DestinationOver),
            s.img("c.png", 0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
        ]
    );
}

const EDGES: &str = "charset,UTF-8
    surface0
{
element0,overlay,a.png,0,0
}
    surface60
{
element0,overlay,c.png,0,0
element1,reduce,61,0,0
}
    surface61
{
element0,overlay,e.png,1,1
}
    surface62
{
element0,overlay,a.png,0,0
element1,overlay,60,1,1
}
    surface80
{
element0,asis,81,0,0
}
    surface81
{
element0,overlay,t.png,0,0
}
    surface90
{
element0,overlay,a.png,0,0
element1,asis,t.png,1,1
}
    surface91
{
element0,overlay,a.png,0,0
element1,asis,f.png,0,0
}
    surface92
{
element0,overlay,a.png,0,0
element1,asis,100,1,1
}
    surface93
{
animation1.interval,random,2
animation1.pattern0,asis,101,0,0,0
}
";

/// `SourceOver` の語で入れた面の中に `SourceOver` でない語の入れ子が在れば、外の面もまとめで囲む。
#[test]
fn a_nest_holding_a_non_source_over_nest_is_grouped() {
    let s = Scene::new(EDGES);
    assert_eq!(
        s.still(62),
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            group(1, 1, Mix::SourceOver),
            s.img("c.png", 0, 0, Mix::SourceOver),
            group(0, 0, Mix::DestinationIn),
            s.img("e.png", 1, 1, Mix::SourceOver),
            PlanItem::GroupEnd,
            PlanItem::GroupEnd,
        ]
    );
}

/// 中身が `asis` の黒の塗りだけの入れ子も描くものが在る面に数え、指す語のまとめで囲む。
#[test]
fn a_nest_holding_only_the_asis_black_is_drawn() {
    let s = Scene::new(EDGES);
    let mut pattern = PatternState::default();
    pattern.set(1, with(ComposeMethod::Reduce, 80, 1, 1));
    let outline = s.world.extent(81);
    assert_eq!(
        s.plan(0, &BindSet::default(), &pattern).items,
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            group(1, 1, Mix::DestinationIn),
            PlanItem::Fill {
                rect: rect(0, 0, outline.w, outline.h)
            },
            group(0, 0, Mix::SourceOver),
            PlanItem::GroupEnd,
            PlanItem::GroupEnd,
        ]
    );
}

/// 通常の形では全透明の絵も、`asis` なら `asis` 用の絵を置く（element定義の行・element定義を持たない面）。
#[test]
fn asis_draws_pictures_that_are_fully_transparent_in_the_normal_form() {
    let s = Scene::new(EDGES);
    assert_eq!(
        s.still(90),
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            image(s.asis("t.png"), 1, 1, Mix::SourceOver, None),
        ]
    );
    let mut pattern = PatternState::default();
    pattern.set(1, with(ComposeMethod::Asis, 101, 2, 2));
    assert_eq!(
        s.plan(0, &BindSet::default(), &pattern).items,
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            image(s.asis("t101.png"), 2, 2, Mix::SourceOver, None),
        ]
    );
}

/// `asis` の動く絵は、今のコマの `asis` 用の絵を置く。
#[test]
fn asis_film_draws_the_asis_picture_of_the_current_frame() {
    let s = Scene::new(EDGES);
    let frames = &s
        .atlas
        .animation(s.asis("f.png"))
        .expect("asis の動く絵")
        .frames;
    assert_eq!(
        s.still(91),
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            image(frames[1], 0, 0, Mix::SourceOver, None),
        ]
    );
}

/// element定義の行が `asis` で element定義を持たない面を指せば、その面の `asis` 用の絵を置く。
#[test]
fn asis_element_row_pointing_to_a_face_draws_its_asis_picture() {
    let s = Scene::new(EDGES);
    assert_eq!(
        s.still(92),
        [
            s.img("a.png", 0, 0, Mix::SourceOver),
            image(s.asis("p100.png"), 1, 1, Mix::SourceOver, None),
        ]
    );
}

/// 画素の上限で命令を切った所から始まる入れ子は、黒の塗りも空のまとめも作らない。
#[test]
fn a_nest_cut_by_the_pixel_cap_leaves_no_fill_or_empty_group() {
    let mut text = String::from(
        "charset,UTF-8
surface0
{
",
    );
    for i in 0..64 {
        text += &format!(
            "element{i},overlay,a.png,0,0
"
        );
    }
    text += "element64,asis,70,0,0
}
surface70
{
element0,overlay,b.png,0,0
}
";
    let s = Scene::new(&text);
    let items = s.still(0);
    assert_eq!(items, vec![s.img("a.png", 0, 0, Mix::SourceOver); 64]);
}
