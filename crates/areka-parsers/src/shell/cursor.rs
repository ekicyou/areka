//! cursorブレスの転記（parse_cursor_braces）— surfaces.txt の cursorブレスを原文のまま並べる。
//!
//! 画像の読み手（`parse`）・箱の転記（`parse_boxes`）とは別に、同じ文面を読む転記を担う。
//! 転記だけを行い、検証も記録もしない（失敗しない）。厳しく読むのは下流（areka の
//! `emo2_boot` の cursorブレスの表）。

use super::parse::parse_all;

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
/// `parse_all` の射影。閉じたブレスだけを見る（閉じていないブレスは字句解析が中身を落とす）。
pub fn parse_cursor_braces(text: &str) -> Vec<CursorBrace> {
    parse_all(text).cursor_braces
}

/// 閉じたブレス 1 つが cursorブレスなら転記する（`parse_all` が呼ぶ）。
pub(crate) fn cursor_brace(header: &[String], body: &[Vec<String>]) -> Option<CursorBrace> {
    header
        .first()
        .is_some_and(|head| head.ends_with(".cursor"))
        .then(|| CursorBrace {
            heading: header.join(","),
            lines: body.to_vec(),
        })
}
