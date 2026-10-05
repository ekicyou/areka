//! 検体を読み手から合成まで通す（要件 8.2・8.3・タスク 8.1）。
//!
//! `tests/fixtures/surface-nesting/` の surfaces.txt を 解析 → 面の表 → 焼く → 合成 の順に通す。
//! 焼く一覧は本番の入口（`areka-emo-present` の `build_shell_target_with_boxes`）と同じ形
//! （数字だけの element定義を外し、使う `surface<数字>.png` を層 0 の面として足す）で組む。
//! 画像は `MemoryDecoder` に、検体の PNG の見出し（IHDR）から読んだ原寸の単色を入れる
//! （ファイルごとに色を変え、合成の画素で「どの絵か」を見分ける）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface, parse, parse_boxes};

use super::{ElementKind, NestIssue, element_kind};
use crate::boxes::{BoxIssue, fold_boxes};
use crate::hit::{RegionPriority, hit_region_in};
use crate::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState};

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

/// 検体の絵と、冒頭の説明に書いた原寸と、合成で見分ける色（B, G, R）。
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

/// PNG の見出しから原寸を読む（署名と IHDR が無ければ赤）。
fn png_size(path: &Path) -> (u32, u32) {
    let bytes =
        std::fs::read(path).unwrap_or_else(|e| panic!("{} が読めない: {e}", path.display()));
    assert!(
        bytes.len() >= 24 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") && &bytes[12..16] == b"IHDR",
        "{} は PNG の見出しを持たない",
        path.display()
    );
    let be = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    (be(16), be(20))
}

fn file(name: &str) -> &'static (&'static str, u32, u32, [u8; 3]) {
    FILES
        .iter()
        .find(|f| f.0 == name)
        .unwrap_or_else(|| panic!("{name} は検体の絵の一覧に無い"))
}

/// 合成の画素として見える色（不透明の premultiplied BGRA）。
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

