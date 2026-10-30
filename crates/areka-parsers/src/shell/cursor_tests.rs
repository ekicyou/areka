//! cursorブレスの転記（`parse_cursor_braces`）の単体テスト（要件 1.1・1.5）。
//!
//! 検体ファイルは読まず、テストの中に surfaces.txt の文面を持つ。

use super::{CursorBrace, parse, parse_cursor_braces};

fn line(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|s| s.to_string()).collect()
}

/// cursorブレス 3 つと、surface*ブレス・balloon.*ブレス・descriptブレスの同居。
const WITH_CURSORS: &str = "\
charset,UTF-8

descript
{
version,1
}

sakura.cursor
{
mouseup0,Head,system:hand
mousedown0,Head,grip.cur
}

surface0
{
element0,overlay,surface0.png,0,0
collision0,10,20,30,40,Head
animation0.interval,always
animation0.pattern0,overlay,100,50,0,0
}

kero.cursor
{
mousehover1,Bust,a,b.cur
}

balloon.吹き出し
{
size,200,100
}

char2.cursor
{
mousewheel0,Face,wheel.ani
}
";

/// 上の文面から cursorブレスを除いたもの。
const WITHOUT_CURSORS: &str = "\
charset,UTF-8

descript
{
version,1
}

surface0
{
element0,overlay,surface0.png,0,0
collision0,10,20,30,40,Head
animation0.interval,always
animation0.pattern0,overlay,100,50,0,0
}

balloon.吹き出し
{
size,200,100
}
";

/// 3 つの見出しと本体の行を原文のまま登場順に拾い、他のブレスは拾わない（要件 1.1）。
#[test]
fn transcribes_three_headings_and_lines_verbatim() {
    assert_eq!(
        parse_cursor_braces(WITH_CURSORS),
        vec![
            CursorBrace {
                heading: "sakura.cursor".to_string(),
                lines: vec![
                    line(&["mouseup0", "Head", "system:hand"]),
                    line(&["mousedown0", "Head", "grip.cur"]),
                ],
            },
            CursorBrace {
                heading: "kero.cursor".to_string(),
                lines: vec![line(&["mousehover1", "Bust", "a", "b.cur"])],
            },
            CursorBrace {
                heading: "char2.cursor".to_string(),
                lines: vec![line(&["mousewheel0", "Face", "wheel.ani"])],
            },
        ]
    );
}

/// cursorブレスの無い文面からは何も拾わない。
#[test]
fn text_without_cursor_braces_has_none() {
    assert_eq!(parse_cursor_braces(WITHOUT_CURSORS), vec![]);
    assert_eq!(parse_cursor_braces(""), vec![]);
}

/// 閉じていないブレスは拾わない（前の閉じたブレスは拾う）。
#[test]
fn unclosed_brace_is_not_picked() {
    let text = "sakura.cursor\n{\nmouseup0,Head,a.cur\n}\nkero.cursor\n{\nmouseup0,Bust,b.cur\n";
    assert_eq!(
        parse_cursor_braces(text),
        vec![CursorBrace {
            heading: "sakura.cursor".to_string(),
            lines: vec![line(&["mouseup0", "Head", "a.cur"])],
        }]
    );
}

/// cursorブレスを足しても面・当たり判定・element定義・アニメーションの読み取りは同じ（要件 1.5）。
#[test]
fn image_reader_ignores_cursor_braces() {
    let with = parse(WITH_CURSORS);
    assert_eq!(with, parse(WITHOUT_CURSORS));
    assert_eq!(with.surfaces.len(), 1);
    assert_eq!(with.surfaces[0].elements.len(), 1);
    assert_eq!(with.surfaces[0].collisions.len(), 1);
    assert_eq!(with.surfaces[0].animations.len(), 1);
}
