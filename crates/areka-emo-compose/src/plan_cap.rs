//! 合成 1 回で重ねる画素の上限（命令の置き場の矩形と、上限を越える命令の切り詰め）。

use areka_emo_atlas::AtlasTable;

use crate::plan::{BlitOp, Extent};

/// 半開の矩形 `[x0, x1) × [y0, y1)`（i64・空は `x0 >= x1 || y0 >= y1`）。
#[derive(Clone, Copy)]
pub(crate) struct Area {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Area {
    pub(crate) const EMPTY: Area = Area {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
    };

    pub(crate) fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    pub(crate) fn intersect(self, o: Area) -> Area {
        Area {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    pub(crate) fn union(self, o: Area) -> Area {
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

/// 命令の置き場（左上が配置＋`trim_offset`・大きさが `uv_rect`・座標は i64）。
/// `placement` が無い命令は空。
pub(crate) fn op_area(op: &BlitOp, atlas: &AtlasTable) -> Area {
    let Some(p) = atlas.entry(op.element).placement.as_ref() else {
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
/// 切り詰める（いちからの合成も矩形の重ね直しも、この命令列より多くは重ねない）。描かない描画メソッドの命令は
/// 重ねないので 0 画素に数える。捨てた命令の数を返す。
pub(crate) fn cap_pixels(ops: &mut Vec<BlitOp>, extent: Extent, atlas: &AtlasTable) -> usize {
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
            let a = op_area(op, atlas).intersect(canvas);
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
