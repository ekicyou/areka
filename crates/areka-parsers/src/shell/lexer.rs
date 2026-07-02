//! 構文層（Lexer）— surfaces.txt の一般構文分割（手書き線形スキャナ）。
//!
//! 入力（UTF-8 デコード済み `&str`・charset 変換は上流 foundation の領分）を
//! **行指向**で線形走査し、構文トークン列へ分割する。意味割当は行わない（→ `decode`）。
//!
//! surfaces.txt は sakura のインライン `\tag[args]` 走査と構文クラスが異なり、
//! ブロック構造（ヘッダ行＋別行 `{` … `}`）＋行指向 CSV ＋ドット付きキー ＋
//! `[id,...]` 配列値で構成される。ゆえに走査コードは sakura から流用できないが、
//! 「`char_indices`/線形・`Raw` 寛容吸収・走査を中断しない・トークンは `pub(crate)`」
//! という骨格と規律は sakura を完全踏襲する。
//!
//! 本ファイル（タスク 3.1）が担うのは**構文区切りのみ**:
//! - `//` コメント行・空行の無視（要件 9.1）— トークンを生じない。
//! - ブロック境界の認識（要件 4.1）: ヘッダ行の直後（次の非コメント/非空行）に
//!   単独 `{` が来ればブロック開始。`}` でブロック終了。fixture の複数行形
//!   （`descript` と `{` が別行）を許容する。
//! - ブロックヘッダ行・本体行の CSV（カンマ）分割。ただし `[...]` 配列内カンマは
//!   保護して分割しない（要件 8.1 の `[id,...]`）。
//! - ドット付きキー（`animationN.interval`）は**分割せず** CSV 1 フィールドとして
//!   丸ごと保持する（`.` 分割・`animationN` 集約は decode の責務・要件 4.1）。
//! - `charset,VALUE` 行および descript ブロックを decode が認識できる形で emit する
//!   （実際のスキップ・非保持は decode の責務・要件 3.1/3.2）。本 lexer は charset を
//!   `TopLevel` 行として、descript を通常ブロックとして素朴に emit するにとどめ、
//!   値を削除・正規化しない（情報を失わない）。
//! - 未閉じブロック（`}` 無しで EOF）・未知先頭語を `Raw` として寛容吸収し、走査を
//!   中断しない（要件 9.2）。前後の正常トークンは欠落しない。
//!
//! 本モジュールの実体は `decode`（→ `parse`）から非テスト経路で到達される
//! （依存方向 `model ← lexer ← decode ← parse`・後続タスクで結線）。model には依存しない
//! （lexer は std のみ）。

/// 構文トークン（lexer 内部型・**非公開**）。
///
/// `decode`（タスク 4.x）がこの列を意味正規化済みモデルへ写像する。`sakura` の
/// `Token` と同じく `shell` モジュール外へは公開しない（`pub(crate)`）。
///
/// フィールドの `Vec<String>` は CSV 分割後のフィールド列（`[...]` 内カンマ保護済み・
/// ドット付きキーは 1 フィールド）。数値化・意味解釈は一切していない。
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    /// ブロック開始。ヘッダ行を CSV 分割したフィールド列を保持する。
    /// 例: `surface0` → `["surface0"]`、
    /// `surface.append10,2100-2110,2200-2210` → `["surface.append10","2100-2110","2200-2210"]`。
    /// 範囲 `a-b` はテキストのまま（展開は decode/下流・要件 7.2）。
    BlockStart(Vec<String>),
    /// ブロック内の本体行を CSV 分割したフィールド列。
    /// 例: `element0,overlay,surface0.png,0,0` / `6,[2106,2206]`。
    Line(Vec<String>),
    /// ブロック終了（`}`）。
    BlockEnd,
    /// ブロックに属さないトップレベル行を CSV 分割したフィールド列。
    /// 例: `charset,UTF-8`（decode がスキップ判定する・要件 3.1）。未知先頭語の
    /// トップレベル行もここへ入り、スキップ/吸収は decode に委ねる（要件 9.2）。
    TopLevel(Vec<String>),
    /// 区切れたが正準でない／不正な断片を verbatim 吸収する（未閉じブロック等・要件 9.2）。
    Raw(String),
}

