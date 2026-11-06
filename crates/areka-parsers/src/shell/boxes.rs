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

use super::decode::{Reading, field_string, is_box_element_method, parse_targets};
use super::model::AppendTarget;
use super::parse::parse_all;

/// 箱に関わるブレス 1 つ（`balloon.*`・`surface*`・`surface.append*` のどれか）。
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

/// surfaces.txt の文面から箱に関わるブレスを登場順に並べて返す（純粋・失敗しない）。
///
/// `parse_all` の射影。閉じたブレスだけを見る。見出しの判定順は画像の読み手と同じく
/// `surface.append*` を `surface*` より先にする（`kero.surface.alias` は `surface` で始まらないので当たらない）。
pub fn parse_boxes(text: &str) -> Vec<BoxDefinition> {
    parse_all(text).boxes
}

/// 閉じたブレス 1 つを、箱に関わるものなら転記する（他のブレスは `None`・`parse_all` が呼ぶ）。
///
/// `reading` は画像の読み手と同じ上限で見出しを読むための箱の側の状態（崩れは `parse_all` が捨てる＝
/// 記録は画像の側の 1 回だけ）。記述子が 1 つも残らない `surface*` 系のブレスは画像の読み手と同じく並べない。
pub(crate) fn transcribe_block(
    header: &[String],
    body: &[Vec<String>],
    reading: &mut Reading,
) -> Option<BoxDefinition> {
    let head = header.first().map(String::as_str).unwrap_or("");
    let rest = header.get(1..).unwrap_or(&[]);

    if head.starts_with("balloon.") {
        // 見出しに `,` があっても原文のまま名前へ残す（名前の検証は畳み込み）。
        let name = header.join(",")["balloon.".len()..].to_string();
        return Some(BoxDefinition::Brace(BoxBrace {
            name,
            lines: body.to_vec(),
        }));
    }
    if head == "surface.alias" {
        return None;
    }
    if let Some(number) = head.strip_prefix("surface.append") {
        return surface_lines(number, rest, body, reading).map(BoxDefinition::Append);
    }
    if let Some(number) = head.strip_prefix("surface") {
        return surface_lines(number, rest, body, reading).map(BoxDefinition::Surface);
    }
    None
}

/// `surface*`・`surface.append*`ブレスの見出しと箱の element定義を転記する。
fn surface_lines(
    number: &str,
    rest: &[String],
    body: &[Vec<String>],
    reading: &mut Reading,
) -> Option<BoxSurfaceLines> {
    let targets = parse_targets(number, rest, reading);
    if targets.is_empty() {
        return None;
    }
    let elements = body
        .iter()
        .filter(|fields| is_box_element_method(fields.get(1).map_or("", String::as_str)))
        .filter_map(|fields| {
            let element = fields.first()?.strip_prefix("element")?.to_string();
            Some(BoxElementLine {
                element,
                name: field_string(fields, 2),
                x: field_string(fields, 3),
                y: field_string(fields, 4),
            })
        })
        .collect();
    Some(BoxSurfaceLines { targets, elements })
}
