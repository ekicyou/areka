//! 表示対象ごとの ID の描き先（`HitPicture`）と、表示の成立点での塗り（`PresentTarget::paint_hit`）。

use std::cell::Cell;
use std::collections::HashSet;

use wintf::ecs::{GraphicsCore, IdTarget};

use super::TargetId;
use super::target::PresentTarget;
use crate::hit_shapes::{HIT_SHAPE_LIMIT, hit_shapes};

#[derive(Debug)]
pub(super) struct HitPicture {
    pub(super) surface_id: u32,
    /// `HitShapePlan::first`。
    pub(super) first: usize,
    pub(super) target: IdTarget,
}

/// 記録を 1 度に抑える控え。
#[derive(Debug, Default)]
pub(super) struct HitNotes {
    /// 「読めない」をもう記録した（点の判定が `&self` から立てるので `Cell`）。
    pub(super) unreadable: Cell<bool>,
    /// 上限越えを記録したサーフェス。
    pub(super) over_limit: HashSet<u32>,
}

impl PresentTarget {
    /// 描き先が無い・塗ってあるサーフェスが違う・大きさが原寸と違うときだけ塗る。
    /// 失敗は wintf が記録し、描き先を捨てて次の表示の成立でまた試す。
    pub(super) fn paint_hit(
        &mut self,
        core: Option<&GraphicsCore>,
        target_id: TargetId,
        surface_id: u32,
        native: (u32, u32),
    ) {
        if self
            .hit_picture
            .as_ref()
            .is_some_and(|p| p.surface_id == surface_id && p.target.size() == native)
        {
            return;
        }
        let reuse = self
            .hit_picture
            .take()
            .map(|p| p.target)
            .filter(|t| t.size() == native);
        let regions = self.emo_world.hit_regions(surface_id).unwrap_or_default();
        if regions.is_empty() {
            return;
        }
        let masks = self.emo_world.region_masks().map(|m| &**m);
        let plan = hit_shapes(regions, masks, &mut self.hit_masks, native, HIT_SHAPE_LIMIT);
        if plan.dropped > 0 && self.hit_notes.over_limit.insert(surface_id) {
            tracing::warn!(
                ?target_id,
                surface_id,
                regions = regions.len(),
                dropped = plan.dropped,
                "apply(ShowSurface): 当たり判定の並びが上限を越えた（先頭の側を塗らない）"
            );
        }
        let Some(core) = core else {
            return;
        };
        let mut target = match reuse {
            Some(target) => target,
            None => match IdTarget::new(core, native) {
                Ok(target) => target,
                Err(_) => return,
            },
        };
        if target
            .paint(core, &plan.shapes, self.hit_masks.pictures())
            .is_err()
        {
            return;
        }
        self.hit_notes.unreadable.set(false);
        self.hit_picture = Some(HitPicture {
            surface_id,
            first: plan.first,
            target,
        });
    }

    /// 装置の変化の追従でだけ呼ぶ（`region` のマスクの直しは装置に依らないので残す）。
    pub(super) fn discard_hit_gpu(&mut self) {
        self.hit_picture = None;
        self.hit_masks.discard_pictures();
    }
}

#[cfg(test)]
#[path = "hit_picture_tests.rs"]
mod tests;
