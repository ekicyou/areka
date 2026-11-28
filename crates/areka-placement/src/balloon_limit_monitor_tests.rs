//! 押し戻しの関門が見る画面の選び方のテスト。

use super::*;
use crate::shared_test_support::placement_sized;
use crate::test_support::{contained, point};
use log_capture_kit::{ExpectField, capture, expect_one};

const LEFT: RectPx = RectPx {
    left: -1920,
    top: 0,
    right: 0,
    bottom: 1040,
};
const RIGHT: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 3840,
    bottom: 2100,
};
const CHAR_SIZE: SizePx = SizePx { w: 434, h: 687 };
const BALLOON_SIZE: SizePx = SizePx { w: 223, h: 158 };

fn two_monitors() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![LEFT, RIGHT],
    }
}

fn char_rect() -> RectPx {
    rect_at(point(200, 2100 - CHAR_SIZE.h), CHAR_SIZE)
}

fn push(pos: PointPx, script: bool) -> (PointPx, LimitMonitor, RectPx) {
    let (area, monitor) = limit_work_area(
        &two_monitors(),
        char_rect(),
        rect_at(pos, BALLOON_SIZE),
        script,
    )
    .expect("画面が 2 つあれば作業領域が決まる");
    let to = limit_correction(pos, BALLOON_SIZE, area).unwrap_or(pos);
    (to, monitor, area)
}

#[test]
fn script_balloon_fitting_on_the_left_monitor_stays_on_the_balloon_monitor() {
    let pos = point(-600, 500);

    let (to, monitor, area) = push(pos, true);

    assert_eq!(monitor, LimitMonitor::Balloon);
    assert_eq!(area, LEFT);
    assert_eq!(to, pos, "左の画面に収まるバルーンは押し戻さない");
}

#[test]
fn script_balloon_straddling_is_pushed_into_the_monitor_of_its_center() {
    let pos = point(-150, 500);

    let (to, monitor, area) = push(pos, true);

    assert_eq!(monitor, LimitMonitor::Balloon);
    assert_eq!(area, LEFT, "中心 x=-39 は左の画面");
    assert_eq!(to, point(-BALLOON_SIZE.w, 500));
    assert!(contained(to, BALLOON_SIZE, LEFT));
}

#[test]
fn without_script_the_balloon_is_pushed_back_to_the_body_monitor() {
    let pos = point(-600, 500);

    let (to, monitor, area) = push(pos, false);

    assert_eq!(monitor, LimitMonitor::Shell);
    assert_eq!(area, RIGHT);
    assert_eq!(to, point(0, 500));
}

#[test]
fn reapplying_the_pushed_result_with_the_same_size_chooses_the_same_monitor() {
    for (pos, script) in [(point(-150, 500), true), (point(-600, 500), false)] {
        let (to, monitor, area) = push(pos, script);

        let (again, monitor_again, area_again) = push(to, script);

        assert_eq!((monitor_again, area_again), (monitor, area));
        assert_eq!(again, to);
    }
}

#[test]
fn boot_gate_clamp_line_names_the_body_monitor_and_no_script() {
    let char_pos = point(200, 2100 - CHAR_SIZE.h);
    let offset = point(-600, 0);
    let placement = placement_sized(0, char_pos, offset, true, CHAR_SIZE, BALLOON_SIZE);

    let (out, events) = capture(|| apply_balloon_limit(vec![placement], &two_monitors()));

    assert_eq!(out[0].balloon_pos, point(0, char_pos.y));
    let hit = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert!(hit.expect_field("monitor").contains("shell"));
    assert_eq!(hit.expect_field("script"), "false");
}
