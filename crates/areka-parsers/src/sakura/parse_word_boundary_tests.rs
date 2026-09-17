//! タグ名の終端規則の是正を、公開入口 `parse` の通しで固定するテスト。
//!
//! 字句層のトークン列ではなく、利用者から見える結果（`Instruction` 列＝表示本文と
//! 素通し断片）を固定する。固定するのは次の 3 方向:
//! - **意味の保持**: タグの後ろの本文に角括弧があっても、話者切替・改行・トーク終了・
//!   終了指令・消去・引数なしの文字装飾の意味が保たれ、本文が逐語で残る（要件 1.2・2.1〜2.7）。
//! - **読みすぎない**: 本文中の未閉じの開き括弧が残り全部を飲み込まない（要件 3.1）。
//! - **既存規律の不変**: 直後に角括弧が来る形・影響しない 4 形の意味が是正の前後で同一で、
//!   架空の多文字綴りは 1 文字タグ ＋ 本文へ分かれる（要件 3.2・4.1・4.2・4.9・4.11）。
//!
//! すべて決定論（`#[test]`・時計／GPU／実機に依存しない・要件 5.10）。
//! 設計の Testing Strategy「通しテスト」P1〜P9 に一対一で対応する。

use super::super::model::{Instruction, NewLineRatio, SurfaceArg};
use super::parse;
use std::time::Duration;

// ───────────────────────────────────────────────────────────────────
// 期待値の組み立て補助（本ファイル内のみで使う）。
// ───────────────────────────────────────────────────────────────────

fn raw(s: &str) -> Instruction {
    Instruction::Raw(s.to_string())
}

fn text(s: &str) -> Instruction {
    Instruction::Text(s.to_string())
}

fn speaker(n: u32) -> Instruction {
    Instruction::SpeakerScope { n }
}

fn newline() -> Instruction {
    Instruction::NewLine(NewLineRatio::new(1.0))
}

// ───────────────────────────────────────────────────────────────────
// P1〜P2: 本文中に角括弧があっても意味が保たれる（要件 1.2・2.1〜2.5・2.7・5.9）。
// ───────────────────────────────────────────────────────────────────

/// P1（要件 1.2・2.1・5.9）: 要件の代表例。話者切替とトーク終了が保たれ、
/// 本文「青い[1]ノートさんから交代したよ〜。」が 1 文字も欠けずに表示される。
#[test]
fn representative_example_keeps_speaker_and_end() {
    assert_eq!(
        parse(r"\1青い[1]ノートさんから交代したよ〜。\e"),
        vec![
            speaker(1),
            text("青い[1]ノートさんから交代したよ〜。"),
            Instruction::End,
        ]
    );
}

/// P2（要件 2.1〜2.5・2.7・5.9）: areka が意味を与えている 9 綴りの各々について、
/// 本文 `本文[注]` が続いてもタグの意味が保たれ、本文が逐語で残る。
#[test]
fn meaningful_spellings_keep_meaning_before_body_with_bracket() {
    let cases = [
        (r"\0", speaker(0)),
        (r"\h", speaker(0)),
        (r"\1", speaker(1)),
        (r"\u", speaker(1)),
        (r"\n", newline()),
        (r"\e", Instruction::End),
        (r"\-", Instruction::Quit),
        (r"\c", Instruction::Clear),
        (r"\f", Instruction::Font { args: Vec::new() }),
    ];
    for (tag, meaning) in cases {
        let input = format!("{tag}本文[注]");
        assert_eq!(
            parse(&input),
            vec![meaning, text("本文[注]")],
            "input: {input}"
        );
    }
}

// ───────────────────────────────────────────────────────────────────
// P3〜P4: 新しい動作を得ない・読みすぎない（要件 1.4・1.9・2.6・3.1・6.3・6.4）。
// ───────────────────────────────────────────────────────────────────

/// P3（要件 1.4・1.9・2.6・6.3・6.4）: 意味を持たない綴り（短縮対象語・`\_` タグ・
/// 未知の 1 文字綴り）は従来どおりタグ部分だけの `Raw` に留まり、本文は表示本文になる。
#[test]
fn meaningless_spellings_stay_raw_before_body_with_bracket() {
    assert_eq!(
        parse(r"\wテキスト[注]です"),
        vec![raw(r"\w"), text("テキスト[注]です")]
    );
    assert_eq!(
        parse(r"\_q文字[注]を瞬間表示する。\_q"),
        vec![raw(r"\_q"), text("文字[注]を瞬間表示する。"), raw(r"\_q")]
    );
    assert_eq!(
        parse(r"\4テキスト[注]です"),
        vec![raw(r"\4"), text("テキスト[注]です")]
    );
}

