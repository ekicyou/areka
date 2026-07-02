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
use super::model::{
    Animation, AppendTarget, Collision, CollisionName, Element, ElementPath, Interval, Pattern,
    Shell, Surface,
};

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
                surface_id: 100,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
}

// --- タスク 4.3: animationN 集約（interval 3 種・疎 pattern・負センチネル） ---
//
// 検証範囲（要件 5.1/5.2/5.3/5.4/5.5/5.6）:
// - `animationN.interval` と複数の `animationN.patternM` を同一 animation ID へ束ねる（要件 5.6）。
// - interval を bind / random,K / bind+random,K の 3 種として正規化する（要件 5.1/5.2/5.3）。
// - pattern index を明示保持し（連番前提なし・疎許容・要件 5.4）、負の surface 参照 ID を
//   センチネルとして失わず i64 保持する（要件 5.5・意味付けは下流）。
// - animation は id の初出順で保持する（z-order 実順序付けはしない・要件 5.6）。
// interval 3 種以外（sometimes/periodic 等・要件 5.7）と非 overlay pattern は
// タスク 4.6 の寛容吸収シームゆえここでは検証しない。

/// `animationN.interval,bind` ＋ 単一 pattern → interval Bind・pattern 正規化（要件 5.1/5.4）。
#[test]
fn animation_bind_interval_with_single_pattern() {
    let input = "surface1\n{\nanimation100.interval,bind\nanimation100.pattern0,overlay,1100,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0].animations,
        vec![Animation {
            id: 100,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                surface_id: 1100,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
}

/// `animationN.interval,random,K` → Interval::Random{k}（要件 5.2）。
#[test]
fn animation_random_interval_parses_k() {
    let input = "surface1\n{\nanimation0.interval,random,4\nanimation0.pattern0,overlay,10,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 0);
    assert_eq!(shell.surfaces[0].animations[0].interval, Interval::Random { k: 4 });
}

/// `animationN.interval,bind+random,K` → Interval::BindRandom{k}（要件 5.3）。
#[test]
fn animation_bind_random_interval_parses_k() {
    let input =
        "surface1\n{\nanimation1400.interval,bind+random,4\nanimation1400.pattern1,overlay,1410,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 1400);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::BindRandom { k: 4 }
    );
}

/// 疎 pattern（pattern0/pattern1/pattern3・pattern2 欠番）→ index [0,1,3] を合成せず保持（要件 5.4）。
#[test]
fn sparse_pattern_indices_are_preserved_not_synthesized() {
    let input = "\
surface1
{
animation0.interval,random,4
animation0.pattern0,overlay,10,50,0,0
animation0.pattern1,overlay,11,60,0,0
animation0.pattern3,overlay,13,70,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    let indices: Vec<u32> = shell.surfaces[0].animations[0]
        .patterns
        .iter()
        .map(|p| p.index)
        .collect();
    assert_eq!(indices, vec![0, 1, 3]);
}

/// 負のサーフェス参照 ID（`overlay,-1`）→ surface_id を i64 の -1 として失わず保持（要件 5.5）。
#[test]
fn negative_surface_id_is_preserved_as_sentinel() {
    let input = "surface1\n{\nanimation0.interval,random,4\nanimation0.pattern3,overlay,-1,80,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(
        shell.surfaces[0].animations[0].patterns,
        vec![Pattern {
            index: 3,
            surface_id: -1,
            wait: 80,
            x: 0,
            y: 0,
        }]
    );
}

/// 複数 animation は id 初出順で集約される（要件 5.6）。
#[test]
fn multiple_animations_aggregate_in_first_appearance_order() {
    let input = "\
surface1000
{
animation1100.interval,bind
animation1100.pattern0,overlay,1100,0,0,0
animation1200.interval,bind
animation1200.pattern0,overlay,1200,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 2);
    assert_eq!(shell.surfaces[0].animations[0].id, 1100);
    assert_eq!(shell.surfaces[0].animations[1].id, 1200);
}

/// 同一 animation id の複数 pattern 行は同一 animation の patterns へ束ねる（別 animation にしない・要件 5.6）。
#[test]
fn multiple_pattern_lines_bind_into_same_animation() {
    let input = "\
surface1000
{
animation1400.interval,bind+random,4
animation1400.pattern1,overlay,1410,0,0,0
animation1400.pattern2,overlay,1420,0,0,0
animation1400.pattern3,overlay,1430,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 1400);
    let indices: Vec<u32> = shell.surfaces[0].animations[0]
        .patterns
        .iter()
        .map(|p| p.index)
        .collect();
    assert_eq!(indices, vec![1, 2, 3]);
}

/// interval 行が pattern 行より後でも同一 id へ正しく束ねる（順序寛容・堅牢性）。
#[test]
fn interval_after_patterns_still_binds() {
    let input = "\
surface1
{
animation5.pattern0,overlay,50,0,0,0
animation5.pattern1,overlay,51,0,0,0
animation5.interval,random,2
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 5);
    assert_eq!(shell.surfaces[0].animations[0].interval, Interval::Random { k: 2 });
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 2);
}

/// interval 行が無い（pattern のみ）animation → 既定 Interval::Bind で pattern を失わない（堅牢性）。
#[test]
fn animation_without_interval_defaults_to_bind() {
    let input = "surface1\n{\nanimation7.pattern0,overlay,70,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 7);
    assert_eq!(shell.surfaces[0].animations[0].interval, Interval::Bind);
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 1);
}

