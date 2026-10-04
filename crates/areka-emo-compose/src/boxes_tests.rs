//! `boxes.rs` の型と置き場所の表の問い合わせのテスト（spec: areka-P0-shell-balloon 要件 2.3・2.4）。
//!
//! 表は畳み込み（`fold_boxes`）を通さず、子モジュールの特権で非公開の欄から手で組む。

use std::collections::BTreeMap;

use areka_parsers::balloon::parse_str;

// 他のクレートから名指しできる公開の道（lib.rs の再公開）を通して型を引く。
use crate::{BoxDef, BoxLayout, BoxName, BoxPlacement, FontFollow};

fn name(s: &str) -> BoxName {
    // 非公開の構築子は再公開の道を通らないため、定義元から直に呼ぶ。
    super::BoxName(s.to_string())
}

fn def(w: u32, h: u32, follow: FontFollow) -> BoxDef {
    BoxDef {
        model: parse_str("", None),
        size: (w, h),
        follow,
    }
}

fn place(element: u32, n: &str, x: i64, y: i64) -> BoxPlacement {
    BoxPlacement {
        element,
        name: name(n),
        x,
        y,
    }
}

#[test]
fn empty_layout_has_no_placements_for_any_surface() {
    let layout = BoxLayout::default();
    assert!(layout.is_empty());
    for id in [0, 1, 10, u32::MAX] {
        assert!(layout.placements(id).is_empty(), "surface {id}");
    }
    assert_eq!(layout.def(&name("main")), None);
}

#[test]
fn layout_with_definitions_but_no_placements_is_empty() {
    let layout = BoxLayout {
        defs: BTreeMap::from([(name("main"), def(200, 80, FontFollow::Scope))]),
        surfaces: BTreeMap::new(),
    };
    assert!(layout.is_empty());
    assert!(layout.def(&name("main")).is_some());
}

#[test]
fn hand_built_layout_answers_lookups() {
    // 要件 2.3（1 つのサーフェスに複数の箱）・2.4（同じ名前を別の位置に）。
    let layout = BoxLayout {
        defs: BTreeMap::from([
            (name("main"), def(200, 80, FontFollow::Scope)),
            (name("sub"), def(120, 40, FontFollow::Balloon)),
        ]),
        surfaces: BTreeMap::from([
            (0, vec![place(10, "main", 5, 6), place(11, "sub", 7, 8)]),
            (3, vec![place(2, "main", -4, 100)]),
        ]),
    };

    assert!(!layout.is_empty());
    assert_eq!(
        layout.def(&name("main")),
        Some(&def(200, 80, FontFollow::Scope))
    );
    assert_eq!(
        layout.def(&name("sub")).map(|d| d.follow),
        Some(FontFollow::Balloon)
    );
    assert_eq!(layout.def(&name("none")), None);

    assert_eq!(
        layout.placements(0),
        &[place(10, "main", 5, 6), place(11, "sub", 7, 8)]
    );
    assert_eq!(layout.placements(3), &[place(2, "main", -4, 100)]);
    assert!(layout.placements(1).is_empty());
    assert!(layout.placements(u32::MAX).is_empty());

    // 置き場所の名前はすべて定義の表で引ける（設計の事後条件）。
    for id in [0, 3] {
        for p in layout.placements(id) {
            assert!(layout.def(&p.name).is_some(), "{}", p.name.as_str());
        }
    }
}

#[test]
fn box_name_reads_back_as_str_and_font_follow_defaults_to_scope() {
    assert_eq!(name("main").as_str(), "main");
    assert_eq!(FontFollow::default(), FontFollow::Scope);
}

// ---- `balloon.*`ブレスを定義として読む（fold_boxes の定義の側・要件 1.1〜1.5・1.8・1.9・
// 3.14・3.17・3.18・10.1・10.2）。文面はテストの中に持つ（検体は読まない）。

use areka_parsers::shell::parse_boxes;

use crate::{BoxIssue, BoxReport, EmoWorld, fold_boxes};

/// 文面を転記して畳む（面の画像は 0 件・画像の World は同じ文面から組む）。
fn fold(text: &str) -> (BoxLayout, BoxReport) {
    let world = EmoWorld::build(&areka_parsers::shell::parse(text));
    fold_boxes(&parse_boxes(text), &BTreeMap::new(), &world)
}

