//! `\_l` 座標の**解決の意味論**（純粋層・`areka-P0-cursor-tag-canon` design.md
//! 「解決 `cursor_tag`」正本）。
//!
//! 語彙層（[`crate::state::CursorCoord`]）が忠実転写した 1 軸ぶんの書式を、式 1 本
//!
//! ```text
//! 位置 = 基点 + 値 × 係数
//! ```
//!
//! で image px の絶対座標へ解決する。基点は書式が決め（絶対＝文字描画開始点／`@` 相対＝現在の
//! 文字描画位置／`centerx`・`centery`＝バルーン画像原寸の半分）、係数は単位が決める
//! （[`unit_coefficient`]）。
//!
//! ## 本モジュールが知らないこと
//!
//! - **書字方向**（`writing.rs` の `WritingMode`。intra-doc リンクも張らない）——軸の役割
//!   （行内／行送り）の写像は呼び手
//!   （配線層 `layout.rs`）の領分で、本モジュールは image 軸（X 正＝右・Y 正＝下）だけを見る。
//!   正典が「座標軸はバルーン画像そのまま」と定めるので、3 書字方向で式は同一になる
//!   （原点の位置だけが書字方向で変わり、それは呼び手が `TextRegion::start()` で解決済みの値を
//!   [`CursorBasis::origin`] に渡すことで表現される）。
//! - **配線**（`layout.rs`）——依存方向は `state` → `cursor_tag` → `layout` → `actor`／`canvas`
//!   の一方向であり、本モジュールから `layout.rs` を参照しない。
//!
//! ## 失敗経路を持たない
//!
//! 解決は全入力で値を返す全域関数である（panic せず、`error!` も使わない）。縮退は値で表す——
//! `Ok(None)` が「省略＝動かさない・無音」、`Err(`[`CursorDegrade`]`)` が「解釈不能・軸取り違え
//! ＝動かさない・警告対象」であって、いずれも**失敗ではない**（R5.1）。記録は本モジュールの
//! 別の 2 口——[`warn_cursor_degrade`]（縮退の `warn!`・キャラクターと分岐ごとに初回 1 回）と
//! [`note_out_of_range`]（範囲外の `debug!`・一回化しない）——が担う。こちらも全域で、
//! `error!`／panic を持たない。
//!
//! 解決値は**そのまま返す**——負値・小数・文字描画範囲の外の値を内側へ寄せない（R2.6 の
//! 「字義どおり用い、内側への自動的な寄せを行わない」）。

use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit};
use areka_sakura::contract::ActorKey;
use std::collections::BTreeSet;

/// `\_l` の軸（image 軸・X 正＝右・Y 正＝下）。
///
/// 書字方向による行内／行送りの読み替えは呼び手（配線層）が行う。本モジュールは
/// 「バルーン画像そのままの座標軸」だけを扱う（R2.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorAxis {
    /// 画像の横方向（正＝右）。
    X,
    /// 画像の縦方向（正＝下）。
    Y,
}

impl CursorAxis {
    /// `(x, y)` の組から当該軸の成分を取り出す（基点束の軸読み）。
    fn component(self, pair: (f32, f32)) -> f32 {
        match self {
            CursorAxis::X => pair.0,
            CursorAxis::Y => pair.1,
        }
    }

    /// 文字描画範囲（validrect）の当該軸範囲 `[min, max]`（**閉区間**）。
    ///
    /// X ＝ `[left, right]`・Y ＝ `[top, bottom]`。バルーン画像の辺（`image_size`）ではなく
    /// validrect の辺であり、書字方向にも依らない（範囲外記録の判定はこの 1 か所だけが軸から
    /// 辺を引く——判定側で `left`／`top` を直に読むと軸の取り違えが静かに入る）。
    fn range(self, region: &TextRegion) -> (f32, f32) {
        match self {
            CursorAxis::X => (region.left(), region.right()),
            CursorAxis::Y => (region.top(), region.bottom()),
        }
    }
}

/// 解決の基点束（すべて image px・呼び手が軸読み替えと metrics 解決を済ませて渡す）。
///
/// 「どの書式がどの基点を使うか」は [`resolve_cursor_axis`] の解決表が決める。ここは値の器で
/// あって、器の側で書式を判断しない。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorBasis {
    /// 絶対座標の原点＝解決後の文字描画開始点（`TextRegion::start()`）。宣言された `origin`
    /// 成分は字義どおり・未宣言成分は書字開始角へ縮退した後の値である（R2.1〜2.4/2.9）。
    pub origin: (f32, f32),
    /// `@` 相対の基点＝現在の文字描画位置（実効位置。次の文字が置かれる位置・R3.1/3.5）。
    pub current: (f32, f32),
    /// `centerx`／`centery` の基準＝バルーン画像の原寸（`TextRegion::image_size()`）。
    /// **文字描画開始点でも文字描画範囲でもない**（正典 付録 A・R4.3）。
    pub image_size: (f32, f32),
    /// `em`（および `%`）の係数の源＝タグを書いた時点での文字高さ（R1.3）。
    pub font_height: f32,
    /// `lh` の係数の源＝行送り（1em＋行間）。metrics が返す値をそのまま受け取り、
    /// 本モジュールでは算出しない。
    pub line_pitch: f32,
}

