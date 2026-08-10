
use super::test_support::{
    DPIS, char_size, grounded_y, left_wa, mixed_layout, point, px, rect, right_wa, win,
};
use super::{WorkAreaResolution, work_area_for_window_with_origin};
use crate::placement::resolver::{SizePx};

// -------------------------------------------------------------------------
// MonitorSnapshot / work_area_for_window（task 8.1・DD15 基盤・4.7）
// -------------------------------------------------------------------------

use super::{MonitorSnapshot, work_area_for_window};

/// 複数モニタ: 窓中心が属するモニタの work area が返る（縦位置・寸法の異なる
/// 2 面で中心帰属を固定する）。
#[test]
fn work_area_for_window_picks_monitor_containing_center() {
    let snapshot = MonitorSnapshot {
        work_areas: vec![
            rect(0, 0, 1920, 1040),       // primary
            rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（負 top）
        ],
    };
    // 中心 (2500, 500) → 右モニタ
    let window = rect(2100, 100, 2900, 900);
    assert_eq!(
        work_area_for_window(&snapshot, window),
        Some(rect(1920, -213, 4480, 1227))
    );
    // 中心 (960, 520) → primary
    let window = rect(660, 220, 1260, 820);
    assert_eq!(
        work_area_for_window(&snapshot, window),
        Some(rect(0, 0, 1920, 1040))
    );
}

/// 負座標モニタ（プライマリの左）でも中心帰属が成立する。
#[test]
fn work_area_for_window_handles_negative_coords() {
    let snapshot = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1920, 1040), rect(-1920, -40, 0, 1000)],
    };
    let window = rect(-1500, 100, -700, 700); // 中心 (-1100, 400)
    assert_eq!(
        work_area_for_window(&snapshot, window),
        Some(rect(-1920, -40, 0, 1000))
    );
}

/// 境界中心の決定論: 帰属判定は half-open（right/bottom 排他）＝共有辺上の中心は
/// 右隣モニタへ属する。複数矩形が同一中心を含む（重複）場合は昇順 index 先勝ち。
#[test]
fn work_area_for_window_boundary_center_is_half_open_and_first_match_wins() {
    let a = rect(0, 0, 1920, 1040);
    let b = rect(1920, 0, 3840, 1040);
    let snapshot = MonitorSnapshot {
        work_areas: vec![a, b],
    };
    // 中心 x=1920 ちょうど（共有辺）→ a の right は排他ゆえ b
    let window = rect(1520, 220, 2320, 820); // 中心 (1920, 520)
    assert_eq!(work_area_for_window(&snapshot, window), Some(b));

    // 重複 2 面が同一中心を含む → 先勝ち（昇順 index）
    let overlap = MonitorSnapshot {
        work_areas: vec![a, rect(-10, -10, 2000, 1100)],
    };
    let window = rect(700, 300, 1300, 700); // 中心 (1000, 500) は両方に属す
    assert_eq!(work_area_for_window(&overlap, window), Some(a));
}

/// どのモニタにも属さない中心 → 最近傍（中心→矩形 clamp 点の自乗距離最小・
/// 等距離は昇順 index 先勝ち）。
#[test]
fn work_area_for_window_off_all_monitors_returns_nearest() {
    let a = rect(0, 0, 1920, 1040);
    let b = rect(1920, 0, 3840, 1040);
    let snapshot = MonitorSnapshot {
        work_areas: vec![a, b],
    };
    // 中心 (4340, 500): b の右外 500px・a の右外 2420px → b
    let window = rect(4040, 200, 4640, 800);
    assert_eq!(work_area_for_window(&snapshot, window), Some(b));
    // 中心 (-1000, 2000): a の clamp 点 (0,1040) が b の (1920,1040) より近い → a
    let window = rect(-1300, 1700, -700, 2300);
    assert_eq!(work_area_for_window(&snapshot, window), Some(a));
    // 等距離: 中心 (1920, 2000) は a clamp (1920,1040)・b clamp (1920,1040) と
    // 同距離 → 先勝ちで a
    let window = rect(1620, 1700, 2220, 2300);
    assert_eq!(work_area_for_window(&snapshot, window), Some(a));
}

/// 空 snapshot → `None`（架空の既定矩形を発明しない）。
#[test]
fn work_area_for_window_empty_snapshot_is_none() {
    let snapshot = MonitorSnapshot { work_areas: vec![] };
    assert_eq!(work_area_for_window(&snapshot, rect(0, 0, 100, 100)), None);
}

// --- work_area_for_window_with_origin -------------------------------------

