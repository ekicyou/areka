//! 箱（element定義の描画メソッド `balloon` で置かれたシェル内バルーンの文字の場所）の型
//! （spec: areka-P0-shell-balloon）。
//!
//! 「箱の定義の表」（`balloon.*`ブレス・名前で引く・シェルに 1 つ）と「サーフェス番号ごとの
//! 箱の置き場所の表」（element定義・番号ごと）を [`BoxLayout`] に持ち、畳み込みで読み捨てた
//! 事実を [`BoxReport`] に載せる。表の鍵は構造のある型（[`BoxName`]・`u32`）で持ち、
//! 文字列の連結を鍵にしない。

use std::collections::{BTreeMap, BTreeSet};

use areka_parsers::balloon::{self, BalloonModel};
use areka_parsers::shell::{BoxBrace, BoxDefinition, ShellBoxes};

use crate::world::EmoWorld;

/// 箱の名前（`balloon.名前`ブレスの名前）。中身は文字列として読むだけ。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoxName(String);

impl BoxName {
    /// 名前の原文。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 箱の定義（`balloon.*`ブレス 1 つ分）。
#[derive(Clone, Debug, PartialEq)]
pub struct BoxDef {
    /// 普通のバルーンの descript.txt と同じキーで読んだ定義。
    pub model: BalloonModel,
    /// 箱の大きさ（`size`・サーフェス画像のピクセル単位）。
    pub size: (u32, u32),
    /// 装飾の指定の行き先（`font.follow`）。
    pub follow: FontFollow,
}

/// `font.follow` の値（areka 独自のキー）。書かなければ `Scope`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FontFollow {
    /// 装飾の指定はスコープに付いて回る。
    #[default]
    Scope,
    /// 装飾の指定はその箱だけに持つ。
    Balloon,
}

/// 箱の置き場所（element定義 1 つ分）。X・Y はサーフェス画像の左上からのピクセル。
#[derive(Clone, Debug, PartialEq)]
pub struct BoxPlacement {
    pub element: u32,
    pub name: BoxName,
    pub x: i64,
    pub y: i64,
}

/// 箱の定義の表とサーフェス番号ごとの置き場所の表。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxLayout {
    defs: BTreeMap<BoxName, BoxDef>,
    /// サーフェス番号 → element番号の昇順の置き場所。
    surfaces: BTreeMap<u32, Vec<BoxPlacement>>,
}

impl BoxLayout {
    /// 置き場所が 1 つも無いか（定義だけがあっても空と答える）。
    pub fn is_empty(&self) -> bool {
        self.surfaces.values().all(Vec::is_empty)
    }

    /// 名前から箱の定義を引く。
    pub fn def(&self, name: &BoxName) -> Option<&BoxDef> {
        self.defs.get(name)
    }

    /// サーフェス番号の置き場所の列（element番号の昇順）。無ければ空の列。
    pub fn placements(&self, surface_id: u32) -> &[BoxPlacement] {
        self.surfaces.get(&surface_id).map_or(&[], Vec::as_slice)
    }
}

/// 畳み込みの報告（読み捨てと断りの 1 件ごとに 1 つ）。記録の水準は入口が決める。
#[derive(Clone, Debug, PartialEq)]
pub struct BoxReport {
    pub issues: Vec<BoxIssue>,
}

/// 報告の種類。すべて原因と対象を欄に持つ（欄の文字列は原文のまま）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoxIssue {
    /// `balloon.` の後ろが空のブレス。採らない（要件 10.1）。
    BraceEmptyName { heading: String },
    /// `size` の無いブレス。採らない（要件 1.4）。
    BraceMissingSize { name: String },
    /// `size` が正の整数として読めないブレス。採らない（要件 1.4）。
    BraceBadSize { name: String, value: String },
    /// 名前が整数として読めるブレス。採らない（要件 1.9）。
    BraceNumericName { name: String },
    /// 同じ名前のブレスを後のもので置き換えた（要件 1.8）。
    BraceReplaced { name: String },
    /// 箱に当てはまらないキーを読み捨てた（要件 1.5）。
    BraceKeyIgnored { name: String, key: String },
    /// `font.follow` の値が不正。ブレスは採り、`scope` として扱う（要件 3.18）。
    BraceBadFollow { name: String, value: String },
    /// element番号が読めない element定義を読み捨てた（要件 10.1）。
    ElementBadNumber { surface: u32, element: String },
    /// 名前のブレスが無い（採られなかった）element定義を読み捨てた（要件 2.5）。
    ElementUnknownBrace {
        surface: u32,
        element: u32,
        name: String,
    },
    /// X・Y が整数として読めない element定義を読み捨てた（要件 2.6）。
    ElementBadPosition {
        surface: u32,
        element: u32,
        name: String,
        x: String,
        y: String,
    },
    /// 同じ名前の element定義のうち element番号が最小でないものを読み捨てた（要件 2.7）。
    ElementDuplicateName {
        surface: u32,
        element: u32,
        name: String,
        kept: u32,
    },
    /// 画像の element番号より小さい番号の箱（採ったうえで断る・要件 3.8）。
    ElementBelowImage {
        surface: u32,
        element: u32,
        name: String,
        image_element: u32,
    },
    /// `surface.append*`ブレスの対象のサーフェスがその時点で無い（要件 2.2・10.1）。
    AppendTargetMissing { surface: u32, name: String },
}