// --- タスク 4.4: surface.append ターゲット指定の捕捉（展開しない転記） ---
//
// 検証範囲（要件 7.1/7.2/7.3）:
// - `surface.appendNNN,tgt,a-b,...` のターゲット指定を、ヘッダ数値 NNN を第1要素とし、
//   後続の単一 ID・範囲 a-b を同格の順序付き記述子リスト（Single/Range）として保持する。
//   範囲は展開しない・ヘッダのカテゴリ番号的特別扱いはしない（要件 7.2）。
// - 追記ブロック内の collision/animation を通常 surface と同一のモデル表現で保持する。
//   animation は decode_animations（タスク 4.3）の集約を再利用する（要件 7.3）。
// - 範囲展開と実 surface ツリーへの転記は下流に委ね、パーサは忠実な転記のみ行う（要件 7.2）。

/// 混在ターゲット（ヘッダ＋範囲＋範囲）＋ collision 捕捉（要件 7.1/7.2）。
/// `surface.append10,2100-2110,2200-2210` → [Single(10), Range{2100,2110}, Range{2200,2210}]。
/// ヘッダ数値 10 は第1要素として一様に Single で扱い、範囲は展開しない。
#[test]
fn append_mixed_targets_with_collision_are_captured() {
    let input = "\
surface.append10,2100-2110,2200-2210
{
collision0,52,38,156,80,Head
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(
        shell.appends[0].targets,
        vec![
            AppendTarget::Single(10),
            AppendTarget::Range {
                start: 2100,
                end: 2110,
            },
            AppendTarget::Range {
                start: 2200,
                end: 2210,
            },
        ]
    );
    // collision は通常 surface と同一表現（要件 7.3）。
    assert_eq!(
        shell.appends[0].collisions,
        vec![Collision {
            index: 0,
            left: 52,
            top: 38,
            right: 156,
            bottom: 80,
            name: CollisionName::new("Head".to_string()),
        }]
    );
    // surface には偽の append が混入しない。
    assert!(shell.surfaces.is_empty());
}

/// 列挙なしヘッダ（単一ヘッダ数値のみ）→ targets == [Single(2200)]・animation 集約（要件 7.2/7.3）。
#[test]
fn append_header_only_captures_single_target_and_animation() {
    let input = "\
surface.append2200
{
animation0.interval,random,4
animation0.pattern0,overlay,2206,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(shell.appends[0].targets, vec![AppendTarget::Single(2200)]);
    // animation は decode_animations の集約を再利用する（要件 7.3）。
    assert_eq!(
        shell.appends[0].animations,
        vec![Animation {
            id: 0,
            interval: Interval::Random { k: 4 },
            patterns: vec![Pattern {
                index: 0,
                surface_id: 2206,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
    // collision は無い。
    assert!(shell.appends[0].collisions.is_empty());
}

/// 単一 ID 列挙（範囲でない）→ [Single(10), Single(2100)]（要件 7.2）。
#[test]
fn append_single_enumeration_targets() {
    let input = "surface.append10,2100\n{\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(
        shell.appends[0].targets,
        vec![AppendTarget::Single(10), AppendTarget::Single(2100)]
    );
}

/// 範囲は展開しない: [Single(10), Range{2100,2110}] は記述子 2 個（個別 ID 12 個ではない・要件 7.2）。
#[test]
fn append_range_is_not_expanded() {
    let input = "surface.append10,2100-2110\n{\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    // targets の長さは記述子数（2）であり、範囲展開後の個別 ID 数（1 + 11 = 12）ではない。
    assert_eq!(shell.appends[0].targets.len(), 2);
    assert_eq!(
        shell.appends[0].targets,
        vec![
            AppendTarget::Single(10),
            AppendTarget::Range {
                start: 2100,
                end: 2110,
            },
        ]
    );
}

/// append の animation はタスク 4.3 の集約（疎 index ＋負センチネル）を再利用する（要件 7.3）。
/// pattern0（surface_id 2106）＋ pattern3（surface_id -1）→ 1 個の Animation・index [0,3]・-1 保持。
#[test]
fn append_animation_reuses_sparse_and_negative_sentinel_aggregation() {
    let input = "\
surface.append2200
{
animation0.interval,random,4
animation0.pattern0,overlay,2106,0,0,0
animation0.pattern3,overlay,-1,80,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(shell.appends[0].animations.len(), 1);
    let anim = &shell.appends[0].animations[0];
    assert_eq!(anim.id, 0);
    let indices: Vec<u32> = anim.patterns.iter().map(|p| p.index).collect();
    assert_eq!(indices, vec![0, 3]);
    // 負センチネル -1 を i64 で失わず保持する。
    assert_eq!(anim.patterns[1].surface_id, -1);
}

/// 複数 append ブロックは出現順で保持する（要件 7.1）。
#[test]
fn multiple_append_blocks_keep_appearance_order() {
    let input = "\
surface.append10,2100
{
}
surface.append2200
{
}
surface.append2110
{
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 3);
    assert_eq!(
        shell.appends[0].targets,
        vec![AppendTarget::Single(10), AppendTarget::Single(2100)]
    );
    assert_eq!(shell.appends[1].targets, vec![AppendTarget::Single(2200)]);
    assert_eq!(shell.appends[2].targets, vec![AppendTarget::Single(2110)]);
}

/// append の collision は通常 surface の collision と完全に同一構造（同一 Collision 型・要件 7.3）。
#[test]
fn append_collision_uses_same_representation_as_surface() {
    let surface_input = "surface1000\n{\ncollision0,93,62,271,130,Head\n}\n";
    let append_input = "surface.append1000\n{\ncollision0,93,62,271,130,Head\n}\n";
    let surface_shell = decode(lex(surface_input));
    let append_shell = decode(lex(append_input));
    // 同一 collision 行は surface でも append でも同じ Collision 値になる。
    assert_eq!(
        append_shell.appends[0].collisions,
        surface_shell.surfaces[0].collisions
    );
}