/// 中心が帰属するときは `Contains` を返す（左右どちらのモニタでも・全水準）。
#[test]
fn with_origin_reports_contains_when_center_belongs() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);

        // 右モニタの中央付近
        let pos = point(px(800, dpi), grounded_y(right_wa(dpi), size));
        assert_eq!(
            work_area_for_window_with_origin(&snapshot, win(pos, size)),
            Some((right_wa(dpi), WorkAreaResolution::Contains)),
            "dpi={dpi}: 右モニタ内の窓は Contains"
        );

        // 左モニタ（負座標）の中央付近
        let pos = point(-1200, grounded_y(left_wa(), size));
        assert_eq!(
            work_area_for_window_with_origin(&snapshot, win(pos, size)),
            Some((left_wa(), WorkAreaResolution::Contains)),
            "dpi={dpi}: 左モニタ（負座標）内の窓は Contains"
        );
    }
}

/// どのモニタにも属さない中心は `NearestFallback` として判別される
/// （S3 後半＝最近傍フォールバックが異常を無観測で吸収する性質の是正・Req 3.2）。
#[test]
fn with_origin_reports_nearest_fallback_when_center_belongs_nowhere() {
    for dpi in DPIS {
        let snapshot = mixed_layout(dpi);
        let size = char_size(dpi);

        // ① 右モニタの右外（192 では 3200 超座標）
        let far_right = point(
            px(1920, dpi) + px(400, dpi),
            grounded_y(right_wa(dpi), size),
        );
        let (wa, origin) = work_area_for_window_with_origin(&snapshot, win(far_right, size))
            .expect("非空 snapshot ゆえ Some");
        assert_eq!(
            origin,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 右外の窓は最近傍フォールバック"
        );
        assert_eq!(wa, right_wa(dpi), "dpi={dpi}: 最近傍は右モニタ");

        // ② 左モニタの左外（負座標側）
        let far_left = point(-4000, 400);
        let (wa, origin) = work_area_for_window_with_origin(&snapshot, win(far_left, size))
            .expect("非空 snapshot ゆえ Some");
        assert_eq!(
            origin,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 左外の窓は最近傍フォールバック"
        );
        assert_eq!(wa, left_wa(), "dpi={dpi}: 最近傍は左モニタ");

        // ③ 2 面のあいだの帯（右モニタのタスクバー上・非対称 work area 由来）
        //    幅 px(60) の窓を帯へ完全に収め、中心を帯の中へ落とす
        let strip_size = SizePx {
            w: px(40, dpi),
            h: px(40, dpi),
        };
        let strip = point(px(12, dpi), px(400, dpi));
        let (_, origin) = work_area_for_window_with_origin(&snapshot, win(strip, strip_size))
            .expect("非空 snapshot ゆえ Some");
        assert_eq!(
            origin,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 非対称 work area の帯（タスクバー上）は帰属なし"
        );
    }
}

/// 空 snapshot は判別付き版でも `None`（架空の既定矩形を発明しない）。
#[test]
fn with_origin_empty_snapshot_is_none() {
    let snapshot = MonitorSnapshot { work_areas: vec![] };
    assert_eq!(
        work_area_for_window_with_origin(&snapshot, rect(0, 0, 100, 100)),
        None
    );
}

/// **委譲の等価性**（task 2.2 完了条件）: 既存 `work_area_for_window` の戻り値は
/// 判別付き版の第 1 要素と常に一致する＝既存呼出元の挙動が 1 bit も変わらない。
///
/// 帰属・最近傍・境界・重複・空 snapshot の全経路を同一の probe 集合で走らせる。
#[test]
fn work_area_for_window_delegates_to_with_origin() {
    for dpi in DPIS {
        let size = char_size(dpi);
        let snapshots = [
            mixed_layout(dpi),
            // 重複（先勝ち）と共有辺（half-open）を含む合成
            MonitorSnapshot {
                work_areas: vec![
                    rect(0, 0, px(1920, dpi), px(1040, dpi)),
                    rect(px(1920, dpi), 0, px(3840, dpi), px(1040, dpi)),
                    rect(-40, -40, px(2000, dpi), px(1100, dpi)),
                ],
            },
            MonitorSnapshot { work_areas: vec![] },
        ];
        let probes = [
            point(px(800, dpi), grounded_y(right_wa(dpi), size)),
            point(-1200, 400),
            point(px(1920, dpi) + px(400, dpi), 100),
            point(-4000, 2000),
            point(px(12, dpi), px(400, dpi)),
            // 共有辺ちょうどに中心が来る位置（half-open の分岐点）
            point(px(1920, dpi) - size.w / 2, px(500, dpi)),
        ];
        for snapshot in &snapshots {
            for pos in probes {
                let window = win(pos, size);
                assert_eq!(
                    work_area_for_window(snapshot, window),
                    work_area_for_window_with_origin(snapshot, window).map(|(wa, _)| wa),
                    "dpi={dpi}: 委譲の等価性が崩れた（pos={pos:?}）"
                );
            }
        }
    }
}
