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
