//! 計画の組み立て（[`build`]）の決定論テスト（要件 9.1）。
//!
//! 確かめること: 7 枠の並び・未登記の枠が出ないこと・区切り線の位置・サブメニューの子の順と
//! チェック位置・無効の写し・識別子の一意性と逆引き・アクセラレータ記法 `&` の 2 系統。
//!
//! 期待値はすべて書き下した値で、[`Frame::ORDER`] や [`build`] 自身から導かない
//! （群分けや識別子の払い出しを変えたら赤になるように。[`build`] は入力が既に枠の並び順であることを
//! 前提にするので、[`Frame::ORDER`] そのものの入れ替えを見張るのは登記側のテストである）。

use std::rc::Rc;

use bevy_ecs::prelude::*;

use super::*;
use crate::menu::MenuContext;

/// 選ばれた項目が自分の名札を残す場所。逆引きの取り違えはここに出る順序で見分ける。
#[derive(Resource, Default)]
struct Trace(Vec<&'static str>);

/// 動作を持つ葉。選ばれると `tag` を [`Trace`] へ積む。
fn leaf(label: &str, tag: &'static str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        caption_resource: None,
        enabled: true,
        checked: None,
        body: ItemBody::Action(Rc::new(move |world: &mut World, _| {
            world.resource_mut::<Trace>().0.push(tag);
        })),
    }
}

/// 子項目を持つ見出し。
fn submenu(label: &str, children: Vec<MenuItem>) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        caption_resource: None,
        enabled: true,
        checked: None,
        body: ItemBody::Submenu(children),
    }
}

/// 期待値側の「有効でチェックの無い項目」。
fn plain(id: u32, label: &str) -> PlanEntry {
    PlanEntry::Item {
        id,
        label: label.to_string(),
        enabled: true,
        checked: false,
    }
}

/// 文言だけを 1 件持つ表。
fn captions_with(id: &'static str, caption: &str) -> CaptionMap {
    let mut map = CaptionMap::default();
    map.insert(id, caption.to_string());
    map
}

/// 動作を 1 回行って、残った名札を返す。
fn run(plan: &MenuPlan, id: u32) -> (Frame, Vec<&'static str>) {
    let (frame, action) = plan.action(id).expect("選べる項目には動作がある");
    let mut world = World::new();
    world.init_resource::<Trace>();
    action(&mut world, &MenuContext { scope: 0 });
    let trace = world.resource::<Trace>().0.clone();
    (frame, trace)
}

/// 7 枠すべてが登記されているとき、並びは ①〜⑦で、群 {①②③}{④⑤}{⑥}{⑦} の間に
/// 区切り線が 1 本ずつ入る（要件 2.1・2.6）。先頭と末尾には入らない。
#[test]
fn all_seven_frames_follow_the_declared_order_with_three_separators() {
    let snapshot = vec![
        (Frame::Ghost, leaf("ゴースト", "ghost")),
        (Frame::Shell, leaf("シェル", "shell")),
        (Frame::Balloon, leaf("バルーン", "balloon")),
        (Frame::Update, leaf("ネットワーク更新", "update")),
        (Frame::Install, leaf("インストール…", "install")),
        (Frame::Readme, leaf("説明書", "readme")),
        (Frame::Close, leaf("終了", "close")),
    ];

    let plan = build(snapshot, &CaptionMap::default());

    assert_eq!(
        plan.entries,
        vec![
            plain(1, "ゴースト"),
            plain(2, "シェル"),
            plain(3, "バルーン"),
            PlanEntry::Separator,
            plain(4, "ネットワーク更新"),
            plain(5, "インストール…"),
            PlanEntry::Separator,
            plain(6, "説明書"),
            PlanEntry::Separator,
            plain(7, "終了"),
        ]
    );
    assert_eq!(plan.item_count(), 7);
}

/// 本仕様が登記する 2 枠だけのとき（第 1 スライスの既定の姿）: 説明書・区切り・終了。
#[test]
fn readme_and_close_only_are_separated_by_one_line() {
    let snapshot = vec![
        (Frame::Readme, leaf("説明書", "readme")),
        (Frame::Close, leaf("終了", "close")),
    ];

    let plan = build(snapshot, &CaptionMap::default());

    assert_eq!(
        plan.entries,
        vec![plain(1, "説明書"), PlanEntry::Separator, plain(2, "終了")]
    );
}

