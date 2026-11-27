//! element定義の語とオプションの並びの原文の檻。

use super::{Element, decode};
use crate::shell::lexer::lex;

fn element_of(row: &str) -> Element {
    let shell = decode(lex(&format!("surface0\n{{\n{row}\n}}\n")));
    shell.surfaces[0].elements[0].clone()
}

/// 語は書かれた原文のまま運ばれ、`base` も `overlay` に化けない。
#[test]
fn element_carries_the_written_word() {
    for word in ["overlay", "base", "replace", "blend-multiply"] {
        let e = element_of(&format!("element0,{word},a.png,1,2"));
        assert_eq!(e.method.as_str(), word);
        assert_eq!(e.options.as_str(), "");
    }
}

/// 座標の後の並びは `,` で継いだ原文のまま運ばれる。
#[test]
fn element_carries_options_after_coordinates_verbatim() {
    let e = element_of("element0,overlay,a.png,1,2,clipping,0,0,10,10");
    assert_eq!(e.options.as_str(), "clipping,0,0,10,10");
    assert_eq!((e.x, e.y), (1, 2));
}
