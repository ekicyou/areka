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
