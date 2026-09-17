//! タグ名の終端位置（語境界）を字句層で固定するテスト。
//!
//! 対象は `lex` の出力トークン列のみ（意味付けは `decode` の領分ゆえここでは見ない）。
//! 固定するのは、タグ名が綴りによらず固定長規律（`\` ＋ 1 文字・`\_` ＋ 1 文字・
//! `\__` ＋ 1 文字）で終端し、その後ろの本文に半角 `[` があっても角括弧経路へ入らないこと
//! （要件 1.5〜1.8・3.2・5.1〜5.6・5.8）。あわせて、タグの直後に `[` が来る形・短縮形・
//! 旧仕様の選択肢 `\q*[…]` など、本仕様で変わってはならない形を固定する（要件 4.4・4.9〜4.11）。
//!
//! エスケープ・クォート・引数分割・`\_` の固定長規律は `lexer_bare_tag_tests.rs` と
//! `lexer_tests.rs` が既に固定しているため、ここでは重複させない。
//! すべて決定論（`#[test]`・時計／GPU／実機に依存しない・要件 5.10）。

use super::{Token, lex};

// ───────────────────────────────────────────────────────────────────
// 期待値の組み立て補助（本ファイル内のみで使う）。
// ───────────────────────────────────────────────────────────────────

fn bare(spelling: &str) -> Token {
    Token::Bare(spelling.to_string())
}

fn text(s: &str) -> Token {
    Token::Text(s.to_string())
}

fn sysvar(keyword: &str) -> Token {
    Token::SysVar(keyword.to_string())
}

fn raw(s: &str) -> Token {
    Token::Raw(s.to_string())
}