/// 間の群が空でも区切り線は重ならない（要件 2.3 の「出さない」が区切りに漏れない）。
/// ①と⑦の間は群が 3 つ飛ぶが、入る区切りは 1 本。
#[test]
fn empty_groups_in_between_do_not_double_the_separator() {
    let snapshot = vec![
        (Frame::Ghost, leaf("ゴースト", "ghost")),
        (Frame::Close, leaf("終了", "close")),
    ];

    let plan = build(snapshot, &CaptionMap::default());

    assert_eq!(
        plan.entries,
        vec![plain(1, "ゴースト"), PlanEntry::Separator, plain(2, "終了")]
    );
}

/// 同じ群の中には区切り線を入れない。登記の無い枠（ここでは②）は現れない（要件 2.3）。
#[test]
fn frames_in_the_same_group_have_no_separator_and_unregistered_frames_are_absent() {
    let snapshot = vec![
        (Frame::Ghost, leaf("ゴースト", "ghost")),
        (Frame::Balloon, leaf("バルーン", "balloon")),
    ];

    let plan = build(snapshot, &CaptionMap::default());

    assert_eq!(
        plan.entries,
        vec![plain(1, "ゴースト"), plain(2, "バルーン")]
    );
}

/// 何も登記されていなければ計画は空（区切り線だけが残ることは無い）。
#[test]
fn empty_snapshot_makes_an_empty_plan() {
    let plan = build(Vec::new(), &CaptionMap::default());

    assert_eq!(plan.entries, vec![]);
    assert_eq!(plan.item_count(), 0);
}

/// サブメニューは見出し＋子項目の列になり、子は登記された順に並び、チェックは
/// 「現在」と示された子だけに付く（要件 2.4）。見出しは識別子を持たない。
#[test]
fn submenu_keeps_child_order_and_marks_only_the_checked_child() {
    let mut current = leaf("さくら", "sakura");
    current.checked = Some(true);
    let mut other = leaf("うにゅう", "unyu");
    other.checked = Some(false);
    let snapshot = vec![(
        Frame::Ghost,
        submenu("ゴースト", vec![leaf("先", "first"), current, other]),
    )];

    let plan = build(snapshot, &CaptionMap::default());

    assert_eq!(
        plan.entries,
        vec![PlanEntry::Submenu {
            label: "ゴースト".to_string(),
            enabled: true,
            children: vec![
                plain(1, "先"),
                PlanEntry::Item {
                    id: 2,
                    label: "さくら".to_string(),
                    enabled: true,
                    checked: true,
                },
                plain(3, "うにゅう"),
            ],
        }]
    );
    assert_eq!(plan.item_count(), 3, "見出しは選べる項目に数えない");
}

/// 「無効」は葉にもサブメニューの見出しにも子項目にもそのまま写る（要件 2.5）。
#[test]
fn disabled_is_copied_for_leaf_header_and_child() {
    let mut readme = leaf("説明書", "readme");
    readme.enabled = false;
    let mut child = leaf("配布元", "origin");
    child.enabled = false;
    let mut header = submenu("ゴースト", vec![child]);
    header.enabled = false;
    let snapshot = vec![(Frame::Ghost, header), (Frame::Readme, readme)];

    let plan = build(snapshot, &CaptionMap::default());

    assert_eq!(
        plan.entries,
        vec![
            PlanEntry::Submenu {
                label: "ゴースト".to_string(),
                enabled: false,
                children: vec![PlanEntry::Item {
                    id: 1,
                    label: "配布元".to_string(),
                    enabled: false,
                    checked: false,
                }],
            },
            PlanEntry::Separator,
            PlanEntry::Item {
                id: 2,
                label: "説明書".to_string(),
                enabled: false,
                checked: false,
            },
        ]
    );
}

