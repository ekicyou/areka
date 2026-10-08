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
    // 印の型が公開の道から見え、`Read::notes` の要素であること。
    let reads = parse_noted("やあ");
    let notes: &Vec<ReadNote> = &reads[0].notes;
    assert!(notes.is_empty());
}

// ───────────────────────────────────────────────────────────────────
// 印（mcp-author-tools 要件 3.2・3.4・3.10・3.11）
//
// 印は読む段が自分で下した扱いだけを言う。各テストは「印の付いた命令の綴りと印」の
// 全部を比べる（付くべき命令にだけ付き、他の命令には付かない）。
// ───────────────────────────────────────────────────────────────────

use ReadNote::{ArgumentDefaulted, MarkerIgnored, Unclosed, UnknownTag};

/// 印の付いた命令だけを、綴りと印の組で返す。
fn noted(s: &str) -> Vec<(&str, Vec<ReadNote>)> {
    parse_noted(s)
        .into_iter()
        .filter(|r| !r.notes.is_empty())
        .map(|r| (&s[r.span], r.notes))
        .collect()
}

/// 印がどの命令にも付かないことを確かめる。
fn assert_no_notes(scripts: &[&str]) {
    for s in scripts {
        let got = noted(s);
        assert!(got.is_empty(), "{got:?} in {s:?}");
    }
}

#[test]
fn unknown_tag_is_noted_on_spellings_without_an_arm() {
    for (s, spelling) in [
        // 素通しの素の綴り（角括弧なし）。
        (r"前\x後", r"\x"),
        (r"\_a本文", r"\_a"),
        (r"末尾の\", r"\"),
        // 素通しのタグ（角括弧つき）。
        (r"\i[5]", r"\i[5]"),
        (r"\&[amp]", r"\&[amp]"),
        (r"\w[2]", r"\w[2]"),
        (r"\q*[ID]", r"\q*[ID]"),
        // 旧い 2 連の `\q`（畳んだ全部に 1 つ）。
        (r"\q[ID][題]\e", r"\q[ID][題]"),
        (r"前\q*[ID][題]", r"\q*[ID][題]"),
    ] {
        assert_eq!(noted(s), [(spelling, vec![UnknownTag])], "script: {s:?}");
    }
}

#[test]
fn unknown_tag_is_not_noted_on_spellings_with_an_arm() {
    assert_no_notes(&[
        r"\0\h\1\u\e\c\-\n",
        r"\w5\b2\p3",
        r"\s[0]\b[1]\_l[1,2]",
        r"\q[題,ID]",
        r"\![raise,a]\![move,1,2]\![*x]",
        r"\f[sub,1]\f\+\_+",
        r"前\\後\%名%username",
    ]);
}

#[test]
fn unclosed_is_noted_where_bracket_or_quote_does_not_close() {
    for (s, spelling) in [
        (r"本文\s[0", r"\s[0"),
        (r#"\![raise,"a"#, r#"\![raise,"a"#),
        (r#"\![raise,"a]"#, r#"\![raise,"a]"#),
        // 腕の無い綴りでも、閉じていなければ閉じていないの印だけ。
        (r"\x[5", r"\x[5"),
    ] {
        assert_eq!(noted(s), [(spelling, vec![Unclosed])], "script: {s:?}");
    }
}

#[test]
fn unclosed_is_not_noted_when_closed() {
    assert_no_notes(&[
        r"\s[0]",
        r#"\![raise,"a"]"#,
        r#"\![raise,"a,b",c]"#,
        r"\s[\]]",
    ]);
}

#[test]
fn argument_defaulted_is_noted_where_reading_falls_back() {
    for s in [
        // 待ち: 引数なし・非数。
        r"\_w[abc]",
        r"\_w[]",
        r"\_w[-5]",
        // 改行の比: 引数が在って `half` でも数でもない。
        r"\n[abc]",
        r#"\n[""]"#,
        // 話者のスコープ: 引数なし・非数。
        r"\p[x]",
        r"\p[]",
        // 選択肢: 引数が 2 つ未満。
        r"\q[題]",
        r"\q[]",
    ] {
        assert_eq!(noted(s), [(s, vec![ArgumentDefaulted])], "script: {s:?}");
    }
}

#[test]
fn argument_defaulted_is_not_noted_when_read_or_carried_downstream() {
    assert_no_notes(&[
        r"\_w[500]",
        r"\n[half]\n[150]\n[-50]",
        r"\p[1]",
        r"\q[題,ID]\q[題,ID,r0]",
        // 素の値と同じ・下流へ運ぶもの（`\n[]`・`\_l` の欠けた座標・`\s[]`・`\b[]`）。
        r"\n[]\n",
        r"\_l[1]\_l[]",
        r"\s[]\b[]",
    ]);
}

#[test]
fn marker_ignored_is_noted_on_both_branches_of_the_choice_marker() {
    // 単独。
    assert_eq!(noted(r"\![*]単独"), [(r"\![*]", vec![MarkerIgnored])]);
    // `\q` に畳む（畳んだ命令 1 つに付く）。
    assert_eq!(
        noted(r"前\![*]\q[題,ID]後"),
        [(r"\![*]\q[題,ID]", vec![MarkerIgnored])]
    );
    // 畳んだ `\q` の引数が足りなければ、既定へ落とした印も同じ命令に付く。
    assert_eq!(
        noted(r"\![*]\q[題]"),
        [(r"\![*]\q[題]", vec![MarkerIgnored, ArgumentDefaulted])]
    );
}

#[test]
fn marker_ignored_is_not_noted_without_the_marker() {
    assert_no_notes(&[r"\q[題,ID]", r"\![raise,*]", r"\![*,x]", r"*"]);
}

#[test]
fn raw_iff_unknown_tag_or_unclosed() {
    let extra: &[&str] = &[
        r"\x\_a\__q\i[5]\&[amp]\w[2]\q*[ID]",
        r"\q[ID][題]\e\q*[ID][題]\q[題]",
        r"\_w[abc]\n[abc]\p[x]\![*]\![*]\q[題,ID]",
        r#"\![raise,"a"#,
        r"\x[5",
        r"\s[]\b[]\_l[]\n[]",
        r"末尾の\",
    ];
    for s in SCRIPTS.iter().chain(extra) {
        for r in parse_noted(s) {
            let is_raw = matches!(r.instruction, Instruction::Raw(_));
            let has = r.notes.contains(&UnknownTag) || r.notes.contains(&Unclosed);
            assert_eq!(is_raw, has, "{r:?} in {s:?}");
        }
    }
}

/// 字句は `w`／`b`／`p` の短縮形しか作らないので、想定外の短縮形の腕は台本からは届かない。
/// トークンを直接組んで本体へ渡し、この腕でも `Raw` と知らないタグの印が揃うことを固定する。
#[test]
fn unexpected_shorthand_arm_is_raw_with_unknown_tag() {
    use crate::sakura::{decode::decode_noted, lexer::Token};
    let reads = decode_noted(vec![(Token::Shorthand { word: 'x', n: 5 }, 0..3)]);
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].instruction, Instruction::Raw(r"\x5".to_string()));
    assert_eq!(reads[0].span, 0..3);
    assert_eq!(reads[0].notes, vec![UnknownTag]);
}
