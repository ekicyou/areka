//! 転記ギャップ4点の値レベル検証。

use super::{
    AppendTarget, DefRef, DrawMethod, Element, ElementOptions, ElementPath, SortOrder, decode, lex,
};

/// (b) 多 id ヘッダ（列挙 `surface0,5` ＋範囲 `surface1-3`）の転記。
/// 列挙は各 id を Single 記述子として保持し、範囲は展開せず Range 記述子で保持する。
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

    assert_eq!(
        shell.surfaces[0].targets,
        vec![AppendTarget::Single(0), AppendTarget::Single(5)]
    );

    // 範囲 `surface1-3` → targets == [Range{1,3}]。
    assert_eq!(
        shell.surfaces[1].targets,
        vec![AppendTarget::Range { start: 1, end: 3 }]
    );
}

/// (c) append 内 element の転記 ＋ 多ターゲット範囲ヘッダ。
/// emo2 由来の `surface.append10,2100-2110,2200-2210` を用い、ヘッダを
/// [Single(10), Range{2100,2110}, Range{2200,2210}] として保持し、
/// ブロック内 `element*,overlay,...` を SurfaceAppend.elements へ転記する。
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

    assert_eq!(
        shell.appends[0].elements,
        vec![Element {
            layer: 0,
            path: ElementPath::new("CityPop.png".to_string()),
            x: 3,
            y: 4,
            method: DrawMethod::new("overlay".to_string()),
            options: ElementOptions::new(String::new()),
        }]
    );
}

/// (a) sort キー値の転記。トップレベル `animation-sort`／`collision-sort` を
/// SortOrder として保持し、知らない値は原文のまま `Other` で運ぶ（空・欠落は None・既定解釈は下流）。
#[test]
fn top_level_sort_keys_are_transcribed_and_unknown_is_other() {
    let input = "animation-sort,ascend\ncollision-sort,descend\n";
    let shell = decode(lex(input));
    assert_eq!(shell.animation_sort, Some(SortOrder::Ascend));
    assert_eq!(shell.collision_sort, Some(SortOrder::Descend));

    let unknown = "animation-sort,sideways\ncollision-sort,\n";
    let shell2 = decode(lex(unknown));
    assert_eq!(
        shell2.animation_sort,
        Some(SortOrder::Other("sideways".into()))
    );
    assert_eq!(shell2.collision_sort, None);
    // 大文字は今の照合（完全一致）のまま知らない値。キーが無ければ None。
    let shell3 = decode(lex("collision-sort,Ascend\n"));
    assert_eq!(
        shell3.collision_sort,
        Some(SortOrder::Other("Ascend".into()))
    );
    assert_eq!(shell3.animation_sort, None);
}

/// (d) definitions の登場順（種別間 interleaving）保持。
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
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(
        shell.definitions,
        vec![DefRef::Surface(0), DefRef::Append(0), DefRef::Alias(0),]
    );
}

/// descriptブレスの中の並びの鍵を読む（ukadoc の descriptブレス記述例
/// 「descript { version,1 / collision-sort,ascend / animation-sort,ascend }」）。
/// 後の行が勝ち、トップレベルの行の道も残る。
#[test]
fn sort_keys_inside_descript_brace_are_transcribed() {
    let input = "\
descript
{
version,1
collision-sort,ascend
animation-sort,descend
animation-sort,ascend
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.animation_sort, Some(SortOrder::Ascend));
    assert_eq!(shell.collision_sort, Some(SortOrder::Ascend));

    // ブレスの後のトップレベルの行が勝つ（登場順で後の行が勝つ）。
    let shell = decode(lex(
        "descript\n{\ncollision-sort,ascend\n}\ncollision-sort,descend\n",
    ));
    assert_eq!(shell.collision_sort, Some(SortOrder::Descend));
}