/// `balloon.名前`ブレス 1 つの文面（本体は行の列）。
fn brace(name: &str, lines: &[&str]) -> String {
    let mut text = format!("balloon.{name}\n{{\n");
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("}\n");
    text
}

#[test]
fn valid_brace_enters_table_with_size_and_descript_defaults() {
    // 要件 1.1・1.3・1.2（キーが無ければ descript.txt と同じ既定値）・3.17（書かなければ Scope）。
    let (layout, report) = fold(&brace("main", &["size,200,80"]));
    assert_eq!(report.issues, vec![]);
    assert_eq!(
        layout.def(&name("main")),
        Some(&BoxDef {
            model: parse_str("", None),
            size: (200, 80),
            follow: FontFollow::Scope,
        })
    );
}

#[test]
fn brace_keys_are_read_with_the_descript_meaning() {
    // 要件 1.2: 同じキーを descript.txt に書いたときと同じモデルになる（後勝ち・値の `,` を保つ）。
    let keys = [
        "origin.x,10",
        "origin.y,-5",
        "validrect.left,3",
        "validrect.bottom,-7",
        "wordwrappoint.x,-20",
        "font.height,14",
        "font.color.r,255",
        "font.color.g,1",
        "font.name,Meiryo,MS Gothic",
        "vertical,1",
        "cursor.style,square",
        "origin.x,12",
    ];
    let mut lines = vec!["size,100,50"];
    lines.extend(keys);
    let (layout, report) = fold(&brace("v", &lines));
    assert_eq!(report.issues, vec![]);
    let def = layout.def(&name("v")).expect("採られる");
    assert_eq!(def.model, parse_str(&keys.join("\n"), None));
    assert_eq!(def.model.origin().x(), Some(12), "同じキーは後勝ち");
    assert_eq!(def.size, (100, 50));
}

#[test]
fn font_follow_is_read_and_a_bad_value_is_reported_as_scope() {
    // 要件 3.14・3.17・3.18。
    for (value, want) in [
        ("scope", FontFollow::Scope),
        ("balloon", FontFollow::Balloon),
    ] {
        let line = format!("font.follow,{value}");
        let (layout, report) = fold(&brace("f", &["size,10,10", &line]));
        assert_eq!(report.issues, vec![], "{value}");
        assert_eq!(layout.def(&name("f")).map(|d| d.follow), Some(want));
    }

    let (layout, report) = fold(&brace("f", &["size,10,10", "font.follow,Balloon"]));
    assert_eq!(
        report.issues,
        vec![BoxIssue::BraceBadFollow {
            name: "f".into(),
            value: "Balloon".into(),
        }]
    );
    assert_eq!(
        layout.def(&name("f")).map(|d| d.follow),
        Some(FontFollow::Scope),
        "ブレスは採り scope として扱う"
    );
}

#[test]
fn inapplicable_keys_are_dropped_reported_and_the_rest_is_kept() {
    // 要件 1.5: キーごとに 1 件・ブレスの残りは採る。windowposition は読み手へ渡らないので
    // モデルの windowposition は未指定のまま。
    let (layout, report) = fold(&brace(
        "w",
        &[
            "size,10,20",
            "windowposition.x,10",
            "windowposition.y,20",
            "use_self_alpha,1",
            "use_input_alpha,1",
            "origin.x,4",
        ],
    ));
    let ignored = |key: &str| BoxIssue::BraceKeyIgnored {
        name: "w".into(),
        key: key.into(),
    };
    assert_eq!(
        report.issues,
        vec![
            ignored("use_input_alpha"),
            ignored("use_self_alpha"),
            ignored("windowposition.x"),
            ignored("windowposition.y"),
        ]
    );
    let def = layout.def(&name("w")).expect("残りのキーで採られる");
    assert_eq!(def.model, parse_str("origin.x,4", None));
    assert_eq!(def.model.windowposition().x(), None);
    assert_eq!(def.model.windowposition_raw().x_raw(), None);
}

#[test]
fn size_follow_and_inapplicable_keys_never_reach_the_descript_table() {
    // 要件 1.5・3.14: descript.txt の読み手へ渡る表を直に見る（モデルに現れないキーのため）。
    let lines: Vec<Vec<String>> = [
        "size,10,20",
        "font.follow,balloon",
        "windowposition.x,1",
        "use_self_alpha,1",
        "use_input_alpha,1",
        "origin.x,4",
        "font.name,a,b",
    ]
    .iter()
    .map(|l| l.split(',').map(String::from).collect())
    .collect();
    let mut issues = Vec::new();
    let (table, size, follow) = super::split_brace("t", &lines, &mut issues).expect("採られる");
    assert_eq!(
        table,
        BTreeMap::from([
            ("font.name".to_string(), "a,b".to_string()),
            ("origin.x".to_string(), "4".to_string()),
        ])
    );
    assert_eq!(size, (10, 20));
    assert_eq!(follow, FontFollow::Balloon);
    assert_eq!(issues.len(), 3);
}

