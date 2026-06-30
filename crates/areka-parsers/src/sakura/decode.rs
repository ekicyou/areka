//! 意味層（Decode）— 構文トークン → 値正規化済み `Instruction`（emo2 subset 限定）。
//!
//! Lexer（`lexer::lex`）が産んだ構文トークン列 `Vec<Token>` を消費し、
//! emo2 subset タグの値正規化を施した `Instruction` 列を産む（依存方向
//! `model ← lexer ← decode`）。下流が再パース不要なのは、この層が値を
//! decode しきるため（要件 1.2）。
//!
//! 本ファイル（タスク 4.1）が担うのは **emo2 subset の値正規化のみ**:
//! - 待ち時間 `\w[n]` / `\wN` / `\_w[ms]` → 単一 `Wait(Duration)`
//!   （`\w[n]`/`\wN` = n × 50ms、`\_w[ms]` = 絶対 ms。要件 3）。
//! - 改行 `\n` / `\n[percent]` / `\n[half]` → `NewLine(NewLineRatio)`
//!   （素の `\n` = 1.0、`percent`/100、`half` = 0.5、負値は符号付き保持。要件 4）。
//! - 選択肢 `\q[disp,target,refs...]` → `Choice`（disp/target 分離・要件 5.1/5.2）。
//! - キャラ移動 `\![move,...]` → `Move(MoveArgs)`（引数区切りのみ・要件 7.1）。
//! - 話者 `\p[n]` → `SpeakerScope`、サーフェス `\s[...]` → `Surface`（無加工保持）、
//!   カーソル `\_l[x,y]` → `Cursor`、制御 `\e`/`\c`/`\-` → `End`/`Clear`/`Quit`（要件 2/6）。
//! - システム変数 `%keyword` → `SystemVar`（展開なし・要件 8）、テキスト → `Text`（要件 9）。
//!
//! **スコープ境界（タスク 4.1 / 4.2 シーム）**: 本ファイルは emo2 subset の
//! 値正規化のみを行う。subset 外タグ・`move` 以外の `\!`・`\q` 旧 2 連形・
//! `\![*]` マーカー・不正トークンの吸収（`GenericCommand` / `Raw`）は
//! **タスク 4.2 の領分**であり、ここでは `decode_passthrough` という明示的な
//! シーム関数へ委ねる（現状は最小プレースホルダ。4.2 がここを実装する）。

// `decode` の唯一の非テスト消費者は `parse`（タスク 5）であり、本タスク（4.1）では
// まだ結線されていない。それまでの dead_code 警告は意図的に抑止する（lexer.rs と同様）。
// **タスク 5 で `parse` が `decode` を消費したら、この `#![allow(dead_code)]` を絞る/除去する**
// （真の dead を隠さぬよう）。タスク 4.2 のシーム関数（`decode_passthrough_*`）も同様に
// 結線後に実体化される。
#![allow(dead_code)]

use super::lexer::Token;
use super::model::{Choice, Instruction, MoveArgs, NewLineRatio, SurfaceArg};
use std::time::Duration;

/// 1 ウェイト単位（`\w[n]` / `\wN` の n に乗ずる基準。ukadoc 確定: 50ms）。
const WAIT_UNIT_MS: u64 = 50;

/// 構文トークン列を値正規化済みの `Instruction` 列へ写像する（mod 内・`parse` が結線）。
///
/// - 入力 `tokens` は lexer 出力（構文区切り済み）。
/// - 全 `Token` がいずれかの `Instruction` へ写像され、未デコード文字列断片は残さない
///   （要件 1.2）。出力順は入力順（要件 1.3）。
/// - 失敗しない（`Vec` を返す・`Result` でない・要件 10.2）。
pub(crate) fn decode(tokens: Vec<Token>) -> Vec<Instruction> {
    tokens.into_iter().map(decode_token).collect()
}

