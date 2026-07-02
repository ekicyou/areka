//! 横断バリデーションテスト（主軸）— ukadoc 準拠の自前 in-source 適合テスト。
//!
//! surface 定義＋element overlay＋collision 矩形＋animation（bind / random / bind+random）＋
//! surface.append（範囲）＋alias（重複キー）＋負 ID overlay＋コメント/空行を含む最小
//! `surfaces.txt` 断片を**テスト内で自作**し、公開 `parse` 経由でモデル全構成を
//! end-to-end 検証する（要件 10.1）。
//!
//! 仕様解釈の正典は ukadoc（SERIKO/surfaces.txt 仕様）であり、期待値はリテラル直書きで
//! ukadoc の行文法に接地する。emo2 fixture の偶発的内容を正解の根拠にはしない（要件 10.2）。
//! 断片は `include_str!` を用いずテスト内リテラルで組み立てる。
//!
//! ukadoc 行文法（design.md「ukadoc 正典に基づく行文法」・期待値の唯一の根拠）:
//! - element:   `elementN,overlay,PATH,X,Y` → `Element { layer=N, path(opaque), x, y }`・レイヤ昇順。
//! - collision: `collision*,始点X,始点Y,終点X,終点Y,ID` → `Collision { index=*, left, top, right, bottom, name(opaque) }`。
//! - pattern:   `animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X,Y`（負 ID はセンチネル・i64 保持）。
//! - interval:  `animation*.interval,インターバル` → `Bind` / `Random{k}` / `BindRandom{k}`。
//! - range:     `surface0-2` → `AppendTarget::Range`（展開しない）・ヘッダ数値は第1要素 `Single`。
//! - alias:     `KEY,[id,...]` → `SurfaceAlias { key(opaque), ids }`・重複キーは全保持。

use super::model::{
    AliasKey, Animation, AppendTarget, Collision, CollisionName, Element, ElementPath, Interval,
    Pattern, Shell, Surface, SurfaceAlias, SurfaceAppend,
};
use super::parse;

