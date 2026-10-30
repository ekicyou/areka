//! 矩形の重ね直し: 前の絵（下敷き）を写し、命令列の違う所の矩形だけを全部の命令で重ね直す。
//!
//! 前の命令列と今の命令列の、先頭と末尾で同じ命令が続く部分を除いた残りの命令の置き場の和を
//! 矩形とする。矩形の外の画素に重なる命令の並びは前と今で同じなので、前の絵の値がそのまま
//! いちから合成した値になる（合成は画素ごとに独立）。矩形の中は消してから全命令を重ねる。
//! 重ね方は [`crate::blit`] と同じ整数の premultiplied SourceOver をこのファイルの中に持ち、
//! 浮動小数を使わない。下敷きは読むだけで、確保をしない（`out` の容量を使い回す）。
// 2.2 の口（`Composer::compose_into_reusing`）がつながるまで本番から呼ばれない。つないだらこの行を消す。
#![cfg_attr(not(test), expect(dead_code, reason = "Composer の口から呼ぶまでの間"))]

use areka_emo_atlas::AtlasTable;

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

/// `blit.rs` と同じ丸め除算（`div255(v) = (v + 127) / 255`）。
#[inline]
fn div255(v: u32) -> u32 {
    (v + 127) / 255
}

/// `blit.rs` と同じ premultiplied SourceOver 1 チャンネル。
#[inline]
fn source_over_channel(src_c: u8, dst_c: u8, inv_src_a: u32) -> u8 {
    let out = src_c as u32 + div255(dst_c as u32 * inv_src_a);
    out.min(255) as u8
}

/// 半開の矩形 `[x0, x1) × [y0, y1)`（i64・空は `x0 >= x1 || y0 >= y1`）。
#[derive(Clone, Copy)]
struct Area {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl Area {
    const EMPTY: Area = Area {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
    };

    fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    fn intersect(self, o: Area) -> Area {
        Area {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    fn union(self, o: Area) -> Area {
        if self.is_empty() {
            return o;
        }
        if o.is_empty() {
            return self;
        }
        Area {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }
}

/// 命令の置き場（`blit.rs` と同じ求め方＝左上が配置＋`trim_offset`・大きさが `uv_rect`）。
/// `placement` が無い命令は空。
fn op_area(op: &BlitOp, atlas: &AtlasTable) -> Area {
    let Some(p) = atlas.entry(op.element).placement.as_ref() else {
        return Area::EMPTY;
    };
    let (ox, oy) = op.transform.offset();
    let x0 = ox + p.trim_offset.x as i64;
    let y0 = oy + p.trim_offset.y as i64;
    Area {
        x0,
        y0,
        x1: x0 + p.uv_rect.w as i64,
        y1: y0 + p.uv_rect.h as i64,
    }
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

    out.resize_and_clear(extent.w, extent.h);
    let stride = out.stride() as usize;
    let dst = out.bytes_mut();
    dst.copy_from_slice(base_img.bytes());

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
        let Some(placement) = atlas.entry(op.element).placement.as_ref() else {
            continue;
        };
        let Some(page) = atlas.page(placement.page) else {
            tracing::warn!(
                target: "areka_emo_compose",
                element = op.element.0,
                page = placement.page,
                "転写元アトラス頁が存在しない: 命令を skip"
            );
            continue;
        };
        let placed = op_area(op, atlas);
        let clip = placed.intersect(area);
        if clip.is_empty() {
            continue;
        }
        let src_stride = page.stride as usize;
        let src_bytes: &[u8] = &page.bytes;
        let uv = &placement.uv_rect;
        // 転写先座標 − 置き場の左上 ＝ 転写元の uv 内座標。
        let left = clip.x0 - placed.x0;
        let top = clip.y0 - placed.y0;

        for (row_i, dy) in (clip.y0..clip.y1).enumerate() {
            let sy = top + row_i as i64;
            let src_row = (uv.y as i64 + sy) as usize * src_stride;
            let dst_row = dy as usize * stride;
            for (col_i, dx) in (clip.x0..clip.x1).enumerate() {
                let sx = left + col_i as i64;
                let si = src_row + (uv.x as i64 + sx) as usize * 4;
                let di = dst_row + dx as usize * 4;

                let src_a = src_bytes[si + 3];
                let inv_src_a = 255u32 - src_a as u32;
                dst[di] = source_over_channel(src_bytes[si], dst[di], inv_src_a);
                dst[di + 1] = source_over_channel(src_bytes[si + 1], dst[di + 1], inv_src_a);
                dst[di + 2] = source_over_channel(src_bytes[si + 2], dst[di + 2], inv_src_a);
                dst[di + 3] = source_over_channel(src_a, dst[di + 3], inv_src_a);
            }
        }
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
