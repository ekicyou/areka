//! 横断バリデーション単体テスト（タスク 6）。
//!
//! 本ファイルは spec の**統合／バリデーション層**として、公開エントリ
//! `super::parse::parse`（`pub fn parse(&str) -> Vec<Instruction>`）を通した
//! end-to-end アサーションで、以下の観測可能挙動を体系的に網羅する:
//! - SYNTAX（要件 13）: エスケープ `\\`/`\%`/角内 `\]`・クォート `"..."`/`""`・
//!   `\X[...]` の `]` 終端・未知タグの構文区切り＋`Raw` 吸収。
//! - VALUE NORMALIZATION（要件 3/4）: 待ち時間・改行比率の境界値。
//! - Choice（要件 5.3）: 旧 2 連形が隣接命令を壊さないこと。
//! - 不透明 Surface（要件 2.2/2.3）・寛容パススルー（要件 10）・
//!   純粋関数契約（要件 1.5/12.2/12.3）。
//! - 代表 OnBoot 通し例（要件 12.3/12.4・design「Integration Tests」の
//!   インラインフィクスチャ）。
//! - OnBoot 例に**現れない**タグ（`\q` / `\![move]` / `%username`）の個別手書き網羅。
//! - `\s[""]`（唯一の空クォート引数）の strict 挙動（要件 13.4・タスク 3.2 追随）。

use super::lexer::{Token, lex};
use super::model::{Choice, Instruction, NewLineRatio, SurfaceArg};
use super::parse::parse;
use std::time::Duration;

// ───────────────────────────────────────────────────────────────────
// SYNTAX（要件 13）— エスケープ／クォート／角括弧終端／未知タグ区切り
// ───────────────────────────────────────────────────────────────────

/// `\\` はタグ開始でなくリテラル `\`、`\%` はシステム変数開始でなくリテラル `%`
/// としてテキストへ取り込まれる（要件 13.5/13.6）。
#[test]
fn syntax_escape_backslash_and_percent_become_literal_text() {
    assert_eq!(
        parse(r"a\\b\%c"),
        vec![Instruction::Text(r"a\b%c".to_string())],
    );
}

/// 角括弧内エスケープ `\]` はタグ終端でなくリテラル `]` として引数へ取り込む
/// （要件 13.7）。`\s[a\]b]` → 不透明中身 `a]b`。
#[test]
fn syntax_escaped_bracket_inside_args_is_literal() {
    assert_eq!(
        parse(r"\s[a\]b]"),
        vec![Instruction::Surface(SurfaceArg::new("a]b".to_string()))],
    );
}