/// ukadoc 準拠の自前通し断片（主軸）— surface 定義・element overlay・collision 矩形・
/// animation（bind / random / bind+random）・負 ID overlay・surface.append（範囲）・
/// alias（重複キー）・コメント/空行を 1 断片に集約し、`parse` の全出力を
/// リテラル期待値と end-to-end で照合する（要件 10.1/10.2）。
///
/// 期待値は上記モジュールコメントの ukadoc 行文法に接地しており、emo2 の偶発的内容
/// （特定 surface 番号・画像名・領域名）に依存しない（要件 10.2）。
#[test]
fn ukadoc_end_to_end_conformance_fragment() {
    // 自作断片: コメント/空行を意図的に散在させ、無視されることを断片自体で検証する。
    let input = "\
// ---- ukadoc 準拠 自前適合断片（コメント行は無視される・要件 9.1/10.1）----
charset,UTF-8

// surface 定義: element を降順（layer 2 → 0）で書き、昇順ソートを証明する（要件 4.4）。
surface10
{
element2,overlay,parts/hair.png,3,4
// このコメント行はブロック内でも無視される。
element0,overlay,parts/body.png,0,0
element1,overlay,parts\\face.png,-5,6

collision0,10,20,110,220,Head
collision1,30,40,130,240,Bust

// interval bind ＋ 単一 pattern（要件 5.1）。
animation0.interval,bind
animation0.pattern0,overlay,20,100,0,0

// interval random,K（要件 5.2）＋ 負 ID overlay（-1 センチネル・i64 保持・要件 5.5）。
animation1.interval,random,4
animation1.pattern0,overlay,21,50,0,0
animation1.pattern1,overlay,-1,80,0,0

// interval bind+random,K（要件 5.3）＋ 疎 pattern index（0,2・要件 5.4）。
animation2.interval,bind+random,7
animation2.pattern0,overlay,22,60,0,0
animation2.pattern2,overlay,23,60,0,0
}

// surface.append: ヘッダ数値＋範囲＋単一の混在ターゲット（範囲は展開しない・要件 7.2）。
surface.append10,2100-2110,2200
{
collision0,52,38,156,80,Head
animation0.interval,random,3
animation0.pattern0,overlay,2106,0,0,0
}

// alias: 重複キー（同一キー 2 回）は潰さず全保持・順序保持（要件 8.4）。日本語キーも不透明保持。
kero.surface.alias
{
6,[2106,2206]
静観,[10,11]
6,[3106,3206]
}
";

    let shell = parse(input);

    let expected = Shell {
        surfaces: vec![Surface {
            id: 10,
            // element は入力順（2,0,1）に依らずレイヤ昇順（0,1,2）で保持される（要件 4.4）。
            elements: vec![
                Element {
                    layer: 0,
                    path: ElementPath::new("parts/body.png".to_string()),
                    x: 0,
                    y: 0,
                },
                Element {
                    // バックスラッシュ区切りは無加工保持（要件 4.3）・負座標は i64 保持（要件 4.2）。
                    layer: 1,
                    path: ElementPath::new("parts\\face.png".to_string()),
                    x: -5,
                    y: 6,
                },
                Element {
                    layer: 2,
                    path: ElementPath::new("parts/hair.png".to_string()),
                    x: 3,
                    y: 4,
                },
            ],
            // collision は 始点X/始点Y/終点X/終点Y = left/top/right/bottom（ukadoc 順序・要件 6.1/6.2）。
            collisions: vec![
                Collision {
                    index: 0,
                    left: 10,
                    top: 20,
                    right: 110,
                    bottom: 220,
                    name: CollisionName::new("Head".to_string()),
                },
                Collision {
                    index: 1,
                    left: 30,
                    top: 40,
                    right: 130,
                    bottom: 240,
                    name: CollisionName::new("Bust".to_string()),
                },
            ],
            // animation は id 初出順（0,1,2）で保持（順序付け実行はしない・要件 5.6）。
            animations: vec![
                Animation {
                    id: 0,
                    interval: Interval::Bind,
                    patterns: vec![Pattern {
                        index: 0,
                        surface_id: 20,
                        wait: 100,
                        x: 0,
                        y: 0,
                    }],
                },
                Animation {
                    id: 1,
                    interval: Interval::Random { k: 4 },
                    patterns: vec![
                        Pattern {
                            index: 0,
                            surface_id: 21,
                            wait: 50,
                            x: 0,
                            y: 0,
                        },
                        Pattern {
                            // 負 ID overlay は -1 センチネルとして i64 保持（要件 5.5）。
                            index: 1,
                            surface_id: -1,
                            wait: 80,
                            x: 0,
                            y: 0,
                        },
                    ],
                },
                Animation {
                    id: 2,
                    interval: Interval::BindRandom { k: 7 },
                    // 疎 index（0,2・pattern1 欠番）を合成せず保持（要件 5.4）。
                    patterns: vec![
                        Pattern {
                            index: 0,
                            surface_id: 22,
                            wait: 60,
                            x: 0,
                            y: 0,
                        },
                        Pattern {
                            index: 2,
                            surface_id: 23,
                            wait: 60,
                            x: 0,
                            y: 0,
                        },
                    ],
                },
            ],
        }],
        appends: vec![SurfaceAppend {
            // ヘッダ数値 10 は第1要素 Single、`2100-2110` は Range（展開しない）、末尾 2200 は Single（要件 7.2）。
            targets: vec![
                AppendTarget::Single(10),
                AppendTarget::Range {
                    start: 2100,
                    end: 2110,
                },
                AppendTarget::Single(2200),
            ],
            collisions: vec![Collision {
                index: 0,
                left: 52,
                top: 38,
                right: 156,
                bottom: 80,
                name: CollisionName::new("Head".to_string()),
            }],
            animations: vec![Animation {
                id: 0,
                interval: Interval::Random { k: 3 },
                patterns: vec![Pattern {
                    index: 0,
                    surface_id: 2106,
                    wait: 0,
                    x: 0,
                    y: 0,
                }],
            }],
        }],
        // 重複キー "6" は 2 エントリとも順序保持（要件 8.4）。日本語キー "静観" は不透明保持（要件 8.2）。
        aliases: vec![
            SurfaceAlias {
                key: AliasKey::new("6".to_string()),
                ids: vec![2106, 2206],
            },
            SurfaceAlias {
                key: AliasKey::new("静観".to_string()),
                ids: vec![10, 11],
            },
            SurfaceAlias {
                key: AliasKey::new("6".to_string()),
                ids: vec![3106, 3206],
            },
        ],
    };

    assert_eq!(shell, expected);
}

