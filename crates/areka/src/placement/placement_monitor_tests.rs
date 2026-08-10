use windows::Win32::Foundation::RECT;
use wintf::ecs::window::monitor::Monitor;

use super::shared_test_support::balloon_root;
use super::resolver::RectPx;
use super::*;

/// テスト用合成 Monitor（実 HMONITOR 不要・wintf monitor.rs テストと同流儀）。
fn make_monitor(handle: isize, work: (i32, i32, i32, i32), is_primary: bool) -> Monitor {
    let (left, top, right, bottom) = work;
    Monitor {
        handle,
        bounds: RECT {
            left,
            top,
            right,
            bottom: bottom + 40, // work_area はタスクバーぶん狭い想定
        },
        work_area: RECT {
            left,
            top,
            right,
            bottom,
        },
        dpi: 96,
        is_primary,
    }
}

// ------------------------------------------------------------------
// primary_monitor／work_area_of（純粋・合成 Monitor で決定論）
// ------------------------------------------------------------------

/// モニタ 0 台は `PlacementError::Monitor`（架空の既定矩形を発明しない）。
#[test]
fn primary_work_area_empty_is_monitor_err() {
    let err = work_area_of(primary_monitor(&[])).expect_err("0 台は Err");
    assert!(
        matches!(err, PlacementError::Monitor { .. }),
        "Monitor variant 以外が返った: {err:?}"
    );
}

/// `is_primary` のモニタの work area（物理 px）が RectPx へ忠実転写される
/// （単位変換なし・2.12／U 契約）。
#[test]
fn primary_work_area_picks_is_primary() {
    let monitors = [
        make_monitor(1, (-1920, 0, 0, 1040), false),
        make_monitor(2, (0, 0, 2560, 1400), true),
    ];
    let wa = work_area_of(primary_monitor(&monitors)).expect("primary あり");
    assert_eq!(
        wa,
        RectPx {
            left: 0,
            top: 0,
            right: 2560,
            bottom: 1400
        }
    );
}

/// primary フラグ無し（列挙異常）は先頭モニタで代替（warn・窓は生やす方針）。
#[test]
fn primary_work_area_no_primary_substitutes_first() {
    let monitors = [
        make_monitor(1, (0, 0, 1920, 1040), false),
        make_monitor(2, (1920, 0, 3840, 1040), false),
    ];
    let wa = work_area_of(primary_monitor(&monitors)).expect("非空なら Ok");
    assert_eq!(
        wa,
        RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040
        }
    );
}

// ------------------------------------------------------------------
// MonitorSnapshot::from_monitors（task 8.1・DD15・実モニタ→snapshot 忠実転写）
// ------------------------------------------------------------------

/// `MonitorSnapshot::from_monitors` は全モニタの work area（物理 px）を列挙順の
/// まま**単位変換なしで忠実転写**する（`primary_work_area` と同じ U 契約）。
#[test]
fn monitor_snapshot_from_monitors_transcribes_all_work_areas_in_order() {
    let monitors = [
        make_monitor(1, (-1920, -40, 0, 1000), false),
        make_monitor(2, (0, 0, 2560, 1400), true),
    ];
    let snapshot = follow::MonitorSnapshot::from_monitors(&monitors);
    assert_eq!(
        snapshot.work_areas,
        vec![
            RectPx {
                left: -1920,
                top: -40,
                right: 0,
                bottom: 1000
            },
            RectPx {
                left: 0,
                top: 0,
                right: 2560,
                bottom: 1400
            },
        ]
    );
}

/// 0 台では空 snapshot（panic しない・消費側 `work_area_for_window` が None 防御）。
#[test]
fn monitor_snapshot_from_monitors_empty_is_empty() {
    assert!(
        follow::MonitorSnapshot::from_monitors(&[])
            .work_areas
            .is_empty()
    );
}


// ------------------------------------------------------------------
// 起動時モニタスナップショット（areka-P0-dpi-window-vanish task 1.2・要件 1.1）
// ------------------------------------------------------------------

