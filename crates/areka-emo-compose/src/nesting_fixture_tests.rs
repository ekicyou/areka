//! 検体を読み手から合成まで通す。
//!
//! `tests/fixtures/surface-nesting/` の surfaces.txt を 解析 → 面の表 → 焼く → 計画 の順に通す。
//! 焼く一覧は本番の入口（`areka-emo-present` の `build_shell_target_with_boxes`）と同じ形
//! （数字だけの element定義を外し、使う `surface<数字>.png` を層 0 の面として足す）で組む。
//! 画像は `MemoryDecoder` に、検体の PNG の見出し（IHDR）から読んだ原寸の単色を入れる
//! （計画の項はファイル名で「どの絵か」を見分ける）。

use std::path::PathBuf;

use areka_emo_atlas::{AtlasTable, MemoryDecoder, SetId};
use areka_parsers::shell::{parse, parse_boxes};

use super::{ElementKind, NestIssue, element_kind};
use crate::boxes::{BoxIssue, fold_boxes};
use crate::plan_test_support::{
    bake_surfaces, elem, id_to_path, image_map, named_plan, opaque_wxh, png_size, surface,
};
use crate::{BindSet, Composer, DrawPlan, EmoWorld, PatternState};

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

/// 検体の絵と、冒頭の説明に書いた原寸と、焼く単色（B, G, R）。
const FILES: &[(&str, u32, u32, [u8; 3])] = &[
    ("body.png", 100, 200, [128, 128, 128]),
    ("eye.png", 20, 10, [255, 0, 0]),
    ("eye_closed.png", 20, 10, [255, 255, 0]),
    ("part.png", 10, 10, [0, 0, 255]),
    ("mouth.png", 16, 8, [0, 255, 0]),
    ("mouth_open.png", 16, 8, [255, 0, 255]),
    ("surface2.png", 100, 200, [0, 255, 255]),
    ("surface11.png", 12, 12, [0, 128, 255]),
];

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/surface-nesting")
}

fn file(name: &str) -> &'static (&'static str, u32, u32, [u8; 3]) {
    FILES
        .iter()
        .find(|f| f.0 == name)
        .unwrap_or_else(|| panic!("{name} は検体の絵の一覧に無い"))
}

/// 焼く単色（不透明の premultiplied BGRA）。
fn color(name: &str) -> [u8; 4] {
    let [b, g, r] = file(name).3;
    [b, g, r, 255]
}

/// 報告の件数（仕込んだ種類と、仕込んでいない種類）。
#[derive(Debug, PartialEq, Eq)]
struct Tally {
    missing_target: usize,
    cycle: usize,
    box_in_child: usize,
    other_box_issues: usize,
    dangling_patterns: usize,
    bake_errors: usize,
}

/// 検体に仕込んだ件数（surfaces.txt の冒頭の説明）。
const PLANTED: Tally = Tally {
    missing_target: 2,
    cycle: 3,
    box_in_child: 1,
    other_box_issues: 0,
    dangling_patterns: 0,
    bake_errors: 0,
};

struct Loaded {
    world: EmoWorld,
    atlas: AtlasTable,
    tally: Tally,
}

/// 文面を 解析 → 面の表 → 焼く まで通す（面の表は焼いた表に束縛済み）。
fn load(text: &str) -> Loaded {
    let shell = parse(text);
    // ファイル名の慣習だけで面になる絵（surface_nesting_fixture_test がフォルダの中身を番する）。
    let images = image_map(&[(2, "surface2.png"), (11, "surface11.png")]);
    let mut world = EmoWorld::build_with_images(&shell, &images);
    let nest = world.nest_report();
    let (_, boxes) = fold_boxes(&parse_boxes(text), &images, &world);

    // 本番の焼く一覧と同じ形: 数字だけの element定義を外し、使う慣習の絵を層 0 の面として足す。
    let mut surfaces = shell.surfaces.clone();
    for s in &mut surfaces {
        s.elements
            .retain(|e| element_kind(&e.path) == ElementKind::Image);
    }
    surfaces.extend(
        world
            .base_images()
            .used
            .iter()
            .map(|(&id, name)| surface(id, vec![elem(0, name, 0, 0)])),
    );

    let dir = fixture_dir();
    let mut dec = MemoryDecoder::new();
    for &(name, _, _, _) in FILES {
        let (w, h) = png_size(&dir.join(name));
        let [b, g, r, _] = color(name);
        let (w, h, stride, bgra, has_alpha) = opaque_wxh(w, h, b, g, r);
        dec.insert(dir.join(name), w, h, stride, bgra, has_alpha);
    }
    let baked = bake_surfaces(&surfaces, &dir, &dec);
    world.bind_atlas(&baked.table, SetId(0));

    let count = |f: fn(&NestIssue) -> bool| nest.issues.iter().filter(|i| f(i)).count();
    let box_in_child = boxes
        .issues
        .iter()
        .filter(|i| matches!(i, BoxIssue::InChildSurface { .. }))
        .count();
    let tally = Tally {
        missing_target: count(|i| matches!(i, NestIssue::MissingTarget { .. })),
        cycle: count(|i| matches!(i, NestIssue::Cycle { .. })),
        box_in_child,
        other_box_issues: boxes.issues.len() - box_in_child,
        dangling_patterns: world.dangling_pattern_targets().len(),
        bake_errors: baked.errors.len(),
    };
    Loaded {
        world,
        atlas: baked.table,
        tally,
    }
}

