//! subset 外・不正入力の振り分け（元 decode_tests.rs タスク 4.6/1.2 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

use super::{
    Animation, Collision, CollisionName, DrawMethod, Element, ElementPath, Interval, Pattern,
    decode, lex,
};

// --- タスク 4.6/1.2: subset 外・不正入力の振り分け（吸収 or 忠実転記・既定値・非パニック） ---
//
// 検証範囲（要件 2.3/4.5/4.6/6.3/8.2/9.2/10.4）:
// - element の overlay・base 以外のメソッド・collisionex は値化せず passthrough で吸収する
//   （モデルに materialize しない・要件 4.5/6.3/10.4）。この 2 経路のフィルタは task 1.2 でも不変。
// - pattern メソッド（overlay/replace 等）・未認識 interval キーワード（sometimes 等）は
//   task 1.2 で overlay フィルタ／fallback-Bind を撤去し、吸収でなく忠実転記へ転じた
//   （method＝field[1] verbatim・未認識 interval＝`Interval::Other(原文)`・要件 4.6/8.2）。
// - 非数トークン・欠損フィールドを既定値（0）へ倒し、パニックしない（要件 3.3/2.3）。
// - subset 外を含む断片が、隣接する認識可能ブロックのパースを壊さない（要件 9.2/10.4 の核心）。

/// element定義の描画メソッド `base`（`element0,base,...`）は `overlay` と同じ値になり、
/// 同じ surface の overlay element と並ぶ（areka-P0-element-base-method 要件 1.1・1.2）。
#[test]
fn base_element_method_becomes_value_beside_overlay_sibling() {
    let input = "\
surface0
{
element0,base,base.png,0,0
element1,overlay,over.png,7,8
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    // base の element も overlay と同じ値になり、レイヤ昇順で土台が先に並ぶ。
    assert_eq!(
        shell.surfaces[0].elements,
        vec![
            Element {
                layer: 0,
                path: ElementPath::new("base.png".to_string()),
                x: 0,
                y: 0,
            },
            Element {
                layer: 1,
                path: ElementPath::new("over.png".to_string()),
                x: 7,
                y: 8,
            },
        ]
    );
}

/// pattern を伴わない未知 interval のみの行も忠実転記される（fallback-Bind 撤去後は
/// interval 行が当該 ID の slot を確定させる・要件 8.2）。`interval,yen-e` は
/// `Interval::Other("yen-e")` の animation を初出順で 1 個積む（pattern は空）。
/// 認識可能 interval 単独行が既に slot を作る挙動と対称（語彙を落とさない黙らない）。
/// （検体の語は元 `periodic`。areka-P0-seriko-trigger-intervals で `periodic,数値` が
/// 読める語になったので、未認識のままの語へ替えた。）
#[test]
fn unknown_interval_only_line_is_transcribed_as_other() {
    let input = "surface0\n{\nanimation0.interval,yen-e,3\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    // yen-e は Interval::Other へ転記され、pattern を持たない animation slot が立つ。
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 0);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::Other("yen-e".into())
    );
    assert!(shell.surfaces[0].animations[0].patterns.is_empty());
}

/// `collisionex` 行（円/楕円/多角形）は materialize されない一方、同一ブロック内の
/// 純 `collisionN` 矩形は保持される（要件 6.3・passthrough 吸収）。
#[test]
fn collisionex_is_absorbed_but_plain_collision_survives() {
    let input = "\
surface1000
{
collisionex0,ellipse,10,20,30,40,Face
collision1,80,140,260,300,Bust
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    // collisionex は値化されず、純 collision1 矩形だけが残る。
    assert_eq!(
        shell.surfaces[0].collisions,
        vec![Collision {
            index: 1,
            left: 80,
            top: 140,
            right: 260,
            bottom: 300,
            name: CollisionName::new("Bust".to_string()),
        }]
    );
}

/// 非数トークンを含む collision 行 → 該当フィールドは既定 0 へ倒れ、パニックしない
/// （要件 2.3/3.3）。領域名は非数でも opaque 保持される。
#[test]
fn collision_with_non_numeric_fields_defaults_to_zero() {
    let input = "surface1000\n{\ncollision0,x,y,z,w,Head\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0].collisions,
        vec![Collision {
            index: 0,
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
            name: CollisionName::new("Head".to_string()),
        }]
    );
}

/// フィールド欠損（短い行）→ 欠損フィールドは既定 0・欠損文字列は空へ倒れ、パニックしない
/// （要件 2.3/3.3・9.2）。`collision0` のみで後続座標・名前が無い極端に短い行。
#[test]
fn short_collision_line_defaults_missing_fields() {
    let input = "surface1000\n{\ncollision0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0].collisions,
        vec![Collision {
            index: 0,
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
            name: CollisionName::new(String::new()),
        }]
    );
}

