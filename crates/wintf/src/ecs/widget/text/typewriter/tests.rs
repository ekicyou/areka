use super::*;

/// `TypewriterLayoutCache` の手動 `unsafe impl Send/Sync`（typewriter.rs）の
/// 健全性を固定する特性化テスト。内包フィールド（IDWriteTextLayout は windows-rs が
/// Send+Sync 付与済み・TypewriterTimeline はプレーンデータ）により本構造体は本来
/// 自動で Send+Sync を導出できる＝手動 impl は冗長だが健全、という不変条件をコンパイル時に
/// 検証する。将来 !Send なフィールドが追加された場合は（手動 impl があっても）型として
/// 不健全な状態をここで検出できないため、この境界の更新時は手動 impl の妥当性を再点検すること。
#[test]
fn test_typewriter_layout_cache_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TypewriterLayoutCache>();
}

#[test]
fn test_typewriter_default() {
    let tw = Typewriter::default();
    assert_eq!(tw.font_family, "メイリオ");
    assert_eq!(tw.font_size, 16.0);
    assert!((tw.default_char_wait - 0.05).abs() < f64::EPSILON);
}

#[test]
fn test_typewriter_state_default() {
    let state = TypewriterState::default();
    assert_eq!(state, TypewriterState::Playing);
}

#[test]
fn test_typewriter_state_transitions() {
    assert_eq!(TypewriterState::Playing, TypewriterState::Playing);
    assert_eq!(TypewriterState::Paused, TypewriterState::Paused);
    assert_eq!(TypewriterState::Completed, TypewriterState::Completed);
    assert_ne!(TypewriterState::Playing, TypewriterState::Paused);
}

// ============================================================
// TypewriterTalk — デバイス非依存な再生状態マシンの特性化
// （pause/resume/skip/update のロジックは DirectWrite 非依存。
//  timeline は plain data 構造体として手組みできる）
// ============================================================

/// グリフ N 個を等間隔 `step` 秒で表示するだけの timeline を組み立てる。
/// total_cluster_count = glyph_count。convert_to_timeline を経由せず
/// 直接構築する（DirectWrite 非依存）。
fn make_glyph_timeline(glyph_count: u32, step: f64) -> TypewriterTimeline {
    let mut items = Vec::new();
    let mut t = 0.0;
    for i in 0..glyph_count {
        t += step;
        items.push(TimelineItem::Glyph {
            cluster_index: i,
            show_at: t,
        });
    }
    TypewriterTimeline {
        full_text: "x".repeat(glyph_count as usize),
        items,
        total_duration: t,
        total_cluster_count: glyph_count,
    }
}

#[test]
fn test_typewriter_talk_new_initial_state() {
    let talk = TypewriterTalk::new(vec![TypewriterToken::Text("ab".into())], 10.0);
    assert_eq!(talk.state(), TypewriterState::Playing);
    assert_eq!(talk.start_time(), 10.0);
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.progress(), 0.0);
    assert!(!talk.is_completed());
    assert_eq!(talk.tokens().len(), 1);
}

#[test]
fn test_typewriter_talk_pause_records_elapsed_and_changes_state() {
    let mut talk = TypewriterTalk::new(vec![], 100.0);
    talk.pause(105.0);
    assert_eq!(talk.state(), TypewriterState::Paused);
    // resume で paused_elapsed(=5.0) を使って start_time を再計算するため、
    // resume(200.0) 後の start_time は 200 - 5 = 195 になる。
    talk.resume(200.0);
    assert_eq!(talk.state(), TypewriterState::Playing);
    assert_eq!(talk.start_time(), 195.0);
}

#[test]
fn test_typewriter_talk_pause_is_noop_when_not_playing() {
    let mut talk = TypewriterTalk::new(vec![], 0.0);
    talk.pause(5.0);
    assert_eq!(talk.state(), TypewriterState::Paused);
    let start_after_first_pause = talk.start_time();
    // 既に Paused のとき pause を再呼び出ししても状態・start_time は変化しない。
    talk.pause(50.0);
    assert_eq!(talk.state(), TypewriterState::Paused);
    assert_eq!(talk.start_time(), start_after_first_pause);
}

#[test]
fn test_typewriter_talk_resume_is_noop_when_not_paused() {
    let mut talk = TypewriterTalk::new(vec![], 100.0);
    // Playing 状態での resume は何もしない（start_time 不変）。
    talk.resume(999.0);
    assert_eq!(talk.state(), TypewriterState::Playing);
    assert_eq!(talk.start_time(), 100.0);
}

#[test]
fn test_typewriter_talk_skip_forces_complete() {
    let mut talk = TypewriterTalk::new(vec![], 0.0);
    talk.skip(42);
    assert_eq!(talk.state(), TypewriterState::Completed);
    assert_eq!(talk.visible_cluster_count(), 42);
    assert_eq!(talk.progress(), 1.0);
    assert!(talk.is_completed());
}