/// P4（要件 3.1・5.9）: 本文中の未閉じの開き括弧は、後ろの台詞を飲み込まない。
/// 話者切替とトーク終了の意味も保たれる。
#[test]
fn unclosed_bracket_in_body_does_not_swallow_the_rest() {
    assert_eq!(
        parse(r"\1テキスト[注です。まだ続く台詞"),
        vec![speaker(1), text("テキスト[注です。まだ続く台詞")]
    );
    assert_eq!(
        parse(r"\eテキスト[注です。まだ続く台詞"),
        vec![Instruction::End, text("テキスト[注です。まだ続く台詞")]
    );
}

// ───────────────────────────────────────────────────────────────────
// P5〜P7: 是正の前後で変わってはならない既存規律（要件 3.2・4.1・4.2・4.9）。
// ───────────────────────────────────────────────────────────────────

/// P5（要件 3.2）: タグ直後の未閉じの開き括弧は、従来どおり末尾まで `Raw` 1 個になる。
#[test]
fn unclosed_bracket_right_after_tag_is_still_raw_to_end() {
    assert_eq!(parse(r"\e[注です"), vec![raw(r"\e[注です")]);
}

/// P6（要件 4.1・4.2）: 直後に角括弧が来る形の意味が不変。
#[test]
fn bracket_forms_keep_their_meaning() {
    assert_eq!(
        parse(r"\n[half]"),
        vec![Instruction::NewLine(NewLineRatio::new(0.5))]
    );
    assert_eq!(
        parse(r"\_w[600]"),
        vec![Instruction::Wait(Duration::from_millis(600))]
    );
    assert_eq!(
        parse(r"\_l[x,y]"),
        vec![Instruction::Cursor {
            x: "x".to_string(),
            y: "y".to_string(),
        }]
    );
    assert_eq!(parse(r"\_a[ID]"), vec![raw(r"\_a[ID]")]);
    assert_eq!(parse(r"\p[1]"), vec![speaker(1)]);
}

/// P7（要件 4.9）: 影響しない 4 形——角括弧付きタグの後の本文、真の短縮形の後の本文、
/// 全角の角括弧、タグの無い台詞——の結果が不変。
#[test]
fn unaffected_forms_are_unchanged() {
    assert_eq!(
        parse(r"\s[0]テキスト[注]です"),
        vec![
            Instruction::Surface(SurfaceArg::new("0".to_string())),
            text("テキスト[注]です"),
        ]
    );
    assert_eq!(
        parse(r"\w9テキスト[注]です"),
        vec![
            Instruction::Wait(Duration::from_millis(450)),
            text("テキスト[注]です"),
        ]
    );
    assert_eq!(
        parse(r"\eテキスト［注］です"),
        vec![Instruction::End, text("テキスト［注］です")]
    );
    assert_eq!(
        parse("ふつうの台詞[1]です"),
        vec![text("ふつうの台詞[1]です")]
    );
}

// ───────────────────────────────────────────────────────────────────
// P8〜P9: 架空綴りの分割と、半角英数・後続単位（要件 1.6・1.8・4.11）。
// ───────────────────────────────────────────────────────────────────

/// P8（要件 2.7・4.11）: 架空の多文字綴り `\foo[a,b]` はタグとして構成されず、
/// 1 文字タグ `\f`（引数なしの文字装飾）と本文へ分かれる（意図的な非互換）。
#[test]
fn made_up_multi_char_spelling_splits_into_bare_font_and_text() {
    assert_eq!(
        parse(r"\foo[a,b]い"),
        vec![Instruction::Font { args: Vec::new() }, text("oo[a,b]い")]
    );
}

/// P9（要件 1.6・1.8）: 半角英数だけの本文も 1 文字で終端し、本文の後ろに続く
/// 改行タグとシステム変数は独立した単位として残る。
#[test]
fn ascii_body_and_following_units_survive() {
    assert_eq!(
        parse(r"\0file[1].txt"),
        vec![speaker(0), text("file[1].txt")]
    );
    assert_eq!(
        parse(r"\1青い[1]ノート\nさん%username"),
        vec![
            speaker(1),
            text("青い[1]ノート"),
            newline(),
            text("さん"),
            Instruction::SystemVar("username".to_string()),
        ]
    );
}
