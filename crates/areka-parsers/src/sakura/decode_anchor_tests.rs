//! `\_a`（アンカー）の読み込みの兄弟テスト（anchor-tag-canon 要件 1.1〜1.6・1.11・6.1・6.2・8.1）。
//!
//! 検証対象は「意味を読まずに 4 つの形を転記する」ことだけである（台本の文字列だけから）:
//! - 角括弧付きの `\_a[…]` は「アンカーの開き」になり、第 1 引数が ID、以降が引数の列
//!   （記述順・空のトークンも潰さない）。`On` 始まりの ID も同じ形で、読み手は区別しない。
//! - 角括弧の無い `\_a` は「アンカーの閉じ」になる。
//! - どの形にも「知らないタグ」「読めなかった引数」の印が付かず、`Raw` へ落ちない。
//!
//! 開きと閉じの対応（閉じ無し・重なり・開いていない閉じ）は読み手の仕事ではないので、
//! ここでは見ない（対応の判定の側のテストが持つ）。

use super::{Anchor, Instruction, parse, parse_noted};

fn open(id: &str, references: &[&str]) -> Instruction {
    Instruction::Anchor(Anchor {
        id: id.to_owned(),
        references: references.iter().map(|s| (*s).to_owned()).collect(),
    })
}

fn text(s: &str) -> Instruction {
    Instruction::Text(s.to_owned())
}

/// 要件 1.1: `\_a[ID]` は ID だけを持つ開き（引数の列は空）。
#[test]
fn id_only_form_is_an_open_without_references() {
    assert_eq!(parse(r"\_a[Hint]"), vec![open("Hint", &[])]);
}

/// 要件 1.2: `\_a[ID,r2,r3,…]` は第 2 引数以降を記述順・綴りのまま持つ。
#[test]
fn references_keep_written_order_and_spelling() {
    assert_eq!(
        parse(r"\_a[ID,r2,r3,２番 目]"),
        vec![open("ID", &["r2", "r3", "２番 目"])]
    );
}

/// 要件 1.3: ID が `On` で始まる形も同じ形で読み、読み手は区別しない。
#[test]
fn on_prefixed_id_is_read_in_the_same_shape() {
    assert_eq!(
        parse(r"\_a[OnTest,r0,r1]"),
        vec![open("OnTest", &["r0", "r1"])]
    );
}

/// 要件 1.4: 角括弧の無い `\_a` は閉じ。直後の本文は飲み込まず、文字も漏れない。
#[test]
fn bracketless_form_is_a_close() {
    assert_eq!(parse(r"\_a"), vec![Instruction::AnchorEnd]);
    assert_eq!(
        parse(r"\_aをクリックする。"),
        vec![Instruction::AnchorEnd, text("をクリックする。")]
    );
}

/// 要件 1.11: `\_a[]` は ID が空の開き（閉じにも素通しにもならない）。
#[test]
fn empty_id_is_still_an_open() {
    assert_eq!(parse(r"\_a[]"), vec![open("", &[])]);
    assert_eq!(parse(r#"\_a[""]"#), vec![open("", &[])]);
}

/// 要件 1.2: 空のトークンは ID の位置でも引数の位置でも潰さない（位置がずれない）。
#[test]
fn empty_tokens_are_kept_in_place() {
    assert_eq!(parse(r"\_a[ID,,x,]"), vec![open("ID", &["", "x", ""])]);
    assert_eq!(parse(r"\_a[,x]"), vec![open("", &["x"])]);
}

/// 要件 1.5: 引数の区切りと引用は `\q[…]` と同じ（同じ角括弧の中身を両方に読ませて比べる）。
#[test]
fn separators_and_quotes_match_the_choice_tag() {
    for inner in [
        r"a,b,c",
        r#""a,b",ID,r"#,
        r#"x,"say ""hi""",r"#,
        r"a\],b",
        r"a,b,,c,",
        r" a , b ",
        r#""",b,"""#,
    ] {
        let anchor = match parse(&format!(r"\_a[{inner}]")).as_slice() {
            [Instruction::Anchor(a)] => {
                let mut all = vec![a.id.clone()];
                all.extend(a.references.iter().cloned());
                all
            }
            other => panic!("開きになっていない: {other:?} in {inner:?}"),
        };
        let choice = match parse(&format!(r"\q[{inner}]")).as_slice() {
            [Instruction::Choice(c)] => {
                let mut all = vec![c.disp.clone(), c.target.clone()];
                all.extend(c.references.iter().cloned());
                all
            }
            other => panic!("選択肢になっていない: {other:?} in {inner:?}"),
        };
        assert_eq!(anchor, choice, "inner: {inner:?}");
    }
}

/// 要件 1.6・6.1・6.2: 4 つの形と空の ID のどれにも印が付かず、`Raw`（知らないタグ）にも
/// 汎用コマンド（誰も拾わない命令の候補）にもならない。位置は綴りの全体を覆う。
#[test]
fn no_form_is_noted_raw_or_generic() {
    let s = r"\_a[ID]あ\_a\_a[ID,r2,r3]い\_a\_a[OnID,r0]う\_a\_a[]え\_a";
    let reads = parse_noted(s);
    for r in &reads {
        assert!(r.notes.is_empty(), "印が付いた: {r:?}");
        assert!(
            !matches!(
                r.instruction,
                Instruction::Raw(_) | Instruction::GenericCommand { .. }
            ),
            "読み捨て／汎用コマンドへ落ちた: {r:?}"
        );
    }
    let anchors: Vec<&str> = reads
        .iter()
        .filter(|r| {
            matches!(
                r.instruction,
                Instruction::Anchor(_) | Instruction::AnchorEnd
            )
        })
        .map(|r| &s[r.span.clone()])
        .collect();
    assert_eq!(
        anchors,
        [
            r"\_a[ID]",
            r"\_a",
            r"\_a[ID,r2,r3]",
            r"\_a",
            r"\_a[OnID,r0]",
            r"\_a",
            r"\_a[]",
            r"\_a",
        ]
    );
}
