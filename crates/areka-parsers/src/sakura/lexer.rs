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
//! **タスク 3.2 で実装済み**:
//! - `\\` → リテラル `\`、`\%` → リテラル `%` は `lex` のテキスト走査で解決する。
//! - 角内 `\]` → リテラル `]`、クォート `"..."`（内側 `,` 保護・`""` → `"`）は
//!   `scan_bracket_args` で解決する。
//! - 未閉じ `[` / 未閉じ `"` は `\` から入力末尾までを `Token::Raw` として吸収し、
//!   走査を中断しない（前後の正常トークンは欠落しない）。
//!
//! 本モジュールの実体は `decode`（→ `parse`）から非テスト経路で到達される
//! （依存方向 `model ← lexer ← decode ← parse`・タスク 5 で結線済み）。

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
    /// 短縮形 `\<word>N`（`word` は短縮対象語・`N` は 1 桁）。
    ///
    /// 待ち短縮 `\wN`（`word='w'`）と、バルーン面短縮 `\bN`（`word='b'`）を
    /// 語に依らず同一規律で表す一般化トークン。意味写像（`'w'`→待ち／
    /// `'b'`→バルーン面）は `decode` の責務（→ 対象語は `SHORTHAND_WORDS`）。
    Shorthand { word: char, n: u8 },
    /// `%keyword`（システム変数・`%` を除いたキーワードを保持）。
    SysVar(String),
    /// タグ間プレーンテキスト。
    Text(String),
    /// 区切れたが正準でない／不正（未閉じ `[` / `"` 等を verbatim 吸収・要件 10.3/13.8）。
    Raw(String),
}

/// 短縮形の対象語（角括弧を伴わなければ短縮形として 1 桁を読む）。
///
/// 待ち `\wN` に加え、バルーン面 `\bN`（要件 1.2）も同一の shorthand 規律で扱う。
/// 判定規則（1 桁数字・直後 `[` なら正準タグ優先）は語に依らず共通。
const SHORTHAND_WORDS: &[char] = &['w', 'b'];