#[test]
fn brace_without_size_is_not_taken() {
    // 要件 1.4。
    let (layout, report) = fold(&brace("m", &["origin.x,1"]));
    assert_eq!(
        report.issues,
        vec![BoxIssue::BraceMissingSize { name: "m".into() }]
    );
    assert_eq!(layout.def(&name("m")), None);
}

#[test]
fn brace_with_a_bad_size_is_not_taken() {
    // 要件 1.4: 幅か高さが正の整数として読めない。
    for value in ["0,80", "200,0", "200", "-1,5", "a,b", "200,80,3", ""] {
        let line = format!("size,{value}");
        let (layout, report) = fold(&brace("b", &[&line]));
        assert_eq!(
            report.issues,
            vec![BoxIssue::BraceBadSize {
                name: "b".into(),
                value: value.into(),
            }],
            "{value:?}"
        );
        assert_eq!(layout.def(&name("b")), None, "{value:?}");
    }
}

#[test]
fn brace_with_a_numeric_name_is_not_taken() {
    // 要件 1.9: `\b[ID番号]` と見分けがつかない名前。
    for n in ["123", "-1", "0"] {
        let (layout, report) = fold(&brace(n, &["size,10,10"]));
        assert_eq!(
            report.issues,
            vec![BoxIssue::BraceNumericName { name: n.into() }]
        );
        assert_eq!(layout.def(&name(n)), None);
    }
    let (layout, report) = fold(&brace("1a", &["size,10,10"]));
    assert_eq!(report.issues, vec![]);
    assert!(
        layout.def(&name("1a")).is_some(),
        "整数として読めない名前は採る"
    );
}

#[test]
fn brace_with_an_empty_name_is_not_taken() {
    // 要件 10.1。
    let (layout, report) = fold(&brace("", &["size,10,10"]));
    assert_eq!(
        report.issues,
        vec![BoxIssue::BraceEmptyName {
            heading: "balloon.".into(),
        }]
    );
    assert_eq!(layout.def(&name("")), None);
}

#[test]
fn later_brace_with_the_same_name_replaces_wholesale() {
    // 要件 1.8: 前のブレスのキーは混ざらない。
    let text = brace(
        "main",
        &["size,200,80", "origin.x,5", "font.follow,balloon"],
    ) + &brace("main", &["size,30,40", "origin.y,6"]);
    let (layout, report) = fold(&text);
    assert_eq!(
        report.issues,
        vec![BoxIssue::BraceReplaced {
            name: "main".into(),
        }]
    );
    assert_eq!(
        layout.def(&name("main")),
        Some(&BoxDef {
            model: parse_str("origin.y,6", None),
            size: (30, 40),
            follow: FontFollow::Scope,
        })
    );

    // 置き換えたブレスが採られなければ、前の定義へは戻さない（設計の Implementation Notes）。
    let text = brace("main", &["size,200,80"]) + &brace("main", &["origin.y,6"]);
    let (layout, report) = fold(&text);
    assert_eq!(
        report.issues,
        vec![
            BoxIssue::BraceReplaced {
                name: "main".into(),
            },
            BoxIssue::BraceMissingSize {
                name: "main".into(),
            },
        ]
    );
    assert_eq!(layout.def(&name("main")), None);
}