/// 識別子は 1 から深さ優先の出現順に払い出され、重複しない。逆引きは項目ごとに違う動作を
/// 返す（同じ動作を配り回していないことを、World に残る名札で見分ける・要件 2.7）。
#[test]
fn ids_are_unique_depth_first_and_map_back_to_each_own_action() {
    let snapshot = vec![
        (
            Frame::Ghost,
            submenu(
                "ゴースト",
                vec![leaf("さくら", "sakura"), leaf("うにゅう", "unyu")],
            ),
        ),
        (Frame::Shell, leaf("シェル", "shell")),
        (Frame::Close, leaf("終了", "close")),
    ];

    let plan = build(snapshot, &CaptionMap::default());

    assert_eq!(plan.item_count(), 4);
    assert_eq!(run(&plan, 1), (Frame::Ghost, vec!["sakura"]));
    assert_eq!(run(&plan, 2), (Frame::Ghost, vec!["unyu"]));
    assert_eq!(run(&plan, 3), (Frame::Shell, vec!["shell"]));
    assert_eq!(run(&plan, 4), (Frame::Close, vec!["close"]));
    assert!(plan.action(0).is_none(), "0 は識別子として払い出さない");
    assert!(plan.action(5).is_none(), "払い出していない識別子は引けない");
    assert!(plan.action(u32::MAX).is_none());
}

/// リソース由来の文言は素通し（`&` はアクセラレータ記法のまま）、登記者由来の既定名は
/// `&` を `&&` へ写す（要件 3.5）。表に無い／空のリソース名は既定名へ落ちる。
#[test]
fn ampersand_passes_through_from_resource_but_is_escaped_in_registered_labels() {
    let mut from_resource = leaf("説明書", "readme");
    from_resource.caption_resource = Some("readmebutton.caption");
    let mut missing_resource = leaf("R&D", "rd");
    missing_resource.caption_resource = Some("updatebutton.caption");
    let snapshot = vec![
        (Frame::Ghost, leaf("R&D", "rd")),
        (Frame::Update, missing_resource),
        (Frame::Readme, from_resource),
    ];

    let plan = build(
        snapshot,
        &captions_with("readmebutton.caption", "取扱説明書(&R)"),
    );

    assert_eq!(
        plan.entries,
        vec![
            plain(1, "R&&D"),
            PlanEntry::Separator,
            plain(2, "R&&D"),
            PlanEntry::Separator,
            plain(3, "取扱説明書(&R)"),
        ]
    );
}

/// 文言の決め方は子項目にも同じように効く。
#[test]
fn caption_rule_applies_to_children_too() {
    let mut named = leaf("既定", "named");
    named.caption_resource = Some("ghostrootbutton.caption");
    let snapshot = vec![(
        Frame::Ghost,
        submenu("ゴースト", vec![named, leaf("A&B", "ab")]),
    )];

    let plan = build(
        snapshot,
        &captions_with("ghostrootbutton.caption", "現在の私(&G)"),
    );

    assert_eq!(
        plan.entries,
        vec![PlanEntry::Submenu {
            label: "ゴースト".to_string(),
            enabled: true,
            children: vec![plain(1, "現在の私(&G)"), plain(2, "A&&B")],
        }]
    );
}

/// 空の文言は表に入らないので既定名に落ちる（照会側の写しと同じ扱い）。
#[test]
fn empty_caption_is_not_stored_and_falls_back_to_the_registered_label() {
    let mut item = leaf("説明書", "readme");
    item.caption_resource = Some("readmebutton.caption");

    let plan = build(
        vec![(Frame::Readme, item)],
        &captions_with("readmebutton.caption", ""),
    );

    assert_eq!(plan.entries, vec![plain(1, "説明書")]);
}

/// 同じ入力からは同じ計画ができる（決定論）。
#[test]
fn build_is_deterministic() {
    let make = || {
        vec![
            (
                Frame::Ghost,
                submenu("ゴースト", vec![leaf("さくら", "sakura")]),
            ),
            (Frame::Readme, leaf("説明書", "readme")),
            (Frame::Close, leaf("終了", "close")),
        ]
    };

    let first = build(make(), &CaptionMap::default());
    let second = build(make(), &CaptionMap::default());

    assert_eq!(first.entries, second.entries);
    assert_eq!(first.item_count(), second.item_count());
}

/// `&` の写しは単体でも確かめる（連続した `&` も 1 つずつ倍にする）。
#[test]
fn escape_ampersand_doubles_every_ampersand() {
    assert_eq!(escape_ampersand("ネットワーク更新"), "ネットワーク更新");
    assert_eq!(escape_ampersand("R&D"), "R&&D");
    assert_eq!(escape_ampersand("A&B&C"), "A&&B&&C");
    assert_eq!(escape_ampersand("&&"), "&&&&");
    assert_eq!(escape_ampersand(""), "");
}
