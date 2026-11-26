//! CueSheet 構築と canonical 変換 to_talk_schedule の結合テスト。
//!
//! 昇順ソート・起点の保存・先頭待ち保存・占有の終わりを、ちょうどの 100ns の値で検証する。

use crate::test_support::{barrier, drain, ms, text, wait};
use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CueError, CuePayload, CueSheet, to_talk_schedule,
};
use dola::time::{TimePoint, TimeSpan};

/// 起点 0 からのミリ秒の時刻。
fn pt(millis: i64) -> TimePoint {
    TimePoint::from_ticks(millis * 10_000)
}

fn clear(start_time: TimeSpan) -> Cue {
    Cue {
        actor: ActorKey::from("a"),
        start_time,
        payload: CueCommand::Clear.into(),
        duration: TimeSpan::ZERO,
    }
}

// ============================================================================
// CueSheet 構築テスト
// ============================================================================

#[test]
fn cue_sheet_sorts_by_start_time() {
    let sheet = CueSheet::new(vec![
        clear(ms(3000)),
        text(ms(1000), "first", TimeSpan::ZERO),
        text(ms(2000), "second", TimeSpan::ZERO),
    ]);

    let times: Vec<TimeSpan> = sheet.cues().iter().map(|c| c.start_time).collect();
    assert_eq!(times, vec![ms(1000), ms(2000), ms(3000)]);
}

#[test]
fn cue_sheet_empty() {
    let sheet = CueSheet::new(vec![]);
    assert!(sheet.is_empty());
    assert_eq!(sheet.cues().len(), 0);
}

#[test]
fn cue_sheet_len() {
    let sheet = CueSheet::new(vec![clear(TimeSpan::ZERO), clear(ms(1000))]);
    assert_eq!(sheet.cues().len(), 2);
    assert!(!sheet.is_empty());
}

#[test]
fn cue_sheet_stable_sort_preserves_equal_start_time_order() {
    let sheet = CueSheet::new(vec![
        text(ms(1000), "first", TimeSpan::ZERO),
        text(ms(1000), "second", TimeSpan::ZERO),
        clear(ms(500)),
    ]);

    assert!(matches!(
        sheet.cues()[0].payload,
        CuePayload::Command(CueCommand::Clear)
    ));
    match (&sheet.cues()[1].payload, &sheet.cues()[2].payload) {
        (CuePayload::Command(CueCommand::Text(t1)), CuePayload::Command(CueCommand::Text(t2))) => {
            assert_eq!(t1, "first");
            assert_eq!(t2, "second");
        }
        other => panic!("Expected two Text commands, got {other:?}"),
    }
}

// ============================================================================
// 自己完結した絶対時刻台本（absolute_start_time）テスト
// ============================================================================

/// 起点を押していない台本は、起点も絶対の時刻も持たない。
#[test]
fn unstamped_sheet_has_no_absolute_times() {
    let sheet = CueSheet::new(vec![text(ms(1000), "hi", ms(250))]);

    assert_eq!(sheet.absolute_start_time(), None);
    assert_eq!(sheet.absolute_fire_time(&sheet.cues()[0]), None);
    assert_eq!(sheet.absolute_end_time(), None);
}

/// 各 cue の絶対発火時刻は台本のみから `absolute_start_time + start_time` で導ける。
#[test]
fn absolute_fire_time_of_every_cue_derives_from_sheet_alone() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: TimeSpan::ZERO,
            payload: CueCommand::ClearAll.into(),
            duration: TimeSpan::ZERO,
        },
        text(ms(500), "hello", ms(250)),
        Cue {
            actor: ActorKey::from("a"),
            start_time: ms(2250),
            payload: CueCommand::Emote { key: "3".into() }.into(),
            duration: TimeSpan::ZERO,
        },
    ])
    .with_absolute_start_time(pt(100_000));

    let fired: Vec<Option<TimePoint>> = sheet
        .cues()
        .iter()
        .map(|cue| sheet.absolute_fire_time(cue))
        .collect();

    assert_eq!(
        fired,
        vec![Some(pt(100_000)), Some(pt(100_500)), Some(pt(102_250))]
    );
}

/// talk の絶対終了時刻は `absolute_start_time + max(start_time + duration)`。
///
/// 「最後に発火する cue」でも「最大 start_time」でもなく占有区間の最大端であることを
/// 固定する（最長 duration を持つ cue が末尾でない配置で検証）。
#[test]
fn absolute_end_time_is_max_of_start_plus_duration_not_last_cue_start() {
    let sheet = CueSheet::new(vec![
        text(ms(1000), "long", ms(5000)),
        wait(ms(3000), ms(500)),
    ])
    .with_absolute_start_time(pt(10_000));

    assert_eq!(sheet.absolute_end_time(), Some(pt(16_000)));
}