#[test]
fn test_typewriter_talk_update_returns_empty_when_not_playing() {
    let timeline = make_glyph_timeline(3, 1.0);
    let mut talk = TypewriterTalk::new(vec![], 0.0);
    talk.pause(0.0);
    // Paused 中は update が時刻に関わらず何も進めず空イベントを返す。
    let events = talk.update(1000.0, &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.state(), TypewriterState::Paused);
}

#[test]
fn test_typewriter_talk_update_reveals_glyphs_up_to_elapsed() {
    // step=1.0 で 3 グリフ（show_at = 1,2,3）。start_time=0。
    let timeline = make_glyph_timeline(3, 1.0);
    let mut talk = TypewriterTalk::new(vec![], 0.0);

    // elapsed=0: まだどのグリフも show_at(>=1) に達していない。
    talk.update(0.0, &timeline);
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.state(), TypewriterState::Playing);

    // elapsed=1.5: show_at<=1.5 のグリフ1個のみ表示。
    talk.update(1.5, &timeline);
    assert_eq!(talk.visible_cluster_count(), 1);
    assert!((talk.progress() - (1.0 / 3.0)).abs() < f32::EPSILON);
    assert_eq!(talk.state(), TypewriterState::Playing);
}

#[test]
fn test_typewriter_talk_update_completes_when_all_glyphs_visible() {
    let timeline = make_glyph_timeline(3, 1.0);
    let mut talk = TypewriterTalk::new(vec![], 0.0);
    // elapsed=10 で全グリフ（show_at<=3）が表示され Completed へ遷移。
    let events = talk.update(10.0, &timeline);
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
    let timeline = TypewriterTimeline::empty();
    let mut talk = TypewriterTalk::new(vec![], 0.0);
    talk.update(0.0, &timeline);
    assert_eq!(talk.progress(), 1.0);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

#[test]
fn test_typewriter_talk_update_wait_gates_following_glyph() {
    // Wait(duration=5, start_at=0) の後に Glyph(show_at=5)。
    // next_item_index は Wait を「elapsed >= start_at+duration」まで通過しない。
    let timeline = TypewriterTimeline {
        full_text: "a".into(),
        items: vec![
            TimelineItem::Wait {
                duration: 5.0,
                start_at: 0.0,
            },
            TimelineItem::Glyph {
                cluster_index: 0,
                show_at: 5.0,
            },
        ],
        total_duration: 5.0,
        total_cluster_count: 1,
    };
    let mut talk = TypewriterTalk::new(vec![], 0.0);

    // elapsed=3: Wait 未満で break。グリフ未到達。
    talk.update(3.0, &timeline);
    assert_eq!(talk.visible_cluster_count(), 0);
    assert_eq!(talk.state(), TypewriterState::Playing);

    // elapsed=6: Wait 通過 → グリフ表示 → 全数表示で Completed。
    talk.update(6.0, &timeline);
    assert_eq!(talk.visible_cluster_count(), 1);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

#[test]
fn test_typewriter_talk_update_fires_event_at_threshold() {
    let target = bevy_ecs::entity::Entity::from_raw_u32(1).unwrap();
    // FireEvent(fire_at=2) を含み、その後 Glyph(show_at=3) で完了する timeline。
    let timeline = TypewriterTimeline {
        full_text: "a".into(),
        items: vec![
            TimelineItem::FireEvent {
                target,
                event: TypewriterEventKind::Paused,
                fire_at: 2.0,
            },
            TimelineItem::Glyph {
                cluster_index: 0,
                show_at: 3.0,
            },
        ],
        total_duration: 3.0,
        total_cluster_count: 1,
    };
    let mut talk = TypewriterTalk::new(vec![], 0.0);

    // elapsed=1: fire_at(2) 未満 → イベント未発火・グリフ未到達。
    let events = talk.update(1.0, &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 0);

    // elapsed=2.5: FireEvent 通過（発火）するがグリフ(show_at=3)は未達で break。
    let events = talk.update(2.5, &timeline);
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
            fire_at: 1.0,
        }],
        total_duration: 1.0,
        total_cluster_count: 0,
    };
    let mut talk = TypewriterTalk::new(vec![], 0.0);

    // 1回目: イベント発火（next_item_index が進む）。
    // total_cluster_count=0 のため同時に Completed へ遷移する。
    let first = talk.update(5.0, &timeline);
    assert_eq!(first.len(), 1);

    // 2回目: 既に Completed なので update は早期 return し、再発火しない。
    let second = talk.update(6.0, &timeline);
    assert!(second.is_empty());
}
