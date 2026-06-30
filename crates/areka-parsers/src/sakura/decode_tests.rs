//! `decode` の単体テスト（emo2 subset 値正規化・タスク 4.1 範囲）。
//!
//! 検証対象は構文トークン（`lexer::Token`）→ 値正規化済み `Instruction` の写像:
//! - 待ち時間（`\w[n]` / `\wN` = n×50ms、`\_w[ms]` = 絶対 ms）の境界値。
//! - 改行比率（素の `\n` = 1.0、`\n[percent]` = percent/100、`\n[half]` = 0.5、負値）。
//! - 選択肢 `\q[disp,target,refs...]` の disp/target 分離 ＋ references 順序保持。
//! - `\![move,...]` の Move 化、`\p[n]`/`\s[...]`/`\_l[x,y]`/`\e`/`\c`/`\-`、`%keyword`、テキスト。
//!
//! `move` 以外の `\!`・subset 外タグ・`\q` 旧 2 連形・`\![*]`（タスク 4.2）はここでは検証しない。
//!
//! decode は lexer を消費するため、入力は実スクリプト文字列を `lex` した結果を渡す
//! （構文区切りと意味デコードの結線が正しいことも同時に固定する）。

use super::decode::decode;
use super::lexer::lex;
use super::model::{Choice, Instruction, MoveArgs, NewLineRatio, SurfaceArg};
use std::time::Duration;

/// 文字列をレックスしてデコードするヘルパ（テスト題材は実スクリプト断片）。
fn dec(input: &str) -> Vec<Instruction> {
    decode(lex(input))
}

// ── 待ち時間（要件 3）─────────────────────────────────────────────

/// `\wN` 短縮の境界: `\w1` → 50ms、`\w9` → 450ms（n × 50ms・要件 3.2）。
#[test]
fn wait_shorthand_boundary_values() {
    assert_eq!(dec(r"\w1"), vec![Instruction::Wait(Duration::from_millis(50))]);
    assert_eq!(dec(r"\w9"), vec![Instruction::Wait(Duration::from_millis(450))]);
}

/// `\w[n]` 正準形: `\w[2]` → 100ms、`\w[9]` → 450ms（n × 50ms・要件 3.1）。
#[test]
fn wait_bracket_n_times_50ms() {
    assert_eq!(dec(r"\w[2]"), vec![Instruction::Wait(Duration::from_millis(100))]);
    assert_eq!(dec(r"\w[9]"), vec![Instruction::Wait(Duration::from_millis(450))]);
}

/// `\_w[ms]` 絶対ミリ秒: `\_w[450]` → 450ms、`\_w[950]` → 950ms（要件 3.3）。
#[test]
fn wait_underscore_absolute_ms() {
    assert_eq!(dec(r"\_w[450]"), vec![Instruction::Wait(Duration::from_millis(450))]);
    assert_eq!(dec(r"\_w[950]"), vec![Instruction::Wait(Duration::from_millis(950))]);
}

/// `\w`系/`\_w`系のいずれも同一 `Wait` variant（正規化済み Duration）で表現（要件 3.4）。
#[test]
fn wait_variants_unified_to_same_duration() {
    // \w9 = 450ms と \_w[450] = 450ms は同一 Instruction。
    assert_eq!(dec(r"\w9"), dec(r"\_w[450]"));
}

// ── 改行と割合改行（要件 4）─────────────────────────────────────

/// 素の `\n`（bare）→ 既定比率 1.0（要件 4.2）。
#[test]
fn newline_bare_default_ratio_one() {
    assert_eq!(
        dec(r"\n"),
        vec![Instruction::NewLine(NewLineRatio::new(1.0))],
    );
}

/// `\n[percent]` → percent/100（`\n[150]` → 1.5・要件 4.1）。
#[test]
fn newline_percent_ratio() {
    assert_eq!(
        dec(r"\n[150]"),
        vec![Instruction::NewLine(NewLineRatio::new(1.5))],
    );
    assert_eq!(
        dec(r"\n[100]"),
        vec![Instruction::NewLine(NewLineRatio::new(1.0))],
    );
}

/// `\n[half]` → 0.5（design 値定数）。
#[test]
fn newline_half_ratio() {
    assert_eq!(
        dec(r"\n[half]"),
        vec![Instruction::NewLine(NewLineRatio::new(0.5))],
    );
}

/// 負値の `\n[-100]` → -1.0（戻り・符号付き保持・design 値定数）。
#[test]
fn newline_negative_ratio_back() {
    assert_eq!(
        dec(r"\n[-100]"),
        vec![Instruction::NewLine(NewLineRatio::new(-1.0))],
    );
}

