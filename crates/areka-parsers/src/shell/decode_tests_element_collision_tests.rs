//! element overlay ＋ collision 矩形の正規化（元 decode_tests.rs タスク 4.2 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

use super::{
    Animation, Collision, CollisionName, DrawMethod, Element, ElementPath, Interval, Pattern,
    decode, lex,
};

// --- タスク 4.2: element overlay ＋ collision 矩形の正規化 ---
//
// 検証範囲（要件 4.2/4.3/4.4/6.1/6.2）:
// - element overlay 行を レイヤ/メソッド/画像パス/座標として正規化する（要件 4.2）。
// - 画像パスは無加工保持（区切り `/`・`\` を正規化しない・読込/検証しない・要件 4.3）。
// - element はレイヤインデックス昇順で保持する（安定・要件 4.4）。
// - collision 矩形を 始点X/始点Y/終点X/終点Y ＝ left/top/right/bottom ＋不透明領域名として
//   正規化する（要件 6.1/6.2）。collision は出現順で保持する。
// animation 行・非 overlay メソッド・collisionex はここでは扱わない（タスク 4.3 / 4.6）。

/// 単一 element overlay → レイヤ/パス/座標に正規化（要件 4.2）。
#[test]
fn single_element_overlay_is_normalized() {
    let input = "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0].elements,
        vec![Element {
            layer: 0,
            path: ElementPath::new("surface0.png".to_string()),
            x: 0,
            y: 0,
        }]
    );
}

/// 複数 element は出現順に依らずレイヤ昇順で保持する（要件 4.4）。
/// 入力は element1 が element0 より先に現れるが、出力は element0（layer 0）が先頭。
#[test]
fn multiple_elements_are_sorted_by_layer_ascending() {
    let input = "surface0\n{\nelement1,overlay,b.png,0,0\nelement0,overlay,a.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0].elements,
        vec![
            Element {
                layer: 0,
                path: ElementPath::new("a.png".to_string()),
                x: 0,
                y: 0,
            },
            Element {
                layer: 1,
                path: ElementPath::new("b.png".to_string()),
                x: 0,
                y: 0,
            },
        ]
    );
}

/// 画像パスはバックスラッシュ区切りを無加工で保持する（正規化しない・要件 4.3）。
#[test]
fn element_path_with_backslash_is_verbatim() {
    let input = "surface0\n{\nelement0,overlay,CityPop\\surface0010.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].elements.len(), 1);
    assert_eq!(
        shell.surfaces[0].elements[0].path.as_str(),
        "CityPop\\surface0010.png"
    );
}

/// 画像パスはスラッシュ区切りも無加工で保持する（正規化しない・要件 4.3）。
#[test]
fn element_path_with_slash_is_verbatim() {
    let input = "surface0\n{\nelement0,overlay,purple/2/a.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].elements.len(), 1);
    assert_eq!(
        shell.surfaces[0].elements[0].path.as_str(),
        "purple/2/a.png"
    );
}

/// 非ゼロ・負値座標を i64 として保持する（要件 4.2）。
#[test]
fn element_nonzero_coordinates_are_preserved() {
    let input = "surface0\n{\nelement0,overlay,x.png,12,-3\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].elements.len(), 1);
    assert_eq!(shell.surfaces[0].elements[0].x, 12);
    assert_eq!(shell.surfaces[0].elements[0].y, -3);
}

/// collision 矩形＋不透明領域名を正規化する（要件 6.1/6.2）。
/// ukadoc 順序 始点X/始点Y/終点X/終点Y ＝ left/top/right/bottom。
#[test]
fn collision_rect_with_opaque_name_is_normalized() {
    let input = "surface1000\n{\ncollision0,93,62,271,130,Head\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0].collisions,
        vec![Collision {
            index: 0,
            left: 93,
            top: 62,
            right: 271,
            bottom: 130,
            name: CollisionName::new("Head".to_string()),
        }]
    );
    assert_eq!(shell.surfaces[0].collisions[0].name.as_str(), "Head");
}

/// 複数 collision は出現順で保持する（要件 6.1）。
#[test]
fn multiple_collisions_keep_appearance_order() {
    let input =
        "surface1000\n{\ncollision0,93,62,271,130,Head\ncollision1,80,140,260,300,Bust\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].collisions.len(), 2);
    assert_eq!(shell.surfaces[0].collisions[0].index, 0);
    assert_eq!(shell.surfaces[0].collisions[0].name.as_str(), "Head");
    assert_eq!(shell.surfaces[0].collisions[1].index, 1);
    assert_eq!(shell.surfaces[0].collisions[1].name.as_str(), "Bust");
}

/// element ＋ collision ＋ animation 行が混在 → 各々が独立に充填され互いを壊さない・非パニック。
/// animation 行はタスク 4.3 で `decode_animations` が集約する（elements/collisions と共存する）。
#[test]
fn surface_mixing_elements_collisions_and_animation() {
    let input = "\
surface1410
{
element1,overlay,b.png,5,6
element0,overlay,a.png,0,0
collision0,10,20,30,40,Head
animation0.interval,bind
animation0.pattern0,overlay,100,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    // element はレイヤ昇順（element0 が先）。
    assert_eq!(
        shell.surfaces[0].elements,
        vec![
            Element {
                layer: 0,
                path: ElementPath::new("a.png".to_string()),
                x: 0,
                y: 0,
            },
            Element {
                layer: 1,
                path: ElementPath::new("b.png".to_string()),
                x: 5,
                y: 6,
            },
        ]
    );
    // collision は 1 個。
    assert_eq!(
        shell.surfaces[0].collisions,
        vec![Collision {
            index: 0,
            left: 10,
            top: 20,
            right: 30,
            bottom: 40,
            name: CollisionName::new("Head".to_string()),
        }]
    );
    // animation はタスク 4.3 で集約される（interval Bind ＋ pattern0）。
    assert_eq!(
        shell.surfaces[0].animations,
        vec![Animation {
            id: 0,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 100,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
}