/// 単一の構文トークンを `Instruction` へ写像する。
///
/// emo2 subset（本タスク 4.1）に該当するものはここで値正規化し、それ以外は
/// `decode_passthrough`（タスク 4.2 のシーム）へ委ねる。
fn decode_token(token: Token) -> Instruction {
    match token {
        Token::Text(s) => Instruction::Text(s),
        Token::SysVar(keyword) => Instruction::SystemVar(keyword),
        Token::WaitShorthand(n) => Instruction::Wait(wait_units(n as u64)),
        Token::Bare(c) => decode_bare(c),
        Token::Tag { word, args } => decode_tag(word, args),
        // タスク 4.2 のシーム: 構文上区切れたが正準でない／不正な生保持。
        Token::Raw(s) => decode_passthrough_raw(s),
    }
}

/// bare タグ（角括弧なし 1 文字）を写像する。
///
/// `\n`（bare）は既定比率 1.0 の改行（要件 4.2）。`\e`/`\c`/`\-` は制御命令（要件 6.2-6.4）。
fn decode_bare(c: char) -> Instruction {
    match c {
        'e' => Instruction::End,
        'c' => Instruction::Clear,
        '-' => Instruction::Quit,
        'n' => Instruction::NewLine(NewLineRatio::new(1.0)),
        // subset 外の bare タグ（`\0` `\1` 等）はタスク 4.2 のパススルー領分。
        other => decode_passthrough_bare(other),
    }
}

/// 正準タグ `\word[args]` を写像する。
///
/// word ごとに emo2 subset の値正規化を施す。subset 外 word はタスク 4.2 のシームへ。
fn decode_tag(word: String, args: Vec<String>) -> Instruction {
    match word.as_str() {
        // 待ち時間（要件 3.1）: `\w[n]` = n × 50ms。
        "w" => Instruction::Wait(wait_from_arg(args.first())),
        // 待ち時間（要件 3.3）: `\_w[ms]` = 絶対 ms。
        "_w" => Instruction::Wait(wait_absolute_ms(args.first())),
        // 改行（要件 4.1）: `\n[percent]` = percent/100、`\n[half]` = 0.5。
        "n" => Instruction::NewLine(newline_ratio_from_arg(args.first())),
        // 話者スコープ（要件 2.1）: `\p[n]`。
        "p" => Instruction::SpeakerScope {
            n: speaker_scope_n(args.first()),
        },
        // サーフェス（要件 2.2/2.3）: `\s[...]` 中身は不透明文字列で無加工保持。
        "s" => Instruction::Surface(SurfaceArg::new(args.into_iter().next().unwrap_or_default())),
        // カーソル絶対位置（要件 6.1）: `\_l[x,y]`（x/y は文字列のまま保持）。
        "_l" => decode_cursor(args),
        // 選択肢（要件 5.1/5.2）: `\q[disp,target,refs...]`。
        "q" => decode_choice(args),
        // `\!` コマンド（要件 7.1）: 第 1 引数が `move` のみ本タスクで Move へ decode。
        // move 以外（要件 7.2/7.3）はタスク 4.2 の GenericCommand 領分。
        "!" => decode_bang(args),
        // subset 外タグ（`\b` `\i` 等）はタスク 4.2 のパススルー領分。
        _ => decode_passthrough_tag(word, args),
    }
}

/// `\_l[x,y]` → カーソル絶対位置。x/y は文字列のまま保持（要件 6.1・design 値定数）。
fn decode_cursor(args: Vec<String>) -> Instruction {
    let mut it = args.into_iter();
    let x = it.next().unwrap_or_default();
    let y = it.next().unwrap_or_default();
    Instruction::Cursor { x, y }
}

/// `\q[disp,target,refs...]` → disp/target 分離 ＋ 追加 references（順序保持・要件 5.1/5.2）。
fn decode_choice(args: Vec<String>) -> Instruction {
    let mut it = args.into_iter();
    let disp = it.next().unwrap_or_default();
    let target = it.next().unwrap_or_default();
    let references: Vec<String> = it.collect();
    Instruction::Choice(Choice {
        disp,
        target,
        references,
    })
}

