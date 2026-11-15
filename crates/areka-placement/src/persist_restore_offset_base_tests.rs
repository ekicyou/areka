//! 復元経路が運ぶ**基準対**の檻（areka-P0-balloon-offset-dpi task 2.2・design D15・
//! 要件 5.1／5.2／5.5）。
//!
//! 固定する主張は、**保存値が無い腕は素通し**——配置式が出した基準対（`Some(採寸 DPI)`）を
//! そのまま運ぶ——ことである（片軸だけの保存値も採用されないので同じ腕に入る）。
//!
//! 対になる**保存値を採用した腕は未係留**（基準値は保存値と bit 同一・基準 DPI は `None`）は、
//! `balloon_offset_persist_roundtrip_tests.rs` の `a_saved_offset_survives_the_save_restore_roundtrip_bit_for_bit`
//! がアンカー 5 腕すべてで持つ。保存値は「どの表示 DPI で書かれたか」を記録していない（要件 5.1）
//! ため、基準 DPI を発明して係留すると復元先の DPI 次第で保存値を二重に拡大してしまう。未係留は
//! 「情報が無い」ことの正直な表現であり、最初の観測で値を変えずに係留される（D15）。
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
