//! 箱の転記（`parse_boxes`）の単体テスト（要件 1.1・1.7・2.1・2.8）。
//!
//! 検体ファイルは読まず、テストの中に surfaces.txt の文面を持つ。

use super::{
    AppendTarget, BoxBrace, BoxDefinition, BoxElementLine, BoxSurfaceLines, Element, ElementPath,
    parse, parse_boxes,
};

fn line(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|s| s.to_string()).collect()
}

fn element(element: &str, name: &str, x: &str, y: &str) -> BoxElementLine {
    BoxElementLine {
        element: element.to_string(),
        name: name.to_string(),
        x: x.to_string(),
        y: y.to_string(),
    }
}

/// 箱を含む文面（ブレス・surface・append・画像の element定義の同居）。
const WITH_BOXES: &str = "\
charset,UTF-8

balloon.吹き出し
{
size,200,100
font.follow,scope
font.color,255,0,0
}

surface0
{
element0,overlay,surface0.png,0,0
element1,balloon,吹き出し,10,20
element2,overlay,face.png,5,6
}

surface1,3-4
{
element0,overlay,surface1.png,0,0
}

surface.append0-1
{
element5,balloon,吹き出し,30,40
}

descript
{
version,1
}
";

/// 上の文面から箱に関わる行（`balloon.*`ブレスと描画メソッド `balloon` の行）を除いたもの。
const WITHOUT_BOXES: &str = "\
charset,UTF-8

surface0
{
element0,overlay,surface0.png,0,0
element2,overlay,face.png,5,6
}

surface1,3-4
{
element0,overlay,surface1.png,0,0
}

surface.append0-1
{
}

descript
{
version,1
}
";

/// ブレスの名前と本体、両方のブレスの見出しと箱の element定義が登場順に原文どおり並ぶ（要件 1.1・2.1）。
#[test]
fn transcribes_brace_and_surface_lines_in_order() {
    let boxes = parse_boxes(WITH_BOXES);
    assert_eq!(
        boxes.definitions,
        vec![
            BoxDefinition::Brace(BoxBrace {
                name: "吹き出し".to_string(),
                lines: vec![
                    line(&["size", "200", "100"]),
                    line(&["font.follow", "scope"]),
                    line(&["font.color", "255", "0", "0"]),
                ],
            }),
            BoxDefinition::Surface(BoxSurfaceLines {
                targets: vec![AppendTarget::Single(0)],
                elements: vec![element("1", "吹き出し", "10", "20")],
            }),
            BoxDefinition::Surface(BoxSurfaceLines {
                targets: vec![
                    AppendTarget::Single(1),
                    AppendTarget::Range { start: 3, end: 4 },
                ],
                elements: vec![],
            }),
            BoxDefinition::Append(BoxSurfaceLines {
                targets: vec![AppendTarget::Range { start: 0, end: 1 }],
                elements: vec![element("5", "吹き出し", "30", "40")],
            }),
        ]
    );
}

/// 欠けた欄は空文字列、数として読めない欄も文字列のまま転記する（検証しない・失敗しない）。
#[test]
fn missing_fields_become_empty_strings() {
    let text = "\
surface7
{
element3,balloon,a
element4,balloon
elementX,balloon,b,-1,abc
}
";
    assert_eq!(
        parse_boxes(text).definitions,
        vec![BoxDefinition::Surface(BoxSurfaceLines {
            targets: vec![AppendTarget::Single(7)],
            elements: vec![
                element("3", "a", "", ""),
                element("4", "", "", ""),
                element("X", "b", "-1", "abc"),
            ],
        })]
    );
}

/// 箱の無い文面では `Brace` が 0 件、各ブレスの箱の element定義は空（設計の Postconditions）。
#[test]
fn text_without_boxes_has_no_brace() {
    let boxes = parse_boxes(WITHOUT_BOXES);
    assert!(
        boxes
            .definitions
            .iter()
            .all(|d| !matches!(d, BoxDefinition::Brace(_)))
    );
    assert_eq!(
        boxes.definitions,
        vec![
            BoxDefinition::Surface(BoxSurfaceLines {
                targets: vec![AppendTarget::Single(0)],
                elements: vec![],
            }),
            BoxDefinition::Surface(BoxSurfaceLines {
                targets: vec![
                    AppendTarget::Single(1),
                    AppendTarget::Range { start: 3, end: 4 },
                ],
                elements: vec![],
            }),
            BoxDefinition::Append(BoxSurfaceLines {
                targets: vec![AppendTarget::Range { start: 0, end: 1 }],
                elements: vec![],
            }),
        ]
    );
    assert_eq!(parse_boxes("").definitions, vec![]);
}

/// 同じ文面からは同じ結果。
#[test]
fn transcription_is_deterministic() {
    assert_eq!(parse_boxes(WITH_BOXES), parse_boxes(WITH_BOXES));
}

/// 箱を含む文面でも、画像の読み手の結果は箱の行を除いた文面と同じ（要件 1.7・2.8）。
/// 画像の element は `overlay` と `base` の行だけで、描画メソッド `balloon` の行は画像に入らない。
#[test]
fn image_reader_ignores_box_lines() {
    let with = parse(WITH_BOXES);
    assert_eq!(with, parse(WITHOUT_BOXES));
    assert_eq!(
        with.surfaces[0].elements,
        vec![
            Element {
                layer: 0,
                path: ElementPath::new("surface0.png".to_string()),
                x: 0,
                y: 0,
            },
            Element {
                layer: 2,
                path: ElementPath::new("face.png".to_string()),
                x: 5,
                y: 6,
            },
        ]
    );
    assert!(with.appends[0].elements.is_empty());
}