fn tag(word: &str, args: &[&str]) -> Token {
    Token::Tag {
        word: word.to_string(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
    }
}

fn shorthand(word: char, n: u8) -> Token {
    Token::Shorthand { word, n }
}

// ───────────────────────────────────────────────────────────────────
// L1〜L3: 3 形のタグ名の全件 × 本文中の閉じた `[…]`（要件 1.5・5.1〜5.3）。
// ───────────────────────────────────────────────────────────────────

/// 正典の 1 文字タグ名のうち `\_` を除く 30 綴り。`\_` の直後に本文が続く形は
/// 2 文字形 `\_X` として読まれるため、1 文字の綴りとしては構成できない。
const ONE_CHAR_WORDS: [&str; 30] = [
    "-", "0", "1", "4", "5", "6", "7", "8", "C", "!", "&", "*", "+", "a", "b", "c", "e", "f", "i",
    "j", "m", "n", "p", "q", "s", "t", "v", "w", "x", "z",
];

/// 正典の `\_` 始まりのタグ名 19 綴り（2 文字形 14・3 文字形 5）。
const UNDERSCORE_WORDS: [&str; 19] = [
    "_!", "_+", "_?", "_V", "_a", "_b", "_l", "_m", "_n", "_q", "_s", "_u", "_v", "_w", "__c",
    "__q", "__t", "__v", "__w",
];

/// L1（要件 5.1）: 1 文字タグ名 30 綴りのすべてで、タグ名は 1 文字で終端し、
/// 本文中の `[注]` は本文のまま残る。
#[test]
fn one_char_tag_ends_before_body_containing_brackets() {
    for w in ONE_CHAR_WORDS {
        let input = format!(r"\{w}本文[注]です");
        assert_eq!(
            lex(&input),
            vec![bare(w), text("本文[注]です")],
            "spelling: \\{w}"
        );
    }
}

/// L2（要件 1.3・5.2）: 短縮形の対象語 3 個も、直後が数字でなければ 1 文字で終端する。
#[test]
fn shorthand_words_without_digit_end_before_body_containing_brackets() {
    assert_eq!(
        lex(r"\wテキスト[注]です"),
        vec![bare("w"), text("テキスト[注]です")]
    );
    assert_eq!(
        lex(r"\bテキスト[注]です"),
        vec![bare("b"), text("テキスト[注]です")]
    );
    assert_eq!(
        lex(r"\pテキスト[注]です"),
        vec![bare("p"), text("テキスト[注]です")]
    );
}

/// L3（要件 1.5・5.3）: `\_` 始まりの 19 綴りのすべてで、タグ名は固定長で終端し、
/// 本文中の `[注]` は本文のまま残る。
#[test]
fn underscore_tag_ends_before_body_containing_brackets() {
    for w in UNDERSCORE_WORDS {
        let input = format!(r"\{w}文字[注]を");
        assert_eq!(
            lex(&input),
            vec![bare(w), text("文字[注]を")],
            "spelling: \\{w}"
        );
    }
}

// ───────────────────────────────────────────────────────────────────
// L4〜L7: 本文側の `[` のさまざまな形（要件 1.6〜1.8・5.4・5.8）。
// ───────────────────────────────────────────────────────────────────

/// L4（要件 5.4）: 本文中の未閉じ `[` は残りを吸収せず、3 形とも本文のまま残る。
#[test]
fn unclosed_bracket_in_body_stays_text_for_all_three_forms() {
    assert_eq!(
        lex(r"\1テキスト[注です。まだ続く台詞"),
        vec![bare("1"), text("テキスト[注です。まだ続く台詞")]
    );
    assert_eq!(
        lex(r"\_qテキスト[注です"),
        vec![bare("_q"), text("テキスト[注です")]
    );
    assert_eq!(
        lex(r"\__qテキスト[注です"),
        vec![bare("__q"), text("テキスト[注です")]
    );
}

/// L5（要件 1.6・5.8）: 本文が半角英数だけでも、タグ名は 1 文字で終端する。
#[test]
fn ascii_only_body_does_not_extend_tag_name() {
    assert_eq!(lex(r"\0file[1].txt"), vec![bare("0"), text("file[1].txt")]);
}

/// L6（要件 1.7・5.8）: 本文中の `[` が複数あっても、`]` が単独で現れても本文のまま。
#[test]
fn multiple_or_stray_brackets_in_body_stay_text() {
    assert_eq!(
        lex(r"\eあ[1]い[2]う"),
        vec![bare("e"), text("あ[1]い[2]う")]
    );
    assert_eq!(lex(r"\eあ]い"), vec![bare("e"), text("あ]い")]);
}

/// L7（要件 1.8・5.8）: 本文中の `[` の後に別のタグやシステム変数が続いても、
/// それぞれが独立した単位として読まれる。
#[test]
fn tags_and_sysvars_after_body_bracket_are_separate_units() {
    assert_eq!(
        lex(r"\1青い[1]ノート\nさん%username"),
        vec![
            bare("1"),
            text("青い[1]ノート"),
            bare("n"),
            text("さん"),
            sysvar("username"),
        ]
    );
}

// ───────────────────────────────────────────────────────────────────
// L8〜L11: 変わってはならない形（要件 3.2・4.4・4.9・4.10・5.5・5.6）。
// ───────────────────────────────────────────────────────────────────

/// L8（要件 4.10・5.5）: タグの直後に `[` が来る形は従来どおり角括弧形として読まれる。
#[test]
fn bracket_immediately_after_tag_name_is_still_bracket_form() {
    let cases: [(&str, &str, &[&str]); 10] = [
        (r"\n[half]", "n", &["half"]),
        (r"\n[50]", "n", &["50"]),
        (r"\w[2]", "w", &["2"]),
        (r"\b[2]", "b", &["2"]),
        (r"\p[1]", "p", &["1"]),
        (r"\_a[ID]", "_a", &["ID"]),
        (r"\_l[x,y]", "_l", &["x", "y"]),
        (r"\_w[600]", "_w", &["600"]),
        (r"\__q[OnTest]", "__q", &["OnTest"]),
        (r"\__v[disable]", "__v", &["disable"]),
    ];
    for (input, word, args) in cases {
        assert_eq!(lex(input), vec![tag(word, args)], "input: {input}");
    }
    assert_eq!(lex(r"\e[注]です"), vec![tag("e", &["注"]), text("です")]);
}

/// L9（要件 3.2・5.5）: タグの直後の未閉じ `[` は従来どおり残り全部を `Raw` として吸収する。
#[test]
fn unclosed_bracket_immediately_after_tag_name_is_still_absorbed_as_raw() {
    assert_eq!(lex(r"\e[注です"), vec![raw(r"\e[注です")]);
    assert_eq!(lex(r"\b1["), vec![raw(r"\b1[")]);
}

/// L10（要件 4.10・5.5）: 短縮形の 1 桁規律と、旧仕様の選択肢 `\q*[ID][タイトル]` は不変。
#[test]
fn shorthand_digit_rule_and_legacy_q_star_are_unchanged() {
    assert_eq!(lex(r"\w2[x]"), vec![tag("w2", &["x"])]);
    assert_eq!(lex(r"\b2[x]"), vec![tag("b2", &["x"])]);
    assert_eq!(lex(r"\p2[x]"), vec![tag("p2", &["x"])]);
    assert_eq!(lex(r"\w2"), vec![shorthand('w', 2)]);
    assert_eq!(lex(r"\b12"), vec![shorthand('b', 1), text("2")]);

    assert_eq!(
        lex(r"\q*[ID][タイトル]"),
        vec![tag("q*", &["ID"]), text("[タイトル]")]
    );
    assert_eq!(lex(r"\q*[ID"), vec![raw(r"\q*[ID")]);
}

/// L10 の補足（要件 1.9・4.10）: `\q*` の直後が `[` でなければ選択肢の例外に入らず、
/// 1 文字タグ `\q` ＋ 本文になる（是正前は `q*テキスト` までをタグ名として読んでいた）。
#[test]
fn q_star_without_immediate_bracket_is_one_char_tag_and_text() {
    assert_eq!(
        lex(r"\q*テキスト[注]"),
        vec![bare("q"), text("*テキスト[注]")]
    );
}

/// L11（要件 4.4・4.9・5.6）: 語境界の是正が影響しない 4 形（角括弧形の後ろの本文・
/// 短縮形の後ろの本文・全角括弧・タグを含まない台詞）。
#[test]
fn forms_unaffected_by_word_boundary_rule() {
    assert_eq!(
        lex(r"\s[0]テキスト[注]です"),
        vec![tag("s", &["0"]), text("テキスト[注]です")]
    );
    assert_eq!(
        lex(r"\w9テキスト[注]です"),
        vec![shorthand('w', 9), text("テキスト[注]です")]
    );
    assert_eq!(
        lex(r"\eテキスト［注］です"),
        vec![bare("e"), text("テキスト［注］です")]
    );
    assert_eq!(
        lex("ふつうの台詞[1]です"),
        vec![text("ふつうの台詞[1]です")]
    );
}

// ───────────────────────────────────────────────────────────────────
// L12: 正典に無い多文字綴り（意図的な非互換・要件 4.11・5.6）。
// ───────────────────────────────────────────────────────────────────

/// L12（要件 4.11・5.6）: 正典に無い多文字綴りの直後の角括弧は、1 文字（または
/// `\_` の固定長）のタグ ＋ 本文に分かれる。従来は丸ごと 1 単位として捨てていた。
#[test]
fn unknown_multi_char_spelling_splits_into_tag_and_text() {
    assert_eq!(
        lex(r"あ\foo[a,b]い"),
        vec![text("あ"), bare("f"), text("oo[a,b]い")]
    );
    assert_eq!(lex(r"\foo["), vec![bare("f"), text("oo[")]);
    assert_eq!(lex(r"\___x[1]"), vec![bare("___"), text("x[1]")]);
}
