//! タイプライターの再生の状態（止める・戻す・全部を現す・進める）

use super::*;
use dola::TimePoint;

#[test]
fn test_typewriter_talk_pause_records_elapsed_and_changes_state() {
    let mut talk = TypewriterTalk::new(vec![], at_ms(100_000));
    talk.pause(at_ms(105_000));
    assert_eq!(talk.state(), TypewriterState::Paused);
    // resume で paused_elapsed(=5 秒) を使って start_time を再計算するため、
    // resume(200 秒) 後の start_time は 200 - 5 = 195 秒になる。
    talk.resume(at_ms(200_000));
    assert_eq!(talk.state(), TypewriterState::Playing);
    assert_eq!(talk.start_time(), at_ms(195_000));
}

#[test]
fn test_typewriter_talk_pause_is_noop_when_not_playing() {
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    talk.pause(at_ms(5_000));
    assert_eq!(talk.state(), TypewriterState::Paused);
    let start_after_first_pause = talk.start_time();
    // 既に Paused のとき pause を再呼び出ししても状態・start_time は変化しない。
    talk.pause(at_ms(50_000));
    assert_eq!(talk.state(), TypewriterState::Paused);
    assert_eq!(talk.start_time(), start_after_first_pause);
}

#[test]
fn test_typewriter_talk_resume_is_noop_when_not_paused() {
    let mut talk = TypewriterTalk::new(vec![], at_ms(100_000));
    // Playing 状態での resume は何もしない（start_time 不変）。
    talk.resume(at_ms(999_000));
    assert_eq!(talk.state(), TypewriterState::Playing);
    assert_eq!(talk.start_time(), at_ms(100_000));
}

/// 開始より前に止めて戻しても、開始までの残りの長さを保つ。
#[test]
fn pause_before_start_keeps_remaining_lead_on_resume() {
    let mut talk = TypewriterTalk::new(vec![], at_ms(1_000));
    talk.pause(at_ms(400));
    talk.resume(at_ms(2_000));
    assert_eq!(talk.start_time(), at_ms(2_600));
}

#[test]
fn test_typewriter_talk_skip_forces_complete() {
    // skip した時点で完了・全文表示・進行度 1.0（次の update を待たない）。
    let timeline = make_glyph_timeline(42, ms(1000));
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    let events = talk.skip(&timeline);
    assert!(events.is_empty());
    assert_eq!(talk.state(), TypewriterState::Completed);
    assert_eq!(talk.visible_cluster_count(), 42);
    assert_eq!(talk.progress(), 1.0);
    assert!(talk.is_completed());
}

/// skip は残りの知らせ（末尾の Complete）をその場で返し、その場で完了する。
#[test]
fn test_typewriter_talk_skip_returns_remaining_events_immediately() {
    let target = bevy_ecs::entity::Entity::from_raw_u32(3).unwrap();
    let timeline = TypewriterTimeline {
        full_text: "a".into(),
        items: vec![
            TimelineItem::Glyph {
                cluster_index: 0,
                show_at: ms(10_000),
            },
            TimelineItem::FireEvent {
                target,
                event: TypewriterEventKind::Complete,
                fire_at: ms(10_000),
            },
        ],
        total_cluster_count: 1,
    };
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    let events = talk.skip(&timeline);
    assert_eq!(events, vec![(target, TypewriterEventKind::Complete)]);
    assert_eq!(talk.visible_cluster_count(), 1);
    assert_eq!(talk.state(), TypewriterState::Completed);
    // 完了後の skip・update は何も出さない
    assert!(talk.skip(&timeline).is_empty());
    assert!(talk.update(at_ms(100_000), &timeline).is_empty());
}

