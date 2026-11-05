//! 公開 facade（parse・parse_all）— 1 回の字句分けから 4 つの結果と崩れを返す純粋関数。
//!
//! デコード済みの surfaces.txt 文字列を 1 回だけ字句に分け（`lexer::lex`）、閉じたブレスの列を
//! 画像の読み手（`decode::decode` → `Shell`）・箱・描けない行・cursorブレスの転記へ 1 本の輪で渡し、
//! 字句の段の崩れを `ShellIssue` へ写す（指摘 35）。状態も I/O も持たず、記録も出さない
//! （記録は読み込む側・依存方向 `model ← lexer ← decode ← parse`）。
//!
//! 契約（design「Facade Layer — parse」）:
//! - 空入力 → 空 `Shell`（surfaces/appends/aliases すべて空・要件 2.2）。
//! - `Result` を返さない（常に `Shell` を返す・subset 外/不正入力は部分認識で吸収・要件 9.3）。
//! - 同一入力 → 同一出力の純粋・決定的関数（副作用なし・host 非依存・要件 2.1/2.4）。
//! - パニックしない（局所吸収・全域継続・要件 9.3）。

use super::boxes::{BoxDefinition, transcribe_block};
use super::cursor::{CursorBrace, cursor_brace};
use super::decode::decode;
use super::lexer::{Token, lex};
use super::model::Shell;
use super::undrawn::{UndrawnElementLine, undrawn_lines};

/// 1 回の読みの結果（`parse_all`）。4 つの公開の読み手はこの射影。
#[derive(Clone, Debug, PartialEq)]
pub struct ShellText {
    /// 画像の読み手の結果（`parse` と同じ）。
    pub shell: Shell,
    /// 箱に関わるブレスの転記（`parse_boxes` と同じ）。
    pub boxes: Vec<BoxDefinition>,
    /// 描けない element定義の行（`parse_undrawn_elements` と同じ）。
    pub undrawn: Vec<UndrawnElementLine>,
    /// cursorブレスの転記（`parse_cursor_braces` と同じ）。
    pub cursor_braces: Vec<CursorBrace>,
    /// 読みで飛ばした・区切り直した崩れ（登場順）。記録は読み込む側が出す。
    pub issues: Vec<ShellIssue>,
}

/// 読みの崩れ 1 件。見出しは原文、行の番号は見出し行の 1 始まりの番号。
#[derive(Clone, Debug, PartialEq)] // #[non_exhaustive] を付けない（指摘 30 と同じ理由）
pub enum ShellIssue {
    /// 終わりで閉じていないブレス（中身は落とした・指摘 34）。
    UnclosedAtEnd { heading: String, line: usize },
    /// `}` が抜け、次の `{` で区切り直したブレス（中身は残した・指摘 33）。
    ClosedByNextBrace { heading: String, line: usize },
}

impl ShellIssue {
    /// 記録の重さ。
    pub fn severity(&self) -> tracing::Level {
        match self {
            ShellIssue::UnclosedAtEnd { .. } | ShellIssue::ClosedByNextBrace { .. } => {
                tracing::Level::WARN
            }
        }
    }

    /// 記録の文言（見出しと行の番号を名指す）。
    pub fn message(&self) -> String {
        match self {
            ShellIssue::UnclosedAtEnd { heading, line } => format!(
                "shell: surfaces.txt の {line} 行目のブレス `{heading}` が終わりまで閉じていないので中身を読まない"
            ),
            ShellIssue::ClosedByNextBrace { heading, line } => format!(
                "shell: surfaces.txt の {line} 行目のブレス `{heading}` の `}}` が抜けているので、次の `{{` の手前で閉じた"
            ),
        }
    }
}

/// デコード済み surfaces.txt 文字列を 1 回だけ字句に分け、4 つの結果と崩れを返す純粋関数（失敗しない）。
pub fn parse_all(input: &str) -> ShellText {
    let tokens = lex(input);
    let mut boxes = Vec::new();
    let mut undrawn = Vec::new();
    let mut cursor_braces = Vec::new();
    let mut issues = Vec::new();
    for token in &tokens {
        match token {
            Token::Brace { header, body } => {
                boxes.extend(transcribe_block(header, body));
                undrawn_lines(header, body, &mut undrawn);
                cursor_braces.extend(cursor_brace(header, body));
            }
            Token::TopLevel(_) => {}
            Token::Unclosed { heading, line } => issues.push(ShellIssue::UnclosedAtEnd {
                heading: heading.clone(),
                line: *line,
            }),
            Token::Reopened { heading, line } => issues.push(ShellIssue::ClosedByNextBrace {
                heading: heading.clone(),
                line: *line,
            }),
        }
    }
    ShellText {
        shell: decode(tokens),
        boxes,
        undrawn,
        cursor_braces,
        issues,
    }
}

/// デコード済み surfaces.txt 文字列を値まで正規化した `Shell` モデルへ変換する純粋関数。
///
/// 寛容パススルー: `Result` を返さず常に `Shell` を返す（要件 9.3）。`parse_all` の射影。
///
/// - Preconditions: `input` はデコード済み（charset 適用後の）文字列（要件 2.1）。
/// - Postconditions: 空入力で空 `Shell`（要件 2.2）。順序保持のモデル（要件 2.1）。
/// - Invariants: 純粋・決定的・副作用なし・非パニック（要件 2.4/9.3）。
pub fn parse(input: &str) -> Shell {
    parse_all(input).shell
}
