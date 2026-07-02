//! decode の単体テスト（タスク 4.1: surface ブロックの枠組みとヘッダの寛容スキップ）。
//!
//! 本タスクが検証するのは decode スケルトンの骨格挙動のみ:
//! - 空入力・ヘッダのみ（charset/descript）・ヘッダ欠落が失敗せず既定状態で継続する（要件 3.1/3.2/3.3）。
//! - `surfaceNNN` ブロックから surface ID と（当面空の）構成要素枠を取り出す（要件 4.1）。
//! - ディスパッチ優先順位により `kero.surface.alias` / `surface.append*` が
//!   `surfaceNNN` と誤分類されない（設計 System Flows のブロックディスパッチ）。
//!
//! element/collision/animation の値化・append/alias の値化はタスク 4.2〜4.6 の領分ゆえ
//! ここでは検証しない（本タスクは枠組みのみ）。期待値はリテラル直書き（sakura 規律）。

use super::decode::decode;
use super::lexer::lex;
use super::model::{Collision, CollisionName, Element, ElementPath, Shell, Surface};

/// 空入力 → 空 `Shell`・非パニック（要件 3.3）。
#[test]
fn empty_input_yields_empty_shell() {
    let shell = decode(lex(""));
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
        }
    );
}

/// ヘッダのみ（charset 行 ＋ descript ブロック）→ 何も保持しない空 `Shell`（要件 3.1/3.2）。
#[test]
fn header_only_is_not_retained() {
    let input = "charset,UTF-8\ndescript\n{\nversion,1\n}\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
        }
    );
}

/// `surface0` ブロック → surface ID 0 を持つ枠が 1 個。element 本体はタスク 4.2 で充填。
/// （animation はタスク 4.3 のシームゆえ空・append/alias は本タスクで生成しない。）
#[test]
fn surface_zero_block_extracts_id_zero() {
    let input = "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0],
        Surface {
            id: 0,
            elements: vec![Element {
                layer: 0,
                path: ElementPath::new("surface0.png".to_string()),
                x: 0,
                y: 0,
            }],
            collisions: vec![],
            animations: vec![],
        }
    );
    // append/alias は本タスクでは生成しない。
    assert!(shell.appends.is_empty());
    assert!(shell.aliases.is_empty());
}

/// `surface1000` → id 1000 を正しく取り出す（数値抽出）。
#[test]
fn surface_large_id_is_extracted() {
    let input = "surface1000\n{\ncollision0,93,62,271,130,Head\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].id, 1000);
}

/// 非数値 ID（`surface` の後ろに数字がない崩れヘッダ）→ パニックせず既定 0 に倒す（要件 3.3）。
#[test]
fn malformed_surface_id_defaults_to_zero() {
    let input = "surface\n{\nelement0,overlay,x.png,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].id, 0);
}

/// ヘッダ欠落（surface 定義がいきなり本体行のみ・崩れ入力）→ 失敗せず継続（要件 3.3）。
#[test]
fn missing_header_does_not_fail() {
    // ブロック開始のない孤立行群。lexer は TopLevel/Raw に落とし、decode は吸収する。
    let input = "element0,overlay,x.png,0,0\ncollision0,1,2,3,4,Head\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
        }
    );
}

/// 同一入力を 2 度 decode → 同一結果（決定性・要件 2.4 系）。
#[test]
fn decode_is_deterministic() {
    let input = "charset,UTF-8\nsurface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface1000\n{\n}\n";
    let a = decode(lex(input));
    let b = decode(lex(input));
    assert_eq!(a, b);
}

/// ディスパッチ優先順位: `kero.surface.alias` / `surface.append*` を `surfaceNNN` と誤分類しない。
/// 本タスクでは alias/append の値化は行わない（4.4/4.5）ので surfaces のみを検査する。
/// 実 surfaceNNN（`surface2100`）だけが surfaces に入り、alias/append ヘッダ由来の
/// 偽 surface が混入しないことを保証する。
#[test]
fn dispatch_precedence_does_not_misclassify_alias_or_append() {
    let input = "\
surface2100
{
element0,overlay,CityPop.png,0,0
}
surface.append10,2100-2110,2200-2210
{
collision0,52,38,156,80,Head
}
kero.surface.alias
{
6,[2106,2206]
}
";
    let shell = decode(lex(input));
    // surfaces に入るのは実 surface2100 のみ（alias/append を surface と誤読しない）。
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].id, 2100);
}

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
    assert_eq!(shell.surfaces[0].elements[0].path.as_str(), "purple/2/a.png");
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

/// element ＋ collision ＋（無視される）animation 行が混在 → element/collision のみ充填・非パニック。
/// animation 行はタスク 4.3 のシームゆえここでは吸収され elements/collisions を壊さない。
#[test]
fn surface_mixing_elements_collisions_and_ignored_animation() {
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
    // animation はタスク 4.3 のシームゆえ本タスクでは充填しない。
    assert!(shell.surfaces[0].animations.is_empty());
}