/// 範囲ターゲットが展開されず記述子のまま保持されることを、記述子数で直接示す（要件 7.2）。
/// `surface.append0-2,5` → [Range{0,2}, Single(5)]（範囲展開後の個別 ID 4 個ではない）。
/// ここでは意図的にヘッダ自体を範囲 `append0-2` にし、ヘッダ数値の一律 Single 扱いに
/// 依存しない範囲保持を証明する。
#[test]
fn append_range_target_is_not_expanded() {
    let input = "\
surface.append0-2,5
{
}
";
    let shell = parse(input);
    assert_eq!(shell.appends.len(), 1);
    // 記述子は 2 個（Range + Single）であり、範囲展開後の個別 ID 数（0,1,2,5 の 4 個）ではない。
    assert_eq!(shell.appends[0].targets.len(), 2);
    assert_eq!(
        shell.appends[0].targets,
        vec![
            AppendTarget::Range { start: 0, end: 2 },
            AppendTarget::Single(5),
        ]
    );
}

/// コメント行・空行のみの入力（意味行ゼロ）は空 `Shell` を返す（要件 9.1/10.1）。
/// コメント/空行が「無視される（トークンを生じない）」ことを孤立させて確認する。
#[test]
fn comment_and_blank_only_input_yields_empty_shell() {
    let input = "\
// これはコメントだけの断片。

// 空行とコメントのみで、認識可能なブロックは存在しない。

";
    let shell = parse(input);
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
        }
    );
}

// --- タスク 6.2: emo2 実物 fixture スモーク（要件 10.3）と subset 外吸収（要件 4.5/5.7/6.3/9.2）---
//
// 6.1 の自前適合断片が「唯一の適合基準」（ukadoc）を厳密照合するのに対し、ここでは
// emo2 実物 fixture（crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt）
// の代表抜粋を**リテラル転記**し、パニックせず・スコープ内機能（surface/element/animation/
// collision/append/alias）を解釈し切ることを**緩め**に確認する（emo2 は最小適合サンプルであって
// 書式の聖典ではない・要件 10.2/10.3）。`include_str!` はクレート跨ぎを避けるため用いない。