/// 検体に仕込んだ件数（surfaces.txt の冒頭の説明・tasks.md 1.2 の記録）。
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
    let images = BTreeMap::from([
        (2, "surface2.png".to_string()),
        (11, "surface11.png".to_string()),
    ]);
    let mut world = EmoWorld::build_with_images(&shell, &images);
    let nest = world.nest_report();
    let (_, boxes) = fold_boxes(&parse_boxes(text), &images, &world);

    // 本番の焼く一覧と同じ形: 数字だけの element定義を外し、使う慣習の絵を層 0 の面として足す。
    let mut surfaces = shell.surfaces.clone();
    for s in &mut surfaces {
        s.elements
            .retain(|e| element_kind(&e.path) == ElementKind::Image);
    }
    surfaces.extend(world.base_images().used.iter().map(|(&id, name)| Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new(name.clone()),
            x: 0,
            y: 0,
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }));

    let dir = fixture_dir();
    let mut dec = MemoryDecoder::new();
    for &(name, _, _, _) in FILES {
        let (w, h) = png_size(&dir.join(name));
        let px = color(name);
        dec.insert(
            dir.join(name),
            w,
            h,
            w * 4,
            px.repeat((w * h) as usize),
            true,
        );
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: &dir,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(std::slice::from_ref(&set), &dec, PackConfig::default());
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

fn compose(loaded: &Loaded, top: u32) -> ComposedSurface {
    Composer::new()
        .compose(
            &loaded.world,
            &loaded.atlas,
            top,
            &BindSet::default(),
            &PatternState::default(),
        )
        .unwrap_or_else(|e| panic!("surface{top} の合成に失敗した: {e:?}"))
}

fn pixel(s: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    assert!(x < s.width() && y < s.height(), "({x},{y}) は外形の外");
    let at = (y * s.stride() + x * 4) as usize;
    s.bytes()[at..at + 4].try_into().unwrap()
}

/// 検体の PNG は冒頭の説明どおりの原寸を見出しに持つ（以下の画素の座標はこの原寸で数える）。
#[test]
fn fixture_pngs_have_the_documented_sizes() {
    for &(name, w, h, _) in FILES {
        assert_eq!(png_size(&fixture_dir().join(name)), (w, h), "{name}");
    }
}

/// 報告の件数が仕込んだ件数と一致し、仕込んでいない種類は 0 件（要件 8.2・design「検体を通す」）。
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

/// 3 つの親（0・1・2）がどれも同じ子 10 の絵（eye.png）を、置いた位置に含む（要件 8.2）。
#[test]
fn every_parent_contains_the_shared_child_image() {
    let loaded = load(FIXTURE);
    // 親 A: 子 10 を (20,30)。
    let s0 = compose(&loaded, 0);
    assert_eq!(pixel(&s0, 25, 35), color("eye.png"));
    assert_eq!(pixel(&s0, 19, 35), color("body.png"));
    // 親 B: 子 10 を (90,50)。親の画像（幅 100）の右へはみ出した所も描かれる。
    let s1 = compose(&loaded, 1);
    assert_eq!((s1.width(), s1.height()), (110, 200));
    assert_eq!(pixel(&s1, 105, 55), color("eye.png"));
    assert_eq!(pixel(&s1, 89, 55), color("body.png"));
    // 親 C: element0 が子 10（surface2.png は使われない）・ブレスの無い子 11 を (50,50)。
    let s2 = compose(&loaded, 2);
    assert_eq!((s2.width(), s2.height()), (62, 62));
    assert_eq!(pixel(&s2, 5, 5), color("eye.png"));
    assert_eq!(pixel(&s2, 55, 55), color("surface11.png"));
    assert_eq!(
        pixel(&s2, 30, 30),
        [0, 0, 0, 0],
        "surface2.png は描かれない"
    );
}

/// 3 段目（32）の絵は各段の位置の和に出る（要件 2.3）。part.png（10x10）の右下の角で見る:
/// 30・31 は 32 の右下の角を覆わないので、そこが part.png の色なのは 32 だけのため。
#[test]
fn third_level_image_is_at_the_sum_of_every_offset() {
    let loaded = load(FIXTURE);
    // 親 A: 30 を (5,5)・31 を (3,4)・32 を (2,1) → 32 は (10,10)〜(20,20)。
    let s0 = compose(&loaded, 0);
    assert_eq!(pixel(&s0, 19, 19), color("part.png"));
    assert_eq!(pixel(&s0, 20, 20), color("body.png"));
    // 親 B: surface.append1 が 30 を (60,100) → 32 は (65,105)〜(75,115)。
    let s1 = compose(&loaded, 1);
    assert_eq!(pixel(&s1, 74, 114), color("part.png"));
    assert_eq!(pixel(&s1, 75, 115), color("body.png"));
}

/// 子 10 の領域が親の列に持ち込まれている（要件 4.1・4.2）。
#[test]
fn child_regions_are_imported_into_the_parents() {
    let loaded = load(FIXTURE);
    let names = |id: u32| -> Vec<String> {
        let list = loaded.world.hit_regions(id).expect("面の表に在る番号");
        list.iter().map(|c| c.name.as_str().to_string()).collect()
    };
    let hit = |id: u32, x: i64, y: i64| {
        let list = loaded.world.hit_regions(id).expect("面の表に在る番号");
        hit_region_in(list, x, y, RegionPriority::Painter).map(str::to_string)
    };
    // 親 A は Head を自分で持つので、持ち込むのは Eye だけ（Eye は親の Head の下に隠れる）。
    assert_eq!(names(0), ["Eye", "Head", "Body"]);
    // 親 B: 子を (90,50) → Head (90,50)-(110,60)・Eye (95,50)-(105,60)。
    assert_eq!(names(1), ["Head", "Eye"]);
    assert_eq!(hit(1, 100, 55).as_deref(), Some("Eye"));
    assert_eq!(hit(1, 92, 55).as_deref(), Some("Head"));
    assert_eq!(hit(1, 50, 55), None);
    // 親 C: 子を (0,0)。
    assert_eq!(hit(2, 10, 5).as_deref(), Some("Eye"));
    assert_eq!(hit(2, 2, 5).as_deref(), Some("Head"));
}
