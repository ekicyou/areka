//! 転記ギャップ4点の値レベル検証（元 decode_tests.rs タスク 1.4 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

use super::{AppendTarget, DefRef, Element, ElementPath, SortOrder, decode, lex};

// --- タスク 1.4: 転記ギャップ4点の値レベル検証（要件 12.5(a)/(b)/(c)/(d)） ---
//
// タスク 1.2 で decode.rs へ実装済みの4つの転記経路が「転記結果の値を直接返す」ことを
// 確定的にアサートする（確認テスト・純粋追加）。
// (b) 多 id ヘッダ（列挙・範囲）／(c) append 内 element ＋多ターゲット範囲ヘッダ／
// (a) sort キー値（寛容 None 含む）／(d) definitions の登場順（interleaving）保持。
// 断片は emo2 由来の形（`surface.append10,2100-2110,2200-2210` 等）を用いる。
// 期待値はリテラル直書き（sakura 規律）。

/// (b) 多 id ヘッダ（列挙 `surface0,5` ＋範囲 `surface1-3`）の忠実転記（要件 12.5(b)）。
/// 列挙は各 id を Single 記述子として保持し、代表 id は先頭ターゲット値。
/// 範囲は展開せず Range 記述子で保持し、代表 id は範囲始点（旧 `unwrap_or(0)` 破損の修正を固定）。
#[test]
fn multi_id_surface_header_enumeration_and_range_are_transcribed() {
    let input = "\
surface0,5
{
element0,overlay,a.png,0,0
}
surface1-3
{
element0,overlay,b.png,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 2);

    // 列挙 `surface0,5` → targets == [Single(0), Single(5)]・代表 id == 0。
    assert_eq!(shell.surfaces[0].id, 0);
    assert_eq!(
        shell.surfaces[0].targets,
        vec![AppendTarget::Single(0), AppendTarget::Single(5)]
    );

    // 範囲 `surface1-3` → targets == [Range{1,3}]・代表 id == 1（範囲始点・unwrap_or(0) 破損でない）。
    assert_eq!(shell.surfaces[1].id, 1);
    assert_eq!(
        shell.surfaces[1].targets,
        vec![AppendTarget::Range { start: 1, end: 3 }]
    );
}

/// (c) append 内 element の転記 ＋ 多ターゲット範囲ヘッダ（要件 12.5(c)/(b)）。
/// emo2 由来の `surface.append10,2100-2110,2200-2210` を用い、ヘッダを
/// [Single(10), Range{2100,2110}, Range{2200,2210}] として保持し、
/// ブロック内 `element*,overlay,...` を従来黙殺していた分まで SurfaceAppend.elements へ転記する。
#[test]
fn append_inner_element_and_multi_range_header_are_transcribed() {
    let input = "\
surface.append10,2100-2110,2200-2210
{
element0,overlay,CityPop.png,3,4
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);

    // 多ターゲット範囲ヘッダの忠実転記（展開しない）。
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

    // append 内 element を通常 surface と同一表現で転記（layer/path/x/y）。
    assert_eq!(
        shell.appends[0].elements,
        vec![Element {
            layer: 0,
            path: ElementPath::new("CityPop.png".to_string()),
            x: 3,
            y: 4,
        }]
    );
}

/// (a) sort キー値の転記（要件 12.5(a)）。トップレベル `animation-sort`／`collision-sort` を
/// SortOrder として保持し、未知値は寛容に None へ倒す（既定解釈は下流）。
#[test]
fn top_level_sort_keys_are_transcribed_and_unknown_is_none() {
    // 認識可能な 2 値: animation-sort,ascend / collision-sort,descend。
    let input = "animation-sort,ascend\ncollision-sort,descend\n";
    let shell = decode(lex(input));
    assert_eq!(shell.animation_sort, Some(SortOrder::Ascend));
    assert_eq!(shell.collision_sort, Some(SortOrder::Descend));

    // 未知値は寛容に None（materialize しない）。
    let unknown = "animation-sort,sideways\ncollision-sort,\n";
    let shell2 = decode(lex(unknown));
    assert_eq!(shell2.animation_sort, None);
    assert_eq!(shell2.collision_sort, None);
}

/// (d) definitions の登場順（種別間 interleaving）保持（要件 12.5(d)）。
/// surface → append → alias の並びを、各 Vec への index 参照ストリームとして
/// 正確に保持する（データ複製なし・登場順のまま）。
#[test]
fn definitions_preserve_interleaved_appearance_order() {
    let input = "\
surface0
{
element0,overlay,a.png,0,0
}
surface.append10,2100-2110
{
collision0,1,2,3,4,Head
}
kero.surface.alias
{
6,[2106,2206]
}
";
    let shell = decode(lex(input));
    // 各 Vec には 1 件ずつ入り、登場順ストリームは index 0 参照が 3 種順に並ぶ。
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(
        shell.definitions,
        vec![
            DefRef::Surface(0),
            DefRef::Append(0),
            DefRef::Alias(0),
        ]
    );
}
