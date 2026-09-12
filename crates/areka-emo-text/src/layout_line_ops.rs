//! # layout_line_ops — 行配置の自己完結した補助 2 つ（純粋層・`layout.rs` の子）
//!
//! [`segment_advance_sum`]（塊の advance 合計）と [`resolve_cursor_component`]
//! （`\_l` の 1 軸ぶんの解決と記録への配線）を担う。親 `layout.rs` からの**純移動**——
//! 合計の畳み込み順・縮退の分類・記録の一回化はいずれも移動前と同一である。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//!
//! 親が `use` で引き直すため、`layout.rs` 内から見た呼び出し方は分割前と変わらない。

use areka_sakura::contract::ActorKey;

use crate::cursor_tag::{
    CursorAxis, CursorBasis, CursorWarnGuard, note_out_of_range, resolve_cursor_axis,
    warn_cursor_degrade,
};
use crate::look::GlyphStyles;
use crate::region::TextRegion;
use crate::state::{CursorCoord, TextItem};

use super::GlyphMetrics;
use super::styled::glyph_style_advance;

/// 塊の advance 合計（塊先決の判定式の左辺 `seg_sum`）。
///
/// glyph 通し番号 `[start_serial, start_serial + len)`（`items` 中の `Glyph` のみを
/// 0 起点で数えた範囲）のグリフ送り幅を、通し番号昇順＝**左畳み込み順**で合計する
/// （配置も同順ゆえ浮動小数の順序依存を実装と一致させる・design Service Interface）。
/// 全 `items` を走るため合計は `visible_count` に依存しない（INV-1/7.1）。
pub(super) fn segment_advance_sum(
    items: &[TextItem],
    start_serial: usize,
    len: usize,
    font_height: f32,
    metrics: &dyn GlyphMetrics,
    styles: Option<&GlyphStyles<'_>>,
) -> f32 {
    let end = start_serial + len;
    let mut sum = 0.0f32;
    let mut serial = 0usize;
    for item in items {
        if let TextItem::Glyph { ch } = *item {
            if serial >= end {
                break;
            }
            if serial >= start_serial {
                // 送り幅の決め方は配置ループと**同じ 1 か所**を通す（要件 11.2）——
                // 塊の合計だけが旧幅のまま取り残されると、塊の収まり判定が
                // 見た目込みの配置と食い違って折返し位置がずれる。
                let (_, advance, _) = glyph_style_advance(ch, serial, font_height, metrics, styles);
                sum += advance;
            }
            serial += 1;
        }
    }
    sum
}

/// `\_l` の 1 軸ぶんを解決層へ委譲し、記録の 2 口へ配線する（配線層の責務そのもの）。
///
/// 意味論（基点＋値×係数・縮退の分類）は [`crate::cursor_tag::resolve_cursor_axis`] が持つ。
/// 本関数が足すのは戻り値の 3 形への振り分けだけで、**採る契約は次の 1 行に尽きる**:
///
/// - `Ok(Some(px))`＝移動が成立 → [`note_out_of_range`] で範囲外なら DEBUG を 1 件残し
///   （**位置は動かさない**＝内側へ寄せない・R2.6）、値をそのまま返す。
/// - `Ok(None)`＝軸省略 → 当該軸不動・**無音**（正典の正常形・R5.5）。
/// - `Err(degrade)`＝縮退 → guard があれば [`warn_cursor_degrade`]（キャラクター・分岐ごと
///   初回 1 回）。guard 不在（[`LayoutEngine::layout`] 経路）は警告を抑止するだけで、
///   当該軸不動という**純挙動は同一**である。
///
/// すなわち「`Err` のときだけ警告する」——`cursor_tag_resolve_tests.rs` の局所ヘルパ
/// `warn_if_degraded` が写しているのはこの契約である。
pub(super) fn resolve_cursor_component(
    coord: CursorCoord,
    axis: CursorAxis,
    basis: &CursorBasis,
    region: &TextRegion,
    cursor_warn: &mut Option<(&ActorKey, &mut CursorWarnGuard)>,
) -> Option<f32> {
    match resolve_cursor_axis(coord, axis, basis) {
        Ok(Some(value)) => {
            // 範囲外は記録するだけ（値は素通し）。戻り値を使って寄せてはならない（R2.6）。
            note_out_of_range(axis, value, region);
            Some(value)
        }
        Ok(None) => None,
        Err(degrade) => {
            if let Some((actor, guard)) = cursor_warn.as_mut() {
                warn_cursor_degrade(actor, axis, coord, degrade, guard);
            }
            None
        }
    }
}
