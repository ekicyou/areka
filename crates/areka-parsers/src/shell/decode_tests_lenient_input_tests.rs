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
// - element の overlay 以外のメソッド・collisionex は値化せず passthrough で吸収する
//   （モデルに materialize しない・要件 4.5/6.3/10.4）。この 2 経路のフィルタは task 1.2 でも不変。
// - pattern メソッド（overlay/replace 等）・未認識 interval キーワード（sometimes 等）は
//   task 1.2 で overlay フィルタ／fallback-Bind を撤去し、吸収でなく忠実転記へ転じた
//   （method＝field[1] verbatim・未認識 interval＝`Interval::Other(原文)`・要件 4.6/8.2）。
// - 非数トークン・欠損フィールドを既定値（0）へ倒し、パニックしない（要件 3.3/2.3）。
// - subset 外を含む断片が、隣接する認識可能ブロックのパースを壊さない（要件 9.2/10.4 の核心）。

/// 非 overlay element メソッド（`element0,base,...`）は materialize されない一方、
/// 同一 surface 内の overlay element は保持される（要件 4.5・passthrough 吸収）。
#[test]
fn non_overlay_element_method_is_absorbed_but_overlay_sibling_survives() {
    let input = "\
surface0
{
element0,base,base.png,0,0
element1,overlay,over.png,7,8
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    // base メソッド element は値化されず、overlay element だけが残る。
    assert_eq!(
        shell.surfaces[0].elements,
        vec![Element {
            layer: 1,
            path: ElementPath::new("over.png".to_string()),
            x: 7,
            y: 8,
        }]
    );
}

/// pattern メソッドは overlay も 非 overlay（`replace`）も落とさず method を忠実転記する
/// （overlay フィルタ撤去・要件 4.6/8.4）。従来は replace 行を黙殺していた＝転記の穴の是正。
#[test]
fn both_overlay_and_non_overlay_pattern_methods_are_transcribed() {
    let input = "\
surface0
{
animation0.interval,bind
animation0.pattern0,replace,100,0,0,0
animation0.pattern1,overlay,101,5,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    // replace・overlay 双方が method を忠実転記して出現順で残る（どちらも落とさない）。
    assert_eq!(
        shell.surfaces[0].animations,
        vec![Animation {
            id: 0,
            interval: Interval::Bind,
            patterns: vec![
                Pattern {
                    index: 0,
                    method: DrawMethod::new("replace".to_string()),
                    surface_id: 100,
                    wait: 0,
                    x: 0,
                    y: 0,
                },
                Pattern {
                    index: 1,
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: 101,
                    wait: 5,
                    x: 0,
                    y: 0,
                },
            ],
        }]
    );
}

/// 3 種以外の interval（`interval,sometimes,5`）は正規化されず `Interval::Other("sometimes")`
/// へ忠実転記される（fallback-Bind 撤去・要件 8.2・討議 #1）。K は非採録（keyword のみ転記）。
/// 併せて隣接する正当な animation（random,4）が正しく decode されることを確認する。
#[test]
fn unrecognized_interval_becomes_other_and_neighbor_survives() {
    let input = "\
surface0
{
animation0.interval,sometimes,5
animation0.pattern0,overlay,10,0,0,0
animation1.interval,random,4
animation1.pattern0,overlay,20,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].animations.len(), 2);
    // sometimes は認識不能ゆえ Interval::Other へ忠実転記され、pattern は失われない。
    assert_eq!(shell.surfaces[0].animations[0].id, 0);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::Other("sometimes".into())
    );
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 1);
    // 隣接する正当な animation は影響を受けず random,4 として decode される。
    assert_eq!(shell.surfaces[0].animations[1].id, 1);
    assert_eq!(
        shell.surfaces[0].animations[1].interval,
        Interval::Random { k: 4 }
    );
}

/// pattern を伴わない未知 interval のみの行も忠実転記される（fallback-Bind 撤去後は
/// interval 行が当該 ID の slot を確定させる・要件 8.2）。`interval,periodic` は
/// `Interval::Other("periodic")` の animation を初出順で 1 個積む（pattern は空）。
/// 認識可能 interval 単独行が既に slot を作る挙動と対称（語彙を落とさない黙らない）。
#[test]
fn unknown_interval_only_line_is_transcribed_as_other() {
    let input = "surface0\n{\nanimation0.interval,periodic,3\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    // periodic は Interval::Other へ転記され、pattern を持たない animation slot が立つ。
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 0);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::Other("periodic".into())
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
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 2206,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
}

/// 単一 surface 内で吸収対象（element base・collisionex）と忠実転記対象（pattern method・
/// interval）を混在させ、各々が正しく振り分けられ互いを壊さないことを確認する
/// （要件 4.5/4.6/6.3/8.2/9.2）。overlay element・純 collision は残り、base element・
/// collisionex は吸収される一方、replace/overlay 両 pattern と `always` interval は忠実転記される。
#[test]
fn mixed_valid_and_subset_out_lines_in_one_surface() {
    let input = "\
surface3000
{
element0,base,skip.png,1,1
element1,overlay,keep.png,2,3
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
    // overlay element のみ残る（base 吸収）。
    assert_eq!(
        s.elements,
        vec![Element {
            layer: 1,
            path: ElementPath::new("keep.png".to_string()),
            x: 2,
            y: 3,
        }]
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
            patterns: vec![
                Pattern {
                    index: 0,
                    method: DrawMethod::new("replace".to_string()),
                    surface_id: 900,
                    wait: 0,
                    x: 0,
                    y: 0,
                },
                Pattern {
                    index: 1,
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: 901,
                    wait: 4,
                    x: 0,
                    y: 0,
                },
            ],
        }]
    );
}