/// `\![...]` の第 1 引数で分岐する。`move` のみ本タスクで `Move` へ decode（要件 7.1）。
///
/// `move` 以外（および空）はタスク 4.2 のシーム（`GenericCommand`）へ委ねる。
fn decode_bang(args: Vec<String>) -> Instruction {
    match args.first().map(String::as_str) {
        Some("move") => Instruction::Move(MoveArgs {
            args: args.into_iter().skip(1).collect(),
        }),
        // move 以外の `\!`（要件 7.2/7.3）・`\![*]` マーカー（要件 5.4）はタスク 4.2 領分。
        _ => decode_passthrough_bang(args),
    }
}

/// n ウェイト単位を `Duration` へ（`\w[n]` / `\wN` = n × 50ms・要件 3.1/3.2）。
fn wait_units(n: u64) -> Duration {
    Duration::from_millis(n.saturating_mul(WAIT_UNIT_MS))
}

/// `\w[n]` の引数 n（10 進）から待ち時間を求める。引数欠落・非数は 0 ウェイト。
fn wait_from_arg(arg: Option<&String>) -> Duration {
    let n = arg.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    wait_units(n)
}

/// `\_w[ms]` の引数（絶対ミリ秒・要件 3.3）から待ち時間を求める。引数欠落・非数は 0ms。
fn wait_absolute_ms(arg: Option<&String>) -> Duration {
    let ms = arg.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    Duration::from_millis(ms)
}

/// `\n[percent]` / `\n[half]` から比率を求める（要件 4.1・design 値定数）。
///
/// - `half` → 0.5。
/// - 数値 percent → percent / 100（`150` → 1.5・負値は符号付き保持）。
/// - 引数欠落・非数 → 既定 1.0（素の `\n` と同等）。
fn newline_ratio_from_arg(arg: Option<&String>) -> NewLineRatio {
    match arg.map(String::as_str) {
        Some("half") => NewLineRatio::new(0.5),
        Some(s) => match s.parse::<f32>() {
            Ok(percent) => NewLineRatio::new(percent / 100.0),
            Err(_) => NewLineRatio::new(1.0),
        },
        None => NewLineRatio::new(1.0),
    }
}

/// `\p[n]` の話者スコープ番号（要件 2.1）。引数欠落・非数は 0。
fn speaker_scope_n(arg: Option<&String>) -> u32 {
    arg.and_then(|s| s.parse::<u32>().ok()).unwrap_or(0)
}

// ───────────────────────────────────────────────────────────────────
// タスク 4.1 / 4.2 シーム（寛容パススルー）
//
// 以下は **タスク 4.2 の領分**（`move` 以外の `\!` → GenericCommand、subset 外
// タグ・不正トークン → Raw、`\q` 旧 2 連形、`\![*]` マーカー）の明示的な接続点。
// タスク 4.1 では emo2 subset の正しい decode を最優先し、subset 外は意味を
// 詐称しない最小プレースホルダ（生情報を失わない `Raw`）に留める。4.2 がこれらを
// 正式な吸収規則（GenericCommand / Raw / Choice 畳み込み）へ差し替える。
// ───────────────────────────────────────────────────────────────────

/// 【タスク 4.2 シーム】subset 外の正準タグ。生情報を保持して `Raw` 化（最小）。
fn decode_passthrough_tag(word: String, args: Vec<String>) -> Instruction {
    Instruction::Raw(reconstruct_tag(&word, &args))
}

/// 【タスク 4.2 シーム】`move` 以外（および空）の `\!`。生情報を保持して `Raw` 化（最小）。
fn decode_passthrough_bang(args: Vec<String>) -> Instruction {
    Instruction::Raw(reconstruct_tag("!", &args))
}

/// 【タスク 4.2 シーム】subset 外の bare タグ。生情報を保持して `Raw` 化（最小）。
fn decode_passthrough_bare(c: char) -> Instruction {
    Instruction::Raw(format!("\\{c}"))
}

/// 【タスク 4.2 シーム】lexer が区切れず `Raw` 吸収した断片。そのまま `Raw` で保持。
fn decode_passthrough_raw(s: String) -> Instruction {
    Instruction::Raw(s)
}

/// `Raw` プレースホルダ用に `\word[args]` を概形再構成する（情報を失わないため）。
fn reconstruct_tag(word: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("\\{word}")
    } else {
        format!("\\{word}[{}]", args.join(","))
    }
}
