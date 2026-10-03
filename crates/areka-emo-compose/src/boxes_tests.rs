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
