//! 単位空間契約の純関数の檻（task 1.2 ぶん・要件 2.2/2.4/2.5/3.1/3.3/3.6）。
//!
//! 本ファイルは task 1.2 が設計を駆動するための最小限の主張だけを持つ。
//! **判断分岐の全網羅（表示 DPI 行列・負値・下限近傍・作者基準 DPI の食い違い・
//! 判定 4 腕と縮退の全到達）は task 1.3 が本ファイルへ追加する**——ここで先取りしない。

use areka_emo_compose::ScaleRatio;
use wintf::ecs::DPI;

use super::offset_space::{
    OffsetBase, OffsetRescale, UnresolvedScale, rescale_follow_offset, scale_author_offset,
};
use crate::placement::resolver::PointPx;

/// 表示 DPI（両軸同値）。
fn dpi(v: u16) -> DPI {
    DPI::from_dpi(v, v)
}

/// 基準対（係留済み）。
fn base_at(x: i32, y: i32, d: u16) -> OffsetBase {
    OffsetBase {
        offset: PointPx { x, y },
        dpi: Some(dpi(d)),
    }
}

// -------------------------------------------------------------------------
// 遷移の変換規則（`rescale_follow_offset`）
// -------------------------------------------------------------------------

/// 未係留（永続値の腕・5.2）は**値を変えずに**現在の表示 DPI へ係留する。
#[test]
fn unpinned_base_anchors_without_changing_value() {
    let base = OffsetBase {
        offset: PointPx { x: 10, y: -20 },
        dpi: None,
    };
    assert_eq!(
        rescale_follow_offset(base, dpi(192)),
        OffsetRescale::Anchored {
            base_dpi: dpi(192)
        }
    );
}

/// 基準 DPI と現在 DPI が同一なら値も基準も 1 bit も動かない（恒等・2.2）。
#[test]
fn identical_dpi_is_unchanged() {
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 96), dpi(96)),
        OffsetRescale::Unchanged
    );
}

/// 表示 DPI 比だけで追随する（96→192＝2 倍・3.1）。
#[test]
fn rescales_by_display_dpi_ratio() {
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 96), dpi(192)),
        OffsetRescale::Rescaled {
            offset: PointPx { x: 20, y: -40 },
            saturated: false,
        }
    );
}

/// 往復しても基準から引き直すため bit 同一で戻る（3.3）。
#[test]
fn roundtrip_returns_bit_identical_value() {
    let base = base_at(10, -20, 96);
    let first = rescale_follow_offset(base, dpi(144));
    assert_eq!(rescale_follow_offset(base, dpi(96)), OffsetRescale::Unchanged);
    assert_eq!(rescale_follow_offset(base, dpi(144)), first);
    assert_eq!(
        first,
        OffsetRescale::Rescaled {
            offset: PointPx { x: 15, y: -30 },
            saturated: false,
        }
    );
}

/// 拡大率を解決できない腕は値も基準も変えず、理由を値として返す（3.6・9.4）。
#[test]
fn zero_dpi_is_unresolved_with_reason() {
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 0), dpi(192)),
        OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroBaseDpi
        }
    );
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 96), dpi(0)),
        OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroCurrentDpi
        }
    );
}

// -------------------------------------------------------------------------
// 供給時の換算（`scale_author_offset`）
// -------------------------------------------------------------------------

/// 恒等比は生値を素通しする（2.2）。
#[test]
fn identity_ratio_passes_raw_through() {
    let (x, y) = scale_author_offset((7, -13), ScaleRatio::ONE);
    assert_eq!((x.value, y.value), (7, -13));
    assert!(!x.saturated && !y.saturated);
}

/// 大きさは既存の丸め権威へ委譲する（5/4 倍・2.4）。
#[test]
fn scales_author_offset_by_ratio() {
    let k = ScaleRatio::new(120, 96).expect("120/96");
    let (x, y) = scale_author_offset((10, -10), k);
    assert_eq!((x.value, y.value), (13, -13));
}

/// 域を超える換算は回り込まず飽和し、その事実を値として返す（2.5）。
#[test]
fn saturating_axis_is_reported() {
    let k = ScaleRatio::new(192, 96).expect("192/96");
    let (x, y) = scale_author_offset((i32::MAX, 0), k);
    assert_eq!(x.value, i32::MAX);
    assert!(x.saturated);
    assert_eq!(y.value, 0);
    assert!(!y.saturated);
}
