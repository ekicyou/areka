//! 箱（element定義の描画メソッド `balloon` で置かれたシェル内バルーンの文字の場所）の型
//! （spec: areka-P0-shell-balloon）。
//!
//! 「箱の定義の表」（`balloon.*`ブレス・名前で引く・シェルに 1 つ）と「サーフェス番号ごとの
//! 箱の置き場所の表」（element定義・番号ごと）を [`BoxLayout`] に持ち、畳み込みで読み捨てた
//! 事実を [`BoxReport`] に載せる。表の鍵は構造のある型（[`BoxName`]・`u32`）で持ち、
//! 文字列の連結を鍵にしない。

use std::collections::BTreeMap;

use areka_parsers::balloon::BalloonModel;

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

#[cfg(test)]
#[path = "boxes_tests.rs"]
mod tests;
