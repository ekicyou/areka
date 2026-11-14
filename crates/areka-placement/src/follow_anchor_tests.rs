use super::test_support::{odd_edge_snapshot, rect};
use super::{MonitorSnapshot, project_anchor};
use crate::resolver::{Anchor, PointPx, SizePx};

/// emo2 scope0 実寸のキャラ窓寸法（物理 px）。
const CHAR_SIZE: SizePx = SizePx { w: 434, h: 687 };

// -------------------------------------------------------------------------
// project_anchor（変換 T・task 2.1・4.2/DD15・Req1.1/1.2/2.1-2.5/3.1/3.4/5.4）
// 5 アンカー射影の純粋檻: アンカー辺固定・非アンカー軸保持・Bottom 委譲・
// Free identity・縮退・モニタ跨ぎ live 算出・べき等の不動点。
// 座標・work area 辺は 96 の非倍数を含め、隠れた dpi/96 再スケールの檻とする。
// -------------------------------------------------------------------------

/// #1 Bottom: X 保持・Y=wa.bottom−h（96 非倍数の bottom で再計算を固定・Req1.2/2.1）。
/// 既に下端一致なら不動点。
#[test]
fn project_anchor_bottom_pins_bottom_edge_and_keeps_x() {
    let snapshot = odd_edge_snapshot();
    // 中心 (700+217, 300+343)=(917, 643) は単一モニタ内
    let raw = PointPx { x: 700, y: 300 };
    let mapped = project_anchor(Anchor::Bottom, raw, CHAR_SIZE, Some(&snapshot));
    assert_eq!(
        mapped,
        PointPx {
            x: 700,
            y: 1043 - 687
        },
        "X 保持・Y=下端−h"
    );
    assert_eq!(
        project_anchor(Anchor::Bottom, mapped, CHAR_SIZE, Some(&snapshot)),
        mapped,
        "下端一致の位置は不動点"
    );
}

/// #1 Top: X 保持・Y=wa.top（96 非倍数の top で再計算を固定・Req2.2）。
#[test]
fn project_anchor_top_pins_top_edge_and_keeps_x() {
    let snapshot = odd_edge_snapshot();
    let raw = PointPx { x: 700, y: 300 };
    assert_eq!(
        project_anchor(Anchor::Top, raw, CHAR_SIZE, Some(&snapshot)),
        PointPx { x: 700, y: 37 }
    );
}

/// #1 Left: X=wa.left・Y 保持（96 非倍数の left で再計算を固定・Req2.3）。
#[test]
fn project_anchor_left_pins_left_edge_and_keeps_y() {
    let snapshot = odd_edge_snapshot();
    let raw = PointPx { x: 700, y: 300 };
    assert_eq!(
        project_anchor(Anchor::Left, raw, CHAR_SIZE, Some(&snapshot)),
        PointPx { x: 53, y: 300 }
    );
}

/// #1 Right: X=wa.right−w・Y 保持（96 非倍数の right で再計算を固定・Req2.4）。
#[test]
fn project_anchor_right_pins_right_edge_and_keeps_y() {
    let snapshot = odd_edge_snapshot();
    let raw = PointPx { x: 700, y: 300 };
    assert_eq!(
        project_anchor(Anchor::Right, raw, CHAR_SIZE, Some(&snapshot)),
        PointPx {
            x: 1877 - 434,
            y: 300
        }
    );
}

/// #1/#2 Free: raw 素通し（identity・position 再計算なし・Req2.5）。snapshot 有無・
/// 寸法（非正含む）を問わず常に identity。
#[test]
fn project_anchor_free_is_always_identity() {
    let snapshot = odd_edge_snapshot();
    let raw = PointPx { x: 700, y: 300 };
    assert_eq!(
        project_anchor(Anchor::Free, raw, CHAR_SIZE, Some(&snapshot)),
        raw,
        "snapshot 有・正寸でも Free は identity"
    );
    assert_eq!(
        project_anchor(Anchor::Free, raw, CHAR_SIZE, None),
        raw,
        "snapshot 不在でも identity"
    );
    assert_eq!(
        project_anchor(Anchor::Free, raw, SizePx { w: 0, h: 0 }, Some(&snapshot)),
        raw,
        "非正寸でも Free は identity（寸法を問わない）"
    );
    assert_eq!(
        project_anchor(
            Anchor::Free,
            raw,
            SizePx {
                w: i32::MIN,
                h: i32::MIN,
            },
            None,
        ),
        raw,
    );
}

