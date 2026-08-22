use super::test_support::{
    DPIS, char_size, grounded_y, left_wa, mixed_layout, point, px, rect, right_wa, win,
};
use super::{WorkAreaResolution, work_area_for_window_with_origin};
use crate::placement::resolver::SizePx;

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

// -------------------------------------------------------------------------
// MonitorDpiTable / MonitorSources / same_monitors
// （areka-P0-dpi-transition-atomicity task 5.1・設計 C6・要件 5.1／5.4）
// -------------------------------------------------------------------------

use super::{MonitorDpiEntry, MonitorDpiTable, MonitorSources, same_monitors};
use windows::Win32::Foundation::RECT;
use wintf::ecs::layout::systems::monitor_systems::monitor_containing;
use wintf::ecs::window::monitor::Monitor;

/// 合成モニタ 1 台（`bounds` と `work_area` を別々に与える＝両者の取り違えを検出できる形）。
fn monitor(
    handle: isize,
    bounds: (i32, i32, i32, i32),
    work_area: (i32, i32, i32, i32),
    dpi: u32,
) -> Monitor {
    Monitor {
        handle,
        bounds: RECT {
            left: bounds.0,
            top: bounds.1,
            right: bounds.2,
            bottom: bounds.3,
        },
        work_area: RECT {
            left: work_area.0,
            top: work_area.1,
            right: work_area.2,
            bottom: work_area.3,
        },
        dpi,
        is_primary: handle == 1,
    }
}

/// 混在 DPI・負座標の 2 台（作業領域と矩形が別値＝転写の取り違えが値で判る）。
fn two_monitors() -> Vec<Monitor> {
    vec![
        monitor(1, (0, 0, 2560, 1440), (0, 0, 2560, 1344), 192),
        monitor(2, (-1920, -40, 0, 1040), (-1840, -40, 0, 1000), 96),
    ]
}

/// モニタ別拡大率表は列挙順のまま、**`bounds`**（`work_area` ではない）と拡大率を
/// 単位変換なしで忠実転写する。
#[test]
fn monitor_dpi_table_transcribes_bounds_and_dpi_in_enumeration_order() {
    let table = MonitorDpiTable::from_monitors(&two_monitors());
    assert_eq!(
        table.entries,
        vec![
            MonitorDpiEntry {
                bounds: rect(0, 0, 2560, 1440),
                dpi: 192,
            },
            MonitorDpiEntry {
                bounds: rect(-1920, -40, 0, 1040),
                dpi: 96,
            },
        ],
        "拡大率表が矩形（bounds）と拡大率を列挙順で忠実転写していない"
    );
}

/// 0 台は空表（panic しない・`MonitorSnapshot::from_monitors` の 0 台契約と同じ）。
#[test]
fn monitor_dpi_table_from_no_monitors_is_empty() {
    assert!(MonitorDpiTable::from_monitors(&[]).entries.is_empty());
    assert!(MonitorSources::from_monitors(&[]).is_empty());
}

/// 2 源は同じ列から同時に作られ、台数と並びが一致する（片方だけ古い運転を作らない）。
#[test]
fn monitor_sources_build_both_sides_from_one_list() {
    let monitors = two_monitors();
    let sources = MonitorSources::from_monitors(&monitors);
    assert_eq!(sources.len(), 2);
    assert_eq!(
        sources.snapshot,
        MonitorSnapshot::from_monitors(&monitors),
        "作業領域源が単独構築と食い違う（構築関数が二重化している）"
    );
    assert_eq!(sources.dpi_table, MonitorDpiTable::from_monitors(&monitors));
}

/// 順序非依存の比較: 同じ台の集合なら**並びが逆でも同じ**と見なす。
#[test]
fn same_monitors_ignores_the_order_of_the_table() {
    let forward = MonitorSources::from_monitors(&two_monitors());
    let mut reversed_list = two_monitors();
    reversed_list.reverse();
    let reversed = MonitorSources::from_monitors(&reversed_list);
    assert_ne!(
        forward.snapshot, reversed.snapshot,
        "前提: 並びが違えば素の等価比較は偽になる（この前提が崩れると本檻は何も見ていない）"
    );
    assert!(same_monitors(&forward, &reversed));
}

/// 対（比較が恒真に潰れていない）: 作業領域が 1px 違えば別物。
#[test]
fn same_monitors_sees_a_one_pixel_work_area_difference() {
    let a = MonitorSources::from_monitors(&two_monitors());
    let mut moved = two_monitors();
    moved[0].work_area.bottom -= 1;
    assert!(!same_monitors(&a, &MonitorSources::from_monitors(&moved)));
}

/// 対: 作業領域も矩形も同じで**拡大率だけ**違えば別物。
///
/// 作業領域だけを比べていると、この構成変更を静かに取りこぼして拡大率表が古いまま残る。
#[test]
fn same_monitors_sees_a_scale_only_difference() {
    let a = MonitorSources::from_monitors(&two_monitors());
    let mut rescaled = two_monitors();
    rescaled[0].dpi = 144;
    let b = MonitorSources::from_monitors(&rescaled);
    assert_eq!(
        a.snapshot, b.snapshot,
        "前提: 作業領域は同じ（拡大率だけが違う構成を組めている）"
    );
    assert!(!same_monitors(&a, &b));
}

