//! シェルの読み込みで `region` の画像を読み、読めなかったファイルを記録する。

use std::path::Path;
use std::sync::Arc;

use areka_emo_atlas::ElementDecoder;
use areka_emo_compose::{EmoWorld, RegionMasks};
use areka_parsers::shell::CollisionShape;

use crate::load_records::LoadRecords;

/// 面の表の全サーフェスの当たり判定の並び（持ち込みを含む）の `region` から、マスクの集まりを作る。
/// `records` が出すときだけ、読めなかったファイルごとに `warn!` を 1 件残す。
pub(super) fn load_region_masks(
    world: &EmoWorld,
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    records: LoadRecords,
) -> Arc<RegionMasks> {
    let regions = world
        .surface_ids()
        .flat_map(|id| world.hit_regions(id).unwrap_or_default())
        .filter_map(|c| match &c.shape {
            CollisionShape::Region(region) => Some(region),
            _ => None,
        });
    let masks = RegionMasks::load(regions, shell_dir, decoder);
    if records.emits() {
        for (file, reason) in masks.unreadable() {
            tracing::warn!(
                target: "areka_emo_present::shell_target",
                file = file.as_str(),
                reason = reason.as_str(),
                "shell: region の画像が読めないので、その当たり判定を塗らない"
            );
        }
    }
    Arc::new(masks)
}

#[cfg(test)]
#[path = "shell_target_regions_tests.rs"]
mod tests;
