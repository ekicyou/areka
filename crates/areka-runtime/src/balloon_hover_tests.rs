//! `balloon_hover.rs` の兄弟テスト。

use super::BalloonHover;

/// 初期状態は全 scope 偽（未観測の scope を真と誤らない）。
#[test]
fn balloon_hover_defaults_to_false_for_every_scope() {
    let bw = BalloonHover::default();
    assert!(!bw.is_balloon_hovered(0), "未観測 scope 0 は偽");
    assert!(!bw.is_balloon_hovered(7), "未観測 scope 7 は偽");
}

/// 掃除口: 非表示遷移で呼ぶ `clear_balloon_hover` は当該 scope の
/// みを偽にし、再呼出でも壊れない（不可視中は `PointerLeave` が届かないための明示掃除）。
#[test]
fn clear_balloon_hover_clears_that_scope_only_and_is_idempotent() {
    let mut bw = BalloonHover::default();
    bw.set_balloon_hover(0);
    bw.set_balloon_hover(1);

    bw.clear_balloon_hover(0);
    assert!(!bw.is_balloon_hovered(0), "掃除した scope 0 は偽");
    assert!(bw.is_balloon_hovered(1), "他 scope は影響を受けない");

    bw.clear_balloon_hover(0);
    assert!(!bw.is_balloon_hovered(0), "再呼出でも偽のまま（冪等）");

    // 掃除後に再びバルーン窓上へ入れば再び真になる（掃除は恒久禁止ではない）。
    bw.set_balloon_hover(0);
    assert!(bw.is_balloon_hovered(0), "掃除後の再滞在は再び真");
}