/// 一時停止中の skip もその場で完了する。
#[test]
fn test_typewriter_talk_skip_while_paused_completes() {
    let timeline = make_glyph_timeline(3, ms(1000));
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    talk.update(at_ms(1_500), &timeline);
    talk.pause(at_ms(1_500));
    talk.skip(&timeline);
    assert_eq!(talk.visible_cluster_count(), 3);
    assert_eq!(talk.progress(), 1.0);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

/// 開始より前でも、skip は行程の終わりまで進めて完了する。
#[test]
fn skip_before_start_runs_whole_timeline() {
    let target = Entity::from_raw_u32(5).unwrap();
    let timeline = wait_then_fire_timeline(target);
    let mut talk = TypewriterTalk::new(vec![], at_ms(60_000));
    let events = talk.skip(&timeline);
    assert_eq!(events, vec![(target, TypewriterEventKind::Complete)]);
    assert_eq!(talk.visible_cluster_count(), 4);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

#[test]
fn test_typewriter_talk_update_returns_empty_when_not_playing() {
    let timeline = make_glyph_timeline(3, ms(1000));
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    talk.pause(at_ms(0));
    // Paused 中は update が時刻に関わらず何も進めず空イベントを返す。
    let events = talk.update(at_ms(1_000_000), &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.state(), TypewriterState::Paused);
}

#[test]
fn test_typewriter_talk_update_reveals_glyphs_up_to_elapsed() {
    // step=1 秒で 3 グリフ（show_at = 1,2,3 秒）。start_time=0。
    let timeline = make_glyph_timeline(3, ms(1000));
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));

    // elapsed=0: まだどのグリフも show_at(>=1 秒) に達していない。
    talk.update(at_ms(0), &timeline);
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.state(), TypewriterState::Playing);

    // elapsed=1.5 秒: show_at<=1.5 秒のグリフ1個のみ表示。
    talk.update(at_ms(1_500), &timeline);
    assert_eq!(talk.visible_cluster_count(), 1);
    assert!((talk.progress() - (1.0 / 3.0)).abs() < f32::EPSILON);
    assert_eq!(talk.state(), TypewriterState::Playing);
}

/// グリフはちょうど show_at の刻みで現れ、1 刻み前には現れない。
#[test]
fn glyph_appears_exactly_at_show_at() {
    let timeline = make_glyph_timeline(2, ms(1000));
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    talk.update(TimePoint::from_ticks(at_ms(1000).as_ticks() - 1), &timeline);
    assert_eq!(talk.visible_cluster_count(), 0);
    talk.update(at_ms(1000), &timeline);
    assert_eq!(talk.visible_cluster_count(), 1);
}

/// 開始より前の刻みでは、頭の知らせも含めて何も現さず、再生中のまま。
#[test]
fn update_before_start_reveals_nothing() {
    let target = Entity::from_raw_u32(2).unwrap();
    let mut timeline = make_glyph_timeline(1, TimeSpan::ZERO);
    timeline.items.insert(
        0,
        TimelineItem::FireEvent {
            target,
            event: TypewriterEventKind::Paused,
            fire_at: TimeSpan::ZERO,
        },
    );
    let mut talk = TypewriterTalk::new(vec![], at_ms(1_000));

    let events = talk.update(at_ms(999), &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.progress(), 0.0);
    assert_eq!(talk.state(), TypewriterState::Playing);

    let events = talk.update(at_ms(1_000), &timeline);
    assert_eq!(events, vec![(target, TypewriterEventKind::Paused)]);
    assert_eq!(talk.visible_cluster_count(), 1);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

#[test]
fn test_typewriter_talk_update_completes_when_all_glyphs_visible() {
    let timeline = make_glyph_timeline(3, ms(1000));
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    // elapsed=10 秒で全グリフ（show_at<=3 秒）が表示され Completed へ遷移。
    let events = talk.update(at_ms(10_000), &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 3);
    assert_eq!(talk.progress(), 1.0);
    assert_eq!(talk.state(), TypewriterState::Completed);
    assert!(talk.is_completed());
}

#[test]
fn test_typewriter_talk_update_zero_clusters_completes_immediately() {
    // グリフを持たない timeline（total_cluster_count=0）は
    // progress=1.0・即 Completed（0 >= 0）になる退化ケース。
    let timeline = TypewriterTimeline {
        full_text: String::new(),
        items: Vec::new(),
        total_cluster_count: 0,
    };
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));
    talk.update(at_ms(0), &timeline);
    assert_eq!(talk.progress(), 1.0);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

