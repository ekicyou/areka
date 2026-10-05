//! `parse_noted`（位置と印つきの入口）の単体テスト。
//!
//! 公開の道（`crate::sakura::…`）から引き、次を固定する（mcp-author-tools 要件 3.1・3.11）:
//! - `parse(s)` と `parse_noted(s)` の命令の列が等しい（経路は 1 本）。
//! - `span` は入力の順に並び、重ならず、文字の境界に在り、`&input[span]` が該当の綴り。
//! - 隣り合うトークンを畳んだ命令（`\![*]`＋`\q[...]`・旧い 2 連の `\q[ID][題]`）の
//!   `span` は、畳んだ全部を覆う。

use crate::sakura::{Instruction, Read, ReadNote, parse, parse_noted};

/// 形の違う台本の見本（エスケープ・日本語・未閉じ・畳む命令・短縮形・環境変数を含む）。
const SCRIPTS: &[&str] = &[
    "",
    "こんにちは",
    r"\0\s[0]やあ\w5\_w[300]\n[half]\e",
    r"\h\s[10]こんにちは、\u\s[11]世界\n\1\b2さよなら\e",
    r"前\\後\%名%username末",
    r"\![*]\q[はい,OnYes]\![*]\q[いいえ,OnNo]",
    r"\q[ID][題]\eあと",
    r"\q*[ID][題]",
    r"\![*]単独",
    r"\x\i[5]\&[amp]\w[2]",
    r"本文\s[0",
    r#"\![raise,"a,b",c]\![move,1,2]"#,
    r"\f[color,red]色\f[default]\+\_+\p3\_l[1,2]",
    r"末尾の\",
];

fn slices<'a>(input: &'a str, reads: &[Read]) -> Vec<&'a str> {
    reads.iter().map(|r| &input[r.span.clone()]).collect()
}

#[test]
fn parse_equals_instructions_of_parse_noted() {
    for s in SCRIPTS {
        let noted: Vec<Instruction> = parse_noted(s).into_iter().map(|r| r.instruction).collect();
        assert_eq!(parse(s), noted, "script: {s:?}");
    }
}

#[test]
fn spans_are_ordered_disjoint_and_on_char_boundaries() {
    for s in SCRIPTS {
        let reads = parse_noted(s);
        let mut prev_end = 0;
        for r in &reads {
            assert!(r.span.start < r.span.end, "empty span {r:?} in {s:?}");
            assert!(
                prev_end <= r.span.start,
                "overlap/out of order {r:?} in {s:?}"
            );
            assert!(s.is_char_boundary(r.span.start), "{r:?} in {s:?}");
            assert!(s.is_char_boundary(r.span.end), "{r:?} in {s:?}");
            assert!(r.span.end <= s.len(), "{r:?} in {s:?}");
            prev_end = r.span.end;
        }
    }
}

#[test]
fn span_slices_are_the_spelling_of_each_instruction_with_japanese() {
    let s = r"こんにちは\s[0]世界\_w[300]%username\x\\終";
    assert_eq!(
        slices(s, &parse_noted(s)),
        [
            "こんにちは",
            r"\s[0]",
            "世界",
            r"\_w[300]",
            "%username",
            r"\x",
            r"\\終"
        ]
    );
}

#[test]
fn unclosed_span_runs_to_end_of_input() {
    let s = r"本文\s[0";
    let reads = parse_noted(s);
    assert_eq!(slices(s, &reads), ["本文", r"\s[0"]);
    assert!(matches!(reads[1].instruction, Instruction::Raw(_)));
}

#[test]
fn folded_choice_marker_span_covers_marker_and_q() {
    let s = r"前\![*]\q[はい,OnYes]後";
    let reads = parse_noted(s);
    assert_eq!(slices(s, &reads), ["前", r"\![*]\q[はい,OnYes]", "後"]);
    assert!(matches!(reads[1].instruction, Instruction::Choice(_)));
}

#[test]
fn lone_choice_marker_span_is_marker_only() {
    let s = r"\![*]単独";
    assert_eq!(slices(s, &parse_noted(s)), [r"\![*]", "単独"]);
}

#[test]
fn folded_legacy_q_span_covers_both_brackets() {
    for (s, folded) in [
        // 浮く `[...]` が次のタグの手前で切れるときだけ畳まれる（既存の規則）。
        (r"\q[ID][題]\e", r"\q[ID][題]"),
        (r"前\q*[ID][題]", r"\q*[ID][題]"),
    ] {
        let reads = parse_noted(s);
        let got = slices(s, &reads);
        assert!(got.contains(&folded), "{got:?} for {s:?}");
        let r = reads.iter().find(|r| &s[r.span.clone()] == folded).unwrap();
        assert!(matches!(r.instruction, Instruction::Raw(_)));
    }
}

#[test]
fn notes_are_read_notes() {
    // 印の型が公開の道から見え、`Read::notes` の要素であること（印を付ける規則は 1.2）。
    let reads = parse_noted("やあ");
    let notes: &Vec<ReadNote> = &reads[0].notes;
    assert!(notes.is_empty());
    let _all = [
        ReadNote::UnknownTag,
        ReadNote::Unclosed,
        ReadNote::ArgumentDefaulted,
        ReadNote::MarkerIgnored,
    ];
}
