//! 復元経路が運ぶ**基準対**の檻（areka-P0-balloon-offset-dpi task 2.2・design D15・
//! 要件 5.1／5.2／5.5）。
//!
//! 固定する主張は 2 つである。
//!
//! 1. **保存値を採用した腕は未係留**——基準値は保存値と bit 同一で、基準 DPI は `None`。
//!    保存値は「どの表示 DPI で書かれたか」を記録していない（要件 5.1）ため、基準 DPI を
//!    発明して係留すると復元先の DPI 次第で保存値を二重に拡大してしまう。未係留は
//!    「情報が無い」ことの正直な表現であり、最初の観測で値を変えずに係留される（D15）。
//! 2. **保存値が無い腕は素通し**——配置式が出した基準対（`Some(採寸 DPI)`）をそのまま運ぶ。
//!
//! あわせて、保存値の採否の優先順位（要件 5.5）が基準対を足しても動かないことを、同じ入力の
//! `balloon_offset` で突き合わせる。「補正を焼き付けない」規約（要件 5.6）は起動時の関門を
//! 通す `balloon_offset_persist_roundtrip_tests.rs` の
//! `the_restore_never_bakes_a_correction_into_the_saved_offset` が持つ（ここは関門を通さない）。

use super::*;
use crate::follow::OffsetBase;

use super::balloon_offset_persist_roundtrip_tests::{MEASURE_DPI, pinned_placement};
use super::restore_tests::bo;

fn empty_snapshot() -> MonitorSnapshot {
    MonitorSnapshot { work_areas: vec![] }
}

/// 5.1／5.2／D15: 保存値を採用した腕は基準 DPI を持たず（未係留）、基準値は保存値と
/// bit 同一である（換算しない・採寸 DPI を継がない）。
#[test]
fn restored_persisted_arm_carries_unpinned_base_equal_to_saved_value() {
    let placements = vec![pinned_placement(0, Anchor::Free, PointPx { x: -50, y: 10 })];
    let entries = vec![bo(0, Axis::X, "-512"), bo(0, Axis::Y, "-48")];

    let out = apply_restored_placements(placements, &entries, &empty_snapshot());

    let saved = PointPx { x: -512, y: -48 };
    assert_eq!(
        out[0].balloon_offset_base,
        OffsetBase::unpinned(saved),
        "保存値採用腕の基準対は未係留（dpi: None・採寸 DPI を係留しない）かつ保存値と bit 同一である"
    );
    // 5.5 の非回帰: 採否の優先順位は基準対で動かない。
    assert_eq!(
        out[0].balloon_offset, saved,
        "保存値があれば保存値が勝つ（優先順位は不変）・生値のまま採る"
    );
}

/// 5.5: 保存値が無い腕は配置式が出した基準対（`Some(採寸 DPI)`）をそのまま運ぶ。
#[test]
fn restored_default_arm_carries_placement_base_unchanged() {
    let default_offset = PointPx { x: -50, y: 10 };
    let placements = vec![pinned_placement(0, Anchor::Free, default_offset)];

    let out = apply_restored_placements(placements, &[], &empty_snapshot());

    assert_eq!(
        out[0].balloon_offset_base,
        OffsetBase {
            offset: default_offset,
            dpi: Some(MEASURE_DPI),
        },
        "保存値が無い腕は配置式の基準対を素通しする（未係留へ落とさない）"
    );
    assert_eq!(
        out[0].balloon_offset, default_offset,
        "保存値が無ければ配置式の既定 offset を保持する（優先順位は不変）"
    );
}

/// 5.5: 片軸だけの保存値は採用されない——offset も基準対も配置式の既定のままである
/// （既存の受理規約＝両軸そろったときのみ採用を、基準対の腕も同じ境界で共有する）。
#[test]
fn restored_single_axis_saved_value_keeps_placement_base() {
    let default_offset = PointPx { x: -50, y: 10 };
    let placements = vec![pinned_placement(0, Anchor::Free, default_offset)];
    let entries = vec![bo(0, Axis::X, "-512")];

    let out = apply_restored_placements(placements, &entries, &empty_snapshot());

    assert_eq!(
        out[0].balloon_offset_base,
        OffsetBase {
            offset: default_offset,
            dpi: Some(MEASURE_DPI),
        },
        "片軸のみの保存値では採用腕へ入らない＝基準対も既定のまま"
    );
    assert_eq!(out[0].balloon_offset, default_offset);
}
