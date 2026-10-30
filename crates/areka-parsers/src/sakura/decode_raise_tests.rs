//! 裸の `\v`（手前へ出す）の読み込みの兄弟テスト（stay-on-top-directives 要件 1.1・1.8・7.1）。
//!
//! 検証対象は「意味を読まずに汎用の運び手へ転記する」ことだけである:
//! - 裸の `\v` は運搬名 [`RAISE_TAG_CARRIER`] の `GenericCommand`（引数 0 個）になり、
//!   `Raw` へ落ちず、「知らないタグ」の印も付かない。
//! - 角括弧付きの `\v[…]` は正典に無いので腕を作らない（今のまま素通し）。

use super::{Instruction, RAISE_TAG_CARRIER, ReadNote, parse, parse_noted};

#[test]
fn bare_raise_becomes_generic_command_with_carrier_name() {
    assert_eq!(RAISE_TAG_CARRIER, r"\v");
    assert_eq!(
        parse(r"\v"),
        vec![Instruction::GenericCommand {
            name: RAISE_TAG_CARRIER.to_owned(),
            raw_args: Vec::new(),
        }]
    );
}

#[test]
fn bare_raise_has_no_unknown_tag_note() {
    for input in [r"\v", r"\0\v", r"\1\v本文"] {
        let reads = parse_noted(input);
        assert!(
            reads
                .iter()
                .all(|r| !r.notes.contains(&ReadNote::UnknownTag)),
            "{input}: UnknownTag の印が付いた: {reads:?}"
        );
    }
}

#[test]
fn bracketed_raise_has_no_arm() {
    assert!(
        !parse(r"\v[1]").iter().any(
            |i| matches!(i, Instruction::GenericCommand { name, .. } if name == RAISE_TAG_CARRIER)
        ),
        r"角括弧付きの \v[…] が運搬名へ写された"
    );
}
