//! 構文層（Lexer）— さくらスクリプトの一般構文分割（手書き線形スキャナ）。
//!
//! 入力を `char_indices` で UTF-8 走査し、構文トークン列へ分割する
//! （charset 変換なし・要件 12.1）。意味デコードは行わない（→ `decode`）。
//!
//! 本ファイル（タスク 3.1）が担うのは**一般構文分割**のみ:
//! - 正準タグ（`\` ＋ word ＋ `[args]`）— `[` がワード終端、`]` が引数終端（要件 13.2）。
//! - bare タグ（`\e` `\c` `\-` `\n`）— 角括弧を伴わない 1 文字タグ（要件 13.1）。
//! - `\wN` 短縮（N は 1 桁）— 角括弧を伴わない短縮待ち（要件 3.2 の構文側）。
//! - `%keyword` システム変数（要件 8.1）。
//! - タグ間テキスト（要件 9.1/9.2）。
//! - 角括弧内のカンマ区切り複数引数分割（要件 13.3）。
//!
//! エスケープ（`\\` / `\%` / 角内 `\]`・要件 13.5-13.7）、引数クォート
//! （`"..."` / `""`・要件 13.4）、未閉じ `[`/`"` 等の寛容境界吸収（要件 10.3/13.8）は
//! **タスク 3.2 の領分**ゆえ本ファイルでは扱わない。3.2 はこの線形スキャナの
//! 各読み取りヘルパ（角括弧引数走査・テキスト走査）にエスケープ／クォート解決と
//! `Raw` 吸収を差し込む。`Raw` variant はその吸収先として今から定義しておく。
//!
//! 本モジュールの実体は現状テストからのみ参照される（`decode`＝タスク 4.x が
//! 唯一の非テスト消費者になる）。それまでの dead_code 警告は意図的に抑止する。
#![allow(dead_code)]

/// 構文トークン（lexer 内部型・**非公開**）。
///
/// `Instruction`（公開 I/O 契約）とは別物で、`sakura` モジュール外へは公開しない
/// （`pub(crate)`）。`decode`（タスク 4.x）がこの列を `Instruction` へ写像する。
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    /// 正準形 `\word[args]`（word は `[` まで、args は `]` までをカンマ分割）。
    Tag { word: String, args: Vec<String> },
    /// bare タグ `\e` `\c` `\-` `\n`（角括弧なし 1 文字）。
    Bare(char),
    /// `\wN`（N は 1 桁の待ち短縮）。
    WaitShorthand(u8),
    /// `%keyword`（システム変数・`%` を除いたキーワードを保持）。
    SysVar(String),
    /// タグ間プレーンテキスト。
    Text(String),
    /// 区切れたが正準でない／不正（タスク 3.2 が吸収先として使用）。
    Raw(String),
}

/// `\wN` 短縮の対象語（角括弧を伴わなければ短縮形として 1 桁を読む）。
const SHORTHAND_WORDS: &[char] = &['w'];

/// さくらスクリプト文字列を構文トークン列へ分割する（mod 内非公開）。
///
/// - 入力は UTF-8 前提（要件 12.1）。`char_indices` で 1 パス線形走査する。
/// - トークンは入力順を保持する（要件 9.2/1.3）。
/// - 本タスク（3.1）はエスケープ／クォート／寛容境界を扱わない（3.2 が追加）。
pub(crate) fn lex(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    // 蓄積中のテキストラン（タグ／sysvar の手前で flush する）。
    let mut text = String::new();
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut i = 0;

    macro_rules! flush_text {
        () => {
            if !text.is_empty() {
                tokens.push(Token::Text(std::mem::take(&mut text)));
            }
        };
    }

    while i < chars.len() {
        let (_, c) = chars[i];
        match c {
            '\\' => {
                flush_text!();
                let (tok, next) = scan_tag(&chars, i);
                tokens.push(tok);
                i = next;
            }
            '%' => {
                flush_text!();
                let (tok, next) = scan_sysvar(&chars, i);
                tokens.push(tok);
                i = next;
            }
            other => {
                text.push(other);
                i += 1;
            }
        }
    }
    flush_text!();
    tokens
}