/// emo2 実物 fixture の代表抜粋（VERBATIM 転記）を `parse` に通し、パニックせず
/// スコープ内機能を解釈し切ることをスモーク検証する（要件 10.3）。
///
/// 転記元の実行（fixture 行番号は 2026-07-02 時点）:
/// - surface0（L10-13）: `element0,overlay,surface0.png,0,0`（element overlay）。
/// - surface1000（L20-117 抜粋）: `collision0,93,62,271,130,Head` / `animation1100.interval,bind` /
///   `animation1400.interval,bind+random,4` ＋疎 pattern（pattern1/2/3・pattern0 欠番）。
/// - surface1100（L125-128）: 単一 element overlay のヘルパー surface。
/// - surface1410（L211-215）: 複数 element（element0/element1）。
/// - surface.append10,2100-2110,2200-2210（L415-419）: 範囲ターゲット＋collision。
/// - surface.append2110（L443-448）: random interval＋負 ID センチネル pattern（`-1`）。
/// - kero.surface.alias（L458-507 抜粋）: 数値/日本語キー・重複キー・複数 ID。
///
/// アサートは 6.1 より緩く「パースが通り・スコープ内機能が解釈されている」ことに絞る
/// （非空・期待 kind の存在）。偶発的な全値の固定化はしない（要件 10.3）。
#[test]
fn emo2_fixture_representative_excerpt_smoke() {
    // emo2 実物 fixture からの VERBATIM 抜粋（改行・区切り・番号は転記元のまま）。
    let input = "\
charset,UTF-8
descript
{
version,1
}

//########################################################################################
// \\0
//########################################################################################
surface0
{
element0,overlay,surface0.png,0,0
}

surface1000
{
// --- collision ---
collision0,93,62,271,130,Head
collision1,133,270,229,326,Bust

// --- 腕 [z=1] ---
animation1100.interval,bind
animation1100.pattern0,overlay,1100,0,0,0

// まばたき [z=4]：通常
animation1400.interval,bind+random,4
animation1400.pattern1,overlay,1412,0,0,0
animation1400.pattern2,overlay,1411,150,0,0
animation1400.pattern3,overlay,1410,22,0,0
}

// --- 腕 ---
surface1100
{
element0,overlay,purple/0/base1.png,0,0
}

surface1410
{
element0,overlay,purple/a/eyebase.png,0,0
element1,overlay,purple/4/normal.png,0,0
}

//\\1
surface.append10,2100-2110,2200-2210
{
collision0,52,38,156,80,Head
collision1,82,163,140,186,Bust
}

surface.append2110
{
animation0.interval,random,4
animation0.pattern0,overlay,2106,0,0,0
animation0.pattern3,overlay,-1,160,0,0
}

//########################################################################################
//エイリアス
kero.surface.alias
{
0,[2100]
6,[2106,2206]
通常,[2100]
静観,[2106,2206]
100,[2100]
100,[2100]
}
";

    // (a) パニックしない（スモークの第一義・要件 10.3）。
    let shell = parse(input);

    // (b) スコープ内機能が解釈され切っている（非空・期待 kind の存在）。

    // --- surface / element ---
    assert!(!shell.surfaces.is_empty(), "surface が解釈されていない");
    // surface0 の存在と単一 element overlay。
    let s0 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 0)
        .expect("surface0 が存在しない");
    assert_eq!(s0.elements.len(), 1);
    assert_eq!(s0.elements[0].layer, 0);
    // 画像パスは不透明保持（無加工・要件 4.3）。
    assert_eq!(s0.elements[0].path.as_str(), "surface0.png");
    // 複数 element を持つ surface1410（element0/element1）。
    let s1410 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 1410)
        .expect("surface1410 が存在しない");
    assert_eq!(s1410.elements.len(), 2, "複数 element が解釈されていない");

    // --- collision（surface1000）---
    let s1000 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 1000)
        .expect("surface1000 が存在しない");
    assert_eq!(s1000.collisions.len(), 2, "collision が解釈されていない");
    // ukadoc 順序 left/top/right/bottom（要件 6.1）で転記値が入っている。
    assert_eq!(
        s1000.collisions[0],
        Collision {
            index: 0,
            left: 93,
            top: 62,
            right: 271,
            bottom: 130,
            name: CollisionName::new("Head".to_string()),
        }
    );

    // --- animation（interval 3 種と疎 pattern）---
    // bind interval（animation1100）。
    let a1100 = s1000
        .animations
        .iter()
        .find(|a| a.id == 1100)
        .expect("animation1100 が存在しない");
    assert_eq!(a1100.interval, Interval::Bind);
    // bind+random,4 interval（animation1400）＋疎 pattern（pattern0 欠番・1/2/3 のみ）。
    let a1400 = s1000
        .animations
        .iter()
        .find(|a| a.id == 1400)
        .expect("animation1400 が存在しない");
    assert_eq!(a1400.interval, Interval::BindRandom { k: 4 });
    assert_eq!(a1400.patterns.len(), 3, "疎 pattern が解釈されていない");
    // pattern index は転記元の 1/2/3（連番前提を置かない・要件 5.4）。
    assert_eq!(
        a1400.patterns.iter().map(|p| p.index).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );

    // --- append（範囲ターゲット＋負 ID センチネル）---
    assert!(!shell.appends.is_empty(), "append が解釈されていない");
    // 範囲ターゲットを持つ append（surface.append10,2100-2110,2200-2210）。
    let range_append = shell
        .appends
        .iter()
        .find(|a| {
            a.targets
                .contains(&AppendTarget::Range { start: 2100, end: 2110 })
        })
        .expect("範囲ターゲットの append が存在しない");
    // ヘッダ数値 10 は第1要素 Single、範囲は展開せず記述子保持（要件 7.2）。
    assert_eq!(
        range_append.targets,
        vec![
            AppendTarget::Single(10),
            AppendTarget::Range { start: 2100, end: 2110 },
            AppendTarget::Range { start: 2200, end: 2210 },
        ]
    );
    assert_eq!(range_append.collisions.len(), 2);
    // 負 ID センチネル pattern を持つ append（surface.append2110）。
    let blink_append = shell
        .appends
        .iter()
        .find(|a| a.targets == vec![AppendTarget::Single(2110)])
        .expect("surface.append2110 が存在しない");
    assert_eq!(blink_append.animations.len(), 1);
    let blink_anim = &blink_append.animations[0];
    assert_eq!(blink_anim.interval, Interval::Random { k: 4 });
    // `pattern3,overlay,-1,...` の -1 は負センチネルとして i64 保持（要件 5.5）。
    let neg = blink_anim
        .patterns
        .iter()
        .find(|p| p.surface_id < 0)
        .expect("負 ID センチネル pattern が存在しない");
    assert_eq!(neg.surface_id, -1);

    // --- alias（数値/日本語キー・重複キー・複数 ID）---
    assert!(!shell.aliases.is_empty(), "alias が解釈されていない");
    // 日本語キーが不透明保持されている（要件 8.2）。
    assert!(
        shell.aliases.iter().any(|a| a.key.as_str() == "通常"),
        "日本語 alias キーが解釈されていない"
    );
    // 複数 ID を持つ alias（`6,[2106,2206]`）。
    let alias6 = shell
        .aliases
        .iter()
        .find(|a| a.key.as_str() == "6")
        .expect("alias キー 6 が存在しない");
    assert_eq!(alias6.ids, vec![2106, 2206]);
    // 重複キー `100`（fixture に 2 回出現）は潰さず全保持（要件 8.4）。
    let key100_count = shell.aliases.iter().filter(|a| a.key.as_str() == "100").count();
    assert_eq!(key100_count, 2, "重複 alias キーが潰されている");
}

