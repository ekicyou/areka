//! 描けない行の転記（parse_undrawn_elements）— surfaces.txt から、areka が描けない描画メソッドの
//! element定義の行だけを原文のまま並べる。画像の element定義の行も同じ形で原文のまま写す。
//!
//! element定義の行は第 2 欄の語で 3 つの転記のちょうど 1 つに入る: `is_image_element_method` が
//! 真の語は画像の読み手（`Element`）、`balloon` は箱の転記（`boxes.rs`）、それ以外（空の欄・大文字の
//! 綴り・`add`・`bind`・`auto`・ukadoc に無い語）は本転記。転記だけを行い、展開・検証・記録はしない
//! （失敗しない）。記録は入口（`load_shell_target`）が出す。画像の行のうち合成が描かない語の行
//! （`blend-*`）も、入口がこの写しから見出しを展開せず 1 行 1 件で出す。

use super::decode::is_box_element_method;
use super::element_method_words::is_image_element_method;
use super::lexer::heading_text;
use super::parse::parse_all;

/// element定義 1 行（欄は原文のまま）。
#[derive(Clone, Debug, PartialEq)]
pub struct ElementLine {
    /// ブレスの見出し（欄を `,` でつなぎ直した原文。例 `surface0,1`・`surface.append10,2100-2110`）。
    pub heading: String,
    /// `element` に続く番号の文字列（例 `0`）。
    pub element: String,
    /// 書かれていた描画メソッドの語（欄が無ければ空文字列）。
    pub method: String,
}

/// surfaces.txt の文面から、描けない描画メソッドの element定義の行を登場順に並べる（純粋・失敗しない）。
///
/// `parse_all` の射影。閉じたブレスだけを見る（閉じずに終わるブレスは字句解析が中身を落とす）。
/// 見出しの先頭の欄が `surface` で始まるブレスだけが対象で、これは読み手の `surface.append*` と `surface*` の 2 つの枝を合わせた範囲と同じ
/// （`kero.surface.alias` は `surface` で始まらないので当たらず、`surface.alias` は 2 つの読み手と同じく外す）。見出しは展開せず 1 行 1 件。
pub fn parse_undrawn_elements(text: &str) -> Vec<ElementLine> {
    parse_all(text).undrawn
}

/// 閉じたブレス 1 つから、描けない element定義の行を `undrawn` へ、画像の element定義の行を
/// `images` へ積む（`parse_all` が呼ぶ）。
pub(crate) fn undrawn_lines(
    header: &[String],
    body: &[Vec<String>],
    undrawn: &mut Vec<ElementLine>,
    images: &mut Vec<ElementLine>,
) {
    if !header
        .first()
        .is_some_and(|head| head.starts_with("surface") && head != "surface.alias")
    {
        return;
    }
    let heading = heading_text(header);
    for fields in body {
        let Some(element) = fields.first().and_then(|k| k.strip_prefix("element")) else {
            continue;
        };
        let method = fields.get(1).map(String::as_str).unwrap_or("");
        if is_box_element_method(method) {
            continue;
        }
        let out = if is_image_element_method(method) {
            &mut *images
        } else {
            &mut *undrawn
        };
        out.push(ElementLine {
            heading: heading.clone(),
            element: element.to_string(),
            method: method.to_string(),
        });
    }
}
