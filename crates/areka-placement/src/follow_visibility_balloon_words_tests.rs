//! バルーンの守りが出す判定語の檻（audit-placement-follow・指摘 4）。
//!
//! バルーンには射影の段が無く、守りは入力と提案に同じ位置を渡す。動かしていない窓を
//! 「引き寄せた」（`OffscreenPull`）と記録しないことを、寄せ（`ClampX`）と留置（`Keep`）の
//! 2 つの腕で固める。寄せた後も縦に見えないときは `ClampX` でなく `HiddenAfterClampX` と
//! 記録することも固める（指摘 6）。守りの関数を矩形で直接呼ぶ（バルーンの呼び口と同じ引数の形）。

use super::evaluate_visibility_guard;
use super::test_support::{
    CLAMP_TAG, DPIS, HIDDEN_AFTER_CLAMP_TAG, OFFSCREEN_PULL_TAG, balloon_size, guard_events,
    mixed_layout, point, px, right_wa, visible_in, win,
};
use super::{MonitorSnapshot, PlacementRoute};
use crate::resolver::PointPx;
use bevy_ecs::prelude::*;
use log_capture_kit::{CapturedEvent, capture, expect_one};

/// バルーンの呼び口と同じ形（入力＝提案）で守りを呼び、決めた位置と記録を返す。
fn guard_balloon(
    snapshot: &MonitorSnapshot,
    old_pos: PointPx,
    proposed: PointPx,
    dpi: i32,
) -> (PointPx, Vec<CapturedEvent>) {
    let size = balloon_size(dpi);
    let entity = World::new().spawn_empty().id();
    capture(|| {
        evaluate_visibility_guard(
            entity,
            PlacementRoute::BalloonFollow,
            Some(snapshot),
            Some(win(old_pos, size)),
            proposed,
            proposed,
            size,
        )
    })
}

/// 右モニタの右端の外（x=2000 から幅 500）＝どの作業領域とも交わらない提案。
fn off_right(dpi: i32) -> PointPx {
    point(px(2000, dpi), px(240, dpi))
}

/// 見えていたバルーンが外へ出る提案は `ClampX` で寄せ、`OffscreenPull` は出さない。
#[test]
fn a_clamped_balloon_is_not_recorded_as_offscreen_pull() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let size = balloon_size(dpi);
        let old = point(px(800, dpi), px(240, dpi));
        let proposed = off_right(dpi);
        // 自己検査: 旧は見え、提案は見えない（ClampX の腕に入る形）。
        assert!(visible_in(&layout, old, size), "dpi={dpi}");
        assert!(!visible_in(&layout, proposed, size), "dpi={dpi}");

        let (pos, events) = guard_balloon(&layout, old, proposed, dpi);

        assert_eq!(
            pos,
            point(right_wa(dpi).right - size.w, proposed.y),
            "dpi={dpi}: X は右モニタの中へ寄る"
        );
        expect_one(&events, CLAMP_TAG);
        assert_eq!(
            guard_events(&events, OFFSCREEN_PULL_TAG).len(),
            0,
            "dpi={dpi}: 入力と提案が同じなのに OffscreenPull が出た"
        );
    }
}

/// もともと外に置かれていたバルーンは留置（`Keep`）で、`OffscreenPull` は出さない。
#[test]
fn a_kept_offscreen_balloon_is_not_recorded_as_offscreen_pull() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let size = balloon_size(dpi);
        let old = point(px(2100, dpi), px(240, dpi));
        let proposed = off_right(dpi);
        // 自己検査: 旧も提案も見えない（留置の腕に入る形）。
        assert!(!visible_in(&layout, old, size), "dpi={dpi}");
        assert!(!visible_in(&layout, proposed, size), "dpi={dpi}");

        let (pos, events) = guard_balloon(&layout, old, proposed, dpi);

        assert_eq!(pos, proposed, "dpi={dpi}: 留置は位置を変えない");
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: 留置で ClampX が出た"
        );
        assert_eq!(
            guard_events(&events, OFFSCREEN_PULL_TAG).len(),
            0,
            "dpi={dpi}: 入力と提案が同じなのに OffscreenPull が出た"
        );
    }
}

/// 縦に見えないまま寄せた（下に揃えたキャラ窓 y=640・offset (100,−1000)・提案 (1600,−360)）
/// バルーンは `HiddenAfterClampX` が 1 行・`ClampX` が 0 行で、位置は今と同じ x=1420。
#[test]
fn a_balloon_still_hidden_after_clamp_is_recorded_as_hidden_after_clamp_x() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let size = balloon_size(dpi);
        let old = point(px(800, dpi), px(240, dpi));
        let proposed = point(px(1600, dpi), px(-360, dpi));
        // 自己検査: 旧は見え、提案は見えない（ClampX の腕に入る形）。
        assert!(visible_in(&layout, old, size), "dpi={dpi}");
        assert!(!visible_in(&layout, proposed, size), "dpi={dpi}");

        let (pos, events) = guard_balloon(&layout, old, proposed, dpi);

        assert_eq!(
            pos,
            point(px(1420, dpi), proposed.y),
            "dpi={dpi}: 位置は今と同じ（X だけ寄せ、Y は不変）"
        );
        assert!(
            !visible_in(&layout, pos, size),
            "dpi={dpi}: 寄せた後も見えない形"
        );
        let hidden = expect_one(&events, HIDDEN_AFTER_CLAMP_TAG);
        assert_eq!(hidden.level, tracing::Level::WARN, "dpi={dpi}");
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: 寄せた後も見えないのに ClampX が出た: {events:?}"
        );
    }
}

/// 寄せた後に作業領域と交わる正常系は `ClampX` が 1 行・`HiddenAfterClampX` が 0 行（対）。
#[test]
fn a_balloon_visible_after_clamp_is_not_recorded_as_hidden_after_clamp_x() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let size = balloon_size(dpi);
        let old = point(px(800, dpi), px(240, dpi));
        let proposed = off_right(dpi);
        assert!(visible_in(&layout, old, size), "dpi={dpi}");
        assert!(!visible_in(&layout, proposed, size), "dpi={dpi}");

        let (pos, events) = guard_balloon(&layout, old, proposed, dpi);

        assert!(
            visible_in(&layout, pos, size),
            "dpi={dpi}: 寄せた後は見える形"
        );
        expect_one(&events, CLAMP_TAG);
        assert!(
            guard_events(&events, HIDDEN_AFTER_CLAMP_TAG).is_empty(),
            "dpi={dpi}: 寄せた後に見えるのに HiddenAfterClampX が出た: {events:?}"
        );
    }
}
