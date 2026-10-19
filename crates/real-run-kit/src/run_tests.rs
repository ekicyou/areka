//! 既定の時間の大小（要件 1.8・D-13）。

use super::Limits;
use areka_mcp_bridge::config::Timeouts;

/// 保険の自動終了が窓口の締切より先に来る（窓口が止めるより先に子が自分で終わる）。
#[test]
fn safety_exit_comes_before_deadline() {
    let l = Limits::default();
    assert!(l.safety_exit < l.deadline, "{l:?}");
}

/// 中継の答えの上限（`config::Timeouts` の既定）が窓口の締切より短い。
#[test]
fn bridge_reply_limit_is_shorter_than_deadline() {
    let l = Limits::default();
    let reply = Timeouts::default().reply;
    assert!(reply < l.deadline, "中継の答えの上限 {reply:?}・{l:?}");
}