/// `top` の計画と、その絵の項の (画像の名前, X, Y) の列。
fn plan(loaded: &Loaded, top: u32) -> (DrawPlan, Vec<(&'static str, i64, i64)>) {
    let plan = Composer::new()
        .plan(
            &loaded.world,
            &loaded.atlas,
            top,
            &BindSet::default(),
            &PatternState::default(),
        )
        .unwrap_or_else(|e| panic!("surface{top} の計画に失敗した: {e:?}"));
    let names: Vec<&'static str> = FILES
        .iter()
        .map(|f| f.0)
        .filter(|n| loaded.atlas.resolve(SetId(0), n).is_some())
        .collect();
    let named = named_plan(&plan, &id_to_path(&loaded.atlas, &names));
    (plan, named)
}

/// 検体の PNG は冒頭の説明どおりの原寸を見出しに持つ（以下の画素の座標はこの原寸で数える）。
#[test]
fn fixture_pngs_have_the_documented_sizes() {
    for &(name, w, h, _) in FILES {
        assert_eq!(png_size(&fixture_dir().join(name)), (w, h), "{name}");
    }
}

/// 報告の件数が仕込んだ件数と一致し、仕込んでいない種類は 0 件。
#[test]
fn report_counts_equal_the_planted_counts() {
    assert_eq!(load(FIXTURE).tally, PLANTED);
}

/// 仕込んだ行を 1 つ消すと件数の比べ合いが赤になる（上の比べ合いが検体の行を見ていることの確かめ）。
#[test]
fn removing_any_planted_line_breaks_the_count() {
    for line in [
        "element1,overlay,9999,0,0",
        "element2,overlay,4294967296,0,0",
        "element1,overlay,60,0,0",
        "element1,overlay,62,0,0",
        "element1,overlay,61,0,0",
        "element1,balloon,fuda,0,0",
    ] {
        assert_eq!(FIXTURE.lines().filter(|l| *l == line).count(), 1, "{line}");
        let text: String = FIXTURE
            .lines()
            .filter(|l| *l != line)
            .map(|l| format!("{l}\n"))
            .collect();
        assert_ne!(
            load(&text).tally,
            PLANTED,
            "{line} を消しても件数が変わらない"
        );
    }
}

/// 3 つの親（0・1・2）がどれも同じ子 10 の絵（eye.png）を、置いた位置に含む。
#[test]
fn every_parent_contains_the_shared_child_image() {
    let loaded = load(FIXTURE);
    // 親 A: 子 10 を (20,30)。
    let (_, s0) = plan(&loaded, 0);
    assert_eq!(s0[0], ("body.png", 0, 0));
    assert!(s0.contains(&("eye.png", 20, 30)), "{s0:?}");
    // 親 B: 子 10 を (90,50)。親の画像（幅 100）の右へはみ出した所も外形に入る。
    let (p1, s1) = plan(&loaded, 1);
    assert_eq!((p1.extent.w, p1.extent.h), (110, 200));
    assert_eq!(s1[..2], [("body.png", 0, 0), ("eye.png", 90, 50)]);
    // 親 C: element0 が子 10（surface2.png は使われない）・ブレスの無い子 11 を (50,50)。
    let (p2, s2) = plan(&loaded, 2);
    assert_eq!((p2.extent.w, p2.extent.h), (62, 62));
    assert_eq!(
        s2,
        [("eye.png", 0, 0), ("surface11.png", 50, 50)],
        "surface2.png は描かれない"
    );
}

/// 3 段目（32）の絵は各段の位置の和に出る。
#[test]
fn third_level_image_is_at_the_sum_of_every_offset() {
    let loaded = load(FIXTURE);
    // 親 A: 30 を (5,5)・31 を (3,4)・32 を (2,1) → 32 は (10,10)。
    let (_, s0) = plan(&loaded, 0);
    for at in [(5, 5), (8, 9), (10, 10)] {
        assert!(s0.contains(&("part.png", at.0, at.1)), "{at:?}: {s0:?}");
    }
    // 親 B: surface.append1 が 30 を (60,100) → 32 は (65,105)。
    let (_, s1) = plan(&loaded, 1);
    assert!(s1.contains(&("part.png", 65, 105)), "{s1:?}");
}