/// 混在 DPI マルチモニタ実機相当（96/120/192・非対称 work area・負座標・3200 超）の
/// 列挙結果が、識別子・bounds・work_area・DPI・primary の**全 5 フィールド**を
/// 単位変換も丸めもせず（物理 px そのまま）列挙順で転写される（要件 1.1）。
///
/// 実モニタを要さない純関数ゆえ、実機で 1 度しか踏めない構成をここで全部踏む。
#[test]
fn monitor_records_transcribe_every_field_in_physical_px() {
    let monitors = [
        // 左隣・負座標・非対称 work area（左にタスクバー）・192dpi
        Monitor {
            handle: -3,
            bounds: RECT {
                left: -1920,
                top: -40,
                right: 0,
                bottom: 1040,
            },
            work_area: RECT {
                left: -1840,
                top: -40,
                right: 0,
                bottom: 1000,
            },
            dpi: 192,
            is_primary: false,
        },
        // primary・96dpi
        Monitor {
            handle: 65537,
            bounds: RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            work_area: RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            },
            dpi: 96,
            is_primary: true,
        },
        // 右隣・3200 超座標（実機の消失事象と同域）・120dpi
        Monitor {
            handle: 65539,
            bounds: RECT {
                left: 1920,
                top: -200,
                right: 5120,
                bottom: 1600,
            },
            work_area: RECT {
                left: 1920,
                top: -200,
                right: 5120,
                bottom: 1520,
            },
            dpi: 120,
            is_primary: false,
        },
    ];

    assert_eq!(
        monitor_records(&monitors),
        vec![
            diag::MonitorRecord {
                handle: -3,
                bounds: (-1920, -40, 0, 1040),
                work_area: (-1840, -40, 0, 1000),
                dpi: 192,
                is_primary: false,
            },
            diag::MonitorRecord {
                handle: 65537,
                bounds: (0, 0, 1920, 1080),
                work_area: (0, 0, 1920, 1040),
                dpi: 96,
                is_primary: true,
            },
            diag::MonitorRecord {
                handle: 65539,
                bounds: (1920, -200, 5120, 1600),
                work_area: (1920, -200, 5120, 1520),
                dpi: 120,
                is_primary: false,
            },
        ],
        "wintf Monitor → MonitorRecord は列挙順の忠実転写（単位変換・丸めなし・U 契約）"
    );
}

/// 0 台（列挙異常・headless）でも panic せず空を返す——台数 0 の観測は
/// `log_monitor_snapshot` の `count=0` 見出しが担う（レコード不在では
/// 「出力が無効だった」と区別できない）。
#[test]
fn monitor_records_of_zero_monitors_is_empty() {
    assert!(monitor_records(&[]).is_empty());
}

/// 呼出点タグは 2 つの列挙点で**異なる**（要件 1.1「呼出点タグで区別できる」）。
/// 片方をもう片方へ collapse する退行はここと各呼出点の檻で赤になる。
#[test]
fn monitor_snapshot_call_site_tags_are_distinct() {
    assert_eq!(PREPARE_GHOST_WINDOWS_CONTEXT, "prepare_ghost_windows");
    assert_eq!(MONITOR_SNAPSHOT_CONTEXT, "monitor_snapshot");
    assert_ne!(
        PREPARE_GHOST_WINDOWS_CONTEXT, MONITOR_SNAPSHOT_CONTEXT,
        "2 つの列挙点が同じタグを名乗るとログ上で出所を弁別できない"
    );
}

/// ゴースト窓配置準備のモニタ列挙点が、共有ヘルパ経由で**自分の呼出点タグ**付きの
/// スナップショットを出す（要件 1.1）。
///
/// 出力は列挙の直後＝準備段の失敗より**手前**に置く。不在 root（`Mount` で落ちる
/// 最短経路）でもモニタ構成が残ることが、フォールバック窓へ落ちた運転のログからも
/// モニタ構成を再構成できる条件である。
#[test]
fn prepare_ghost_windows_logs_snapshot_with_its_own_call_site_tag() {
    let root = std::env::temp_dir()
        .join("areka_placement_prepare_snapshot_log")
        .join("no_such_ghost");
    let (result, events) =
        crate::placement::test_support::capture_logs(|| prepare_ghost_windows(&root, &balloon_root()));
    assert!(result.is_err(), "不在 root は Err（列挙点の出力はその手前）");

    let expected = monitor_records(&enumerate_monitors());
    let lines: Vec<&str> = events
        .iter()
        .map(|e| e.message())
        .filter(|m| {
            m.starts_with(diag::MONITOR_SNAPSHOT_TAG) || m.starts_with(diag::MONITOR_RECORD_TAG)
        })
        .collect();

    let mut want = vec![diag::monitor_snapshot_header_line(
        "prepare_ghost_windows",
        expected.len(),
    )];
    want.extend(
        expected
            .iter()
            .enumerate()
            .map(|(i, r)| diag::monitor_record_line(r, i)),
    );
    assert_eq!(
        lines,
        want.iter().map(String::as_str).collect::<Vec<_>>(),
        "列挙点の出力が見出し＋全モニタ 1 行ずつ・呼出点タグ prepare_ghost_windows で出ていない"
    );
    assert!(
        !lines[0].contains("context=monitor_snapshot"),
        "main.rs 構築点の呼出点タグと collapse している（弁別不能）: {}",
        lines[0]
    );
}