/// surfaces.txt 文字列を構文トークン列へ分割する（mod 内非公開）。
///
/// - 行指向。改行（`\n`・`\r\n`）で行へ分割し、各行を左右トリムして分類する。
/// - `//` コメント行・空行は無視（トークンを生じない・要件 9.1）。
/// - ヘッダ行は「直後の有意行が単独 `{` である非 `{`/`}` 行」。`{` を確認するまで
///   pending として保持し、`{` が来れば `BlockStart`、来なければ `TopLevel` として emit する。
/// - `}` のみの行でブロックを閉じる。ブロック外の `}` は `Raw` 吸収する。
/// - `}` 無しで EOF に達したブロックは、その `BlockStart` 以降の生テキストを 1 個の
///   `Raw` として吸収する（走査を中断しない・要件 9.2）。前段の正常トークンは保持済み。
pub(crate) fn lex(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    // 行を保持（原文行と、前後空白を除いた表示用を両方参照する）。
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let raw_line = lines[i];
        let line = raw_line.trim();

        // 空行・コメント行は無視（要件 9.1）。
        if line.is_empty() || is_comment(line) {
            i += 1;
            continue;
        }

        // ブロック外の孤立 `}` は不正 → `Raw` 吸収して継続（要件 9.2）。
        if line == "}" {
            tokens.push(Token::Raw(raw_line.to_string()));
            i += 1;
            continue;
        }

        // ここに来る行はヘッダ候補（`{`/`}`/空/コメント以外）。孤立 `{` が
        // ヘッダより先に来る崩れた入力は `Raw` 吸収する（要件 9.2）。
        if line == "{" {
            tokens.push(Token::Raw(raw_line.to_string()));
            i += 1;
            continue;
        }

        // ヘッダ候補。直後の有意行（空/コメントを飛ばした最初の行）を覗く。
        let header_fields = split_csv(line);
        let next_significant = peek_next_significant(&lines, i + 1);

        match next_significant {
            Some((brace_idx, "{")) => {
                // ブロック開始。本体行を `}` まで走査する。
                let (body_tokens, end_idx, closed) = scan_block_body(&lines, brace_idx + 1);
                if closed {
                    tokens.push(Token::BlockStart(header_fields));
                    tokens.extend(body_tokens);
                    tokens.push(Token::BlockEnd);
                    i = end_idx + 1; // `}` の次行へ。
                } else {
                    // 未閉じブロック（`}` 無しで EOF）→ ヘッダ行から末尾までを 1 個の
                    // `Raw` として吸収する（要件 9.2）。前段トークンは保持済み。
                    let raw = raw_lines_from(&lines, i);
                    tokens.push(Token::Raw(raw));
                    break;
                }
            }
            _ => {
                // 直後に `{` が来ない → トップレベル行（`charset,...` 等）。
                // スキップ/吸収の判定は decode に委ねる（要件 3.1/9.2）。
                tokens.push(Token::TopLevel(header_fields));
                i += 1;
            }
        }
    }

    tokens
}

/// `//` で始まる行はコメント（前後トリム済みの行を渡す・要件 9.1）。
fn is_comment(line: &str) -> bool {
    line.starts_with("//")
}

/// `start` 以降の最初の有意行（空/コメント以外）を、その行インデックスと
/// トリム済み文字列で返す。無ければ `None`。
fn peek_next_significant<'a>(lines: &[&'a str], start: usize) -> Option<(usize, &'a str)> {
    let mut k = start;
    while k < lines.len() {
        let t = lines[k].trim();
        if t.is_empty() || is_comment(t) {
            k += 1;
            continue;
        }
        return Some((k, t));
    }
    None
}

/// `{` の次行（`start`）からブロック本体を `}` まで走査する。
///
/// 返り値: (本体トークン列, 終端行インデックス, 閉じたか)。
/// - 閉じた場合、終端インデックスは `}` の行。
/// - `}` 無しで EOF に達した場合、`closed=false`（呼び出し側が `Raw` 吸収する・要件 9.2）。
///
/// 本体内の空行・コメント行は無視する（要件 9.1）。
fn scan_block_body(lines: &[&str], start: usize) -> (Vec<Token>, usize, bool) {
    let mut body: Vec<Token> = Vec::new();
    let mut k = start;
    while k < lines.len() {
        let line = lines[k].trim();
        if line.is_empty() || is_comment(line) {
            k += 1;
            continue;
        }
        if line == "}" {
            return (body, k, true);
        }
        // ネストした `{` は emo2 subset に存在しないが、崩れた入力でも走査を止めない。
        // 素朴に本体行として CSV 分割し吸収する（意味判定は decode）。
        body.push(Token::Line(split_csv(line)));
        k += 1;
    }
    // `}` 無しで EOF。
    (body, k, false)
}

/// `from` 行から入力末尾までの原文行を改行結合して 1 本の文字列で返す
/// （未閉じブロックの `Raw` 吸収用・要件 9.2）。
fn raw_lines_from(lines: &[&str], from: usize) -> String {
    lines[from..].join("\n")
}

/// 1 行を CSV（カンマ区切り）フィールドへ分割する。
///
/// - `[` … `]` の内側のカンマは**保護**し、分割しない（`[id,...]` 配列値・要件 8.1）。
///   角括弧は verbatim に保持する（`[2106,2206]` はそのまま 1 フィールド）。
/// - ドット付きキー（`animation1400.interval`）は `.` を含んだまま 1 フィールドとして
///   保持する（`.` 分割は decode の責務・要件 4.1）。
/// - フィールドの前後空白はトリムする（fixture のインデント吸収）。
/// - 角括弧が閉じないまま行末に達した場合も、そこまでを 1 フィールドとして保持し、
///   走査を中断しない（寛容・要件 9.2）。
fn split_csv(line: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut cur = String::new();
    // 角括弧のネスト深度（>0 の間はカンマを保護する）。
    let mut depth = 0i32;

    for ch in line.chars() {
        match ch {
            '[' => {
                depth += 1;
                cur.push(ch);
            }
            ']' => {
                if depth > 0 {
                    depth -= 1;
                }
                cur.push(ch);
            }
            ',' if depth == 0 => {
                fields.push(cur.trim().to_string());
                cur = String::new();
            }
            other => cur.push(other),
        }
    }
    fields.push(cur.trim().to_string());
    fields
}
