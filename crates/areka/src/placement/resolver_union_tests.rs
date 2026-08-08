use super::test_support::{DPIS, px, work_area};
use super::*;

// ------------------------------------------------------------------
// T-R7: virtual_desktop_union（4.6・DD8）
// ------------------------------------------------------------------

/// T-R7: 複数モニタ矩形の和＝各辺の min/max。プライマリ左に負座標モニタ・
/// 上に高さ違いモニタを置いた 3 面構成で固定する。
#[test]
fn t_r7_union_spans_monitors_including_negative_coords() {
    for dpi in DPIS {
        let primary = RectPx {
            left: 0,
            top: 0,
            right: px(1920, dpi),
            bottom: px(1080, dpi),
        };
        // プライマリの左（負座標・少し上へずれた縦位置）
        let left_monitor = RectPx {
            left: -px(1920, dpi),
            top: -px(40, dpi),
            right: 0,
            bottom: px(1040, dpi),
        };
        // プライマリの上（小型・右へ寄せ）
        let top_monitor = RectPx {
            left: px(480, dpi),
            top: -px(720, dpi),
            right: px(480 + 1280, dpi),
            bottom: 0,
        };

        let union = virtual_desktop_union(&[primary, left_monitor, top_monitor]);

        assert_eq!(
            union,
            Some(RectPx {
                left: -px(1920, dpi),
                top: -px(720, dpi),
                right: px(1920, dpi),
                bottom: px(1080, dpi),
            }),
            "dpi={dpi}: 和＝min(left,top)/max(right,bottom)"
        );
    }
}

/// T-R7 補: 単一モニタの和はその矩形そのもの。
#[test]
fn t_r7_union_single_monitor_is_identity() {
    for dpi in DPIS {
        let m = work_area(dpi);
        assert_eq!(virtual_desktop_union(&[m]), Some(m), "dpi={dpi}");
    }
}

/// T-R7 補: 空入力は `None`（モニタ 0 面に架空の既定矩形を発明しない）。
#[test]
fn t_r7_union_empty_input_is_none() {
    assert_eq!(virtual_desktop_union(&[]), None);
}