/// 縮退の分類（design.md 縮退表・2 分岐）。
///
/// キャラクターごと・分岐ごとの警告一回化（R5.3）の鍵になるので、`BTreeSet` のキーとして
/// 使えるよう全順序を導出している。
///
/// 負値絶対・`%`・`@` 相対はいずれも「基点＋値×係数」で**実導出**する形であって、本分類には
/// 含まれない（R5.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CursorDegrade {
    /// 数値として解釈できない形（[`CursorCoord::Invalid`]）。非有限値は語彙層で既に
    /// `Invalid` へ落ちている（`state.rs` の `parse_cursor_coord`）。
    Unparsable,
    /// 中央指定の軸取り違え（`centerx` を Y に・`centery` を X に書いた）。
    CenterAxisMismatch,
}

/// 単位の係数（**軸に依らないスカラー**・R1.3/1.4）。
///
/// 正典（design.md 解決表）どおり `Px` ＝ 1（裸数値＝image px 恒等）・`Em` ＝ 文字高さ・
/// `Lh` ＝ 行送り・`Percent` ＝ 文字高さ / 100（100%＝文字高さ 1 個ぶん）。
///
/// 引数に軸を取らないことが「書字方向や適用軸に応じた単位の再写像・拒否を行わない」（R1.4）の
/// 型レベルの表現である。
pub fn unit_coefficient(unit: CursorUnit, font_height: f32, line_pitch: f32) -> f32 {
    match unit {
        CursorUnit::Px => 1.0,
        CursorUnit::Em => font_height,
        CursorUnit::Lh => line_pitch,
        CursorUnit::Percent => font_height / 100.0,
    }
}

/// 1 軸ぶんの解決（純粋・全域・design.md 解決表の実体）。
///
/// | 語彙 | 基点 | 係数 | 結果 |
/// |---|---|---|---|
/// | [`CursorCoord::Omitted`] | — | — | `Ok(None)`＝動かさない・無音 |
/// | [`CursorCoord::Absolute`] | `basis.origin[axis]` | 単位どおり | `Ok(Some(基点 + 値 × 係数))` |
/// | [`CursorCoord::Relative`] | `basis.current[axis]` | 単位どおり | `Ok(Some(基点 + 値 × 係数))` |
/// | [`CursorCoord::CenterX`] on X | 画像 | — | `Ok(Some(image_size.0 / 2))` |
/// | [`CursorCoord::CenterY`] on Y | 画像 | — | `Ok(Some(image_size.1 / 2))` |
/// | [`CursorCoord::CenterX`] on Y・[`CursorCoord::CenterY`] on X | — | — | `Err(`[`CursorDegrade::CenterAxisMismatch`]`)` |
/// | [`CursorCoord::Invalid`] | — | — | `Err(`[`CursorDegrade::Unparsable`]`)` |
///
/// 事前条件: `basis` の各値は image px（呼び手が軸読み替え・metrics 解決済みで渡す）。
///
/// 不変条件: 同一入力 → 同一出力。負値・小数・文字描画範囲の外の値をそのまま返す（内側へ
/// 寄せない・R2.6）。panic しない。`Err` は縮退の表現であって失敗経路ではない（R5.1）。
pub fn resolve_cursor_axis(
    coord: CursorCoord,
    axis: CursorAxis,
    basis: &CursorBasis,
) -> Result<Option<f32>, CursorDegrade> {
    match coord {
        // 省略＝当該軸不動。正典の正常形なので警告対象ではない（R1.6/5.5）。
        CursorCoord::Omitted => Ok(None),
        // 絶対＝文字描画開始点が基点（R2.1）。
        CursorCoord::Absolute { value, unit } => Ok(Some(
            axis.component(basis.origin)
                + value * unit_coefficient(unit, basis.font_height, basis.line_pitch),
        )),
        // `@` 相対＝現在の文字描画位置が基点（R3.1）。単位との共存も同じ式で通る（R3.2）。
        CursorCoord::Relative { value, unit } => Ok(Some(
            axis.component(basis.current)
                + value * unit_coefficient(unit, basis.font_height, basis.line_pitch),
        )),
        // 中央指定は基準がバルーン画像そのもの（R4.1〜4.3）。書字方向に依らない（R4.4）。
        // 軸に対応しない側に書かれたときだけ縮退する（軸の判定は解決層の責務）。
        CursorCoord::CenterX => match axis {
            CursorAxis::X => Ok(Some(basis.image_size.0 / 2.0)),
            CursorAxis::Y => Err(CursorDegrade::CenterAxisMismatch),
        },
        CursorCoord::CenterY => match axis {
            CursorAxis::Y => Ok(Some(basis.image_size.1 / 2.0)),
            CursorAxis::X => Err(CursorDegrade::CenterAxisMismatch),
        },
        // 解釈不能・非有限（語彙層が既に `Invalid` へ落としている）＝当該軸不動・警告対象。
        CursorCoord::Invalid => Err(CursorDegrade::Unparsable),
    }
}

