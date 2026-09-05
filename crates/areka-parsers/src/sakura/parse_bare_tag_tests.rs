//! 角括弧を伴わないタグの是正を、公開入口 `parse` の通しで固定するテスト。
//!
//! 字句層のトークン列ではなく、利用者から見える結果（`Instruction` 列＝表示本文と
//! 素通し断片）を固定する。固定するのは次の 3 方向:
//! - **表示本文**: タグに挟まれた台詞が 1 文字も欠けず 1 文字も増えない（要件 2.1〜2.3・2.5・2.6）。
//! - **意味を付けない**: 角括弧なし `\_` タグは `Instruction::Raw`（タグ全体の文字列）に
//!   留まり、待ちにも表示にも改行にもならない（要件 3.1〜3.5）。
//! - **既存規律の不変**: 角括弧付き `\_w`／`\_l` の値正規化と既知 1 文字タグの意味が
//!   本仕様の前後で同一（要件 4.1・4.6・4.7・5.7）。
//!
//! すべて決定論（`#[test]`・時計／GPU／実機に依存しない・要件 5.8）。
//! 設計の Testing Strategy「通しテスト」P1〜P10 に一対一で対応する。

use super::super::model::{Instruction, NewLineRatio};
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

/// 正典が定める角括弧なし `\_` タグの全 12 綴り（`\` を除く）。
/// 2 文字形 8 個 ＋ 3 文字形 4 個（要件 1.2・5.1）。
const CANONICAL_BRACKETLESS_SPELLINGS: [&str; 12] = [
    "_a", "_q", "_n", "_s", "_V", "_?", "_+", "_!", "__c", "__t", "__q", "__v",
];

// ───────────────────────────────────────────────────────────────────
// P1〜P4: 開始形と終了形の対 4 組（要件 2.1〜2.3・2.6・5.2）。
// ───────────────────────────────────────────────────────────────────

/// P1（要件 2.1・2.2・3.2・5.2）: `\_q…\_q` の表示本文は「文字を瞬間表示する。」だけ。
/// 旧欠陥では前後に `q` が漏れていた。
#[test]
fn quick_section_pair_shows_only_the_body() {
    assert_eq!(
        parse(r"\_q文字を瞬間表示する。\_q"),
        vec![raw(r"\_q"), text("文字を瞬間表示する。"), raw(r"\_q")]
    );
}

/// P2（要件 2.3・5.2・5.5・5.6）: 里々のアンカー形。開始は角括弧付き（不変）、終了は
/// 角括弧なし。表示本文は「アンカー」＋「をクリックする。」で、`a` は漏れず
/// 「をクリックする。」も飲み込まれない（読み足りない／読みすぎの両方向を塞ぐ対）。
#[test]
fn anchor_pair_shows_only_the_body() {
    assert_eq!(
        parse(r"\_a[Hint]アンカー\_aをクリックする。"),
        vec![
            raw(r"\_a[Hint]"),
            text("アンカー"),
            raw(r"\_a"),
            text("をクリックする。"),
        ]
    );
}

/// P3（要件 2.6・5.2・5.6）: 3 文字形の選択肢範囲。`\__q` の `_q` が本文へ漏れず、
/// 直後の「この例の場合」も飲み込まれない。
#[test]
fn choice_range_pair_shows_only_the_body() {
    assert_eq!(
        parse(r"\__q[OnTest]選ぶ\__qこの例の場合"),
        vec![
            raw(r"\__q[OnTest]"),
            text("選ぶ"),
            raw(r"\__q"),
            text("この例の場合"),
        ]
    );
}

/// P4（要件 5.2）: 3 文字形の音声合成調整範囲。終了形が入力末尾に来る対。
#[test]
fn voice_range_pair_shows_only_the_body() {
    assert_eq!(
        parse(r"\__v[disable]しゃべらない。\__v"),
        vec![raw(r"\__v[disable]"), text("しゃべらない。"), raw(r"\__v")]
    );
}

// ───────────────────────────────────────────────────────────────────
// P5〜P7: 意味を付けずに素通しする（要件 3.1〜3.5・5.1）。
// ───────────────────────────────────────────────────────────────────

/// P5（要件 3.1・3.2・3.3・3.4・5.1）: 正典 12 タグの各々が単独入力で
/// `Raw`（タグ全体の文字列）1 個ちょうどになる。命令列を丸ごと固定するので、
/// `Wait`／`Cursor`／`NewLine`／`Text` などが 1 個でも増えれば赤になる。
#[test]
fn each_canonical_bracketless_tag_yields_exactly_one_raw() {
    for spelling in CANONICAL_BRACKETLESS_SPELLINGS {
        let input = format!(r"\{spelling}");
        assert_eq!(parse(&input), vec![raw(&input)], "input: {input}");
    }
}

