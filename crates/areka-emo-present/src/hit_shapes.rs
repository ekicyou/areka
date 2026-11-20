//! 当たり判定の並びを ID の形の列へ直す（GPU に触らない）。

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use areka_emo_compose::{RegionMask, RegionMasks};
use areka_parsers::shell::{Collision, CollisionBounds, CollisionShape};
use wintf::ecs::{IdForm, IdMask, IdMaskPictures, IdShape, MAX_ID};

pub(crate) const HIT_SHAPE_LIMIT: usize = 65_536;

/// f32 が整数を正確に持てる幅。
const COORD_LIMIT: i64 = 1 << 24;

#[derive(Debug)]
pub(crate) struct HitShapePlan {
    pub shapes: Vec<IdShape>,
    /// ID 1 に当たる並びの中の位置（ID n の名前は 並び[first + n - 1]）。
    pub first: usize,
    /// 上限を越えて塗らなかった件数（並びの先頭の側）。
    pub dropped: usize,
}

/// `region` のマスクの控え: `RegionMask` の実体 → `IdMask` と、その GPU の絵。
#[derive(Debug, Default)]
pub(crate) struct HitMasks {
    /// `Arc` の実体の番地 → 実体（握って番地を生かす）と直したマスク。
    masks: HashMap<usize, (Arc<RegionMask>, Option<Arc<IdMask>>)>,
    pictures: IdMaskPictures,
}

impl HitMasks {
    /// 同じ実体は 1 度だけ直す。一辺が描き先の上限を越えるマスクは `None`。
    fn id_mask(&mut self, mask: &Arc<RegionMask>) -> Option<Arc<IdMask>> {
        let key = Arc::as_ptr(mask) as usize;
        self.masks
            .entry(key)
            .or_insert_with(|| {
                let id_mask = IdMask::new(mask.width(), mask.height(), |x, y| mask.is_set(x, y));
                (mask.clone(), id_mask.map(Arc::new))
            })
            .1
            .clone()
    }

    pub(crate) fn pictures(&mut self) -> &mut IdMaskPictures {
        &mut self.pictures
    }

    /// 装置の作り直しのとき、GPU の絵だけを捨てる（直したマスクは装置に依らないので残す）。
    pub(crate) fn discard_pictures(&mut self) {
        self.pictures = IdMaskPictures::new();
    }
}

fn coord(v: i64) -> i64 {
    v.clamp(-COORD_LIMIT, COORD_LIMIT)
}

/// 並べ直した (left, top, right, bottom)。
fn ordered(b: &CollisionBounds) -> (i64, i64, i64, i64) {
    let (l, r) = (coord(b.left.min(b.right)), coord(b.left.max(b.right)));
    let (t, bt) = (coord(b.top.min(b.bottom)), coord(b.top.max(b.bottom)));
    (l, t, r, bt)
}

fn overlaps((l, t, r, b): (i64, i64, i64, i64), (w, h): (i64, i64)) -> bool {
    l < r && t < b && l < w && r > 0 && t < h && b > 0
}

fn form(
    shape: &CollisionShape,
    masks: Option<&RegionMasks>,
    id_masks: &mut HitMasks,
    face: (i64, i64),
) -> Option<IdForm> {
    match shape {
        CollisionShape::Rect(b) => {
            let (l, t, r, bt) = ordered(b);
            let clipped = (l.max(0), t.max(0), r.min(face.0), bt.min(face.1));
            overlaps(clipped, face).then_some(IdForm::Rect {
                left: clipped.0 as f32,
                top: clipped.1 as f32,
                right: clipped.2 as f32,
                bottom: clipped.3 as f32,
            })
        }
        CollisionShape::Ellipse(b) => {
            let (l, t, r, bt) = ordered(b);
            overlaps((l, t, r, bt), face).then_some(IdForm::Ellipse {
                center: (
                    ((l as f64 + r as f64) / 2.0) as f32,
                    ((t as f64 + bt as f64) / 2.0) as f32,
                ),
                radius: (
                    ((r as f64 - l as f64) / 2.0) as f32,
                    ((bt as f64 - t as f64) / 2.0) as f32,
                ),
            })
        }
        CollisionShape::Circle { x, y, radius } => {
            let (x, y) = (coord(*x), coord(*y));
            let r = radius.unsigned_abs().min(COORD_LIMIT as u64) as i64;
            overlaps((x - r, y - r, x + r, y + r), face).then_some(IdForm::Ellipse {
                center: (x as f32, y as f32),
                radius: (r as f32, r as f32),
            })
        }
        CollisionShape::Polygon(points) => (points.len() >= 3).then(|| {
            IdForm::Polygon(
                points
                    .iter()
                    .map(|&(x, y)| (coord(x) as f32, coord(y) as f32))
                    .collect(),
            )
        }),
        CollisionShape::Region(region) => {
            let mask = id_masks.id_mask(masks?.get(region)?)?;
            let (x, y) = (coord(region.x), coord(region.y));
            let placed = (
                x,
                y,
                x + i64::from(mask.width()),
                y + i64::from(mask.height()),
            );
            overlaps(placed, face).then_some(IdForm::Mask {
                mask,
                left: x as i32,
                top: y as i32,
            })
        }
    }
}

/// 並びの順は変えない（後ろほど手前）。塗らない件も ID の席を使う。
/// 並びが `limit` を越えたら、後ろから `limit` 件だけを塗る。
pub(crate) fn hit_shapes(
    regions: &[Collision],
    masks: Option<&RegionMasks>,
    id_masks: &mut HitMasks,
    extent: (u32, u32),
    limit: usize,
) -> HitShapePlan {
    let first = regions.len().saturating_sub(limit.min(MAX_ID as usize));
    let face = (i64::from(extent.0), i64::from(extent.1));
    let shapes = regions[first..]
        .iter()
        .zip(1u32..)
        .filter_map(|(c, id)| {
            let form = form(&c.shape, masks, id_masks, face)?;
            Some(IdShape {
                id: NonZeroU32::new(id)?,
                form,
            })
        })
        .collect();
    HitShapePlan {
        shapes,
        first,
        dropped: first,
    }
}

#[cfg(test)]
#[path = "hit_shapes_tests.rs"]
mod tests;