#[test]
fn erroneous_braces_never_fail_the_fold_and_only_valid_ones_are_taken() {
    // 要件 10.1・10.2: 誤り 1 件ごとに原因と名前を持つ報告が 1 件・誤りの無いブレスだけ採る。
    let text = [
        brace("ok1", &["size,10,10"]),
        brace("nosize", &["origin.x,1"]),
        brace("42", &["size,10,10"]),
        brace("bad", &["size,x,1"]),
        brace("", &["size,10,10"]),
        brace("ok2", &["size,5,6", "use_self_alpha,1", "font.follow,zzz"]),
        "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n".to_string(),
    ]
    .concat();
    let (layout, report) = fold(&text);
    assert_eq!(
        report.issues,
        vec![
            BoxIssue::BraceMissingSize {
                name: "nosize".into(),
            },
            BoxIssue::BraceNumericName { name: "42".into() },
            BoxIssue::BraceBadSize {
                name: "bad".into(),
                value: "x,1".into(),
            },
            BoxIssue::BraceEmptyName {
                heading: "balloon.".into(),
            },
            BoxIssue::BraceBadFollow {
                name: "ok2".into(),
                value: "zzz".into(),
            },
            BoxIssue::BraceKeyIgnored {
                name: "ok2".into(),
                key: "use_self_alpha".into(),
            },
        ]
    );
    for taken in ["ok1", "ok2"] {
        assert!(layout.def(&name(taken)).is_some(), "{taken}");
    }
    for dropped in ["nosize", "42", "bad", ""] {
        assert_eq!(layout.def(&name(dropped)), None, "{dropped}");
    }
}

#[test]
fn text_without_braces_folds_to_an_empty_layout_and_no_report() {
    // 要件 1.7 の側から: 箱の無い文面では表も報告も空。
    let (layout, report) = fold("surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n");
    assert_eq!(layout, BoxLayout::default());
    assert_eq!(report.issues, vec![]);
}

// ---- 箱の element定義をサーフェス番号ごとに配る（fold_boxes の置き場所の側・要件 2.1〜2.7・
// 3.8・10.1・10.2）。文面はテストの中に持つ（検体は読まない）。

/// 面の画像の対応を渡して畳む（画像の World も同じ文面と同じ対応から組む）。
fn fold_images(text: &str, given: &[(u32, &str)]) -> (BoxLayout, BoxReport, EmoWorld) {
    let images: BTreeMap<u32, String> = given.iter().map(|(id, f)| (*id, f.to_string())).collect();
    let world = EmoWorld::build_with_images(&areka_parsers::shell::parse(text), &images);
    let (layout, report) = fold_boxes(&parse_boxes(text), &images, &world);
    (layout, report, world)
}

/// 見出しと本体の行から `surface*`ブレス（または `surface.append*`ブレス）の文面を作る。
fn block(heading: &str, lines: &[&str]) -> String {
    let mut text = format!("{heading}\n{{\n");
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("}\n");
    text
}

/// 箱 `a`・`b` の定義（大きさの違う 2 つ）。
fn two_braces() -> String {
    [brace("a", &["size,100,50"]), brace("b", &["size,80,40"])].concat()
}

const NONE: &[BoxPlacement] = &[];

