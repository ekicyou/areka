//! バルーンの矢印（スクロールの印）の鍵の型と、マージ済みの表からの読み
//! （balloon-scroll-arrows 要件 1.1〜1.6・4.2・7.1・設計「areka-parsers / ArrowKeys」）。
//!
//! 鍵は完全一致で引く（`arrow0.x`・`arrow0.y`・`arrow1.x`・`arrow1.y`・`arrow.filename`）。
//! 面ごとの上書きは `parse` の 2 層の後勝ちのまま。読み手は記録を出さない寛容な写像であり、
//! 読めなかった座標の鍵は [`KeyIssue`] として事実だけを返す（警告は fs の入口の責務）。
//! `--数値` の読みは矢印の座標の鍵だけで行う（他の座標の鍵へ広げない）。

use std::collections::BTreeMap;

/// 印の座標 1 つ（正典の脚注 *1）。広がりは横は画像（箱）の幅、縦は高さ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerCoord {
    /// 0 以上: 左上から。
    FromStart(i32),
    /// マイナス: 右端・下端からの相対（値は負）。
    FromEnd(i32),
    /// `--数値`: マイナスの座標そのもの（値は負）。
    Literal(i32),
}

impl MarkerCoord {
    /// 広がり `extent`（幅または高さ）に対する左上原点の座標へ解く。縦書きでも意味は入れ替えない。
    pub fn resolve(self, extent: i32) -> i32 {
        match self {
            MarkerCoord::FromStart(v) | MarkerCoord::Literal(v) => v,
            MarkerCoord::FromEnd(v) => extent + v,
        }
    }
}

/// 矢印 1 つの位置（x と y を個別に持つ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarkerPoint {
    pub x: Option<MarkerCoord>,
    pub y: Option<MarkerCoord>,
}

impl MarkerPoint {
    /// x と y が両方そろったときだけ座標を返す。片方だけは置き場を決められないので「座標なし」。
    pub fn both(&self) -> Option<(MarkerCoord, MarkerCoord)> {
        Some((self.x?, self.y?))
    }
}

/// 読めなかった座標の鍵と、その生の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyIssue {
    pub key: &'static str,
    pub value: String,
}

/// 矢印の 5 つの鍵。鍵が 1 つも無い定義では `Default`（全部 `None`・issues 空）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArrowKeys {
    /// 上（縦書きでは右）。
    pub arrow0: MarkerPoint,
    /// 下（縦書きでは左）。
    pub arrow1: MarkerPoint,
    /// `arrow.filename`（拡張子なし・そのまま）。
    pub filename: Option<String>,
    /// 読めなかった座標の鍵と値（鍵の並び順）。
    pub issues: Vec<KeyIssue>,
}

/// マージ済みの表から矢印の鍵を読む。
pub(super) fn read_arrow_keys(merged: &BTreeMap<String, String>) -> ArrowKeys {
    let mut issues = Vec::new();
    let mut coord = |key: &'static str| {
        let value = merged.get(key)?;
        let parsed = parse_marker_coord(value);
        if parsed.is_none() {
            issues.push(KeyIssue {
                key,
                value: value.clone(),
            });
        }
        parsed
    };
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#arrow0.x_2c_5ea7_6a19_20_2a1:1
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#arrow0.y_2c_5ea7_6a19_20_2a1:1
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#arrow1.x_2c_5ea7_6a19_20_2a1:1
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#arrow1.y_2c_5ea7_6a19_20_2a1:1
    let arrow0 = MarkerPoint {
        x: coord("arrow0.x"),
        y: coord("arrow0.y"),
    };
    let arrow1 = MarkerPoint {
        x: coord("arrow1.x"),
        y: coord("arrow1.y"),
    };
    ArrowKeys {
        arrow0,
        arrow1,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#arrow.filename_2c_30d5_30a1_30a4_30eb_540d:1
        filename: merged.get("arrow.filename").cloned(),
        issues,
    }
}

/// `12` → `FromStart`、`-12` → `FromEnd`、`--12` → `Literal(-12)`。それ以外は `None`。
/// 空白の扱いは他の数値の鍵と同じ（trim は kv 層が済ませ、ここでは削らない）。
fn parse_marker_coord(value: &str) -> Option<MarkerCoord> {
    if let Some(rest) = value.strip_prefix("--") {
        // `---1` を通さないよう、残りは符号なしの数字だけに限る。
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        return rest.parse::<i32>().ok().map(|v| MarkerCoord::Literal(-v));
    }
    let v = value.parse::<i32>().ok()?;
    Some(if v < 0 {
        MarkerCoord::FromEnd(v)
    } else {
        MarkerCoord::FromStart(v)
    })
}
