//! `parse` 公開 facade の単体テスト（タスク 5・lexer→decode 結線）。
//!
//! 検証対象は公開エントリ `areka_parsers::sakura::parse`（`pub fn parse(&str) -> Vec<Instruction>`）:
//! - 空入力 → 空命令列（要件 1.5）。
//! - タグ＋テキスト混在 → 入力順保持の型付き命令列（要件 1.1/1.3/9.2）。
//! - 同一入力 → 同一出力の純粋・決定的関数（要件 12.2）。
//! - 命令を実行・解釈しない／エラーを送出しない（要件 1.4/10.2）。
//! - 不正トークン前後の正常命令を欠落させない（要件 10.3）。
//!
//! parse は lexer→decode の結線にすぎないため、ここでは個々の値正規化の境界値ではなく
//! 「結線が順序保持で正しく行われること」を固定する（境界値は decode_tests の領分）。

use super::model::{Choice, Instruction, NewLineRatio};
use super::parse::parse;
use std::time::Duration;

/// 空入力 → 空 `Vec`（要件 1.5）。命令を一切産まない。
#[test]
fn empty_input_yields_empty_vec() {
    assert_eq!(parse(""), Vec::<Instruction>::new());
    assert!(parse("").is_empty());
}

/// タグ＋テキスト混在が入力順を保持して型付き命令列になる（要件 1.1/1.3/9.2）。
/// `\s[0]テキスト\n\e` → Surface, Text, NewLine, End の順。
#[test]
fn mixed_tags_and_text_preserve_input_order() {
    let got = parse(r"\s[0]こんにちは\n\e");
    let want = vec![
        Instruction::Surface(super::model::SurfaceArg::new("0".to_string())),
        Instruction::Text("こんにちは".to_string()),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::End,
    ];
    assert_eq!(got, want);
}

/// 複数タグ＋テキストの長めの混在でも順序が保たれる（要件 1.3）。
/// `\p[0]\s[10]はい\w[2]\q[ok,id0]いいえ` の順序を固定。
#[test]
fn longer_mixed_sequence_preserves_order() {
    let got = parse(r"\p[0]\s[10]はい\w[2]\q[ok,id0]いいえ");
    let want = vec![
        Instruction::SpeakerScope { n: 0 },
        Instruction::Surface(super::model::SurfaceArg::new("10".to_string())),
        Instruction::Text("はい".to_string()),
        Instruction::Wait(Duration::from_millis(100)),
        Instruction::Choice(Choice {
            disp: "ok".to_string(),
            target: "id0".to_string(),
            references: Vec::new(),
        }),
        Instruction::Text("いいえ".to_string()),
    ];
    assert_eq!(got, want);
}

/// 同一入力 → 同一出力の純粋・決定的関数（要件 12.2）。2 回呼び出しで等価。
#[test]
fn same_input_yields_same_output_pure() {
    let input = r"\s[0]あ\w[1]い\n\e%username";
    let a = parse(input);
    let b = parse(input);
    assert_eq!(a, b);
}

/// 純粋テキストのみ（タグ無し）→ 単一 `Text`（要件 9.1）。
#[test]
fn plain_text_only_yields_single_text() {
    assert_eq!(
        parse("ただのテキスト"),
        vec![Instruction::Text("ただのテキスト".to_string())]
    );
}

/// 不正トークン（未閉じ `[`）の前にある正常命令は欠落しない（要件 10.3/10.2）。
/// `\e\foo[` → End ＋ 未閉じ吸収の `Raw`（エラーを送出せず継続）。
#[test]
fn malformed_token_does_not_drop_preceding_instruction() {
    let got = parse(r"\e\foo[");
    assert_eq!(got.first(), Some(&Instruction::End));
    assert_eq!(got.len(), 2);
    assert!(matches!(got[1], Instruction::Raw(_)));
}

/// システム変数 `%keyword` は展開せずトークン保持（要件 8.1）。混在順も保持。
#[test]
fn system_var_kept_as_token_in_order() {
    let got = parse("hi %username !");
    assert_eq!(
        got,
        vec![
            Instruction::Text("hi ".to_string()),
            Instruction::SystemVar("username".to_string()),
            Instruction::Text(" !".to_string()),
        ]
    );
}
