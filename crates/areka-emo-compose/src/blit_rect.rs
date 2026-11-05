//! 矩形の重ね直し: 前の絵（下敷き）を写し、命令列の違う所の矩形だけを全部の命令で重ね直す。
//!
//! 前の命令列と今の命令列の、先頭と末尾で同じ命令が続く部分を除いた残りの命令の置き場の和を
//! 矩形とする。矩形の外の画素に重なる命令の並びは前と今で同じなので、前の絵の値がそのまま
//! いちから合成した値になる（合成は画素ごとに独立）。矩形の中は消してから全命令を重ねる。
//! 重ね方は [`crate::blit::overlay_op`]（いちからの合成と同じ 1 本の整数の premultiplied
//! SourceOver）を呼び、浮動小数を使わない。下敷きは読むだけで、確保をしない（`out` の容量を使い回す）。

use areka_emo_atlas::AtlasTable;

use crate::blit::{Area, op_area, overlay_op};
use crate::composed::ComposedSurface;
use crate::method::ComposeMethod;
use crate::plan::{BlitOp, Extent};

/// 合成の仕方の報告（重ね直しの範囲の行が読む）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recompose {
    /// いちから合成した（合成先を全部消して全命令を重ねた）。
    Full,
    /// 下敷きを写し、この矩形の中だけを全命令で重ね直した。
    /// 幅か高さが 0 なら、命令列が前と同じで写しただけ。
    Partial { x: u32, y: u32, w: u32, h: u32 },
}

/// 前 `prev` と今 `cur` の命令列の違う所の矩形（合成の絵の範囲 `canvas` で切ったもの）。
fn dirty_area(prev: &[BlitOp], cur: &[BlitOp], canvas: Area, atlas: &AtlasTable) -> Area {
    let shorter = prev.len().min(cur.len());
    let p = prev.iter().zip(cur).take_while(|(a, b)| a == b).count();
    let s = prev[p..]
        .iter()
        .rev()
        .zip(cur[p..].iter().rev())
        .take(shorter - p)
        .take_while(|(a, b)| a == b)
        .count();
    prev[p..prev.len() - s]
        .iter()
        .chain(&cur[p..cur.len() - s])
        .fold(Area::EMPTY, |acc, op| {
            acc.union(op_area(op, atlas).intersect(canvas))
        })
}

/// 命令列 `ops` を `out` へ重ねる。下敷き `base`（前の絵と、それを作った命令列）が
/// 使えるなら矩形だけ、使えなければいちから（[`crate::blit::execute`]）。
///
/// 事前条件: `base` の絵は `base` の命令列をこの `atlas` で重ねた絵である（呼び手の責任）。
/// 事後条件: `out` の外形は `extent`、中身は `blit::execute(out, extent, ops, atlas)` と
/// 画素単位で同じ。`base` の絵は変わらない。
///
/// いちからへ倒れるのは、下敷きが無い・下敷きの外形が `extent` と違う・どちらかの命令列に
/// `Overlay` でない命令がある、のどれか（`Overlay` 以外は将来の式と食い違わないよう矩形の道に入れない）。
pub(crate) fn execute_reusing(
    out: &mut ComposedSurface,
    extent: Extent,
    ops: &[BlitOp],
    base: Option<(&ComposedSurface, &[BlitOp])>,
    atlas: &AtlasTable,
) -> Recompose {
    let Some((base_img, base_ops)) = base.filter(|(img, base_ops)| {
        img.width() == extent.w
            && img.height() == extent.h
            && base_ops
                .iter()
                .chain(ops)
                .all(|op| op.method == ComposeMethod::Overlay)
    }) else {
        crate::blit::execute(out, extent, ops, atlas);
        return Recompose::Full;
    };

    let canvas = Area {
        x0: 0,
        y0: 0,
        x1: extent.w as i64,
        y1: extent.h as i64,
    };
    let area = dirty_area(base_ops, ops, canvas, atlas);

    out.resize_from(extent.w, extent.h, base_img.bytes());
    let stride = out.stride() as usize;
    let dst = out.bytes_mut();

    if area.is_empty() {
        return Recompose::Partial {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
    }

    // 矩形の行を消す（矩形は canvas の中に切ってあるので添字は範囲内）。
    for y in area.y0..area.y1 {
        let row = y as usize * stride;
        dst[row + area.x0 as usize * 4..row + area.x1 as usize * 4].fill(0);
    }

    // 全命令を矩形で切って順に重ねる（画家のアルゴリズム・下層から上層）。
    for op in ops {
        overlay_op(dst, stride, area, op, atlas);
    }

    Recompose::Partial {
        x: area.x0 as u32,
        y: area.y0 as u32,
        w: (area.x1 - area.x0) as u32,
        h: (area.y1 - area.y0) as u32,
    }
}

#[cfg(test)]
#[path = "blit_rect_tests.rs"]
mod tests;
