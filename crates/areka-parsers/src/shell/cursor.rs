//! cursorブレスの転記（parse_cursor_braces）— surfaces.txt の cursorブレスを原文のまま並べる。
//!
//! 画像の読み手（`parse`）・箱の転記（`parse_boxes`）とは別に、同じ文面を読む転記を担う。
//! 転記だけを行い、検証も記録もしない（失敗しない）。厳しく読むのは下流（areka の
//! `emo2_boot` の cursorブレスの表）。

use super::lexer::{Token, lex};

/// cursorブレス 1 つの転記。
#[derive(Clone, Debug, PartialEq)]
pub struct CursorBrace {
    /// 見出しの原文（例 `sakura.cursor`・`char2.cursor`）。
    pub heading: String,
    /// 本体の行（欄の列のまま・登場順）。
    pub lines: Vec<Vec<String>>,
}

/// 見出しの先頭の欄が `.cursor` で終わる閉じたブレスだけを転記する（純粋・失敗しない）。
///
/// 字句解析は画像の読み手と同じ `lexer::lex`。閉じたブレス（`BlockStart`..`BlockEnd`）だけを
/// 見る（閉じていないブレスは字句解析が `Raw` に吸収する）。
pub fn parse_cursor_braces(text: &str) -> Vec<CursorBrace> {
    let mut braces = Vec::new();
    let mut open: Option<(Vec<String>, Vec<Vec<String>>)> = None;

    for token in lex(text) {
        match token {
            Token::BlockStart(header) => open = Some((header, Vec::new())),
            Token::Line(fields) => {
                if let Some((_, body)) = open.as_mut() {
                    body.push(fields);
                }
            }
            Token::BlockEnd => {
                if let Some((header, lines)) = open.take()
                    && header.first().is_some_and(|head| head.ends_with(".cursor"))
                {
                    braces.push(CursorBrace {
                        heading: header.join(","),
                        lines,
                    });
                }
            }
            Token::TopLevel(_) | Token::Raw(_) => {}
        }
    }

    braces
}
