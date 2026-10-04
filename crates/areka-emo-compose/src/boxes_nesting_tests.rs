//! 子として置かれたサーフェスの箱（spec: areka-P0-surface-element-nesting 要件 6.1〜6.4・8.1）。
//!
//! 箱の置き場所の表は入れ子で変えない（子の箱は親の中に置かれない）。代わりに面の表の
//! element定義の辺 (親, 子) ごとに、子の箱 1 つにつき [`BoxIssue::InChildSurface`] を 1 件載せる。

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface, parse, parse_boxes};

use super::{BoxIssue, BoxLayout, BoxName, BoxPlacement, fold_boxes};
use crate::Composer;
use crate::bind::BindSet;
use crate::pattern::PatternState;
use crate::world::EmoWorld;

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

/// 検体の、子 70（箱を持つ）を置く行。
const PLACE_70: &str = "element1,overlay,70,10,10\n";

fn fold(text: &str) -> (BoxLayout, Vec<BoxIssue>) {
    let world = EmoWorld::build(&parse(text));
    let (layout, report) = fold_boxes(&parse_boxes(text), &Default::default(), &world);
    (layout, report.issues)
}

fn in_child(parent: u32, child: u32, name: &str) -> BoxIssue {
    BoxIssue::InChildSurface {
        parent,
        child,
        name: name.to_string(),
    }
}

fn only_in_child(issues: &[BoxIssue]) -> Vec<BoxIssue> {
    issues
        .iter()
        .filter(|i| matches!(i, BoxIssue::InChildSurface { .. }))
        .cloned()
        .collect()
}

fn without(text: &str, line: &str) -> String {
    assert!(text.contains(line), "文面に {line:?} が在る");
    text.replacen(line, "", 1)
}

/// 検体: 71 が置く子 70 の箱 `fuda` が 1 件だけ載り、他の報告は無い（要件 6.1・8.1・検体の冒頭の説明）。
#[test]
fn fixture_reports_the_one_box_in_a_child() {
    let (_, issues) = fold(FIXTURE);
    assert_eq!(issues, vec![in_child(71, 70, "fuda")]);
}

/// 箱を持つ子を置くと報告がちょうど 1 件増え、置き場所の表は置く前と同じ（要件 6.1・6.3・6.4）。
#[test]
fn placing_a_box_child_adds_exactly_one_report_and_keeps_the_layout() {
    let (layout_before, issues_before) = fold(&without(FIXTURE, PLACE_70));
    let (layout_after, issues_after) = fold(FIXTURE);

    assert_eq!(layout_after, layout_before, "子の箱は親の中に置かれない");
    assert!(layout_after.placements(71).is_empty());
    let mut expected = issues_before;
    expected.push(in_child(71, 70, "fuda"));
    assert_eq!(issues_after, expected);
}

/// 箱 1 つにつき 1 件・辺 (親, 子) ごと・並びは親 → element定義 → 箱（要件 6.1）。
///
/// 子 5 は箱 2 つ（element番号の昇順に b・a）、子 6 は箱 1 つ。親 1 は 6 と 5 を置き、5 を 2 度置く
/// （同じ辺は 1 本）。親 0 は 5 を置く。3 段目（6 の子 7）は箱を持たないので載らない。
/// 面の表に無い番号（9）を指す行は、無い番号の報告の受け持ちなのでここには載らない。
#[test]
fn one_report_per_box_per_parent_child_edge_in_order() {
    let text = "balloon.a\n{\nsize,10,10\n}\nballoon.b\n{\nsize,10,10\n}\n\
        surface1\n{\nelement0,overlay,6,0,0\nelement1,overlay,5,0,0\nelement2,overlay,5,3,3\nelement3,overlay,9,0,0\n}\n\
        surface0\n{\nelement0,overlay,5,0,0\n}\n\
        surface5\n{\nelement0,overlay,p.png,0,0\nelement2,balloon,a,0,0\nelement1,balloon,b,0,0\n}\n\
        surface6\n{\nelement0,overlay,7,0,0\nelement1,balloon,a,1,1\n}\n\
        surface7\n{\nelement0,overlay,p.png,0,0\n}\n";
    let (_, issues) = fold(text);
    assert_eq!(
        only_in_child(&issues),
        vec![
            in_child(0, 5, "b"),
            in_child(0, 5, "a"),
            in_child(1, 6, "a"),
            in_child(1, 5, "b"),
            in_child(1, 5, "a"),
        ]
    );
}

