//! 構文層（Lexer）— surfaces.txt の一般構文分割（手書き線形スキャナ）。
//!
//! 入力（UTF-8 デコード済み `&str`・charset 変換は上流 foundation の領分）を
//! **行指向**で線形走査し、閉じたブレスとトップレベル行の列へ分割する。意味割当は行わない（→ `decode`）。
//!
//! surfaces.txt は sakura のインライン `\tag[args]` 走査と構文クラスが異なり、
//! ブロック構造（見出し行＋別行 `{` … `}`）＋行指向 CSV ＋ドット付きキー ＋
//! `[id,...]` 配列値で構成される。ゆえに走査コードは sakura から流用できないが、
//! 「線形・寛容吸収・走査を中断しない・トークンは `pub(crate)`」という規律は sakura を踏襲する。
//!
//! 担うのは**構文区切りのみ**:
//! - `//` コメント行・空行の無視（要件 9.1）— トークンを生じない。
//! - ブレスの区切り（要件 4.1）: `{` だけの行が来たら見出しを選び（`pick_heading`）、`}` で閉じる。
//!   見出しと `{` の間の行は ukadoc「ブレス名、{}、各設定として認識されない行はすべてコメント行」
//!   に従いコメントとして捨てる（指摘 71）。開いた本体の中の `{` は `}` の抜けとみなし、前のブレスを
//!   中身ごと閉じて区切り直す（`Reopened`・指摘 33）。終わりで閉じていないブレスは中身を落とす
//!   （`Unclosed`・指摘 34）。ブレスの外の `{`・`}` だけの行は読み飛ばす。
//! - 見出し行・本体行の CSV（カンマ）分割。ただし `[...]` 配列内カンマは
//!   保護して分割しない（要件 8.1 の `[id,...]`）。
//! - ドット付きキー（`animationN.interval`）は**分割せず** CSV 1 フィールドとして
//!   丸ごと保持する（`.` 分割・`animationN` 集約は decode の責務・要件 4.1）。
//! - `charset,VALUE` 行は `TopLevel`、descript は通常のブレスとして素朴に出す
//!   （スキップ・非保持は decode の責務・要件 3.1/3.2）。
//!
//! 依存方向 `model ← lexer ← decode ← parse`。model には依存しない（lexer は std のみ）。

/// 構文トークン（lexer 内部型・**非公開**）。
///
/// `decode`・箱・cursorブレス・描けない行の転記がこの列を読む。`sakura` の `Token` と同じく
/// `shell` モジュール外へは公開しない（`pub(crate)`）。
///
/// フィールドの `Vec<String>` は CSV 分割後のフィールド列（`[...]` 内カンマ保護済み・
/// ドット付きキーは 1 フィールド）。数値化・意味解釈は一切していない。
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    /// 閉じたブレス。見出し行と本体の各行を CSV 分割したフィールド列。
    /// 例: `surface.append10,2100-2110` → header `["surface.append10","2100-2110"]`、
    /// 本体 `element0,overlay,surface0.png,0,0` / `6,[2106,2206]`。
    /// 範囲 `a-b` はテキストのまま（展開は decode/下流・要件 7.2）。
    Brace {
        header: Vec<String>,
        body: Vec<Vec<String>>,
    },
    /// ブレスに属さないトップレベル行を CSV 分割したフィールド列。
    /// 例: `charset,UTF-8`（decode がスキップ判定する・要件 3.1）。
    TopLevel(Vec<String>),
    /// 終わりで閉じていないブレス（中身は落とした・指摘 34）。見出しの原文と、見出し行の 1 始まりの番号。
    Unclosed { heading: String, line: usize },
    /// `}` が抜け、次の `{` で区切り直したブレス（中身は直前の `Brace` に残した・指摘 33）。
    /// 見出しの原文と、見出し行の 1 始まりの番号。直前の `Brace` の後に出す。
    Reopened { heading: String, line: usize },
}

