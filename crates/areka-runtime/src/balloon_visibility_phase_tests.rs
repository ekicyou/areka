// =============================================================================
// バルーン可視性の配線層（相関数）の決定論テスト（task 4.4）
//
// 判断そのものの網羅表は親の `balloon_visibility_tests.rs`（判断中核 `decide`）が所有する。
// ここが押さえるのは**配線**だけである:
//   ⑴ 観測の収集（可視状態・可視グリフ数・ポインタ滞在・ドラッグ中・信号）
//   ⑵ 判断が返した行動の発行（表示は専用入口・非表示は既存漏斗）と失敗時の縮退
//   ⑶ 判断が返した事象のログへの写し（design 決定 D10 の水準・フィールド）
//   ⑷ 自分が発行していない可視性変化の検出（`trigger=explicit`）と滞在記録の掃除
//
// GPU も実時間の待機も用いない。時刻は `FrameTime` 資源と `TalkClock` の epoch で注入する
// （Requirement 9.2 / 9.3）。表示層は実 `EmoPresenter` を headless に用いる——`attach_target`
// は GPU 資源を要さず、可視状態の照会（`target_visible`）と非表示の適用（`apply(Hide)`）は
// そのまま通る。
// =============================================================================

use super::*;

/// ドラッグ中の観測はバルーン窓の標識とドラッグ標識の**連言**で成立する（design 決定 D8）。
#[test]
fn dragging_requires_both_balloon_window_and_drag_marker() {
    let mut world = World::new();
    assert!(!observe_dragging(&mut world), "標識が無ければ成立しない");

    let balloon = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    assert!(
        !observe_dragging(&mut world),
        "バルーン窓だけではドラッグ中ではない"
    );

    let other = world.spawn(WindowDragging).id();
    assert!(
        !observe_dragging(&mut world),
        "バルーン窓でない窓のドラッグは対象外"
    );

    world.entity_mut(balloon).insert(WindowDragging);
    assert!(observe_dragging(&mut world), "両方揃えば成立する");

    world.entity_mut(balloon).remove::<WindowDragging>();
    world.despawn(other);
    assert!(!observe_dragging(&mut world), "ドラッグ終了で戻る");
}