/// 子を一番上に表示したときは、子の箱が今までどおり置かれる（要件 6.3）。
#[test]
fn child_shown_as_top_keeps_its_boxes() {
    let (layout, _) = fold(FIXTURE);
    assert_eq!(
        layout.placements(70),
        &[BoxPlacement {
            element: 1,
            name: BoxName("fuda".to_string()),
            x: 0,
            y: 0,
        }]
    );
    let (unplaced, _) = fold(&without(FIXTURE, PLACE_70));
    assert_eq!(unplaced.placements(70), layout.placements(70));
}

/// 親に直接書いた箱の置き場所は、子を置いているかどうかで変わらない（要件 6.4）。
///
/// サーフェスを置く element定義も絵として描かれるので、「画像の element より小さい番号の箱」の
/// 判定では画像に数える（`place` が面の表の element を種類で分けずに数える今の成り行き）。
#[test]
fn parent_boxes_do_not_depend_on_placing_children() {
    let base = "balloon.a\n{\nsize,10,10\n}\n\
        surface3\n{\nelement0,overlay,p.png,0,0\n}\n\
        surface0\n{\nelement0,overlay,p.png,0,0\nelement1,balloon,a,4,5\n";
    let with_child = format!("{base}element2,overlay,3,0,0\n}}\n");
    let alone = format!("{base}}}\n");

    let (layout_child, issues_child) = fold(&with_child);
    let (layout_alone, issues_alone) = fold(&alone);
    assert_eq!(layout_child.placements(0), layout_alone.placements(0));
    assert_eq!(layout_child.placements(0).len(), 1);
    assert_eq!(issues_alone, vec![]);
    assert_eq!(
        issues_child,
        vec![BoxIssue::ElementBelowImage {
            surface: 0,
            element: 1,
            name: "a".to_string(),
            image_element: 2,
        }],
        "子を置く element定義は画像に数える"
    );
}

// ---- 合成（子の残りの element定義は箱に影響されない・要件 6.2） ----

/// 画像の名前と、それぞれの色（区別できるよう別の値で塗る）。
const IMAGES: &[(&str, u8)] = &[("p.png", 40), ("q.png", 120), ("r.png", 200)];

fn atlas() -> AtlasTable {
    let base = Path::new("shell/master");
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements: IMAGES
            .iter()
            .map(|(rel, _)| Element {
                layer: 0,
                path: ElementPath::new(rel.to_string()),
                x: 0,
                y: 0,
            })
            .collect(),
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let mut dec = MemoryDecoder::new();
    for (rel, value) in IMAGES {
        dec.insert(base.join(rel), 4, 4, 16, vec![*value; 64], true);
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let result = bake(&[set], &dec, PackConfig::default());
    assert!(result.errors.is_empty(), "bake の仕込みは失敗しない");
    result.table
}

fn compose(text: &str, top: u32) -> (u32, u32, Vec<u8>) {
    let atlas = atlas();
    let mut world = EmoWorld::build(&parse(text));
    world.bind_atlas(&atlas, SetId(0));
    let out = Composer::new()
        .compose(
            &world,
            &atlas,
            top,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("合成できる");
    (out.width(), out.height(), out.bytes().to_vec())
}

/// 子 5 は画像（q）・サーフェス（6 の r）・箱を持つ。親 0 で見ても子 5 を一番上で見ても、合成は
/// 箱の行が無いときと同じで、子の画像とサーフェスは描かれる（要件 6.2）。
#[test]
fn child_images_and_surfaces_compose_the_same_with_or_without_its_box() {
    let box_line = "element3,balloon,a,0,0\n";
    let text = format!(
        "balloon.a\n{{\nsize,10,10\n}}\n\
        surface0\n{{\nelement0,overlay,p.png,0,0\nelement1,overlay,5,2,2\n}}\n\
        surface5\n{{\nelement0,overlay,q.png,0,0\nelement1,overlay,6,4,0\n{box_line}}}\n\
        surface6\n{{\nelement0,overlay,r.png,0,0\n}}\n"
    );
    let plain = without(&text, box_line);

    for top in [0, 5] {
        assert_eq!(compose(&text, top), compose(&plain, top), "top {top}");
    }
    let (w, h, bytes) = compose(&text, 0);
    assert_eq!((w, h), (10, 6), "子 5 とその子 6 まで外形に入る");
    for value in [40, 120, 200] {
        assert!(bytes.contains(&value), "色 {value} が描かれる");
    }

    let (layout, issues) = fold(&text);
    assert!(
        layout.placements(0).is_empty(),
        "子の箱は親の中に置かれない"
    );
    assert_eq!(layout.placements(5).len(), 1);
    assert_eq!(issues, vec![in_child(0, 5, "a")]);
}