/// 転記から箱の定義の表と置き場所の表を作り、読み捨てた事実を報告に載せる（純粋・失敗しない）。
///
/// `images`（面の画像だけで在る番号）と `world`（画像の element番号）は element定義を
/// 番号へ配る側が使う。本関数は fs にも記録にも触れない（記録の水準は入口が決める）。
pub fn fold_boxes(
    boxes: &ShellBoxes,
    _images: &BTreeMap<u32, String>,
    _world: &EmoWorld,
) -> (BoxLayout, BoxReport) {
    let mut issues = Vec::new();
    let defs = fold_braces(boxes, &mut issues);
    let layout = BoxLayout {
        defs,
        surfaces: BTreeMap::new(),
    };
    (layout, BoxReport { issues })
}

/// `balloon.*`ブレスを登場順に定義の表へ畳む（同じ名前は後のもので丸ごと置き換え・要件 1.8）。
///
/// 置き換えたブレスが採られなければ、その名前の定義は無くなる（前の定義へは戻さない）。
fn fold_braces(boxes: &ShellBoxes, issues: &mut Vec<BoxIssue>) -> BTreeMap<BoxName, BoxDef> {
    let mut defs = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for definition in &boxes.definitions {
        let BoxDefinition::Brace(BoxBrace { name, lines }) = definition else {
            continue;
        };
        if name.is_empty() {
            issues.push(BoxIssue::BraceEmptyName {
                heading: format!("balloon.{name}"),
            });
            continue;
        }
        // `\b[ID番号]` と見分けがつかない名前（seriko の `resolve_balloon_key` と同じ基準・要件 1.9）。
        if name.parse::<i64>().is_ok() {
            issues.push(BoxIssue::BraceNumericName { name: name.clone() });
            continue;
        }
        let key = BoxName(name.clone());
        if !seen.insert(key.clone()) {
            issues.push(BoxIssue::BraceReplaced { name: name.clone() });
            defs.remove(&key);
        }
        if let Some((table, size, follow)) = split_brace(name, lines, issues) {
            let model = balloon::parse(&table, None);
            defs.insert(
                key,
                BoxDef {
                    model,
                    size,
                    follow,
                },
            );
        }
    }
    defs
}

/// descript.txt の読み手へ渡す表・`size`・`font.follow` の組。
type SplitBrace = (BTreeMap<String, String>, (u32, u32), FontFollow);

/// ブレスの本体を descript.txt の読み手へ渡す表と `size`・`font.follow` に分ける。
///
/// 本体の各行を「先頭の欄＝キー・残りを `,` でつないだもの＝値・同じキーは後勝ち」の表へ写し、
/// `size`・`font.follow`（areka 独自のキー）と当てはまらないキー（要件 1.5）を抜く。
/// `size` が無い・正の整数 2 つとして読めなければ `None`（要件 1.4）。
fn split_brace(
    name: &str,
    lines: &[Vec<String>],
    issues: &mut Vec<BoxIssue>,
) -> Option<SplitBrace> {
    // 欄が 1 つだけの行・キーが空の行は descript.txt の読み手（`kv::parse_kv`）と同じく読まない。
    let mut table: BTreeMap<String, String> = lines
        .iter()
        .filter_map(|fields| match fields.as_slice() {
            [key, value @ ..] if !key.is_empty() && !value.is_empty() => {
                Some((key.clone(), value.join(",")))
            }
            _ => None,
        })
        .collect();

    let Some(size_raw) = table.remove("size") else {
        issues.push(BoxIssue::BraceMissingSize {
            name: name.to_string(),
        });
        return None;
    };
    let positive = |s: &str| s.parse::<u32>().ok().filter(|v| *v > 0);
    let Some(size) = size_raw
        .split_once(',')
        .and_then(|(w, h)| Some((positive(w)?, positive(h)?)))
    else {
        issues.push(BoxIssue::BraceBadSize {
            name: name.to_string(),
            value: size_raw,
        });
        return None;
    };

    let follow = match table.remove("font.follow").as_deref() {
        None | Some("scope") => FontFollow::Scope,
        Some("balloon") => FontFollow::Balloon,
        Some(other) => {
            issues.push(BoxIssue::BraceBadFollow {
                name: name.to_string(),
                value: other.to_string(),
            });
            FontFollow::Scope
        }
    };

    table.retain(|key, _| {
        let inapplicable = key.starts_with("windowposition.")
            || key == "use_self_alpha"
            || key == "use_input_alpha";
        if inapplicable {
            issues.push(BoxIssue::BraceKeyIgnored {
                name: name.to_string(),
                key: key.clone(),
            });
        }
        !inapplicable
    });

    Some((table, size, follow))
}

#[cfg(test)]
#[path = "boxes_tests.rs"]
mod tests;