/// #2 縮退（Req3.4）: Bottom/Top/Left/Right とも snapshot 不在(None)/空・非正寸
/// （0・負・i32::MIN）で identity 縮退（下端吸着の非正寸縮退と整合・
/// `wa.right−w`／`wa.bottom−h` の暴走を先に弾く檻・panic しない）。
#[test]
fn project_anchor_degrades_to_identity_on_missing_snapshot_or_nonpositive_size() {
    let raw = PointPx { x: 700, y: 300 };
    let empty = MonitorSnapshot { work_areas: vec![] };
    let snapshot = odd_edge_snapshot();
    for anchor in [Anchor::Bottom, Anchor::Top, Anchor::Left, Anchor::Right] {
        assert_eq!(
            project_anchor(anchor, raw, CHAR_SIZE, None),
            raw,
            "{anchor:?}: snapshot 不在は identity"
        );
        assert_eq!(
            project_anchor(anchor, raw, CHAR_SIZE, Some(&empty)),
            raw,
            "{anchor:?}: 空 snapshot は identity"
        );
        for size in [
            SizePx { w: 0, h: 687 },
            SizePx { w: 434, h: 0 },
            SizePx { w: -434, h: -687 },
            SizePx {
                w: i32::MIN,
                h: i32::MIN,
            },
        ] {
            assert_eq!(
                project_anchor(anchor, raw, size, Some(&snapshot)),
                raw,
                "{anchor:?}: 非正寸 {size:?} は identity"
            );
        }
    }
}

/// #3 モニタ跨ぎ（Req1.1/2.4）: Right/Bottom は raw 位置の窓中心が属するモニタの
/// 対応辺へ live 算出する（跨いだ先の右端／下端へ再吸着）。下端・右端が異なる
/// 2 面で固定する。
#[test]
fn project_anchor_resolves_per_crossed_monitor() {
    let snapshot = MonitorSnapshot {
        work_areas: vec![
            rect(0, 0, 1920, 1040),       // primary（右端 1920・下端 1040）
            rect(1920, -213, 4477, 1227), // 右モニタ（右端 4477・下端 1227・96 非倍数）
        ],
    };
    // 中心 (700+217, 300+343)=(917, 643) → primary
    let raw_primary = PointPx { x: 700, y: 300 };
    // 中心 (2700+217, 300+343)=(2917, 643) → 右モニタ
    let raw_right = PointPx { x: 2700, y: 300 };

    // Right: 属するモニタの右端で live 算出
    assert_eq!(
        project_anchor(Anchor::Right, raw_primary, CHAR_SIZE, Some(&snapshot)),
        PointPx {
            x: 1920 - 434,
            y: 300
        },
        "primary 帰属 → primary 右端"
    );
    assert_eq!(
        project_anchor(Anchor::Right, raw_right, CHAR_SIZE, Some(&snapshot)),
        PointPx {
            x: 4477 - 434,
            y: 300
        },
        "右モニタ帰属 → 右モニタ右端（跨ぎ再吸着）"
    );
    // Bottom: 属するモニタの下端で live 算出
    assert_eq!(
        project_anchor(Anchor::Bottom, raw_right, CHAR_SIZE, Some(&snapshot)),
        PointPx {
            x: 2700,
            y: 1227 - 687
        },
        "右モニタ帰属 → 右モニタ下端"
    );
    assert_eq!(
        project_anchor(Anchor::Bottom, raw_primary, CHAR_SIZE, Some(&snapshot)),
        PointPx {
            x: 700,
            y: 1040 - 687
        },
        "primary 帰属 → primary 下端"
    );
}

/// #5 べき等の不動点（Req3.1）: 既にアンカー辺一致の位置＋同寸で project_anchor が
/// 同値を返す（drag/resize の再適用が振動を生まない基礎）。加えて T∘T = T
/// （二重適用同値）を Bottom/Right で固定する。
#[test]
fn project_anchor_is_idempotent_at_anchor_aligned_positions() {
    let snapshot = odd_edge_snapshot(); // rect(53, 37, 1877, 1043)
    // 各アンカー辺に既に一致する位置は不動点（中心はいずれも単一モニタ内）
    let bottom_fixed = PointPx {
        x: 700,
        y: 1043 - 687,
    };
    assert_eq!(
        project_anchor(Anchor::Bottom, bottom_fixed, CHAR_SIZE, Some(&snapshot)),
        bottom_fixed,
        "Bottom 不動点"
    );
    let top_fixed = PointPx { x: 700, y: 37 };
    assert_eq!(
        project_anchor(Anchor::Top, top_fixed, CHAR_SIZE, Some(&snapshot)),
        top_fixed,
        "Top 不動点"
    );
    let left_fixed = PointPx { x: 53, y: 300 };
    assert_eq!(
        project_anchor(Anchor::Left, left_fixed, CHAR_SIZE, Some(&snapshot)),
        left_fixed,
        "Left 不動点"
    );
    let right_fixed = PointPx {
        x: 1877 - 434,
        y: 300,
    };
    assert_eq!(
        project_anchor(Anchor::Right, right_fixed, CHAR_SIZE, Some(&snapshot)),
        right_fixed,
        "Right 不動点"
    );

    // T∘T = T: 任意の生位置を一度射影した結果に再射影しても同値
    for anchor in [Anchor::Bottom, Anchor::Right] {
        let once = project_anchor(
            anchor,
            PointPx { x: 700, y: 999 },
            CHAR_SIZE,
            Some(&snapshot),
        );
        assert_eq!(
            project_anchor(anchor, once, CHAR_SIZE, Some(&snapshot)),
            once,
            "{anchor:?}: T∘T = T（べき等）"
        );
    }
}