#[test]
fn box_text_folds_to_expected_table_in_ascending_element_order() {
    // 要件 2.1・2.3・2.4: 箱を 2 つ持つ面・同じ名前を別の位置に置く面・箱の無い面。
    // 列は element番号の昇順（書いた順ではない）・名前はすべて定義の表で引ける。
    let text = [
        two_braces(),
        block(
            "surface0",
            &[
                "element0,overlay,body.png,0,0",
                "element1,balloon,a,10,20",
                "element2,balloon,b,-5,30",
            ],
        ),
        block(
            "surface1",
            &["element3,balloon,a,1,2", "element2,balloon,b,3,4"],
        ),
        block("surface2", &["element0,overlay,body.png,0,0"]),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(report.issues, vec![]);
    assert!(!layout.is_empty());
    assert_eq!(
        layout.placements(0),
        &[place(1, "a", 10, 20), place(2, "b", -5, 30)]
    );
    assert_eq!(
        layout.placements(1),
        &[place(2, "b", 3, 4), place(3, "a", 1, 2)]
    );
    assert_eq!(layout.placements(2), NONE);
    for id in [0, 1] {
        for p in layout.placements(id) {
            assert!(layout.def(&p.name).is_some(), "{id}: {}", p.name.as_str());
        }
    }
}

#[test]
fn shared_header_gives_every_id_the_same_boxes() {
    // 要件 2.1: 見出しの展開は画像と同じ規則（列挙・範囲・除外）。
    let text = [
        two_braces(),
        block("surface0,2-4,!3", &["element1,balloon,a,7,8"]),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(report.issues, vec![]);
    for id in [0, 2, 4] {
        assert_eq!(layout.placements(id), &[place(1, "a", 7, 8)], "{id}");
    }
    for id in [1, 3] {
        assert_eq!(layout.placements(id), NONE, "{id}");
    }
}

#[test]
fn later_surface_brace_replaces_the_placements_wholesale() {
    // 同じ番号の `surface*`ブレスは置き場所を丸ごと置き換える（画像の畳み込みと同じ後勝ち）。
    let text = [
        two_braces(),
        block("surface0", &["element1,balloon,a,0,0"]),
        block("surface.append0", &["element3,balloon,b,0,0"]),
        block("surface0", &["element2,balloon,b,5,5"]),
        block("surface1", &["element1,balloon,a,0,0"]),
        block("surface1", &["element0,overlay,body.png,0,0"]),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(report.issues, vec![]);
    assert_eq!(layout.placements(0), &[place(2, "b", 5, 5)]);
    assert_eq!(layout.placements(1), NONE);
    assert!(!layout.is_empty());
}

#[test]
fn append_adds_boxes_under_the_same_rules() {
    // 要件 2.2: `surface.append*`ブレスの箱は `surface*`ブレスに書いた場合と同じ規則で置かれる。
    let text = [
        two_braces(),
        block("surface0", &["element3,balloon,a,1,1"]),
        block("surface.append0", &["element1,balloon,b,2,2"]),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(report.issues, vec![]);
    assert_eq!(
        layout.placements(0),
        &[place(1, "b", 2, 2), place(3, "a", 1, 1)]
    );
}

#[test]
fn element_with_an_unreadable_number_is_dropped_and_reported() {
    // 要件 10.1: element番号が読めない element定義だけ捨てる。
    let text = [
        two_braces(),
        block(
            "surface0",
            &["elementX,balloon,a,0,0", "element2,balloon,b,0,0"],
        ),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(
        report.issues,
        vec![BoxIssue::ElementBadNumber {
            surface: 0,
            element: "X".into(),
        }]
    );
    assert_eq!(layout.placements(0), &[place(2, "b", 0, 0)]);
}

#[test]
fn element_naming_a_missing_or_untaken_brace_is_dropped_and_reported() {
    // 要件 2.5: 名前のブレスが無い・採られなかった（`size` 無し）element定義だけ捨て、他は採る。
    let text = [
        brace("a", &["size,100,50"]),
        brace("nosize", &["origin.x,1"]),
        block(
            "surface0",
            &[
                "element1,balloon,ghost,0,0",
                "element2,balloon,nosize,0,0",
                "element3,balloon,a,4,4",
            ],
        ),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(
        report.issues,
        vec![
            BoxIssue::BraceMissingSize {
                name: "nosize".into(),
            },
            BoxIssue::ElementUnknownBrace {
                surface: 0,
                element: 1,
                name: "ghost".into(),
            },
            BoxIssue::ElementUnknownBrace {
                surface: 0,
                element: 2,
                name: "nosize".into(),
            },
        ]
    );
    assert_eq!(layout.placements(0), &[place(3, "a", 4, 4)]);
}

#[test]
fn element_with_a_non_integer_position_is_dropped_and_reported() {
    // 要件 2.6: X か Y が整数として読めない（欠けを含む）element定義だけ捨てる。
    let text = [
        two_braces(),
        block(
            "surface0",
            &[
                "element1,balloon,a,1.5,0",
                "element2,balloon,b,3",
                "element3,balloon,a,-1,+2",
            ],
        ),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(
        report.issues,
        vec![
            BoxIssue::ElementBadPosition {
                surface: 0,
                element: 1,
                name: "a".into(),
                x: "1.5".into(),
                y: "0".into(),
            },
            BoxIssue::ElementBadPosition {
                surface: 0,
                element: 2,
                name: "b".into(),
                x: "3".into(),
                y: "".into(),
            },
        ]
    );
    assert_eq!(layout.placements(0), &[place(3, "a", -1, 2)]);
}

#[test]
fn same_name_keeps_the_smallest_element_number_across_surface_and_append() {
    // 要件 2.7: `surface*` と `surface.append*` の両方が同じ名前を置いても element番号最小を採る。
    let text = [
        two_braces(),
        block(
            "surface0",
            &["element3,balloon,a,0,0", "element4,balloon,b,0,0"],
        ),
        block(
            "surface.append0",
            &["element1,balloon,a,9,9", "element5,balloon,a,1,1"],
        ),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(
        report.issues,
        vec![
            BoxIssue::ElementDuplicateName {
                surface: 0,
                element: 3,
                name: "a".into(),
                kept: 1,
            },
            BoxIssue::ElementDuplicateName {
                surface: 0,
                element: 5,
                name: "a".into(),
                kept: 1,
            },
        ]
    );
    assert_eq!(
        layout.placements(0),
        &[place(1, "a", 9, 9), place(4, "b", 0, 0)]
    );
}

#[test]
fn box_below_an_image_element_is_kept_and_reported() {
    // 要件 3.8: 画像の element の最大の番号より小さい番号の箱は採ったうえで断る。
    let text = [
        two_braces(),
        block(
            "surface0",
            &[
                "element1,balloon,a,0,0",
                "element2,overlay,arm.png,0,0",
                "element3,balloon,b,0,0",
            ],
        ),
    ]
    .concat();
    let (layout, report, _) = fold_images(&text, &[]);
    assert_eq!(
        report.issues,
        vec![BoxIssue::ElementBelowImage {
            surface: 0,
            element: 1,
            name: "a".into(),
            image_element: 2,
        }]
    );
    assert_eq!(
        layout.placements(0),
        &[place(1, "a", 0, 0), place(3, "b", 0, 0)]
    );
}

/// 追記の `deco.png`（画像）と箱 `a` が届いた番号を、画像の World と箱の表のそれぞれから引く。
fn reached(layout: &BoxLayout, world: &EmoWorld) -> (Vec<u32>, Vec<u32>) {
    let images = world
        .surface_ids()
        .filter(|id| {
            world
                .surface(*id)
                .is_some_and(|m| m.elements.iter().any(|e| e.path.as_str() == "deco.png"))
        })
        .collect();
    let boxes = (0..=20)
        .filter(|id| layout.placements(*id).iter().any(|p| p.name == name("a")))
        .collect();
    (images, boxes)
}

/// 画像と箱を 1 つずつ持つ追記の本体（画像の畳み込みと箱の畳み込みを同じ行で突き合わせる）。
const APPEND_BODY: &[&str] = &["element1,overlay,deco.png,0,0", "element2,balloon,a,0,0"];

#[test]
fn append_existence_matches_image_fold_for_forward_reference() {
    // 先に書かれた追記は後で定義される番号へ遡及しない（fold_tests の前方参照と同じ形）。
    let text = [
        two_braces(),
        block("surface.append3", APPEND_BODY),
        block("surface3", &["element0,overlay,base.png,0,0"]),
    ]
    .concat();
    let (layout, report, world) = fold_images(&text, &[]);
    assert_eq!(reached(&layout, &world), (vec![], vec![]));
    assert_eq!(
        report.issues,
        vec![BoxIssue::AppendTargetMissing {
            surface: 3,
            name: "a".into(),
        }]
    );
}

#[test]
fn append_existence_matches_image_fold_for_image_only_faces() {
    // 面の画像だけで在る番号にも追記は届く（base_image_tests の画像だけの面と同じ形）。先の追記が
    // 届いた面へ後の追記も届く。宣言も画像も無い番号へは届かず報告に載る。後の `surface*`ブレスは
    // 画像だけの面も丸ごと置き換える。
    let text = [
        two_braces(),
        block("surface.append7,8,9", APPEND_BODY),
        block("surface.append7", &["element4,balloon,b,0,0"]),
        block("surface8", &["element0,overlay,base.png,0,0"]),
    ]
    .concat();
    let (layout, report, world) = fold_images(&text, &[(7, "surface7.png"), (8, "surface8.png")]);
    assert_eq!(reached(&layout, &world), (vec![7], vec![7]));
    assert_eq!(
        layout.placements(7),
        &[place(2, "a", 0, 0), place(4, "b", 0, 0)]
    );
    assert_eq!(
        report.issues,
        vec![BoxIssue::AppendTargetMissing {
            surface: 9,
            name: "a".into(),
        }]
    );
}

#[test]
fn append_existence_matches_image_fold_for_range_and_exclusion() {
    // 範囲と除外（fold_tests の `!N` の減算と同じ形）: 1・2・4 に届き、3 は除外、5 は無い。
    let text = [
        two_braces(),
        block("surface1-4", &["element0,overlay,base.png,0,0"]),
        block("surface.append1-5,!3", APPEND_BODY),
    ]
    .concat();
    let (layout, report, world) = fold_images(&text, &[]);
    assert_eq!(reached(&layout, &world), (vec![1, 2, 4], vec![1, 2, 4]));
    assert_eq!(
        report.issues,
        vec![BoxIssue::AppendTargetMissing {
            surface: 5,
            name: "a".into(),
        }]
    );
}