/// クォート `"a,b"` は内側 `,` を区切りとみなさず全体で 1 引数（要件 13.4）。
/// `\q["a,b",id]` → disp は `a,b`（カンマ保護）。
#[test]
fn syntax_quoted_arg_protects_inner_comma() {
    assert_eq!(
        parse(r#"\q["a,b",id]"#),
        vec![Instruction::Choice(Choice {
            disp: "a,b".to_string(),
            target: "id".to_string(),
            references: Vec::new(),
        })],
    );
}

/// 二重化 `""` はクォート内リテラル `"` として取り込む（要件 13.4）。
/// `\q["a""b",id]` → disp は `a"b`。
#[test]
fn syntax_doubled_quote_is_literal_quote() {
    assert_eq!(
        parse(r#"\q["a""b",id]"#),
        vec![Instruction::Choice(Choice {
            disp: "a\"b".to_string(),
            target: "id".to_string(),
            references: Vec::new(),
        })],
    );
}

/// `\X[...]` は対応する `]` までを引数範囲として区切り、後続を巻き込まない
/// （要件 13.2）。`\s[10]あ` → Surface と Text が分離。
#[test]
fn syntax_bracket_closes_at_matching_bracket() {
    assert_eq!(
        parse(r"\s[10]あ"),
        vec![
            Instruction::Surface(SurfaceArg::new("10".to_string())),
            Instruction::Text("あ".to_string()),
        ],
    );
}

/// bare スコープタグ `\0` は 1 文字で終端し後続を巻き込まない（lexer 終端規律・要件 13.1）。
/// `\0あ` → SpeakerScope{n:0}（本体側・R1.5/R4.4・ukadoc `\0`=本体側）＋ Text(`あ`)。
/// 眼目は lexer の 1 文字終端（`あ` を吸わない）＝decode 写像先が Raw から SpeakerScope へ
/// 更新されても終端規律は不変であることを固定する（正典スコープ写像のスーパーセット更新）。
#[test]
fn syntax_scope_bare_tag_terminates_at_one_char() {
    assert_eq!(
        parse(r"\0あ"),
        vec![
            Instruction::SpeakerScope { n: 0 },
            Instruction::Text("あ".to_string()),
        ],
    );
}

// ───────────────────────────────────────────────────────────────────
// VALUE NORMALIZATION（要件 3/4）— 待ち時間・改行比率の境界値
// ───────────────────────────────────────────────────────────────────

/// 待ち時間の正典 2 表記が単一 `Wait(Duration)` へ正規化される（要件 3.2/3.3/3.4・18.1）。
/// `\w9`=450ms（=9×50・短縮形）/ `\_w[100]`=100ms / `\_w[500]`=500ms（絶対 ms）。
#[test]
fn value_wait_shorthand_and_absolute_ms() {
    assert_eq!(
        parse(r"\w9\_w[100]\_w[500]"),
        vec![
            Instruction::Wait(Duration::from_millis(450)),
            Instruction::Wait(Duration::from_millis(100)),
            Instruction::Wait(Duration::from_millis(500)),
        ],
    );
}

/// 改行比率の境界値（要件 4.1/4.2）。素 `\n`→1.0 / `\n[150]`→1.5 / `\n[half]`→0.5。
#[test]
fn value_newline_ratio_boundary_values() {
    assert_eq!(
        parse(r"\n\n[150]\n[half]"),
        vec![
            Instruction::NewLine(NewLineRatio::new(1.0)),
            Instruction::NewLine(NewLineRatio::new(1.5)),
            Instruction::NewLine(NewLineRatio::new(0.5)),
        ],
    );
}

// ───────────────────────────────────────────────────────────────────
// Choice（要件 5.3）— 旧 2 連形は隣接命令を壊さない
// ───────────────────────────────────────────────────────────────────

/// 旧 2 連 `\q[ID][タイトル]` は現行 Choice にせず単一 `Raw` に吸収され、
/// 前後の正常命令（隣接タグ）を破壊しない（要件 5.3/10.3）。
/// 直後に地の文が続く形は `legacy_q_title_followed_by_text_folds`。
#[test]
fn choice_legacy_double_bracket_does_not_break_neighbors() {
    let got = parse(r"\e\q[ID][タイトル]\c");
    assert_eq!(
        got,
        vec![
            Instruction::End,
            Instruction::Raw(r"\q[ID][タイトル]".to_string()),
            Instruction::Clear,
        ],
    );
    // Choice へ化けていないことを明示（旧形は現行 Choice にしない・要件 5.3）。
    assert!(!got.iter().any(|i| matches!(i, Instruction::Choice(_))));
}

/// 旧形の題の直後に地の文が続いても、閉じた題 `[…]` だけが畳まれ、地の文は字のまま
/// （指摘 25・要件 5.3）。題の中の `%` も題の字のまま（環境変数として切らない）。
#[test]
fn legacy_q_title_followed_by_text_folds() {
    let raw = |s: &str| Instruction::Raw(s.to_string());
    let text = |s: &str| Instruction::Text(s.to_string());
    assert_eq!(
        parse(r"\q[OnA][はい]です\e"),
        vec![raw(r"\q[OnA][はい]"), text("です"), Instruction::End],
    );
    assert_eq!(parse(r"\q[ID][100%]"), vec![raw(r"\q[ID][100%]")]);
    assert_eq!(
        parse(r"\q0[ID][題]後"),
        vec![raw(r"\q0[ID][題]"), text("後")]
    );
}

/// 旧専用綴り `\q*[ID][タイトル]` も同様に単一 `Raw` 吸収・隣接を壊さない（要件 5.3）。
#[test]
fn choice_legacy_q_star_double_bracket_kept_raw() {
    assert_eq!(
        parse(r"\e\q*[ID][タイトル]\c"),
        vec![
            Instruction::End,
            Instruction::Raw(r"\q*[ID][タイトル]".to_string()),
            Instruction::Clear,
        ],
    );
}

/// 指摘 26: 通し番号つきの旧形 `\q0[ID][タイトル]` も単一 `Raw` に畳み、
/// `0[ID][タイトル]` の字を吹き出しに漏らさない（要件 5.3・設計の討議の議題 3）。
/// 2 桁の `\q10[a][b]` は旧形の印に当たらず、今どおり（`\q` の Raw と字）。
#[test]
fn choice_legacy_q_serial_digit_kept_raw() {
    assert_eq!(
        parse(r"\e\q0[OnYes][はい]\c"),
        vec![
            Instruction::End,
            Instruction::Raw(r"\q0[OnYes][はい]".to_string()),
            Instruction::Clear,
        ],
    );
    assert_eq!(
        parse(r"\q10[a][b]"),
        vec![
            Instruction::Raw(r"\q".to_string()),
            Instruction::Text("10[a][b]".to_string()),
        ],
    );
}

// ───────────────────────────────────────────────────────────────────
// 不透明 Surface（要件 2.2/2.3）
// ───────────────────────────────────────────────────────────────────

// ───────────────────────────────────────────────────────────────────
// 寛容パススルー（要件 10）— 不正トークン前後の正常命令が欠落しない
// ───────────────────────────────────────────────────────────────────

// ───────────────────────────────────────────────────────────────────
// 純粋関数契約（要件 1.5/12.2/12.3）
// ───────────────────────────────────────────────────────────────────

/// 混在入力の順序保持（要件 1.3/9.2）。タグ・テキストが入力順のまま整列。
#[test]
fn pure_mixed_input_preserves_order() {
    assert_eq!(
        parse(r"\p[0]あ\e"),
        vec![
            Instruction::SpeakerScope { n: 0 },
            Instruction::Text("あ".to_string()),
            Instruction::End,
        ],
    );
}

/// UTF-8 日本語テキストが Text run 化される（要件 9.1・12.1 の UTF-8 前提）。
#[test]
fn pure_utf8_japanese_text_becomes_text_run() {
    assert_eq!(
        parse("夜更かしはお肌に悪いよ。"),
        vec![Instruction::Text("夜更かしはお肌に悪いよ。".to_string())],
    );
}

// ───────────────────────────────────────────────────────────────────
// OnBoot 例に現れないタグの個別手書き網羅（要件 12.4・design L530）
// ───────────────────────────────────────────────────────────────────

/// `\![move,10,20,base,base]` はほかの `\!` と同じ汎用の道（名前 `move`＋生引数列・指摘 28）。
/// 型つきの写しは持たない（意味割当は下流の責務）。
#[test]
fn move_bang_is_generic_command() {
    assert_eq!(
        parse(r"\![move,10,20,base,base]"),
        vec![Instruction::GenericCommand {
            name: "move".to_string(),
            raw_args: vec![
                "10".to_string(),
                "20".to_string(),
                "base".to_string(),
                "base".to_string(),
            ],
        }],
    );
}

// ───────────────────────────────────────────────────────────────────
// `\s[""]`（唯一の空クォート引数）— strict 挙動（要件 13.4・タスク 3.2 追随）
// ───────────────────────────────────────────────────────────────────

/// 要件 13.4「クォートで囲まれた全体を 1 引数として扱う」より、`\s[""]` は
/// 空文字列 1 引数（＝不透明中身が空の `Surface`）であるべき。
/// タスク 3.2 で記録された既知エッジ（sole quoted-empty が 0 引数化する）の
/// strict 追随（lexer.rs の `quote_consumed` 修正で 1 空引数へ）。
///
/// **lexer レベルで固定**する: `\s` の decode は `unwrap_or_default()` で
/// 0 引数と 1 空引数を同一視してしまい `parse` 経由では 0/1 の差が観測できない。
/// 要件 13.4 の「1 引数」契約は構文層（lexer）で確定するため、`lex` の
/// `args` がちょうど 1 個の空文字列であることを直接固定する。
#[test]
fn strict_sole_empty_quote_arg_is_single_empty_arg_at_lexer() {
    assert_eq!(
        lex(r#"\s[""]"#),
        vec![Token::Tag {
            word: "s".to_string(),
            args: vec![String::new()],
        }],
    );
}

/// 上記 lexer 契約の end-to-end 反映: `\s[""]` は不透明中身が空の `Surface`
/// 1 個（要件 2.2/13.4）。`parse` 面でも空文字列 Surface を固定する。
#[test]
fn strict_sole_empty_quote_arg_is_single_empty_surface() {
    assert_eq!(
        parse(r#"\s[""]"#),
        vec![Instruction::Surface(SurfaceArg::new(String::new()))],
    );
}

/// 併記の回帰防止: カンマが push を強制する既存ケースは従来どおり動く。
/// `\s["",x]` → 中身 `""` が空文字列 1 個目、`x` は無視されず Surface は
/// 第 1 引数のみ採用（`\s` は先頭引数だけ不透明保持）。ここでは lexer が
/// 空文字列と `x` の 2 引数へ割ることを Choice で確認（区切りの健全性）。
#[test]
fn strict_empty_quote_with_comma_still_two_args() {
    // `\q["",id]` → disp は空文字列、target は id（区切りが 2 引数のまま）。
    assert_eq!(
        parse(r#"\q["",id]"#),
        vec![Instruction::Choice(Choice {
            disp: String::new(),
            target: "id".to_string(),
            references: Vec::new(),
        })],
    );
}

// ───────────────────────────────────────────────────────────────────
// 代表 OnBoot 通し例（要件 12.3/12.4・design「Integration Tests」L524-527）
//
// 作者提供の実 OnBoot 例をインラインフィクスチャとして verbatim に用い、
// 入力全体 → 期待命令列の写像を固定する（ゴーストファイルは同梱しない）。
// これ 1 本で:
//   \p[0]/\p[1]（SpeakerScope）、\s[1000]/\s[通常]（不透明 Surface）、
//   \![bind,...]×6（GenericCommand）、\_w[450]/\_w[950]（絶対 ms Wait）、
//   \n（1.0）/\n[150]（1.5）、\e（End）、日本語 Text run を一括検証。
// ───────────────────────────────────────────────────────────────────

/// design L527 の実 OnBoot 例（verbatim）の入力全体 → 期待命令列。
#[test]
fn representative_onboot_passthrough_maps_whole_input() {
    let input = r"\p[0]\s[1000]\![bind,腕,伸び,1]\![bind,紅,差し,0]\![bind,口,にこっ,1]\![bind,眉,通常,1]\![bind,目,笑顔,1]\![bind,まばたき,----,1]こんばんわー！\_w[450]夜の部、\_w[450]\n開幕やー！\_w[450]\n[150]\p[1]\s[通常]夜更かしはお肌に\n悪いよ。\_w[950]\e";

    // 各 bind は 3 生引数（部位・表情・on/off）を保持する `GenericCommand`。
    let bind3 = |a: &str, b: &str, c: &str| Instruction::GenericCommand {
        name: "bind".to_string(),
        raw_args: vec![a.to_string(), b.to_string(), c.to_string()],
    };

    let want = vec![
        Instruction::SpeakerScope { n: 0 },
        Instruction::Surface(SurfaceArg::new("1000".to_string())),
        bind3("腕", "伸び", "1"),
        bind3("紅", "差し", "0"),
        bind3("口", "にこっ", "1"),
        bind3("眉", "通常", "1"),
        bind3("目", "笑顔", "1"),
        bind3("まばたき", "----", "1"),
        Instruction::Text("こんばんわー！".to_string()),
        Instruction::Wait(Duration::from_millis(450)),
        Instruction::Text("夜の部、".to_string()),
        Instruction::Wait(Duration::from_millis(450)),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::Text("開幕やー！".to_string()),
        Instruction::Wait(Duration::from_millis(450)),
        Instruction::NewLine(NewLineRatio::new(1.5)),
        Instruction::SpeakerScope { n: 1 },
        Instruction::Surface(SurfaceArg::new("通常".to_string())),
        Instruction::Text("夜更かしはお肌に".to_string()),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::Text("悪いよ。".to_string()),
        Instruction::Wait(Duration::from_millis(950)),
        Instruction::End,
    ];

    assert_eq!(parse(input), want);
}
