//! 合成 1 回で重ねる画素の上限（命令の置き場の矩形と、上限を越える命令の切り詰め）。

use std::collections::HashMap;

use areka_emo_atlas::{AtlasTable, ElementId};

use crate::plan::{BlitOp, Extent};

/// 半開の矩形 `[x0, x1) × [y0, y1)`（i64・空は `x0 >= x1 || y0 >= y1`）。
#[derive(Clone, Copy)]
struct Area {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
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
}

/// 命令の置き場（左上が配置＋`trim_offset`・大きさが `uv_rect`・座標は i64）。`asis` 用の絵（表
/// `asis` で引く）の在る命令はその絵の置き場。`placement` が無い命令は空。
fn op_area(op: &BlitOp, atlas: &AtlasTable, asis: &HashMap<ElementId, ElementId>) -> Area {
    // `asis` で置く段の中の命令は語を問わず `asis` 用の絵で描く。`asis` 用の絵は切り詰めない原寸なので
    // 通常の絵の置き場より小さくならず、数えが描く画素を下回らない。
    let element = asis.get(&op.element).copied().unwrap_or(op.element);
    let Some(p) = atlas.entry(element).placement.as_ref() else {
        return Area::EMPTY;
    };
    let (ox, oy) = op.transform.offset();
    // 位置は i64 の端まで取りうるので飽和で足す（外形・シェルの外形と同じ）。
    let x0 = ox.saturating_add(p.trim_offset.x as i64);
    let y0 = oy.saturating_add(p.trim_offset.y as i64);
    Area {
        x0,
        y0,
        x1: x0.saturating_add(p.uv_rect.w as i64),
        y1: y0.saturating_add(p.uv_rect.h as i64),
    }
}

/// 合成 1 回で重ねてよい画素の数の、外形の面積に対する倍率（`MAX_AREA` の外形で 2^31 画素）。
/// 仮の値（重ね塗り 64 枚分）で、測り直しの候補。
pub(crate) const PIXEL_FACTOR: u64 = 64;

/// 命令列 `ops` を、外形 `extent` の中で重ねる画素の合計が面積×[`PIXEL_FACTOR`] に収まる頭の部分だけに
/// 切り詰める（計画はこの命令列より多くは重ねない）。描かない描画メソッドの命令は
/// 重ねないので 0 画素に数える。捨てた命令の数を返す。
pub(crate) fn cap_pixels(
    ops: &mut Vec<BlitOp>,
    extent: Extent,
    atlas: &AtlasTable,
    asis: &HashMap<ElementId, ElementId>,
) -> usize {
    let canvas = Area {
        x0: 0,
        y0: 0,
        x1: extent.w as i64,
        y1: extent.h as i64,
    };
    let limit = u64::from(extent.w) * u64::from(extent.h) * PIXEL_FACTOR;
    let mut spent = 0u64;
    let kept = ops
        .iter()
        .take_while(|op| {
            let a = op_area(op, atlas, asis).intersect(canvas);
            if op.method.is_implemented() && !a.is_empty() {
                spent += (a.x1 - a.x0) as u64 * (a.y1 - a.y0) as u64;
            }
            spent <= limit
        })
        .count();
    let dropped = ops.len() - kept;
    ops.truncate(kept);
    dropped
}
