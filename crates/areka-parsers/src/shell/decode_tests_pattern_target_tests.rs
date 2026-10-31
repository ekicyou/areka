//! pattern定義の欄 2 の原文の檻（seriko-pattern-calls タスク 1.1・要件 2.2・2.3・2.6・2.7・9.2）。
//!
//! 見るのは `decode_animations` の pattern定義の枝の判断の分岐だけ: 欄 2 が `(` で始まり
//! `)` を含まないときだけ、`)` を含む欄（無ければ行の終わり）までを `,` で継ぎ直して
//! `target` にし、待ち・X・Y をその次の欄から読むこと。`[ ]` の中のカンマは字句の段で
//! 守られるので継ぎ直さない。並びでない語の行の読みは変えない。期待値はリテラル直書き。

use super::{DrawMethod, Pattern, decode};
use crate::shell::lexer::lex;

/// `animation0.pattern0,<rest>` の 1 行だけを持つ面を読み、その pattern を返す。
fn pattern_of(rest: &str) -> Pattern {
    let input = format!("surface0\n{{\nanimation0.interval,bind\nanimation0.pattern0,{rest}\n}}\n");
    let shell = decode(lex(&input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 1);
    shell.surfaces[0].animations[0].patterns[0].clone()
}

fn expected(method: &str, surface_id: i64, target: &str, wait: u32, x: i64, y: i64) -> Pattern {
    Pattern {
        index: 0,
        method: DrawMethod::new(method.to_string()),
        surface_id,
        target: target.to_string(),
        wait,
        x,
        y,
    }
}

/// `(0,1)` は 1 つの原文に継ぎ直し、待ち・X・Y はその次から読む（要件 2.2）。
#[test]
fn paren_list_is_rejoined_and_tail_shifts() {
    assert_eq!(
        pattern_of("start,(0,1),50,3,4"),
        expected("start", 0, "(0,1)", 50, 3, 4)
    );
}

/// 欄の前後の空白は字句の段で落ちる。継ぎ直しは `,` だけで繋ぐ（要件 2.2）。
#[test]
fn spaced_paren_list_is_rejoined_with_trimmed_fields() {
    assert_eq!(
        pattern_of("insert,( 0 , 1 ),50,3,4"),
        expected("insert", 0, "( 0,1 )", 50, 3, 4)
    );
}

/// `(0.1)` は欄の中に `)` があるので継ぎ直さない（要件 2.2）。
#[test]
fn dotted_paren_list_stays_one_field() {
    assert_eq!(
        pattern_of("start,(0.1),50,3,4"),
        expected("start", 0, "(0.1)", 50, 3, 4)
    );
}

/// `[ ]` の中のカンマは字句の段で守られ、`[0,1]`・`[0.1]` はそのまま 1 欄（要件 2.2）。
#[test]
fn bracket_lists_stay_one_field() {
    assert_eq!(
        pattern_of("start,[0,1],50,3,4"),
        expected("start", 0, "[0,1]", 50, 3, 4)
    );
    assert_eq!(
        pattern_of("start,[0.1],50,3,4"),
        expected("start", 0, "[0.1]", 50, 3, 4)
    );
}

/// 閉じない `(0` は行の終わりまでを継ぎ直し、待ち・X・Y は欠けて 0（要件 2.3・9.2）。
#[test]
fn unclosed_paren_takes_rest_of_line() {
    assert_eq!(
        pattern_of("start,(0,50,3,4"),
        expected("start", 0, "(0,50,3,4", 0, 0, 0)
    );
}

/// 欄が欠けた行でも失敗せず、欠けた待ち・X・Y は 0（要件 2.3・9.2）。
#[test]
fn missing_tail_fields_default_to_zero() {
    assert_eq!(
        pattern_of("alternativestart,(0,1)"),
        expected("alternativestart", 0, "(0,1)", 0, 0, 0)
    );
}

/// 並びでない語の行は今の読みのまま。欄 2 の原文を `target` にも写す（要件 2.6・2.7）。
#[test]
fn plain_overlay_row_is_unchanged() {
    assert_eq!(
        pattern_of("overlay,10,50,0,0"),
        expected("overlay", 10, "10", 50, 0, 0)
    );
}