/// 対: 作業領域も拡大率も同じで**矩形だけ**違えば別物（解像度変更で作業領域が偶然一致する形）。
#[test]
fn same_monitors_sees_a_bounds_only_difference() {
    let a = MonitorSources::from_monitors(&two_monitors());
    let mut resized = two_monitors();
    resized[0].bounds.bottom += 96;
    assert!(!same_monitors(&a, &MonitorSources::from_monitors(&resized)));
}

/// 対: 台数が違えば別物（同じ内容の台が 1 台増えても取りこぼさない）。
#[test]
fn same_monitors_sees_a_count_difference() {
    let a = MonitorSources::from_monitors(&two_monitors());
    let mut added = two_monitors();
    added.push(monitor(3, (2560, 0, 4480, 1080), (2560, 0, 4480, 1040), 96));
    assert!(!same_monitors(&a, &MonitorSources::from_monitors(&added)));
}

/// 同一の台が 2 度並ぶ表（複製）と 1 台だけの表を「同じ」と言わない
/// ——集合ではなく**多重集合**として比べていること。
#[test]
fn same_monitors_compares_as_a_multiset_not_a_set() {
    let one = MonitorSources::from_monitors(&two_monitors()[..1]);
    let mut duplicated = two_monitors()[..1].to_vec();
    duplicated.push(duplicated[0].clone());
    assert!(!same_monitors(
        &one,
        &MonitorSources::from_monitors(&duplicated)
    ));
}

// -------------------------------------------------------------------------
// 帰属規則の共有（areka-P0-dpi-transition-atomicity task 5.4・設計 C5）
// -------------------------------------------------------------------------

/// 表からの拡大率の引き当ては、表示基盤側の再導出が使う帰属規則と**同一の関数**を通る。
///
/// 配置側に同規則の述語を置く案は採らなかったので、ここが確かめるのは「同じ答えを返すか」
/// ではなく「同じ関数へ届いているか」である——`Monitor` の列に対する
/// `monitor_containing` と、同じ列から作った表に対する [`MonitorDpiTable::dpi_for_point`] が
/// 全ての探針で一致することを問う。片方が `work_area` を読むように書き換われば
/// （`MonitorBounds` の実装は表側だけが自前で持つ唯一の部分である）ここが落ちる。
#[test]
fn the_table_lookup_agrees_with_the_display_layer_attribution_rule() {
    let monitors = two_monitors();
    let table = MonitorSources::from_monitors(&monitors).dpi_table;

    // 半開区間の両端・共有辺・どこにも属さない点・作業領域と矩形が食い違う帯を通す。
    let probes = [
        (0, 0),               // 台 1 の左上端（含む）
        (2559, 1439),         // 台 1 の右下端の内側（含む）
        (2560, 700),          // 台 1 の右端（含まない）
        (1000, 1440),         // 台 1 の下端（含まない）
        (1000, 1400),         // 台 1 の矩形内・作業領域の外（bounds を読んでいれば台 1）
        (-1920, -40),         // 台 2 の左上端（含む）
        (-1, 1039),           // 台 2 の右下端の内側（含む）
        (-1900, 1020),        // 台 2 の矩形内・作業領域の外（同上）
        (5000, 5000),         // どのモニタにも属さない
        (i32::MIN, i32::MAX), // 極端値でも規則は同じ（溢れない）
    ];
    for (x, y) in probes {
        assert_eq!(
            table.dpi_for_point(x, y),
            monitor_containing(&monitors, (x, y)).map(|m| m.dpi),
            "点 ({x},{y}) の帰属が表示基盤側と食い違う"
        );
    }

    // 探針の非退化: 上の列には `Some` と `None` の双方が実際に出る（全部 `None` なら
    // 「どちらも何も返さない」で恒真に通ってしまう）。
    let answers: Vec<Option<u32>> = probes
        .iter()
        .map(|&(x, y)| table.dpi_for_point(x, y))
        .collect();
    assert!(
        answers.iter().any(Option::is_some) && answers.iter().any(Option::is_none),
        "探針が退化している（帰属する点と帰属しない点の両方が要る）: {answers:?}"
    );
    assert!(
        answers.contains(&Some(192)) && answers.contains(&Some(96)),
        "探針が退化している（2 台とも引き当てる点が要る）: {answers:?}"
    );
}

/// どこにも属さない点と空の表は、いずれも架空の拡大率を発明せず `None` を返す
/// （最近傍フォールバックを持つ [`work_area_for_window`] とは規則が違う）。
#[test]
fn the_table_lookup_is_none_outside_every_bounds() {
    let table = MonitorSources::from_monitors(&two_monitors()).dpi_table;
    assert_eq!(table.dpi_for_point(2560, 0), None, "右端は含まない");
    assert_eq!(
        MonitorDpiTable::default().dpi_for_point(0, 0),
        None,
        "空の表は架空の拡大率を発明しない"
    );
}