/// 複合断片: 認識可能ブロックの「間」に subset 外ブロック/行を挟んでも、両側の
/// 認識可能ブロックが完全に decode される（要件 9.2/10.4 の核心）。
/// 未知トップレベル行・未知ブロック・不正断片で挟んでも走査は中断しない。
#[test]
fn subset_out_fragment_between_recognizable_blocks_does_not_break_neighbors() {
    let input = "\
surface2100
{
element0,overlay,CityPop.png,0,0
}
unknown.block.head
{
element0,base,junk.png,9,9
collisionex0,polygon,1,2,3
garbage line with no meaning
}
surface2200
{
collision0,52,38,156,80,Head
animation0.interval,bind
animation0.pattern0,overlay,2206,0,0,0
}
";
    let shell = decode(lex(input));
    // 両側の認識可能 surface が完全に decode される（間の subset 外は吸収）。
    assert_eq!(shell.surfaces.len(), 2);

    // 前方ブロック: surface2100 の overlay element が完全に残る。
    assert_eq!(shell.surfaces[0].id, 2100);
    assert_eq!(
        shell.surfaces[0].elements,
        vec![Element {
            layer: 0,
            path: ElementPath::new("CityPop.png".to_string()),
            x: 0,
            y: 0,
        }]
    );

    // 後方ブロック: surface2200 の collision と animation が完全に残る。
    assert_eq!(shell.surfaces[1].id, 2200);
    assert_eq!(
        shell.surfaces[1].collisions,
        vec![Collision {
            index: 0,
            left: 52,
            top: 38,
            right: 156,
            bottom: 80,
            name: CollisionName::new("Head".to_string()),
        }]
    );
    assert_eq!(
        shell.surfaces[1].animations,
        vec![Animation {
            id: 0,
            interval: Interval::Bind,
            interval_written: true,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 2206,
                target: "2206".into(),
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
}

/// 単一 surface 内で吸収対象（element replace・collisionex）と忠実転記対象（pattern method・
/// interval）を混在させ、各々が正しく振り分けられ互いを壊さないことを確認する
/// （要件 4.5/4.6/6.3/8.2/9.2）。overlay element・純 collision は残り、replace element・
/// collisionex は吸収される一方、replace/overlay 両 pattern と `always` interval は忠実転記される
/// （areka-P0-element-base-method 要件 2.4: 描けない語の行だけが吸収され隣の行は残る）。
#[test]
fn mixed_valid_and_subset_out_lines_in_one_surface() {
    let input = "\
surface3000
{
element0,replace,skip.png,1,1
element1,overlay,keep.png,2,3
element2,base,also.png,4,5
collisionex0,circle,5,5,10
collision0,10,20,30,40,Head
animation0.interval,always,9
animation0.pattern0,replace,900,0,0,0
animation0.pattern1,overlay,901,4,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    let s = &shell.surfaces[0];
    // overlay と base の element が残る（replace 吸収）。
    assert_eq!(
        s.elements,
        vec![
            Element {
                layer: 1,
                path: ElementPath::new("keep.png".to_string()),
                x: 2,
                y: 3,
            },
            Element {
                layer: 2,
                path: ElementPath::new("also.png".to_string()),
                x: 4,
                y: 5,
            },
        ]
    );
    // 純 collision のみ残る（collisionex 吸収）。
    assert_eq!(
        s.collisions,
        vec![Collision {
            index: 0,
            left: 10,
            top: 20,
            right: 30,
            bottom: 40,
            name: CollisionName::new("Head".to_string()),
        }]
    );
    // always interval は Interval::Other へ転記、replace/overlay 双方の pattern が method を
    // 忠実転記して出現順で残る（overlay フィルタ・fallback-Bind ともに撤去済み）。
    assert_eq!(
        s.animations,
        vec![Animation {
            id: 0,
            interval: Interval::Other("always".into()),
            interval_written: true,
            patterns: vec![
                Pattern {
                    index: 0,
                    method: DrawMethod::new("replace".to_string()),
                    surface_id: 900,
                    target: "900".into(),
                    wait: 0,
                    x: 0,
                    y: 0,
                },
                Pattern {
                    index: 1,
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: 901,
                    target: "901".into(),
                    wait: 4,
                    x: 0,
                    y: 0,
                },
            ],
        }]
    );
}

/// 読めない数は 0 にせず飛ばして `UnreadableNumber` を積む（指摘 36）。範囲の両端は trim し、
/// 見出しがちょうど `surface.alias` のブレスは読まない（崩れも積まない）。
#[test]
fn unreadable_numbers_are_skipped_not_zero() {
    use crate::shell::{AppendTarget, ShellIssue, parse_all};
    let input = "\
surface0,x, 3 - 5
{
elementx,overlay,a.png,0,0
element1,overlay,b.png,0,0
collision,1,2,3,4,Bad
collision99999999999,1,2,3,4,Big
collision2,1,2,3,4,Head
animationx.interval,bind
animation1.interval,bind
animation1.patternq,overlay,10,5,0,0
animation1.pattern0,overlay,11,5,0,0
}
kero.surface.alias
{
6,[1,a,3]
}
surface.alias
{
0,[1]
}
";
    let text = parse_all(input);
    let shell = &text.shell;
    assert_eq!(shell.surfaces.len(), 1);
    let surface = &shell.surfaces[0];
    assert_eq!(
        surface.targets,
        vec![
            AppendTarget::Single(0),
            AppendTarget::Range { start: 3, end: 5 }
        ]
    );
    assert_eq!(
        surface.elements,
        vec![Element {
            layer: 1,
            path: ElementPath::new("b.png".to_string()),
            x: 0,
            y: 0,
        }]
    );
    assert_eq!(
        surface
            .collisions
            .iter()
            .map(|c| c.index)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(surface.animations.len(), 1);
    assert_eq!(surface.animations[0].id, 1);
    assert_eq!(
        surface.animations[0]
            .patterns
            .iter()
            .map(|p| p.index)
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(shell.aliases[0].ids, vec![1, 3]);
    let unreadable = |heading: &str, text: &str| ShellIssue::UnreadableNumber {
        heading: heading.into(),
        text: text.into(),
    };
    let head = "surface0,x,3 - 5";
    assert_eq!(
        text.issues,
        vec![
            unreadable(head, "x"),
            unreadable(head, "elementx"),
            unreadable(head, "collision"),
            unreadable(head, "collision99999999999"),
            unreadable(head, "animationx.interval"),
            unreadable(head, "animation1.patternq"),
            unreadable("kero.surface.alias", "a"),
        ]
    );
}