/// 末尾の純粋 Wait cue の duration は絶対終了時刻へ算入される（早期終了しない）。
#[test]
fn absolute_end_time_includes_trailing_wait_duration() {
    let sheet = CueSheet::new(vec![
        text(TimeSpan::ZERO, "bye", ms(150)),
        wait(ms(150), ms(2000)),
    ])
    .with_absolute_start_time(pt(50_000));

    assert_eq!(sheet.absolute_end_time(), Some(pt(52_150)));
}

/// 空台本は時間を占有しない＝絶対終了時刻はアンカーそのもの。
#[test]
fn absolute_end_time_of_empty_sheet_is_the_anchor() {
    let sheet = CueSheet::new(vec![]).with_absolute_start_time(pt(7_500));

    assert_eq!(sheet.absolute_end_time(), Some(pt(7_500)));
}

/// 刻印は相対時刻を書き換えない（cue の相対 start_time / duration は不変）。
#[test]
fn stamping_absolute_start_time_leaves_relative_times_untouched() {
    let cues = vec![
        text(TimeSpan::ZERO, "x", ms(500)),
        Cue {
            actor: ActorKey::from("a"),
            start_time: ms(500),
            payload: CueCommand::NewLine { ratio: 1.0 }.into(),
            duration: TimeSpan::ZERO,
        },
    ];
    let unstamped = CueSheet::new(cues.clone());
    let stamped = CueSheet::new(cues).with_absolute_start_time(pt(1_234_500));

    let rel = |s: &CueSheet| -> Vec<(TimeSpan, TimeSpan)> {
        s.cues()
            .iter()
            .map(|c| (c.start_time, c.duration))
            .collect()
    };
    assert_eq!(rel(&stamped), rel(&unstamped));
    assert_eq!(stamped.absolute_start_time(), Some(pt(1_234_500)));
    assert_eq!(stamped.absolute_end_time(), Some(pt(1_234_500) + ms(500)));
    assert_eq!(unstamped.relative_end(), ms(500));
}

// ============================================================================
// canonical 変換 to_talk_schedule
//
// 台本→時刻スケジュールへの唯一の変換で、dola ingress の単一権威。
// 起点＋相対 start_time を保存し、同一 at は記述順（FIFO）、占有の終わりを保持して
// is_completed を占有終了で判定する。起点の押されていない台本は Err。
// ============================================================================

/// 起点の押されていない台本は変換できない。
#[test]
fn to_talk_schedule_of_unstamped_sheet_is_origin_not_stamped() {
    let sheet = CueSheet::new(vec![text(TimeSpan::ZERO, "hi", TimeSpan::ZERO)]);

    assert!(matches!(
        to_talk_schedule(&sheet),
        Err(CueError::OriginNotStamped)
    ));
}

/// 先頭に待ちを持つ台本を変換しても待ちが消えない（相対 start_time を保存）。
#[test]
fn to_talk_schedule_preserves_leading_wait() {
    let sheet = CueSheet::new(vec![
        wait(ms(500), ms(500)),
        text(ms(1000), "hi", TimeSpan::ZERO),
    ])
    .with_absolute_start_time(TimePoint::ZERO);

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    let ready = drain(&mut schedule, pt(400));
    assert!(
        ready.is_empty(),
        "400ms では先頭 Wait(at=500ms) は未 due（食われていない）"
    );
    let ready = drain(&mut schedule, pt(500));
    assert_eq!(ready.len(), 1, "500ms で先頭 Wait が due");
    assert_eq!(ready[0].at, ms(500), "相対 start_time が保存される");
    assert!(matches!(ready[0].command, CueCommand::Wait));
}

/// 有限・非負の duration は envelope へそのまま運ばれる（無変形）。
/// 同一 at は記述順（FIFO）で取り出される。
#[test]
fn to_talk_schedule_carries_duration_untransformed() {
    let durations = [
        TimeSpan::ZERO,
        ms(50),
        ms(250),
        TimeSpan::try_from_ticks(3_333_333).unwrap(),
        TimeSpan::try_from_ticks(123_456_789).unwrap(),
        TimeSpan::try_from_ticks(1).unwrap(),
    ];
    let cues: Vec<Cue> = durations
        .iter()
        .enumerate()
        .map(|(i, &d)| text(TimeSpan::ZERO, &format!("t{i}"), d))
        .collect();
    let sheet = CueSheet::new(cues).with_absolute_start_time(TimePoint::ZERO);

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    let ready = drain(&mut schedule, TimePoint::ZERO);
    assert_eq!(ready.len(), durations.len(), "全 cue が due（同一 at=0）");
    for (i, &d) in durations.iter().enumerate() {
        assert_eq!(
            ready[i].command,
            CueCommand::Text(format!("t{i}")),
            "同一 at 群は記述順（FIFO）で並ぶ"
        );
        assert_eq!(ready[i].duration, d, "duration は無変形で運ばれる");
        assert_eq!(ready[i].at, TimeSpan::ZERO, "発火の隔たりも無変形で運ぶ");
        assert_eq!(ready[i].fire_time, TimePoint::ZERO);
        assert_eq!(ready[i].actor, ActorKey::from("0"), "演者も無変形で運ぶ");
    }
}

