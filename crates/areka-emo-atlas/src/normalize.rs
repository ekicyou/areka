//! 透過正規化（`use_self_alpha` 解釈・premultiplied BGRA 統一）。
//!
//! 設計決定 **D5 / D8**（要件 **R3**）。
//!
//! `use_self_alpha` の宣言 4 通り（`1`／`true`・`full`・`0`・宣言なし）を、絵 1 枚ごとに
//! 「α を 255 にするか」「どの 4 バイトを抜くか」（[`AlphaRule`]）へ読み替えて画素に当てる
//! （spec: areka-P0-self-alpha-declaration 要件 3・4・5・9）。決めるのは [`Normalizer::plan`]
//! の 1 か所で、当てるのは [`apply`]。動く絵は 1 枚目で決めた扱いを全部のコマに当てる。
//! 描けない組み合わせは無く、失敗を返さない。`.pna` の有無は決定に使わない（要件 5.7）。
//! 宣言なしは areka 独自の決まりで、届いた画素に α<255 が在れば α、無ければ抜き色。
//!
//! 入力も出力も乗算済み（premultiplied）の BGRA で、色の乗算・割り戻しはしない（D8）。
//! 宣言の値は呼び手が `SurfaceSet` の [`AlphaParams`] で運び、本モジュールは設定ファイルを
//! 読まない（3.6）。

use crate::decode::DecodedImage;

/// 上流由来の透過パラメータ（`SurfaceSet` 単位で注入・自ら読まない・3.6）。
///
/// descript（shell/balloon 別定義）由来の透過設定を束ねる。ManifestDeriver は本値を
/// 運ぶのみで解釈せず、解釈は Normalizer が担う。
#[derive(Clone, Copy, Debug)]
pub struct AlphaParams {
    /// `use_self_alpha` の宣言（`1`／`full`／`0`／宣言なし）。
    pub use_self_alpha: UseSelfAlpha,
}

/// `use_self_alpha` の宣言（ukadoc の 1／true・full・0）と、宣言が無い場合。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseSelfAlpha {
    /// `1` / `true`（α を使う・α を持たない絵は左上の色を抜く）。
    On,
    /// `full`（α を使う・α を持たない絵は全面不透明）。
    Full,
    /// `0`（α を捨てて全面不透明にし、左上の色を抜く）。
    Off,
    /// 行が無い・値が読めない（areka 独自: 絵の中身を見て決める）。
    Undeclared,
}

/// 1 枚目の絵で決め、その絵と（動く絵なら）全部のコマに当てる画素の扱い。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AlphaRule {
    /// 全画素の α を 255 にする（色のバイトは触らない）。
    pub opaque: bool,
    /// この 4 バイトと完全に同じ画素を 0,0,0,0 にする。
    pub key: Option<[u8; 4]>,
}

/// 正規化済み画像（常に premultiplied BGRA・3.4/D8）。
///
/// `pbgra` の長さは常に `stride * height`。α チャンネル採用腕はデコード出力
/// （WIC PBGRA）を素通しするため実質恒等で、premultiplied のまま保持される
/// （straight α 混入禁止＝にじみ/暗縁防止・D8）。
#[derive(Clone, Debug)]
pub struct NormalizedImage {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// premultiplied BGRA 画素バッファ（`len == stride * height`）。
    pub pbgra: Vec<u8>,
}

/// 透過正規化器（`use_self_alpha` 解釈・premultiplied BGRA 統一）。
///
/// 上流由来の `AlphaParams` を入力として受け、自ら設定ファイルを読まない（3.6）。
pub struct Normalizer;

impl Normalizer {
    /// 宣言と絵から画素の扱いを決める（純粋・画素を変えない・設計の表の 7 行）。
    ///
    /// `On`・`Full` の「α を持つ絵」は読み手の `has_alpha` で決める。`Undeclared` は
    /// `has_alpha` を見ず、届いた画素に α<255 が 1 つでも在るかで決める（要件 9）。
    /// `.pna` の有無は受け取らない（要件 5 の 7）。幅か高さが 0 の絵は `key` を持たない。
    pub(crate) fn plan(img: &DecodedImage, params: AlphaParams) -> AlphaRule {
        let top_left = top_left(img);
        let (opaque, key) = match params.use_self_alpha {
            // 1 / true: α あり→そのまま、α なし→左上の 4 バイトを抜く（要件 3.1・3.2）。
            UseSelfAlpha::On if img.has_alpha => (false, None),
            UseSelfAlpha::On => (false, top_left),
            // full: α あり→そのまま、α なし→全画素 α=255（要件 4.1・4.2）。
            UseSelfAlpha::Full => (!img.has_alpha, None),
            // 0: α を捨てて全画素 α=255、その後の左上と同じ色を抜く（要件 5.1〜5.3）。
            UseSelfAlpha::Off => (true, top_left.map(|[b, g, r, _]| [b, g, r, 255])),
            // 宣言なし: 透明な画素が在れば α、無ければ `On` × α なしと同じ抜き色（要件 9）。
            UseSelfAlpha::Undeclared if has_translucent_pixel(img) => (false, None),
            UseSelfAlpha::Undeclared => (false, top_left),
        };
        AlphaRule { opaque, key }
    }