/// `\` で始まるタグを走査する。`i` は `\` の位置。次に読むべき添字を返す。
///
/// 形態:
/// - `\word[args]` → `Token::Tag`（`[` がワード終端、`]` まで引数）。
/// - `\wN`（短縮対象語＋1 桁）→ `Token::WaitShorthand`。
/// - `\X`（その他 1 文字、角括弧なし）→ `Token::Bare`。
fn scan_tag(chars: &[(usize, char)], i: usize) -> (Token, usize) {
    // `\` の次の文字が無ければ（入力末尾の裸 `\`）bare 扱いで継続。
    let Some(&(_, first)) = chars.get(i + 1) else {
        return (Token::Bare('\\'), i + 1);
    };

    // ワード（コマンド語）を読む: 角括弧／バックスラッシュ／`%`／空白に当たるまで。
    // ただし短縮対象語（`\w`）は 1 文字読んだ時点で次が数字なら短縮形を優先する。
    let word_start = i + 1;
    let mut j = word_start;

    // 先頭 1 文字を確定（少なくとも word は 1 文字ある）。
    j += 1;

    // 短縮形判定: word が 1 文字（短縮対象語）で、続く文字が 1 桁数字、かつ
    // その数字の次が `[` でない（`\w[2]` は正準タグ）。
    if SHORTHAND_WORDS.contains(&first)
        && let Some(&(_, d)) = chars.get(j)
        && d.is_ascii_digit()
        && chars.get(j + 1).map(|&(_, c)| c) != Some('[')
    {
        let n = (d as u8) - b'0';
        return (Token::WaitShorthand(n), j + 1);
    }

    // それ以外: ワードを `[` まで（または非ワード文字まで）読み進める。
    while let Some(&(_, c)) = chars.get(j) {
        if c == '[' || c == '\\' || c == '%' {
            break;
        }
        j += 1;
    }

    let word: String = chars[word_start..j].iter().map(|&(_, c)| c).collect();

    // 角括弧があれば引数を走査、無ければ bare タグ。
    if let Some(&(_, '[')) = chars.get(j) {
        let (args, next) = scan_bracket_args(chars, j);
        (Token::Tag { word, args }, next)
    } else {
        // 角括弧なし。word は通常 1 文字（`\e` `\c` `\-` `\n` 等）。
        // 1 文字のみを bare として消費し、残りはテキスト／後続走査へ委ねる。
        let bare = first;
        (Token::Bare(bare), word_start + 1)
    }
}

/// `[` から始まる角括弧引数をカンマ区切りで走査する。`j` は `[` の位置。
/// `]` の次の添字を返す。
///
/// 本タスク（3.1）では単純なカンマ分割のみ（エスケープ `\]`・クォート `"..."` は
/// タスク 3.2 が本関数へ差し込む）。`]` が無い未閉じケースの寛容吸収も 3.2 の領分。
fn scan_bracket_args(chars: &[(usize, char)], j: usize) -> (Vec<String>, usize) {
    debug_assert_eq!(chars.get(j).map(|&(_, c)| c), Some('['));
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut k = j + 1; // `[` の次から。
    let mut closed = false;

    while let Some(&(_, c)) = chars.get(k) {
        match c {
            ']' => {
                k += 1;
                closed = true;
                break;
            }
            ',' => {
                args.push(std::mem::take(&mut cur));
                k += 1;
            }
            other => {
                cur.push(other);
                k += 1;
            }
        }
    }

    // 引数を確定: `[]`（空）は引数 0 個。何か読んだら最後の 1 個を push。
    if !cur.is_empty() || !args.is_empty() {
        args.push(cur);
    }

    // 未閉じ（`closed == false`）の寛容処理はタスク 3.2 が担う。
    let _ = closed;
    (args, k)
}

/// `%` で始まるシステム変数を走査する。`i` は `%` の位置。次の添字を返す。
///
/// キーワードは英数字／アンダースコアを読み進める（続く非該当文字はテキストへ）。
fn scan_sysvar(chars: &[(usize, char)], i: usize) -> (Token, usize) {
    let start = i + 1;
    let mut j = start;
    while let Some(&(_, c)) = chars.get(j) {
        if c.is_ascii_alphanumeric() || c == '_' {
            j += 1;
        } else {
            break;
        }
    }
    let keyword: String = chars[start..j].iter().map(|&(_, c)| c).collect();
    (Token::SysVar(keyword), j)
}
