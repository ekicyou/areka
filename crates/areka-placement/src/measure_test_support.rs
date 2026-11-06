//! 本体のテスト（`placement_tests/measure_bake_*`）が呼ぶ私有の 2 つの関数へそのまま渡す口。
//! `test-support` のときだけ立ち、本番の可視性は変えない（spec: placement-crate-split 要件 3.8）。

use std::path::Path;

use areka_emo_atlas::ElementDecoder;

use crate::descript_window_keys::DefaultSurfaces;
use crate::error::PlacementError;
use crate::resolver::ScopeInput;

/// [`super::measure_with_reader`] へそのまま渡す。
pub fn measure_with_reader(
    reader: &impl ElementDecoder,
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
    default_surfaces: &DefaultSurfaces,
) -> Result<Vec<ScopeInput>, PlacementError> {
    super::measure_with_reader(reader, shell_dir, balloon_root, scope_ids, default_surfaces)
}

/// [`super::measure_baked`] へそのまま渡す。
pub fn measure_baked(
    decoder: &impl ElementDecoder,
    shell_dir: &Path,
    balloon_root: &Path,
    scope_ids: &[usize],
    default_surfaces: &DefaultSurfaces,
) -> Result<Vec<ScopeInput>, PlacementError> {
    super::measure_baked(
        decoder,
        shell_dir,
        balloon_root,
        scope_ids,
        default_surfaces,
    )
}
