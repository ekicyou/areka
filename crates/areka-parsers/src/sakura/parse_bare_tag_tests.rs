//! 角括弧を伴わないタグを、公開入口 `parse` の通しで固定するテスト。
//!
//! 字句層のトークン列ではなく、利用者から見える結果（`Instruction` 列＝表示本文と
//! 素通し断片）を固定する。固定するのは次の 3 方向:
//! - **表示本文**: タグに挟まれた台詞が 1 文字も欠けず 1 文字も増えない。
//! - **意味を付けない**: 角括弧なし `\_` タグ（切替の別名 `\_+` とアンカーの閉じ `\_a` を除く）は
//!   `Instruction::Raw`（タグ全体の文字列）に留まり、待ちにも表示にも改行にもならない。
//! - **既存規律の不変**: 角括弧付き `\_w`／`\_l` の値正規化と既知 1 文字タグの意味が保たれる。
//!
//! すべて決定論（`#[test]`・時計／GPU／実機に依存しない）。

use super::super::model::{Anchor, Instruction, NewLineRatio};
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

/// 正典が定める角括弧なし `\_` タグのうち、生の綴りのまま残る 10 綴り（`\` を除く）。
/// 2 文字形 6 個 ＋ 3 文字形 4 個。
/// `\_+`（`\![change,ghost,sequential]` の別名）と `\_a`（アンカーの閉じ）は意味を持つので含まない。
const CANONICAL_BRACKETLESS_SPELLINGS: [&str; 10] = [
    "_q", "_n", "_s", "_V", "_?", "_!", "__c", "__t", "__q", "__v",
];

/// 入力末尾の裸 `\` は 1 字の `Raw("\\")` に組み直す（`\` を二重にしない）。
#[test]
fn trailing_lone_backslash_is_single_backslash_raw() {
    assert_eq!(parse(r"あ\"), vec![text("あ"), raw(r"\")]);
}

// ───────────────────────────────────────────────────────────────────
// P1〜P4: 開始形と終了形の対 4 組。
// ───────────────────────────────────────────────────────────────────

/// P1: `\_q…\_q` の表示本文は「文字を瞬間表示する。」だけ。
#[test]
fn quick_section_pair_shows_only_the_body() {
    assert_eq!(
        parse(r"\_q文字を瞬間表示する。\_q"),
        vec![raw(r"\_q"), text("文字を瞬間表示する。"), raw(r"\_q")]
    );
}

/// P2: 里々のアンカー形。開始は角括弧付き、終了は
/// 角括弧なし。表示本文は「アンカー」＋「をクリックする。」で、`a` は漏れず
/// 「をクリックする。」も飲み込まれない（読み足りない／読みすぎの両方向を塞ぐ対）。
/// 対は素通しでなく「開き・文字・閉じ・文字」として読まれる。
#[test]
fn anchor_pair_shows_only_the_body() {
    assert_eq!(
        parse(r"\_a[Hint]アンカー\_aをクリックする。"),
        vec![
            Instruction::Anchor(Anchor {
                id: "Hint".to_string(),
                references: Vec::new(),
            }),
            text("アンカー"),
            Instruction::AnchorEnd,
            text("をクリックする。"),
        ]
    );
}

/// P3: 3 文字形の選択肢範囲。`\__q` の `_q` が本文へ漏れず、
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

/// P4: 3 文字形の音声合成調整範囲。終了形が入力末尾に来る対。
#[test]
fn voice_range_pair_shows_only_the_body() {
    assert_eq!(
        parse(r"\__v[disable]しゃべらない。\__v"),
        vec![raw(r"\__v[disable]"), text("しゃべらない。"), raw(r"\__v")]
    );
}

// ───────────────────────────────────────────────────────────────────
// P5〜P7: 意味を付けずに素通しする。
// ───────────────────────────────────────────────────────────────────

/// P5: 生の綴りのまま残る正典 10 タグの各々が単独入力で
/// `Raw`（タグ全体の文字列）1 個ちょうどになる。命令列を丸ごと固定するので、
/// `Wait`／`Cursor`／`NewLine`／`Text` などが 1 個でも増えれば赤になる。
#[test]
fn each_canonical_bracketless_tag_yields_exactly_one_raw() {
    for spelling in CANONICAL_BRACKETLESS_SPELLINGS {
        let input = format!(r"\{spelling}");
        assert_eq!(parse(&input), vec![raw(&input)], "input: {input}");
    }
}

/// P5: 正典外の角括弧なし `\_w`／`\_l` は素通しの `Raw` に留まり、
/// 角括弧付き形の意味（`Wait`／`Cursor`）を先取りしない。
#[test]
fn bracketless_underscore_w_and_l_never_become_wait_or_cursor() {
    assert_eq!(parse(r"\_w"), vec![raw(r"\_w")]);
    assert_eq!(parse(r"\_l"), vec![raw(r"\_l")]);
}

/// P6: 正典に無い綴り `\_z` と、入力末尾に単独で現れる
/// `\_`／`\__` が `Raw` 1 個になる。命令列を固定することで「panic しない」「空にならない」
/// の両方を同時に押さえる。
#[test]
fn unknown_and_truncated_bracketless_tags_stay_raw() {
    assert_eq!(parse(r"\_z"), vec![raw(r"\_z")]);
    assert_eq!(parse(r"\_"), vec![raw(r"\_")]);
    assert_eq!(parse(r"\__"), vec![raw(r"\__")]);
}

/// P7: 角括弧なしタグだけを並べた入力から表示本文は 1 つも生じない。
#[test]
fn input_of_bracketless_tags_only_yields_no_text() {
    let got = parse(r"\_q\_s\__q\__v\_");
    assert_eq!(
        got,
        vec![
            raw(r"\_q"),
            raw(r"\_s"),
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
// P8〜P9: 既存規律の不変。
// ───────────────────────────────────────────────────────────────────

/// P8: 適合対象フィクスチャ emo2 が使う角括弧付き形の意味が不変。
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

/// P9: 既知 1 文字タグ 8 個の意味が不変（`\e`・`\c`・`\-`・`\n`・
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
// P10: 全角半角混在の本文。
// ───────────────────────────────────────────────────────────────────

/// P10: 全角・半角・記号・数字が混在する本文が、2 文字形と 3 文字形の
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