/// P5（要件 3.1・3.3）: 正典外の角括弧なし `\_w`／`\_l` は素通しの `Raw` に留まり、
/// 角括弧付き形の意味（`Wait`／`Cursor`）を先取りしない（設計の案 B 却下の証拠）。
#[test]
fn bracketless_underscore_w_and_l_never_become_wait_or_cursor() {
    assert_eq!(parse(r"\_w"), vec![raw(r"\_w")]);
    assert_eq!(parse(r"\_l"), vec![raw(r"\_l")]);
}

/// P6（要件 3.5・1.7・1.7a・5.3）: 正典に無い綴り `\_z` と、入力末尾に単独で現れる
/// `\_`／`\__` が `Raw` 1 個になる。命令列を固定することで「panic しない」「空にならない」
/// の両方を同時に押さえる。
#[test]
fn unknown_and_truncated_bracketless_tags_stay_raw() {
    assert_eq!(parse(r"\_z"), vec![raw(r"\_z")]);
    assert_eq!(parse(r"\_"), vec![raw(r"\_")]);
    assert_eq!(parse(r"\__"), vec![raw(r"\__")]);
}

/// P7（要件 3.3）: 角括弧なしタグだけを並べた入力から表示本文は 1 つも生じない。
#[test]
fn input_of_bracketless_tags_only_yields_no_text() {
    let got = parse(r"\_q\_a\__q\__v\_");
    assert_eq!(
        got,
        vec![
            raw(r"\_q"),
            raw(r"\_a"),
            raw(r"\__q"),
            raw(r"\__v"),
            raw(r"\_"),
        ]
    );
    assert!(
        !got.iter().any(|i| matches!(i, Instruction::Text(_))),
        "表示本文が生じた: {got:?}"
    );
}

// ───────────────────────────────────────────────────────────────────
// P8〜P9: 本仕様の前後で変わってはならない既存規律（要件 4.1・4.6・4.7・5.7）。
// ───────────────────────────────────────────────────────────────────

/// P8（要件 4.6・4.7・5.7）: 適合対象フィクスチャ emo2 が使う角括弧付き形の意味が不変。
/// `\_l[5em,2lh]` はカーソル絶対位置、`\_w[450]` は絶対 ms の待ち。
#[test]
fn emo2_bracket_forms_keep_their_meaning() {
    assert_eq!(
        parse(r"\_l[5em,2lh]"),
        vec![Instruction::Cursor {
            x: "5em".to_string(),
            y: "2lh".to_string(),
        }]
    );
    assert_eq!(
        parse(r"\_w[450]"),
        vec![Instruction::Wait(Duration::from_millis(450))]
    );
}

/// P9（要件 4.1・5.7）: 既知 1 文字タグ 8 個の意味が不変（`\e`・`\c`・`\-`・`\n`・
/// `\0`／`\h`・`\1`／`\u`）。角括弧なし経路の消費規律は「先頭が `_` でなければ 1 文字」
/// なので、この 8 個は 1 バイトも動かない。
#[test]
fn known_one_char_bare_tags_keep_their_meaning() {
    assert_eq!(parse(r"\e"), vec![Instruction::End]);
    assert_eq!(parse(r"\c"), vec![Instruction::Clear]);
    assert_eq!(parse(r"\-"), vec![Instruction::Quit]);
    assert_eq!(
        parse(r"\n"),
        vec![Instruction::NewLine(NewLineRatio::new(1.0))]
    );
    assert_eq!(parse(r"\0"), vec![Instruction::SpeakerScope { n: 0 }]);
    assert_eq!(parse(r"\h"), vec![Instruction::SpeakerScope { n: 0 }]);
    assert_eq!(parse(r"\1"), vec![Instruction::SpeakerScope { n: 1 }]);
    assert_eq!(parse(r"\u"), vec![Instruction::SpeakerScope { n: 1 }]);
}

// ───────────────────────────────────────────────────────────────────
// P10: 全角半角混在の本文（要件 2.1・2.5）。
// ───────────────────────────────────────────────────────────────────

/// P10（要件 2.1・2.5）: 全角・半角・記号・数字が混在する本文が、2 文字形と 3 文字形の
/// どちらに挟まれても逐語で残る。
#[test]
fn mixed_width_body_survives_verbatim() {
    assert_eq!(
        parse(r"\_qあaいbうc、１2 ｱイ！\_q"),
        vec![raw(r"\_q"), text("あaいbうc、１2 ｱイ！"), raw(r"\_q")]
    );
    assert_eq!(
        parse(r"\__q[OnTest]ＡＢCd 12３４\__qあとがき"),
        vec![
            raw(r"\__q[OnTest]"),
            text("ＡＢCd 12３４"),
            raw(r"\__q"),
            text("あとがき"),
        ]
    );
}
