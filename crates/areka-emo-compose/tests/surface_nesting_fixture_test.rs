//! # surface_nesting_fixture_test — 入れ子の検体が今の読み手で読めること（areka-P0-surface-element-nesting task 1.2・要件 8.2）
//!
//! 検体 `tests/fixtures/surface-nesting/` を、今ある読み手（`parse`・`parse_boxes`）→ 面の表
//! （`EmoWorld::build_with_images`）→ 箱の畳み込み（`fold_boxes`）へ通し、読み手が検体を
//! 取りこぼさないことと、検体が自分の冒頭の説明どおりの形をしていることを確かめる。
//! 入れ子の意味（数字だけの element定義の読み分け・報告の件数・合成）を確かめるのは後続の
//! タスクの `src/nesting_*_tests.rs` で、ここは検体そのものの番をするだけである。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use areka_emo_compose::{EmoWorld, fold_boxes};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::shell::{parse, parse_boxes};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("surface-nesting")
}

fn fixture_text() -> String {
    let path = fixture_dir().join("surfaces.txt");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("検体 {} の読取に失敗した: {e}", path.display()));
    decode(&bytes, DefaultEncoding::Utf8)
}

/// フォルダ直下の `surface<数字>.png`（ファイル名の慣習だけで面になる絵）の対応。
fn convention_images() -> BTreeMap<u32, String> {
    let mut images = BTreeMap::new();
    for entry in std::fs::read_dir(fixture_dir()).expect("検体のフォルダが読める") {
        let name = entry.expect("フォルダの項目が読める").file_name();
        let name = name.to_string_lossy().into_owned();
        let Some(digits) = name
            .strip_prefix("surface")
            .and_then(|s| s.strip_suffix(".png"))
        else {
            continue;
        };
        if let Ok(id) = digits.parse::<u32>() {
            images.insert(id, name);
        }
    }
    images
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

#[test]
fn the_fixture_parses_into_the_documented_surfaces() {
    let shell = parse(&fixture_text());

    let braces: BTreeSet<u32> = shell.surfaces.iter().map(|s| s.id).collect();
    assert_eq!(
        braces,
        BTreeSet::from([0, 1, 2, 10, 12, 30, 31, 32, 40, 41, 50, 60, 61, 62, 70, 71]),
        "surface*ブレスの番号が冒頭の説明と一致する"
    );
    assert_eq!(shell.appends.len(), 1, "surface.append*ブレスは 1 つ");
    let append_elements = &shell.appends[0].elements;
    assert_eq!(append_elements.len(), 1);
    assert!(
        append_elements.iter().all(|e| is_digits(e.path.as_str())),
        "surface.append*ブレスには画像の element定義を書かない（design.md「残した」）"
    );

    // 数字だけの element定義（子を指す行）は、今の読み手でも欄の原文のまま転記される。
    let numeric: Vec<(u32, &str)> = shell
        .surfaces
        .iter()
        .flat_map(|s| s.elements.iter().map(move |e| (s.id, e.path.as_str())))
        .filter(|(_, p)| is_digits(p))
        .collect();
    assert_eq!(
        numeric,
        vec![
            (0, "10"),
            (0, "30"),
            (1, "10"),
            (2, "10"),
            (2, "11"),
            (30, "31"),
            (31, "32"),
            (50, "9999"),
            (50, "4294967296"),
            (60, "60"),
            (61, "62"),
            (62, "61"),
            (71, "70"),
        ],
        "子を指す element定義の並び（親の出現順・element番号の昇順）"
    );

    // 画像の element定義が指すファイルは、すべてフォルダに在る。
    for surface in &shell.surfaces {
        for element in &surface.elements {
            let path = element.path.as_str();
            if is_digits(path) {
                continue;
            }
            assert!(
                fixture_dir().join(path).is_file(),
                "surface{} の {path} がフォルダに無い",
                surface.id
            );
        }
    }
}

#[test]
fn the_fixture_builds_a_face_table_with_the_convention_only_child() {
    let text = fixture_text();
    let images = convention_images();
    assert_eq!(
        images.keys().copied().collect::<Vec<_>>(),
        vec![2, 11],
        "ファイル名の慣習の絵は surface2.png（element0 に隠される）と surface11.png（ブレスの無い子）"
    );

    let world = EmoWorld::build_with_images(&parse(&text), &images);
    let ids: BTreeSet<u32> = world.surface_ids().collect();
    assert!(ids.contains(&11), "ブレスを持たない子 11 も面の表に在る");
    assert!(!ids.contains(&9999), "無い番号は面の表に無いまま");
    assert_eq!(
        world
            .base_images()
            .shadowed
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![2],
        "surface2.png は element0 が在るので土台に使われない"
    );
    assert_eq!(
        world.dangling_pattern_targets(),
        BTreeSet::new(),
        "pattern定義の先はすべて在る"
    );

    // 箱の読み手が子 70 の箱を拾うこと（報告の件数は見ない・件数の比べ合いは検体を通すテストの役目）。
    let (layout, _report) = fold_boxes(&parse_boxes(&text), &images, &world);
    assert_eq!(layout.placements(70).len(), 1, "子 70 が箱を 1 つ持つ");
}