#[test]
fn test_typewriter_talk_update_wait_gates_following_glyph() {
    // Wait(duration=5 秒, start_at=0) の後に Glyph(show_at=5 秒)。
    // next_item_index は Wait を「elapsed >= start_at+duration」まで通過しない。
    let timeline = TypewriterTimeline {
        full_text: "a".into(),
        items: vec![
            TimelineItem::Wait {
                duration: ms(5_000),
                start_at: TimeSpan::ZERO,
            },
            TimelineItem::Glyph {
                cluster_index: 0,
                show_at: ms(5_000),
            },
        ],
        total_cluster_count: 1,
    };
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));

    // elapsed=3 秒: Wait 未満で break。グリフ未到達。
    talk.update(at_ms(3_000), &timeline);
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.state(), TypewriterState::Playing);

    // elapsed=6 秒: Wait 通過 → グリフ表示 → 全数表示で Completed。
    talk.update(at_ms(6_000), &timeline);
    assert_eq!(talk.visible_cluster_count(), 1);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

#[test]
fn test_typewriter_talk_update_fires_event_at_threshold() {
    let target = bevy_ecs::entity::Entity::from_raw_u32(1).unwrap();
    // FireEvent(fire_at=2 秒) を含み、その後 Glyph(show_at=3 秒) で完了する timeline。
    let timeline = TypewriterTimeline {
        full_text: "a".into(),
        items: vec![
            TimelineItem::FireEvent {
                target,
                event: TypewriterEventKind::Paused,
                fire_at: ms(2_000),
            },
            TimelineItem::Glyph {
                cluster_index: 0,
                show_at: ms(3_000),
            },
        ],
        total_cluster_count: 1,
    };
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));

    // elapsed=1 秒: fire_at(2 秒) 未満 → イベント未発火・グリフ未到達。
    let events = talk.update(at_ms(1_000), &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 0);

    // elapsed=2.5 秒: FireEvent 通過（発火）するがグリフ(show_at=3 秒)は未達で break。
    let events = talk.update(at_ms(2_500), &timeline);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, target);
    assert_eq!(events[0].1, TypewriterEventKind::Paused);
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.state(), TypewriterState::Playing);
}

#[test]
fn test_typewriter_talk_update_does_not_refire_event_on_second_call() {
    let target = bevy_ecs::entity::Entity::from_raw_u32(7).unwrap();
    let timeline = TypewriterTimeline {
        full_text: String::new(),
        items: vec![TimelineItem::FireEvent {
            target,
            event: TypewriterEventKind::Complete,
            fire_at: ms(1_000),
        }],
        total_cluster_count: 0,
    };
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));

    // 1回目: イベント発火（next_item_index が進む）。
    // total_cluster_count=0 のため同時に Completed へ遷移する。
    let first = talk.update(at_ms(5_000), &timeline);
    assert_eq!(first.len(), 1);

    // 2回目: 既に Completed なので update は早期 return し、再発火しない。
    let second = talk.update(at_ms(6_000), &timeline);
    assert!(second.is_empty());
}

#[test]
fn trailing_wait_and_event_run_before_completion() {
    let target = Entity::from_raw_u32(3).unwrap();
    let timeline = wait_then_fire_timeline(target);
    let mut talk = TypewriterTalk::new(vec![], at_ms(0));

    // 全部の文字が出た時刻でも、後ろの待ちが残るので再生中のまま。
    let events = talk.update(at_ms(400), &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 4);
    assert_eq!(talk.progress(), 1.0);
    assert_eq!(talk.state(), TypewriterState::Playing);

    // 1 秒後に知らせが返り、完了する。
    let events = talk.update(at_ms(1_400), &timeline);
    assert_eq!(events, vec![(target, TypewriterEventKind::Complete)]);
    assert_eq!(talk.state(), TypewriterState::Completed);
}