    /// [`Normalizer::plan`] で決めた扱いを [`apply`] で当てた絵を返す（失敗しない）。
    ///
    /// 入力も出力も乗算済みの BGRA で、色の乗算・割り戻しはしない（D8）。動く絵の
    /// 2 枚目以降のコマは、焼きの段が 1 枚目で決めた扱いを `apply` で直に当てる。
    /// 透過パラメータは入力として受け、自ら設定を読みに行かない（3.6）。
    pub fn normalize(&self, img: DecodedImage, params: AlphaParams) -> NormalizedImage {
        let rule = Self::plan(&img, params);
        apply_to(img, rule)
    }
}

/// 扱いを絵 1 枚に当てて正規化済みの絵にする（焼きの段の 1 枚目と、動く絵の残りのコマで共用）。
pub(crate) fn apply_to(img: DecodedImage, rule: AlphaRule) -> NormalizedImage {
    let DecodedImage {
        width,
        height,
        stride,
        mut bgra,
        ..
    } = img;
    apply(&mut bgra, width, height, stride, rule);
    NormalizedImage {
        width,
        height,
        stride,
        pbgra: bgra,
    }
}

/// 左上の 1 画素（`[b, g, r, a]`）。幅か高さが 0 の絵は持たない。
fn top_left(img: &DecodedImage) -> Option<[u8; 4]> {
    if img.width == 0 || img.height == 0 {
        return None;
    }
    img.bgra.get(0..4)?.try_into().ok()
}

/// α が 255 未満の画素が 1 つでも在るか（行の詰め物は読まない・見つけたら止める）。
fn has_translucent_pixel(img: &DecodedImage) -> bool {
    let row_bytes = img.width as usize * 4;
    (0..img.height as usize)
        .map_while(|y| {
            let start = y * img.stride as usize;
            img.bgra.get(start..start + row_bytes)
        })
        .any(|row| row.as_chunks::<4>().0.iter().any(|px| px[3] < 255))
}

/// 扱いを画素に当てる（`opaque` → `key` の順）。行の詰め物は読まない。
///
/// `opaque` は各画素の 4 バイト目だけを 255 にし、色のバイトは触らない。`key` は
/// 既存の [`clear_key_color`]（完全一致・許容幅 0）で抜く（要件 5.3・5.4・9.5）。
pub(crate) fn apply(bgra: &mut [u8], width: u32, height: u32, stride: u32, rule: AlphaRule) {
    if rule.opaque {
        let row_bytes = width as usize * 4;
        for y in 0..height as usize {
            let start = y * stride as usize;
            let Some(row) = bgra.get_mut(start..start + row_bytes) else {
                break;
            };
            for px in row.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
        }
    }
    if let Some(key) = rule.key {
        clear_key_color(bgra, width, height, stride, key);
    }
}

/// `key` と 4 バイトとも同じ画素を `0,0,0,0` にする（[`apply`] の抜き色）。
/// 行の詰め物（`stride > width * 4`）を読まないよう行ごとに歩く。
fn clear_key_color(bgra: &mut [u8], width: u32, height: u32, stride: u32, key: [u8; 4]) {
    let row_bytes = width as usize * 4;
    for y in 0..height as usize {
        let start = y * stride as usize;
        let Some(row) = bgra.get_mut(start..start + row_bytes) else {
            break;
        };
        let (pixels, _) = row.as_chunks_mut::<4>();
        for px in pixels {
            // 完全一致（許容幅 0・要件 4.2）。一致しない画素は 1 バイトも
            // 変えず、一致した画素には色を残さない（要件 4.6）。
            if *px == key {
                *px = [0, 0, 0, 0];
            }
        }
    }
}

#[cfg(test)]
#[path = "normalize_key_color_tests.rs"]
mod normalize_key_color_tests;

#[cfg(test)]
#[path = "normalize_rule_tests.rs"]
mod normalize_rule_tests;
