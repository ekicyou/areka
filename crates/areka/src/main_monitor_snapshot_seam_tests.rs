use super::*;
use placement::diag::{
    MONITOR_RECORD_TAG, MONITOR_SNAPSHOT_TAG, MonitorRecord, monitor_record_line,
    monitor_snapshot_header_line,
};
use placement::follow::MonitorSources;
use placement::test_support::capture_logs;
use windows::Win32::Foundation::RECT;
use wintf::ecs::window::monitor::Monitor;

/// 実機の消失事象と同域の合成構成（混在 DPI 96/120/192・非対称 work area・
/// 負座標・3200 超座標）。実 HMONITOR 不要（placement/mod.rs テストと同流儀）。
fn synthetic_monitors() -> Vec<Monitor> {
    vec![
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
            dpi: 120,
            is_primary: true,
        },
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
            dpi: 96,
            is_primary: false,
        },
    ]
}

/// `[diag.*]` 行だけを抜き出す（同時に走る他の観測ログを混ぜない）。
fn diag_lines(events: &[placement::test_support::LogEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with(MONITOR_SNAPSHOT_TAG) || m.starts_with(MONITOR_RECORD_TAG))
        .collect()
}

/// 要件 1.1 正典出力点: 構築点で全モニタの識別子・bounds・work_area・DPI・primary が
/// 物理 px で 1 回出力され、行は呼出点タグ `monitor_snapshot` を名乗る。
/// ログだけからモニタ構成（台数・各台の全 5 フィールド）を再構成できる。
#[test]
fn boot_snapshot_logs_every_monitor_with_the_canonical_call_site_tag() {
    let monitors = synthetic_monitors();
    let (_, events) = capture_logs(|| boot_monitor_snapshot(&monitors));

    let expected: Vec<MonitorRecord> = placement::monitor_records(&monitors);
    let mut want = vec![monitor_snapshot_header_line("monitor_snapshot", 3)];
    want.extend(
        expected
            .iter()
            .enumerate()
            .map(|(i, r)| monitor_record_line(r, i)),
    );
    assert_eq!(
        diag_lines(&events),
        want,
        "構築点の出力が見出し＋全モニタ 1 行ずつ・呼出点タグ monitor_snapshot で出ていない"
    );

    // ログだけからモニタ構成を再構成できる（work area・DPI・primary が実値で読める）。
    let lines = diag_lines(&events);
    assert!(lines[1].contains("work_area=0,0,1920,1040") && lines[1].contains("dpi=120"));
    assert!(lines[2].contains("work_area=-1840,-40,0,1000") && lines[2].contains("dpi=192"));
    assert!(lines[3].contains("work_area=1920,-200,5120,1520") && lines[3].contains("dpi=96"));
    assert_eq!(
        lines.iter().filter(|l| l.contains("primary=true")).count(),
        1,
        "primary 標識が 1 台ぶんだけ立つ: {lines:?}"
    );
}

/// 呼出点タグは列挙点（`prepare_ghost_windows`）と**別**でなければならない
/// （要件 1.1「呼出点タグで区別できる」）。構築点をもう一方のタグへ collapse する
/// 退行はここで赤になる。
#[test]
fn boot_snapshot_tag_differs_from_the_prepare_call_site_tag() {
    let (_, events) = capture_logs(|| boot_monitor_snapshot(&synthetic_monitors()));
    let header = diag_lines(&events).remove(0);
    assert!(
        header.contains("context=monitor_snapshot"),
        "構築点の呼出点タグが正典値でない: {header}"
    );
    assert!(
        !header.contains(&format!(
            "context={}",
            placement::PREPARE_GHOST_WINDOWS_CONTEXT
        )),
        "構築点が列挙点の呼出点タグを名乗っている（弁別不能）: {header}"
    );
}

/// シームは観測を足すだけで、権威 Resource の中身（placement の全判断が読む値）を
/// 一切変えない（D2: 観測増設は挙動変更に数えない）。
///
/// 起動時が作るのは作業領域源と**モニタ別拡大率表**の 2 源であり、実行時の同期段
/// （`emo2_boot::frame::work_area_sync`）と**同一の構築関数**を通る（atom task 5.1・
/// 要件 5.1）——起動時だけが別の作り方をすると、同期が入った後も起動時の値だけが違う
/// 形になり得る。
#[test]
fn boot_snapshot_returns_the_same_authority_resource_as_before() {
    let monitors = synthetic_monitors();
    assert_eq!(
        boot_monitor_snapshot(&monitors),
        MonitorSources::from_monitors(&monitors),
        "観測増設が権威 Resource の中身を変えている"
    );
}

/// 起動時に挿す 2 源は**同じモニタ列**から作られ、台数も並びも一致する（片方だけ古い運転を
/// 作らない・atom C6）。
#[test]
fn boot_snapshot_builds_both_sources_from_the_same_monitor_list() {
    let monitors = synthetic_monitors();
    let sources = boot_monitor_snapshot(&monitors);
    assert_eq!(sources.len(), monitors.len(), "台数が列挙と食い違っている");
    assert_eq!(
        sources.snapshot.work_areas.len(),
        sources.dpi_table.entries.len(),
        "2 源の台数が食い違っている（同じ列から作られていない）"
    );
    let dpis: Vec<u32> = sources.dpi_table.entries.iter().map(|e| e.dpi).collect();
    assert_eq!(
        dpis,
        monitors.iter().map(|m| m.dpi).collect::<Vec<u32>>(),
        "モニタ別拡大率表が列挙順の拡大率を忠実転写していない"
    );
}

/// モニタ 0 台（列挙異常）でも panic せず `count=0` の見出しが出る
/// （レコード不在では「出力が無効だった」と区別できない）。
#[test]
fn boot_snapshot_with_no_monitors_still_reports_count_zero() {
    let (sources, events) = capture_logs(|| boot_monitor_snapshot(&[]));
    assert!(sources.is_empty());
    assert_eq!(
        diag_lines(&events),
        vec!["[diag.monitor_snapshot] context=monitor_snapshot count=0"]
    );
}