/// 搬送体の duration はコマンド種別を問わない一律フィールド。全 variant が
/// 瞬時（明示的 0）・時間占有の双方を canonical 変換で運べる。
#[test]
fn to_talk_schedule_duration_uniform_across_every_command_variant() {
    use dola::DynamicValue;

    let commands = vec![
        CueCommand::Text("hello".into()),
        CueCommand::Clear,
        CueCommand::Emote {
            key: "smile".into(),
        },
        CueCommand::Choice {
            id: "yes".into(),
            text: "はい".into(),
            references: vec![],
        },
        CueCommand::Custom {
            command: "fade".into(),
            params: DynamicValue::Null,
        },
        CueCommand::NewLine { ratio: 1.0 },
        CueCommand::BalloonSurface { key: "2".into() },
        CueCommand::Wait,
        CueCommand::ClearAll,
        CueCommand::AnchorBegin {
            id: "OnJump".into(),
            references: vec![],
        },
        CueCommand::AnchorEnd,
        CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
    ];

    for command in commands {
        for duration in [TimeSpan::ZERO, ms(1250)] {
            let sheet = CueSheet::new(vec![Cue {
                actor: ActorKey::from("0"),
                start_time: TimeSpan::ZERO,
                payload: CuePayload::Command(command.clone()),
                duration,
            }])
            .with_absolute_start_time(TimePoint::ZERO);
            let mut schedule = to_talk_schedule(&sheet).unwrap();
            let ready = drain(&mut schedule, TimePoint::ZERO);
            assert_eq!(ready.len(), 1);
            assert_eq!(
                ready[0].duration, duration,
                "{command:?} の搬送体も duration を一律に保持する"
            );
            assert_eq!(ready[0].command, command, "command は無変形で運ばれる");
        }
    }
}

/// 純粋な待ち（`CueCommand::Wait`）の時間も搬送体の duration が運ぶ（action は空）。
#[test]
fn to_talk_schedule_carries_wait_duration_without_action() {
    let sheet = CueSheet::new(vec![wait(TimeSpan::ZERO, ms(500))])
        .with_absolute_start_time(TimePoint::ZERO);

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    let ready = drain(&mut schedule, TimePoint::ZERO);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].command, CueCommand::Wait, "action は持たない");
    assert_eq!(
        ready[0].duration,
        ms(500),
        "Wait の時間は搬送体の duration が運ぶ（コマンド埋め込みでない）"
    );
}

/// Barrier ペイロードは skip されず Entry::Barrier へ経路づけられる。
#[test]
fn to_talk_schedule_routes_barrier_not_skipping() {
    let sheet = CueSheet::new(vec![
        text(TimeSpan::ZERO, "keep", TimeSpan::ZERO),
        barrier(ms(200), BarrierKind::WaitForInput),
        text(ms(300), "after", TimeSpan::ZERO),
    ])
    .with_absolute_start_time(TimePoint::ZERO);

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    let ready = drain(&mut schedule, pt(1000));
    assert_eq!(ready.len(), 1, "Barrier 手前の Command のみ配信");
    assert_eq!(ready[0].command, CueCommand::Text("keep".into()));
    assert!(
        schedule.current_barrier().is_some(),
        "Barrier は skip されず Entry::Barrier で停止する"
    );
    assert_eq!(schedule.remaining(), 1, "Barrier 後の after は未配信");
}

/// 変換は `absolute_start_time` をスケジュールの起点に用いる。
/// 相対 start_time は不変で運ばれる（起点＋相対＝絶対発火時刻）。
#[test]
fn to_talk_schedule_anchors_schedule_at_absolute_start_time() {
    let sheet = CueSheet::new(vec![text(ms(500), "hi", TimeSpan::ZERO)])
        .with_absolute_start_time(pt(100_000));

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    let ready = drain(&mut schedule, pt(50_000));
    assert!(ready.is_empty());
    let ready = drain(&mut schedule, pt(100_000));
    assert!(ready.is_empty());
    let ready = drain(&mut schedule, pt(100_499));
    assert!(ready.is_empty(), "1ms 手前では届かない");
    let ready = drain(&mut schedule, pt(100_500));
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].at, ms(500), "相対 start_time は不変で運ばれる");
    assert_eq!(ready[0].fire_time, pt(100_500));
}

