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

use super::lexer::{lex, Token};
use super::model::{Choice, Instruction, MoveArgs, NewLineRatio, SurfaceArg};
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

/// 未知（subset 外）タグ `\foo[a,b]` は構文として 1 単位に区切った上で `Raw`
/// 保持し、意味デコードしない（要件 13.8/11.2）。生情報を失わない。
#[test]
fn syntax_unknown_tag_is_syntactically_split_and_kept_raw() {
    assert_eq!(
        parse(r"\foo[a,b]"),
        vec![Instruction::Raw(r"\foo[a,b]".to_string())],
    );
}

/// 未知の bare タグ `\0` は 1 文字で終端し後続を巻き込まない（要件 13.1）。
/// `\0あ` → Raw(`\0`) ＋ Text(`あ`)。
#[test]
fn syntax_unknown_bare_tag_terminates_at_one_char() {
    assert_eq!(
        parse(r"\0あ"),
        vec![
            Instruction::Raw(r"\0".to_string()),
            Instruction::Text("あ".to_string()),
        ],
    );
}

// ───────────────────────────────────────────────────────────────────
// VALUE NORMALIZATION（要件 3/4）— 待ち時間・改行比率の境界値
// ───────────────────────────────────────────────────────────────────

/// 待ち時間の 3 表記が単一 `Wait(Duration)` へ正規化される（要件 3.1/3.2/3.3/3.4）。
/// `\w9`=450ms（=9×50）/ `\w[2]`=100ms / `\_w[500]`=500ms（絶対）。
#[test]
fn value_wait_shorthand_bracket_and_absolute_ms() {
    assert_eq!(
        parse(r"\w9\w[2]\_w[500]"),
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
/// 隣接は制御タグで挟む（旧形検出は「浮く 2 個目 `[...]`」＝`[`始まり`]`終わりの
/// Text を要するため、直後にプレーンテキストを続けると `[タイトル]` に巻かれる）。
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

// ───────────────────────────────────────────────────────────────────
// 不透明 Surface（要件 2.2/2.3）
// ───────────────────────────────────────────────────────────────────

/// `\s[...]` の中身は数値解釈・エイリアス解決せず無改変で `Surface` に入る
/// （要件 2.2/2.3）。数値・日本語エイリアスの双方を固定。
#[test]
fn surface_inner_is_opaque_and_unmodified() {
    assert_eq!(
        parse(r"\s[1000]\s[通常]"),
        vec![
            Instruction::Surface(SurfaceArg::new("1000".to_string())),
            Instruction::Surface(SurfaceArg::new("通常".to_string())),
        ],
    );
}

// ───────────────────────────────────────────────────────────────────
// 寛容パススルー（要件 10）— 不正トークン前後の正常命令が欠落しない
// ───────────────────────────────────────────────────────────────────

/// 未閉じ `[` の前後に正常命令を置き、両端が欠落せず解析が継続する
/// （要件 10.1/10.2/10.3）。`\e前\foo[` の `\e` は残り、末尾は `Raw` 吸収。
#[test]
fn lenient_passthrough_keeps_instructions_around_malformed_token() {
    let got = parse(r"\e\foo[");
    assert_eq!(got.first(), Some(&Instruction::End));
    assert_eq!(got.len(), 2);
    assert!(matches!(got[1], Instruction::Raw(_)));
}

/// move 以外の `\!` は `GenericCommand`（種別＋生引数保持）として解析継続
/// （要件 7.2/7.3/10）。`\![open,sliderinput]` を固定。
#[test]
fn lenient_passthrough_non_move_bang_is_generic_command() {
    assert_eq!(
        parse(r"\![open,sliderinput]"),
        vec![Instruction::GenericCommand {
            name: "open".to_string(),
            raw_args: vec!["sliderinput".to_string()],
        }],
    );
}

// ───────────────────────────────────────────────────────────────────
// 純粋関数契約（要件 1.5/12.2/12.3）
// ───────────────────────────────────────────────────────────────────

/// 空入力 → 空命令列（要件 1.5）。
#[test]
fn pure_empty_input_yields_empty_vec() {
    assert_eq!(parse(""), Vec::<Instruction>::new());
}

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

/// 同一入力 → 同一出力の純粋・決定的関数（要件 12.2）。
#[test]
fn pure_same_input_yields_equivalent_output() {
    let input = r"\s[1000]こんばんわー！\_w[450]\n\e";
    assert_eq!(parse(input), parse(input));
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

/// `\q[タイトル,ID]` 現行形 → disp/target 分離 Choice（要件 5.1）。
#[test]
fn handwritten_current_choice_splits_disp_and_target() {
    assert_eq!(
        parse(r"\q[はい,OnYes]"),
        vec![Instruction::Choice(Choice {
            disp: "はい".to_string(),
            target: "OnYes".to_string(),
            references: Vec::new(),
        })],
    );
}

/// `\q[t,id,r0,r1]` 第 3 引数以降の追加 Reference を順序保持（要件 5.2）。
#[test]
fn handwritten_choice_keeps_extra_references_in_order() {
    assert_eq!(
        parse(r"\q[t,id,r0,r1]"),
        vec![Instruction::Choice(Choice {
            disp: "t".to_string(),
            target: "id".to_string(),
            references: vec!["r0".to_string(), "r1".to_string()],
        })],
    );
}

/// `\![move,10,20,base,base]` → 引数 decode 済み `Move`（要件 7.1）。
/// move 以降の生引数列を保持（意味割当は下流の責務）。
#[test]
fn handwritten_move_decodes_args() {
    assert_eq!(
        parse(r"\![move,10,20,base,base]"),
        vec![Instruction::Move(MoveArgs {
            args: vec![
                "10".to_string(),
                "20".to_string(),
                "base".to_string(),
                "base".to_string(),
            ],
        })],
    );
}

/// `%username` は展開せずシステム変数トークン化（要件 8.1/8.2）。実値置換しない。
#[test]
fn handwritten_username_is_unexpanded_system_var() {
    assert_eq!(
        parse("%username"),
        vec![Instruction::SystemVar("username".to_string())],
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