/// さくらスクリプト文字列を構文トークン列へ分割する（mod 内非公開）。
///
/// - 入力は UTF-8 前提（要件 12.1）。`char_indices` で 1 パス線形走査する。
/// - トークンは入力順を保持する（要件 9.2/1.3）。
/// - エスケープ（`\\`/`\%`/角内 `\]`）・引数クォート（`"..."`/`""`）・未閉じ境界の
///   `Raw` 吸収を解決する（タスク 3.2・要件 13.4-13.8/10.3）。解析は中断しない。
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
                // エスケープ `\\` / `\%`（要件 13.5/13.6）: タグ開始でなくリテラル
                // 1 文字としてテキストへ取り込む（flush せず text ランを継続）。
                match chars.get(i + 1).map(|&(_, c)| c) {
                    Some('\\') => {
                        text.push('\\');
                        i += 2;
                        continue;
                    }
                    Some('%') => {
                        text.push('%');
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
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
/// - `\<word>N`（短縮対象語＋1 桁・`\wN`/`\bN`）→ `Token::Shorthand`。
/// - `\X`（その他 1 文字、角括弧なし）→ `Token::Bare`。
fn scan_tag(chars: &[(usize, char)], i: usize) -> (Token, usize) {
    // `\` の次の文字が無ければ（入力末尾の裸 `\`）bare 扱いで継続。
    let Some(&(_, first)) = chars.get(i + 1) else {
        return (Token::Bare('\\'), i + 1);
    };

    // ワード（コマンド語）を読む: 角括弧／バックスラッシュ／`%`／空白に当たるまで。
    // ただし短縮対象語（`\w`/`\b`）は 1 文字読んだ時点で次が数字なら短縮形を優先する。
    let word_start = i + 1;
    let mut j = word_start;

    // 先頭 1 文字を確定（少なくとも word は 1 文字ある）。
    j += 1;

    // 短縮形判定（語に依らず共通・要件 1.2）: word が 1 文字（短縮対象語）で、
    // 続く文字が 1 桁数字、かつその数字の次が `[` でない（`\w[2]`/`\b2[x]` は正準タグ）。
    if SHORTHAND_WORDS.contains(&first)
        && let Some(&(_, d)) = chars.get(j)
        && d.is_ascii_digit()
        && chars.get(j + 1).map(|&(_, c)| c) != Some('[')
    {
        let n = (d as u8) - b'0';
        return (Token::Shorthand { word: first, n }, j + 1);
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
        match scan_bracket_args(chars, j) {
            BracketScan::Closed { args, next } => (Token::Tag { word, args }, next),
            // 未閉じ `[` / `"`（要件 10.3/13.8）: 区切れないため `\` から入力末尾までを
            // 1 単位の `Raw` として吸収し、走査を中断しない（クラッシュさせない）。
            BracketScan::Unclosed => {
                let raw: String = chars[i..].iter().map(|&(_, c)| c).collect();
                (Token::Raw(raw), chars.len())
            }
        }
    } else {
        // 角括弧なし。word は通常 1 文字（`\e` `\c` `\-` `\n` 等）。
        // 1 文字のみを bare として消費し、残りはテキスト／後続走査へ委ねる。
        let bare = first;
        (Token::Bare(bare), word_start + 1)
    }
}

/// `scan_bracket_args` の結果。閉じた `]` まで正しく区切れたか、未閉じかを区別する。
enum BracketScan {
    /// `]` で閉じた。`args` は確定引数、`next` は `]` の次の添字。
    Closed { args: Vec<String>, next: usize },
    /// `]`（または閉じ `"`）が無いまま入力末尾に達した（寛容 `Raw` 吸収対象）。
    Unclosed,
}

/// `[` から始まる角括弧引数を走査する。`j` は `[` の位置。
///
/// 区切り規則（要件 13.3/13.4/13.7）:
/// - カンマ `,` を引数区切りとする。
/// - 角括弧内のエスケープ `\]` はリテラルの `]` として引数へ取り込む（要件 13.7）。
/// - クォート `"..."` は囲まれた全体を 1 引数扱い（内側 `,` を保護）、二重化 `""` は
///   リテラル `"` として取り込む（要件 13.4）。
///
/// `]` が現れないまま末尾に達した（または `"` が閉じないまま末尾）場合は
/// `Unclosed` を返し、呼び出し側が `Raw` として吸収する（要件 10.3/13.8）。
fn scan_bracket_args(chars: &[(usize, char)], j: usize) -> BracketScan {
    debug_assert_eq!(chars.get(j).map(|&(_, c)| c), Some('['));
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    // 現在の引数でクォート `"..."` が開かれたか（要件 13.4）。sole `[""]` を
    // 空文字列 1 引数として確定するための識別子。`cur` が空でも「クォートを
    // 消費した」ことを覚え、`[]`（真の空・0 引数）と区別する。
    let mut quote_consumed = false;
    let mut k = j + 1; // `[` の次から。

    loop {
        let Some(&(_, c)) = chars.get(k) else {
            // 閉じ `]` 無しで末尾 → 未閉じ（寛容 `Raw` 吸収）。
            return BracketScan::Unclosed;
        };
        match c {
            ']' => {
                k += 1;
                // 引数を確定: `[]`（真の空）は引数 0 個。何か読んだ／カンマで
                // 既に push 済み／クォートを消費した（sole `[""]`・要件 13.4）
                // いずれかなら最後の 1 個を push する。
                if !cur.is_empty() || !args.is_empty() || quote_consumed {
                    args.push(cur);
                }
                return BracketScan::Closed { args, next: k };
            }
            ',' => {
                args.push(std::mem::take(&mut cur));
                // 次の引数は新規。クォート消費フラグをリセットする。
                quote_consumed = false;
                k += 1;
            }
            '\\' if chars.get(k + 1).map(|&(_, c)| c) == Some(']') => {
                // 角内 `\]` → リテラル `]`（要件 13.7）。
                cur.push(']');
                k += 2;
            }
            '"' => {
                // クォート開始（要件 13.4）。閉じ `"` まで読み、内側 `,` は保護。
                // 二重化 `""` はリテラル `"` として取り込む。
                // この引数はクォートを消費した（sole `[""]` を空 1 引数へ確定するため）。
                quote_consumed = true;
                k += 1; // 開き `"` を消費。
                loop {
                    let Some(&(_, qc)) = chars.get(k) else {
                        // クォート未閉じで末尾 → 未閉じ（寛容 `Raw` 吸収）。
                        return BracketScan::Unclosed;
                    };
                    if qc == '"' {
                        if chars.get(k + 1).map(|&(_, c)| c) == Some('"') {
                            // `""` → リテラル `"`。
                            cur.push('"');
                            k += 2;
                        } else {
                            // クォート終端。
                            k += 1;
                            break;
                        }
                    } else {
                        cur.push(qc);
                        k += 1;
                    }
                }
            }
            other => {
                cur.push(other);
                k += 1;
            }
        }
    }
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