// ── 話者スコープとサーフェス（要件 2）──────────────────────────

/// `\p[0]` / `\p[1]` → 話者スコープ番号（要件 2.1）。
#[test]
fn speaker_scope_number() {
    assert_eq!(dec(r"\p[0]"), vec![Instruction::SpeakerScope { n: 0 }]);
    assert_eq!(dec(r"\p[1]"), vec![Instruction::SpeakerScope { n: 1 }]);
}

/// `\s[...]` の中身は不透明文字列のまま無加工保持（要件 2.2/2.3）。
#[test]
fn surface_opaque_content_preserved() {
    // 数値も日本語エイリアスも改変しない。
    assert_eq!(
        dec(r"\s[1000]"),
        vec![Instruction::Surface(SurfaceArg::new("1000".to_string()))],
    );
    assert_eq!(
        dec(r"\s[通常]"),
        vec![Instruction::Surface(SurfaceArg::new("通常".to_string()))],
    );
}

// ── 選択肢（要件 5）────────────────────────────────────────────

/// `\q[タイトル,ID]` → disp/target 分離（references 空・要件 5.1）。
#[test]
fn choice_disp_target_split() {
    assert_eq!(
        dec(r"\q[はい,OnYes]"),
        vec![Instruction::Choice(Choice {
            disp: "はい".to_string(),
            target: "OnYes".to_string(),
            references: vec![],
        })],
    );
}

/// `\q[タイトル,ID,r0,r1]` → 第 3 引数以降を references に順序保持（要件 5.2）。
#[test]
fn choice_extra_references_order_preserved() {
    assert_eq!(
        dec(r"\q[t,id,r0,r1]"),
        vec![Instruction::Choice(Choice {
            disp: "t".to_string(),
            target: "id".to_string(),
            references: vec!["r0".to_string(), "r1".to_string()],
        })],
    );
}

// ── カーソル位置と制御命令（要件 6）───────────────────────────

/// `\_l[x,y]` → カーソル絶対位置（x/y は文字列のまま・要件 6.1）。
#[test]
fn cursor_absolute_position() {
    assert_eq!(
        dec(r"\_l[10,20]"),
        vec![Instruction::Cursor {
            x: "10".to_string(),
            y: "20".to_string(),
        }],
    );
}

/// `\e` → End、`\c` → Clear、`\-` → Quit（要件 6.2-6.4）。
#[test]
fn control_tags_end_clear_quit() {
    assert_eq!(dec(r"\e"), vec![Instruction::End]);
    assert_eq!(dec(r"\c"), vec![Instruction::Clear]);
    assert_eq!(dec(r"\-"), vec![Instruction::Quit]);
}

// ── キャラ移動（要件 7.1 — move のみ・タスク 4.1）─────────────

/// `\![move,dx,dy]` → Move（move を除いた生引数列を保持・要件 7.1）。
#[test]
fn move_command_decoded_with_args() {
    assert_eq!(
        dec(r"\![move,10,20]"),
        vec![Instruction::Move(MoveArgs {
            args: vec!["10".to_string(), "20".to_string()],
        })],
    );
}

// ── システム変数（要件 8）─────────────────────────────────────

/// `%username` → 展開なし SystemVar トークン（要件 8.1/8.2）。
#[test]
fn system_var_token_unexpanded() {
    assert_eq!(
        dec(r"%username"),
        vec![Instruction::SystemVar("username".to_string())],
    );
}

// ── テキストラン（要件 9）─────────────────────────────────────

/// タグ間プレーンテキスト → Text run（UTF-8 日本語・要件 9.1）。
#[test]
fn text_run_preserved() {
    assert_eq!(
        dec("こんばんわー！"),
        vec![Instruction::Text("こんばんわー！".to_string())],
    );
}

// ── 順序保持 ＋ 未デコード断片を残さない（要件 1.2/1.3）──────

/// 複数 subset タグ＋テキスト混在が入力順を保持し、全トークンが
/// いずれかの `Instruction` へ写像される（未デコード文字列断片を残さない・要件 1.2/1.3）。
#[test]
fn mixed_subset_tags_order_preserved_no_undecoded_fragment() {
    let out = dec(r"\p[0]\s[1000]こんにちは\_w[450]\n[150]\e");
    assert_eq!(
        out,
        vec![
            Instruction::SpeakerScope { n: 0 },
            Instruction::Surface(SurfaceArg::new("1000".to_string())),
            Instruction::Text("こんにちは".to_string()),
            Instruction::Wait(Duration::from_millis(450)),
            Instruction::NewLine(NewLineRatio::new(1.5)),
            Instruction::End,
        ],
    );
}