/// subset 外機能を含む emo2 風の複合断片を**公開 `parse` 経由**で通し、subset 外が
/// 吸収され（materialize されない）、隣接する認識可能ブロックが壊れないことを検証する
/// （要件 4.5/5.7/6.3/9.2）。
///
/// タスク 4.6 は同性質を decode レイヤの合成断片で検証済み。ここは**バリデーション層で
/// 公開 facade を通す**補完で、emo2 風の実在感ある断片を用いる（重複ではない）。
///
/// 断片は認識可能ブロックの「間」に subset 外を挟む:
/// - 非 overlay element メソッド（`element1,base,...`）→ 吸収（overlay 兄弟は残る・要件 4.5）。
/// - 非 overlay pattern メソッド（`pattern0,replace,...`）→ 吸収（overlay 兄弟は残る・要件 5.7）。
/// - 3 種以外の interval（`sometimes` / `periodic`）→ 正規化せず既定 Bind へ倒す（要件 5.7）。
/// - `collisionex`（円/楕円/多角形）→ 吸収（純 collision は残る・要件 6.3）。
#[test]
fn subset_out_features_absorbed_via_public_parse_without_breaking_neighbors() {
    let input = "\
charset,UTF-8

// 先頭の認識可能 surface（完全に解釈されるべき）。
surface100
{
element0,overlay,body.png,0,0
collision0,10,20,110,220,Head
animation0.interval,bind
animation0.pattern0,overlay,200,0,0,0
}

// 間に挟む subset 外を含む surface。valid 行のみ materialize される。
surface200
{
// 非 overlay element メソッド（base）は吸収され、overlay 兄弟は残る（要件 4.5）。
element0,base,bg.png,0,0
element1,overlay,face.png,0,0

// collisionex（多角形）は吸収され、純 collision は残る（要件 6.3）。
collisionex0,polygon,1,2,3,4,5,6,Face
collision0,30,40,130,240,Bust

// 3 種以外の interval（sometimes）は正規化されず既定 Bind へ倒れ、pattern は失われない（要件 5.7）。
animation0.interval,sometimes,5
// 非 overlay pattern メソッド（replace）は吸収され、overlay 兄弟は残る（要件 5.7）。
animation0.pattern0,replace,900,0,0,0
animation0.pattern1,overlay,901,50,0,0
}

// 末尾の認識可能 surface（完全に解釈されるべき）。
surface300
{
element0,overlay,tail.png,7,8
}
";

    let shell = parse(input);

    // --- 隣接する認識可能ブロックが壊れていないこと ---
    let s100 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 100)
        .expect("先頭 surface100 が壊れた");
    assert_eq!(s100.elements.len(), 1);
    assert_eq!(s100.elements[0].path.as_str(), "body.png");
    assert_eq!(s100.collisions.len(), 1);
    assert_eq!(s100.animations.len(), 1);
    assert_eq!(s100.animations[0].interval, Interval::Bind);
    assert_eq!(s100.animations[0].patterns.len(), 1);

    let s300 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 300)
        .expect("末尾 surface300 が壊れた");
    assert_eq!(s300.elements.len(), 1);
    assert_eq!(s300.elements[0].path.as_str(), "tail.png");
    assert_eq!(s300.elements[0].x, 7);
    assert_eq!(s300.elements[0].y, 8);

    // --- subset 外が吸収され、valid 行のみ materialize されていること ---
    let s200 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 200)
        .expect("中間 surface200 が消えた");

    // element: base 吸収・overlay のみ残る。転記した唯一の overlay は face.png（layer 1）。
    assert_eq!(s200.elements.len(), 1, "非 overlay element が吸収されていない");
    assert_eq!(s200.elements[0].layer, 1);
    assert_eq!(s200.elements[0].path.as_str(), "face.png");

    // collision: collisionex 吸収・純 collision のみ残る。
    assert_eq!(
        s200.collisions.len(),
        1,
        "collisionex が吸収されていない"
    );
    assert_eq!(s200.collisions[0].name.as_str(), "Bust");
    assert_eq!(s200.collisions[0].left, 30);

    // animation: sometimes は既定 Bind へ倒れ、replace pattern は吸収・overlay pattern のみ残る。
    assert_eq!(s200.animations.len(), 1);
    let a = &s200.animations[0];
    assert_eq!(a.interval, Interval::Bind, "非 3 種 interval が正規化された");
    assert_eq!(a.patterns.len(), 1, "非 overlay pattern が吸収されていない");
    assert_eq!(a.patterns[0].index, 1);
    assert_eq!(a.patterns[0].surface_id, 901);
}
