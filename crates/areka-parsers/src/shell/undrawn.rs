//! 描けない行の転記（parse_undrawn_elements）— surfaces.txt から、areka が描けない描画メソッドの
//! element定義の行だけを原文のまま並べる（areka-P0-element-base-method 要件 2.1・2.2・2.6）。
//!
//! element定義の行は第 2 欄の語で 3 つの転記のちょうど 1 つに入る: `overlay`・`base` は画像の
//! 読み手（`Element`）、`balloon` は箱の転記（`boxes.rs`）、それ以外（空の欄・大文字の綴り・
//! `replace`・`add`・`bind`・ukadoc に無い語）は本転記。転記だけを行い、展開・検証・記録はしない
//! （失敗しない）。記録は入口（`load_shell_target`）が出す。

use super::decode::is_image_element_method;
use super::lexer::{Token, lex};

/// areka が描けない描画メソッドの element定義 1 行（欄は原文のまま）。
#[derive(Clone, Debug, PartialEq)]
pub struct UndrawnElementLine {
    /// ブレスの見出し（欄を `,` でつなぎ直した原文。例 `surface0,1`・`surface.append10,2100-2110`）。
    pub heading: String,
    /// `element` に続く番号の文字列（例 `0`）。
    pub element: String,
    /// 書かれていた描画メソッドの語（欄が無ければ空文字列）。
    pub method: String,
}

/// surfaces.txt の文面から、描けない描画メソッドの element定義の行を登場順に並べる（純粋・失敗しない）。
///
/// 字句解析は画像の読み手と同じ `lexer::lex`。閉じたブレス（`BlockStart`..`BlockEnd`）だけを見る
/// （閉じずに終わるブレスは字句解析が `Raw` にする）。見出しの先頭の欄が `surface` で始まるブレス
/// だけが対象で、これは読み手の `surface.append*` と `surface*` の 2 つの枝を合わせた範囲と同じ
/// （`kero.surface.alias` は `surface` で始まらないので当たらない）。見出しは展開せず 1 行 1 件。
pub fn parse_undrawn_elements(text: &str) -> Vec<UndrawnElementLine> {
    let mut lines = Vec::new();
    let mut heading: Option<String> = None;

    for token in lex(text) {
        match token {
            Token::BlockStart(header) => {
                let head = header.first().map(String::as_str).unwrap_or("");
                heading = head.starts_with("surface").then(|| header.join(","));
            }
            Token::Line(fields) => {
                let Some(heading) = heading.as_ref() else {
                    continue;
                };
                let Some(element) = fields.first().and_then(|k| k.strip_prefix("element")) else {
                    continue;
                };
                let method = fields.get(1).map(String::as_str).unwrap_or("");
                if is_image_element_method(method) || method == "balloon" {
                    continue;
                }
                lines.push(UndrawnElementLine {
                    heading: heading.clone(),
                    element: element.to_string(),
                    method: method.to_string(),
                });
            }
            Token::BlockEnd => heading = None,
            Token::TopLevel(_) | Token::Raw(_) => {}
        }
    }

    lines
}
