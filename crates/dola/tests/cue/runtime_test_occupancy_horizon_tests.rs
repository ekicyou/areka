//! 占有 horizon の檻——完了判定の閾値（原本の区画 `:515`）と、絶対時刻としての照会
//! （原本の区画 `:1005`）。いずれも同一の horizon 権威を観測する。

use super::test_support::{barrier, choice, text};
use super::{ActorKey, BarrierKind, Cue, CueCommand, CuePlayer, CuePlayerState, CueSheet};

// ============================================================================
// 完了は占有 horizon 到達で（entry 枯渇だけでは完了しない）
// ============================================================================

/// 末尾に待ちを持つ台本では、全 cue の**配送完了**時点ではまだ Playing のままで、
/// 占有 horizon（最終 Wait の duration 端）到達で初めて `Completed` へ遷移する
/// （早期終了しない・D6/R2.5 の CuePlayer レベル固定）。
#[test]
fn player_reaches_completed_only_at_occupancy_horizon() {
    let d = 0.15_f64;
    let wait_dur = 0.8_f64;
    let sheet = CueSheet::new(vec![
        text(0.0, "bye", d),
        Cue {
            actor: ActorKey::from("0"),
            start_time: d,
            payload: CueCommand::Wait.into(),
            duration: wait_dur,
        },
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);

    // 全 entry を配り終える時刻（末尾 Wait の発火時刻＝d）まで進める。
    player.tick(d);
    assert_eq!(player.remaining(), 0, "全 entry を配り終えた");
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "配送完了時点では占有 horizon 未到達ゆえまだ Playing（早期終了しない）"
    );

    // 占有 horizon 直前も未完了。
    player.tick(d + wait_dur - 0.01);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "horizon 直前は未完了"
    );

    // 占有 horizon 到達で Completed。
    player.tick(d + wait_dur);
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "占有 horizon 到達で初めて Completed"
    );
}

/// バリアの無い台本は占有 horizon（全 cue が瞬時なら最終 at）到達で完了する。
#[test]
fn barrier_free_sheet_completes_at_last_cue() {
    let sheet = CueSheet::new(vec![text(0.0, "a", 0.0), text(0.5, "b", 0.0)]);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(0.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "b 未到達ゆえ Playing"
    );
    player.tick(0.5);
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "最終 cue（horizon）到達で完了"
    );
}

// ============================================================================
// 占有 horizon の絶対時刻照会（Task 3.1・R7.2）
// ============================================================================

/// `CuePlayer::occupancy_horizon()` は内部 `TimedSchedule` へ委譲し、アンカー
/// （`CueSheet::absolute_start_time`）＋相対 horizon（`max(start_time + duration)`）を返す。
/// 選択待ちタイムアウトの計測起点はこの **duration 権威**から取得され、再生層の外で
/// 独自の時間基準を作らない（R7.2）。
#[test]
fn occupancy_horizon_returns_anchor_plus_relative_horizon() {
    // 相対 horizon = max(0.0+1.5, 2.0+0.5) = 2.5、アンカー = 100.0 → 102.5。
    let sheet = CueSheet::new(vec![text(0.0, "a", 1.5), text(2.0, "b", 0.5)])
        .with_absolute_start_time(100.0);
    let player = CuePlayer::from_sheet(&sheet);

    assert_eq!(
        player.occupancy_horizon(),
        102.5,
        "アンカー 100.0 ＋ 相対 horizon 2.5 = 102.5"
    );
    assert_eq!(
        player.occupancy_horizon(),
        sheet.absolute_end_time(),
        "台本の絶対終了時刻（duration 権威）と一致する"
    );
}

/// アンカー未刻印（0.0）の台本では相対 horizon がそのまま絶対値になる。
#[test]
fn occupancy_horizon_without_anchor_is_the_relative_horizon() {
    let sheet = CueSheet::new(vec![text(0.0, "a", 0.25), text(1.0, "b", 2.0)]);
    let player = CuePlayer::from_sheet(&sheet);

    assert_eq!(player.occupancy_horizon(), 3.0, "max(0.25, 3.0) = 3.0");
}

/// 選択肢バリアで停止中（`WaitingForChoice`）でも占有 horizon は照会でき、値は不変。
/// これがタイムアウト計測の起点（トークの絶対終了時刻）になる（R7.1/R7.2）。
#[test]
fn occupancy_horizon_is_observable_while_waiting_for_choice() {
    let sheet = CueSheet::new(vec![
        text(0.0, "問い", 1.0),
        choice(1.0, "c1", "はい"),
        barrier(1.0, BarrierKind::WaitForChoice { timeout: None }),
        text(1.0, "続き", 0.5),
    ])
    .with_absolute_start_time(50.0);
    let mut player = CuePlayer::from_sheet(&sheet);

    assert_eq!(player.occupancy_horizon(), 51.5, "50.0 + max(1.0, 1.5) = 51.5");

    player.tick(51.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "選択肢バリアで停止する"
    );
    assert_eq!(
        player.occupancy_horizon(),
        51.5,
        "選択待ち中でも占有 horizon の絶対時刻は不変（計測起点の権威）"
    );
}

/// `occupancy_horizon()` が返す絶対時刻が完了判定の閾値と一致する
/// （`is_completed()` と同一の horizon 権威を見ている）。
#[test]
fn occupancy_horizon_matches_completion_threshold() {
    let sheet =
        CueSheet::new(vec![text(0.0, "a", 2.0)]).with_absolute_start_time(10.0);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(10.0);
    assert!(!player.is_completed(), "duration 端未到達では完了しない");

    player.tick(player.occupancy_horizon());
    assert!(
        player.is_completed(),
        "occupancy_horizon() の絶対時刻到達で占有終了する"
    );
}

/// `stop()`（中断）後はアンカーそのものを返す（schedule の clear で相対 horizon が 0.0 へ
/// 落ちる＝中断終端の talk に占有区間は残らない）。
#[test]
fn occupancy_horizon_after_stop_is_the_anchor() {
    let sheet =
        CueSheet::new(vec![text(0.0, "a", 2.0)]).with_absolute_start_time(10.0);
    let mut player = CuePlayer::from_sheet(&sheet);
    assert_eq!(player.occupancy_horizon(), 12.0);

    player.stop();
    assert_eq!(
        player.occupancy_horizon(),
        10.0,
        "中断後は相対 horizon 0.0 ＝ アンカーそのもの"
    );
}
