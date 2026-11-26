//! 占有 horizon の絶対時刻としての照会の檻（完了判定の閾値と同じ horizon 権威を観測する）。

use super::{BarrierKind, CuePlayer, CuePlayerState, CueSheet, at_ms};
use crate::test_support::{barrier, choice, ms, text};

// ============================================================================
// 占有 horizon の絶対時刻照会
// ============================================================================

/// `CuePlayer::occupancy_horizon()` は内部 `TimedSchedule` へ委譲し、アンカー
/// （`CueSheet::absolute_start_time`）＋相対 horizon（`max(start_time + duration)`）を返す。
/// 選択待ちタイムアウトの計測起点はこの **duration 権威**から取得され、再生層の外で
/// 独自の時間基準を作らない。
#[test]
fn occupancy_horizon_returns_anchor_plus_relative_horizon() {
    // 相対 horizon = max(0+1500, 2000+500) = 2500 ms、アンカー = 100,000 ms → 102,500 ms。
    let sheet = CueSheet::new(vec![
        text(ms(0), "a", ms(1500)),
        text(ms(2000), "b", ms(500)),
    ])
    .with_absolute_start_time(at_ms(100_000));
    let player = CuePlayer::from_sheet(&sheet).unwrap();

    assert_eq!(
        player.occupancy_horizon(),
        at_ms(102_500),
        "アンカー 100 秒 ＋ 相対 horizon 2.5 秒 = 102.5 秒"
    );
    assert_eq!(
        Some(player.occupancy_horizon()),
        sheet.absolute_end_time(),
        "台本の絶対終了時刻（duration 権威）と一致する"
    );
}

/// 選択肢バリアで停止中（`WaitingForChoice`）でも占有 horizon は照会でき、値は不変。
/// これがタイムアウト計測の起点（トークの絶対終了時刻）になる。
#[test]
fn occupancy_horizon_is_observable_while_waiting_for_choice() {
    let sheet = CueSheet::new(vec![
        text(ms(0), "問い", ms(1000)),
        choice(ms(1000), "c1", "はい"),
        barrier(ms(1000), BarrierKind::WaitForChoice { timeout: None }),
        text(ms(1000), "続き", ms(500)),
    ])
    .with_absolute_start_time(at_ms(50_000));
    let mut player = CuePlayer::from_sheet(&sheet).unwrap();

    assert_eq!(
        player.occupancy_horizon(),
        at_ms(51_500),
        "50 秒 + max(1.0, 1.5) 秒 = 51.5 秒"
    );

    player.tick(at_ms(51_000));
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "選択肢バリアで停止する"
    );
    assert_eq!(
        player.occupancy_horizon(),
        at_ms(51_500),
        "選択待ち中でも占有 horizon の絶対時刻は不変（計測起点の権威）"
    );
}

/// `occupancy_horizon()` が返す絶対時刻が完了判定の閾値と一致する
/// （`is_completed()` と同一の horizon 権威を見ている）。
#[test]
fn occupancy_horizon_matches_completion_threshold() {
    let sheet =
        CueSheet::new(vec![text(ms(0), "a", ms(2000))]).with_absolute_start_time(at_ms(10_000));
    let mut player = CuePlayer::from_sheet(&sheet).unwrap();

    player.tick(at_ms(10_000));
    assert!(!player.is_completed(), "duration 端未到達では完了しない");

    player.tick(at_ms(11_999));
    assert!(!player.is_completed(), "horizon の手前では完了しない");

    player.tick(player.occupancy_horizon());
    assert!(
        player.is_completed(),
        "occupancy_horizon() の絶対時刻到達で占有終了する"
    );
}
