//! `\j[ID]`（ジャンプ）の読み込みの兄弟テスト（open-external-tags 要件 1.5/1.6）。
//!
//! 検証対象は「意味を読まずに汎用の運び手へ転記する」ことだけである:
//! - `\j[…]` は運搬名 [`JUMP_TAG_CARRIER`] の `GenericCommand` になり、`Raw` へ落ちない。
//! - 引数は字句解析が割った列を記述順のまま運ぶ（ID を第 1 引数として読むのは消費側）。
//! - 裸の `\j` は角括弧つきと別の経路で、今のまま `Raw` である。

use super::{Instruction, JUMP_TAG_CARRIER, parse};

#[test]
fn jump_tag_becomes_generic_command_with_carrier_name() {
    assert_eq!(JUMP_TAG_CARRIER, "\\j");
    assert_eq!(
        parse("\\j[http://a/]"),
        vec![Instruction::GenericCommand {
            name: JUMP_TAG_CARRIER.to_owned(),
            raw_args: vec!["http://a/".to_owned()],
        }]
    );
}

#[test]
fn jump_tag_args_keep_written_order() {
    assert_eq!(
        parse("\\j[a,b]"),
        vec![Instruction::GenericCommand {
            name: JUMP_TAG_CARRIER.to_owned(),
            raw_args: vec!["a".to_owned(), "b".to_owned()],
        }]
    );
}

#[test]
fn jump_tag_is_not_raw() {
    for input in ["\\j[http://a/]", "\\j[OnTest]", "\\j[]"] {
        let out = parse(input);
        assert!(
            !out.iter().any(|i| matches!(i, Instruction::Raw(_))),
            "{input}: Raw へ落ちている: {out:?}"
        );
    }
}

#[test]
fn bare_jump_stays_raw() {
    assert_eq!(parse("\\j"), vec![Instruction::Raw("\\j".to_owned())]);
}
