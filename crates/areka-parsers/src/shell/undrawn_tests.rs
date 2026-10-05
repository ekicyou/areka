//! 描けない行の転記（`parse_undrawn_elements`）の単体テスト
//! （areka-P0-element-base-method 要件 2.1・2.2・2.6・4.2）。
//!
//! 検体ファイルは読まず、テストの中に surfaces.txt の文面を持つ。

use super::{BoxDefinition, UndrawnElementLine, parse, parse_boxes, parse_undrawn_elements};

fn undrawn(heading: &str, element: &str, method: &str) -> UndrawnElementLine {
    UndrawnElementLine {
        heading: heading.to_string(),
        element: element.to_string(),
        method: method.to_string(),
    }
}

/// 描けない語（`replace`・空の欄・欄が無い・ukadoc に無い語・大文字・`add`・`bind`）が、
/// 描ける語の隣の行と混ざっていても 1 行 1 件で見出し・element番号・語つきで返る。
#[test]
fn each_undrawable_word_is_one_line_with_heading_element_and_method() {
    let text = "\
surface3
{
element0,base,body.png,0,0
element1,replace,a.png,0,0
element2,overlay,face.png,5,6
element3,,b.png,0,0
element4,balloon,吹き出し,10,20
element5,zoom,c.png,0,0
element6,Overlay,d.png,0,0
element7,add,e.png,0,0
element8,bind,f.png,0,0
element9
}
";
    assert_eq!(
        parse_undrawn_elements(text),
        vec![
            undrawn("surface3", "1", "replace"),
            undrawn("surface3", "3", ""),
            undrawn("surface3", "5", "zoom"),
            undrawn("surface3", "6", "Overlay"),
            undrawn("surface3", "7", "add"),
            undrawn("surface3", "8", "bind"),
            undrawn("surface3", "9", ""),
        ]
    );
}

/// 複数の番号を並べた見出し・範囲の見出しでも 1 行は 1 件で、見出しは原文のまま（展開しない）。
#[test]
fn multi_number_headings_give_one_entry_per_line_with_heading_verbatim() {
    let text = "\
surface0,1
{
element0,overlay,surface0.png,0,0
element1,replace,x.png,0,0
}

surface.append0-1
{
element2,add,y.png,0,0
}

surface.append10,2100-2110,!2105
{
element0,bind,z.png,0,0
}
";
    assert_eq!(
        parse_undrawn_elements(text),
        vec![
            undrawn("surface0,1", "1", "replace"),
            undrawn("surface.append0-1", "2", "add"),
            undrawn("surface.append10,2100-2110,!2105", "0", "bind"),
        ]
    );
}

/// 数字でない element番号・先頭に 0 のある番号は原文のまま。
#[test]
fn element_number_is_kept_verbatim() {
    let text = "\
surface0
{
elementface,replace,x.png,0,0
element01,replace,y.png,0,0
element,replace,z.png,0,0
}
";
    assert_eq!(
        parse_undrawn_elements(text),
        vec![
            undrawn("surface0", "face", "replace"),
            undrawn("surface0", "01", "replace"),
            undrawn("surface0", "", "replace"),
        ]
    );
}

/// 描ける語（`overlay`・`base`・`balloon`）だけの文面では空（要件 2.6）。
#[test]
fn drawable_words_only_give_nothing() {
    let text = "\
balloon.吹き出し
{
size,200,100
}

surface0
{
element0,base,surface0.png,0,0
element1,overlay,face.png,5,6
element2,balloon,吹き出し,10,20
}

surface.append0
{
element3,balloon,吹き出し,30,40
element4,base,arm.png,0,0
}
";
    assert_eq!(parse_undrawn_elements(text), Vec::new());
}

/// `surface*`・`surface.append*` でないブレスの中・ブレスの外・閉じずに終わるブレスの中の
/// `element` の行は対象にしない。
#[test]
fn lines_outside_surface_braces_are_not_targets() {
    let text = "\
element0,replace,top.png,0,0

descript
{
element0,replace,d.png,0,0
}

balloon.吹き出し
{
element0,replace,b.png,0,0
}

kero.surface.alias
{
element0,replace,k.png,0,0
}

sakura.surface.alias
{
element0,replace,s.png,0,0
}

unknown.block.head
{
element0,replace,u.png,0,0
}

surface5
{
element0,replace,open.png,0,0
";
    assert_eq!(parse_undrawn_elements(text), Vec::new());
}

/// 3 つの転記（画像の読み手・箱の転記・描けない行の転記）が、`surface*`・`surface.append*`
/// ブレスの中の `element` の行を漏れなく重なりなく分ける。
#[test]
fn three_transcriptions_partition_element_lines_exactly() {
    let text = "\
charset,UTF-8
element0,replace,top.png,0,0

descript
{
version,1
element0,overlay,d.png,0,0
}

balloon.吹き出し
{
size,200,100
element0,replace,b.png,0,0
}

kero.surface.alias
{
element0,replace,k.png,0,0
}

unknown.block.head
{
element0,base,u.png,0,0
}

surface0
{
element0,base,surface0.png,0,0
element1,overlay,face.png,5,6
element2,balloon,吹き出し,10,20
element3,replace,r.png,0,0
element4,,e.png,0,0
element5,Overlay,o.png,0,0
element6,add,a.png,0,0
element7,bind,b.png,0,0
element8,zoom,z.png,0,0
element9
collision0,0,0,10,10,Head
}

surface.append0
{
element10,base,arm.png,0,0
element11,overlay,hand.png,0,0
element12,balloon,吹き出し,30,40
element13,replace,r2.png,0,0
}

surface7
{
element0,overlay,open.png,0,0
element1,replace,open2.png,0,0
";
    // 手で数えた `surface0`・`surface.append0` の中の `element` で始まる行（14 行）。
    const ELEMENT_LINES: usize = 14;

    let shell = parse(text);
    let images: usize = shell
        .surfaces
        .iter()
        .map(|s| s.elements.len())
        .sum::<usize>()
        + shell
            .appends
            .iter()
            .map(|a| a.elements.len())
            .sum::<usize>();
    let boxes: usize = parse_boxes(text)
        .definitions
        .iter()
        .map(|d| match d {
            BoxDefinition::Surface(s) | BoxDefinition::Append(s) => s.elements.len(),
            BoxDefinition::Brace(_) => 0,
        })
        .sum();
    let undrawn = parse_undrawn_elements(text).len();

    // 較正: 3 つのどれもが 0 でない（どれか 1 つが全部を拾う形では成り立たない）。
    assert_eq!((images, boxes, undrawn), (4, 2, 8));
    assert_eq!(images + boxes + undrawn, ELEMENT_LINES);
}
