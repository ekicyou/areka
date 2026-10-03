//! 箱の転記（parse_boxes）— surfaces.txt から箱に関わる行だけを原文のまま並べる。
//!
//! 「箱」＝ element定義の描画メソッド `balloon` で置かれたシェル内バルーンの文字の場所。
//! 本ファイルは画像の読み手（`parse`・`Shell`・`Element`）とは別に、同じ文面を読む
//! 2 つ目の転記を担う。転記だけを行い、検証・展開・記録はしない（失敗しない）。
//! 厳しく読むのは下流の畳み込み（`areka-emo-compose` の `fold_boxes`）。
//!
//! - `balloon.名前`ブレス: 見出しの `balloon.` より後ろを名前とし、本体の行を欄の列のまま持つ。
//! - `surface*`ブレス・`surface.append*`ブレス: 箱の element定義が 0 件でも 1 件ずつ並べる
//!   （畳み込みが「その時点で既にあるサーフェス」を追えるようにするため）。見出しは
//!   画像の読み手と同じ `decode::parse_targets` で読む。
//! - 箱の element定義 `elementN,balloon,名前,X,Y` は番号・名前・X・Y を文字列のまま持ち、
//!   欠けた欄は空文字列にする。

use super::decode::parse_targets;
use super::lexer::{Token, lex};
use super::model::AppendTarget;

/// surfaces.txt の箱に関わるブレスの転記（ブレスの登場順）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShellBoxes {
    /// `balloon.*`・`surface*`・`surface.append*` の各ブレス（登場順）。
    pub definitions: Vec<BoxDefinition>,
}

/// 箱に関わるブレス 1 つ。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum BoxDefinition {
    /// `balloon.*`ブレス。
    Brace(BoxBrace),
    /// `surface*`ブレス。
    Surface(BoxSurfaceLines),
    /// `surface.append*`ブレス。
    Append(BoxSurfaceLines),
}

/// `balloon.名前`ブレスの転記。
#[derive(Clone, Debug, PartialEq)]
pub struct BoxBrace {
    /// 見出しの `balloon.` より後ろ（無加工）。
    pub name: String,
    /// 本体の行（欄の列のまま・登場順）。
    pub lines: Vec<Vec<String>>,
}

/// `surface*`・`surface.append*`ブレスの見出しと箱の element定義。
#[derive(Clone, Debug, PartialEq)]
pub struct BoxSurfaceLines {
    /// 見出しの番号の記述子（展開しない・画像の読み手と同じ読み方）。
    pub targets: Vec<AppendTarget>,
    /// 箱の element定義（登場順）。
    pub elements: Vec<BoxElementLine>,
}

/// 箱の element定義 `elementN,balloon,名前,X,Y` の転記（欄は文字列のまま・欠けは空文字列）。
#[derive(Clone, Debug, PartialEq)]
pub struct BoxElementLine {
    /// `element` に続く番号の文字列。
    pub element: String,
    /// 箱の名前。
    pub name: String,
    /// X。
    pub x: String,
    /// Y。
    pub y: String,
}

/// surfaces.txt の文面から箱に関わるブレスを登場順に転記する（純粋・失敗しない）。
///
/// 字句解析は画像の読み手と同じ `lexer::lex`。閉じたブレス（`BlockStart`..`BlockEnd`）だけを
/// 見る。見出しの判定順は画像の読み手と同じく `surface.append*` を `surface*` より先にする
/// （`kero.surface.alias` は `surface` で始まらないので当たらない）。
pub fn parse_boxes(text: &str) -> ShellBoxes {
    let mut definitions = Vec::new();
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
                if let Some((header, body)) = open.take() {
                    definitions.extend(transcribe_block(&header, body));
                }
            }
            Token::TopLevel(_) | Token::Raw(_) => {}
        }
    }

    ShellBoxes { definitions }
}

/// ブレス 1 つを、箱に関わるものなら転記する（他のブレスは `None`）。
fn transcribe_block(header: &[String], body: Vec<Vec<String>>) -> Option<BoxDefinition> {
    let head = header.first().map(String::as_str).unwrap_or("");
    let rest = header.get(1..).unwrap_or(&[]);

    if head.starts_with("balloon.") {
        // 見出しに `,` があっても原文のまま名前へ残す（名前の検証は畳み込み）。
        let name = header.join(",")["balloon.".len()..].to_string();
        return Some(BoxDefinition::Brace(BoxBrace { name, lines: body }));
    }
    if let Some(number) = head.strip_prefix("surface.append") {
        return Some(BoxDefinition::Append(surface_lines(number, rest, &body)));
    }
    if let Some(number) = head.strip_prefix("surface") {
        return Some(BoxDefinition::Surface(surface_lines(number, rest, &body)));
    }
    None
}

/// `surface*`・`surface.append*`ブレスの見出しと箱の element定義を転記する。
fn surface_lines(number: &str, rest: &[String], body: &[Vec<String>]) -> BoxSurfaceLines {
    let field = |fields: &[String], i: usize| fields.get(i).cloned().unwrap_or_default();
    let elements = body
        .iter()
        .filter(|fields| fields.get(1).map(String::as_str) == Some("balloon"))
        .filter_map(|fields| {
            let element = fields.first()?.strip_prefix("element")?.to_string();
            Some(BoxElementLine {
                element,
                name: field(fields, 2),
                x: field(fields, 3),
                y: field(fields, 4),
            })
        })
        .collect();
    BoxSurfaceLines {
        targets: parse_targets(number, rest),
        elements,
    }
}