/// 開いているブレス（見出しの欄・見出し行の番号・本体の行）。
struct Open {
    header: Vec<String>,
    line: usize,
    body: Vec<(usize, Vec<String>)>,
}

/// surfaces.txt 文字列を構文トークン列へ分割する（mod 内非公開・失敗しない）。
pub(crate) fn lex(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    // ブレスの外で、`{` を待つ有意な行（行の番号・欄）。
    let mut pending: Vec<(usize, Vec<String>)> = Vec::new();
    let mut open: Option<Open> = None;

    for (index, raw_line) in input.lines().enumerate() {
        let number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || is_comment(line) {
            continue;
        }
        match (line, open.take()) {
            ("}", Some(brace)) => tokens.push(close(brace)),
            ("{", Some(mut brace)) => {
                // `}` の抜け: 本体の行から見出しを選べれば、その前の行で前のブレスを閉じる（指摘 33）。
                match pick_heading(&brace.body) {
                    Some(at) => {
                        let (line, header) = brace.body.split_off(at).swap_remove(0);
                        let heading = brace.header.join(",");
                        let reopened = Token::Reopened {
                            heading,
                            line: brace.line,
                        };
                        tokens.push(close(brace));
                        tokens.push(reopened);
                        open = Some(Open {
                            header,
                            line,
                            body: Vec::new(),
                        });
                    }
                    // 見出しに選べる行が無い `{` は読み飛ばす。
                    None => open = Some(brace),
                }
            }
            (_, Some(mut brace)) => {
                brace.body.push((number, split_csv(line)));
                open = Some(brace);
            }
            // 見出しの無い孤立 `{` は読み飛ばす（要件 9.2）。
            ("{", None) => {
                if let Some(at) = pick_heading(&pending) {
                    // 見出しより前の行はトップレベル、見出しと `{` の間の行はコメント（指摘 71）。
                    let (line, header) = pending.split_off(at).swap_remove(0);
                    tokens.extend(pending.drain(..).map(|(_, fields)| Token::TopLevel(fields)));
                    open = Some(Open {
                        header,
                        line,
                        body: Vec::new(),
                    });
                }
            }
            // ブレスの外の孤立 `}` は読み飛ばす（要件 9.2）。
            ("}", None) => {}
            (_, None) => pending.push((number, split_csv(line))),
        }
    }

    tokens.extend(
        pending
            .into_iter()
            .map(|(_, fields)| Token::TopLevel(fields)),
    );
    if let Some(brace) = open {
        tokens.push(Token::Unclosed {
            heading: brace.header.join(","),
            line: brace.line,
        });
    }
    tokens
}

/// 開いているブレスを閉じた `Brace` にする。
fn close(brace: Open) -> Token {
    Token::Brace {
        header: brace.header,
        body: brace.body.into_iter().map(|(_, fields)| fields).collect(),
    }
}

/// `{` の見出しにする行の位置を選ぶ。最初の欄が areka の読むブレスの名である最も近い行、
/// 無ければ `{` の直前の有意な行（指摘 33・71）。行が無ければ `None`。
fn pick_heading(lines: &[(usize, Vec<String>)]) -> Option<usize> {
    lines
        .iter()
        .rposition(|(_, fields)| fields.first().is_some_and(|head| is_brace_name(head)))
        .or_else(|| lines.len().checked_sub(1))
}

/// areka の読むブレスの名（`descript`・`surface*`・`*.surface.alias`・`balloon.*`・`*.cursor`）。
fn is_brace_name(head: &str) -> bool {
    head == "descript"
        || head.starts_with("surface")
        || head.ends_with(".surface.alias")
        || head.starts_with("balloon.")
        || head.ends_with(".cursor")
}

/// `//` で始まる行はコメント（前後トリム済みの行を渡す・要件 9.1）。
fn is_comment(line: &str) -> bool {
    line.starts_with("//")
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