/// 解決値（**点**）が文字描画範囲（validrect）の当該軸範囲 `[min, max]`（**閉区間**）の外に
/// あるとき、`debug!` を 1 件記録する（R2.6）。
///
/// **位置は動かさない**——本関数は記録だけを行い、値を返さない。呼び手が解決値を範囲の内側へ
/// 寄せることは正典違反である（R2.6「字義どおり用い、内側への自動的な寄せを行わない」）。
///
/// 閉区間である（`== min`／`== max` は範囲内＝記録しない）ことは、正典の `vertical_rl` の
/// `\_l[0,0]`（X ＝ `region.right()`）が沈黙するための規定である。
///
/// 検査するのは**点**であって、その点に置かれるグリフの矩形ではない。`x = left` の列矩形
/// `[left − font_height, left]` が範囲の外へはみ出しても記録しない——矩形の可視性は描画側の
/// 責務である。
///
/// **一回化しない**（同じ位置を 2 度渡せば 2 件残る）。一回化するのは
/// [`warn_cursor_degrade`] だけである（縮退表）。
///
/// 構造化フィールドは `axis`・`value`・`range_min`・`range_max`（design.md「Monitoring」）。
pub fn note_out_of_range(axis: CursorAxis, value: f32, region: &TextRegion) {
    let (min, max) = axis.range(region);
    if value >= min && value <= max {
        // 閉区間の内側（境界上を含む）＝正常。記録しない。
        return;
    }
    tracing::debug!(
        axis = ?axis,
        value = value,
        range_min = min,
        range_max = max,
        "[note_out_of_range] \\_l の解決値が文字描画範囲の外にある——字義どおり用い、内側へ寄せない（2.6）"
    );
}

/// `\_l` の縮退警告を**キャラクターごと・分岐ごとに初回 1 回だけ**にする持続 guard（R5.3）。
///
/// 鍵は `(`[`ActorKey`]`, `[`CursorDegrade`]`)` で、**軸は鍵に含まれない**——同一キャラクターで
/// `\_l[centery,centerx]` のように両軸が同じ分岐へ縮退しても警告は 1 件になる
/// （design.md 検証表 H5）。
///
/// 走査（レイアウト呼出）を跨いで持続する必要があるのでランタイム（`actor.rs` の
/// `TextLayerRuntime`）が所有し、レイアウトへ `&mut` で渡す。決定的な行出力には影響しない
/// ——guard が担うのは縮退ログの重複抑止だけである。
///
/// 決定論的な順序のため `BTreeSet` を使う（`ActorKey` の `Ord` は内部文字列の辞書順、
/// [`CursorDegrade`] の `Ord` は宣言順）。
#[derive(Clone, Debug, Default)]
pub struct CursorWarnGuard {
    /// 既に警告済みの `(キャラクター, 縮退分岐)`。
    warned: BTreeSet<(ActorKey, CursorDegrade)>,
}

impl CursorWarnGuard {
    /// `(actor, degrade)` が初回なら記録して `true`（＝今回警告する）、既出なら `false`。
    fn should_warn(&mut self, actor: &ActorKey, degrade: CursorDegrade) -> bool {
        self.warned.insert((actor.clone(), degrade))
    }
}

/// 分類済みの縮退（[`resolve_cursor_axis`] が返した `Err`）を受け取り、キャラクターごと・
/// 分岐ごとに初回のみ `warn!` する（R5.1/5.3）。
///
/// 呼び手は `Err` のときだけ本関数を呼ぶ——[`CursorCoord::Omitted`]（軸省略）と実導出
/// （負値絶対・`%`・`@` 相対・正しい軸の中央指定）は正典の正常形なので 0 件である（R5.2/5.5）。
/// 分類そのものを本関数は行わない（分類は解決表の領分）。
///
/// 縮退は**失敗ではない**（当該軸を動かさないだけ）。`error!`／panic は使わない（R5.1）。
///
/// 構造化フィールドは `actor`・`axis`・`coord`・`degrade`（design.md「Monitoring」）。`axis` は
/// 原因の特定に要るのでログには載せるが、一回化の鍵ではない（[`CursorWarnGuard`] 参照）。
pub fn warn_cursor_degrade(
    actor: &ActorKey,
    axis: CursorAxis,
    coord: CursorCoord,
    degrade: CursorDegrade,
    guard: &mut CursorWarnGuard,
) {
    if !guard.should_warn(actor, degrade) {
        // 同一 (キャラクター, 分岐) の 2 回目以降＝沈黙（走査ごとの重複警告を抑える）。
        return;
    }
    tracing::warn!(
        actor = %actor,
        axis = ?axis,
        coord = ?coord,
        degrade = ?degrade,
        "[warn_cursor_degrade] \\_l 座標が縮退した（当該軸を動かさない・致命ではない）——キャラクター・分岐ごと初回のみ警告する（5.3）"
    );
}

#[cfg(test)]
#[path = "cursor_tag_resolve_tests.rs"]
mod resolve_tests;
#[cfg(test)]
#[path = "cursor_tag_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "cursor_tag_tests.rs"]
mod tests;