/// 末尾に待ちを持つ台本を注入時刻で駆動したとき、全 cue の配送完了時点ではまだ未完了と判定され、
/// 占有の終わり到達で初めて完了と判定される（早期終了しない）。
#[test]
fn to_talk_schedule_not_completed_until_occupancy_horizon_reached() {
    let sheet = CueSheet::new(vec![
        text(TimeSpan::ZERO, "bye", ms(150)),
        wait(ms(150), ms(800)),
    ])
    .with_absolute_start_time(TimePoint::ZERO);

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    drain(&mut schedule, pt(150));
    assert_eq!(schedule.remaining(), 0, "全 entry を配り終えた");
    assert!(
        !schedule.is_completed_at(pt(150)),
        "配送完了時点では占有の終わり（950ms）未到達ゆえ未完了（早期終了しない）"
    );
    drain(&mut schedule, pt(949));
    assert!(
        !schedule.is_completed_at(pt(949)),
        "占有の終わりの 1ms 前は未完了"
    );
    drain(&mut schedule, pt(950));
    assert!(schedule.is_completed_at(pt(950)), "占有の終わり到達で完了");
}

/// 変換した予定の完了時刻は、台本のみから導ける `absolute_end_time()` に一致する。
#[test]
fn to_talk_schedule_completes_exactly_at_sheet_absolute_end_time() {
    let sheet = CueSheet::new(vec![
        text(TimeSpan::ZERO, "hi", ms(250)),
        wait(ms(250), ms(500)),
    ])
    .with_absolute_start_time(pt(10_000));

    let end = sheet.absolute_end_time().unwrap();
    assert_eq!(end, pt(10_750));

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    assert_eq!(schedule.occupancy_horizon(), end);
    drain(&mut schedule, pt(10_749));
    assert!(!schedule.is_completed_at(pt(10_749)));
    drain(&mut schedule, end);
    assert!(schedule.is_completed_at(end));
}

/// 空台本は時間を占有しない＝起点の時刻で即完了。
#[test]
fn to_talk_schedule_of_empty_sheet_is_immediately_completed() {
    let sheet = CueSheet::new(vec![]).with_absolute_start_time(TimePoint::ZERO);
    let mut schedule = to_talk_schedule(&sheet).unwrap();
    drain(&mut schedule, TimePoint::ZERO);
    assert!(
        schedule.is_completed_at(TimePoint::ZERO),
        "空台本は占有の長さ 0 で即完了"
    );
    assert_eq!(schedule.remaining(), 0);
}

// ============================================================================
// 台本の終わりと予定の占有の終わりの 1 本化（ちょうどの 100ns の値）
// ============================================================================

/// 150 ms に始まる 800 ms の合図 1 つの台本。
fn ms_sheet() -> CueSheet {
    CueSheet::new(vec![wait(ms(150), ms(800))])
}

/// 台本の終わりと予定へ焼き込む占有の終わりは、950 ms の整数の値そのもの。
#[test]
fn end_time_and_occupancy_horizon_are_exact() {
    let sheet = ms_sheet().with_absolute_start_time(TimePoint::ZERO);
    assert_eq!(sheet.relative_end(), ms(950));
    assert_eq!(sheet.absolute_end_time(), Some(pt(950)));
    assert_eq!(
        to_talk_schedule(&sheet).unwrap().occupancy_horizon(),
        pt(950)
    );
}

/// アンカー 1000ms の台本は、ちょうどの刻みで届き、ちょうど完了する。
#[test]
fn completes_exactly_at_end() {
    let sheet = ms_sheet().with_absolute_start_time(pt(1000));
    let mut schedule = to_talk_schedule(&sheet).unwrap();
    drain(&mut schedule, pt(1150));
    assert_eq!(schedule.remaining(), 0, "合図はその刻みで届く");
    drain(&mut schedule, pt(1949));
    assert!(
        !schedule.is_completed_at(pt(1949)),
        "終わりの 1 ms 前は未完了"
    );
    drain(&mut schedule, pt(1950));
    assert!(schedule.is_completed_at(pt(1950)), "終わりの刻みで完了");

    assert_eq!(sheet.absolute_fire_time(&sheet.cues()[0]), Some(pt(1150)));
    assert_eq!(sheet.absolute_end_time(), Some(pt(1950)));
}
