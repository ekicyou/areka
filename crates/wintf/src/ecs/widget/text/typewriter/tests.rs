use super::*;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::Arrangement;
use crate::ecs::layout::metrics::Size;
use crate::ecs::widget::text::typewriter_layout::init_typewriter_layout;
use bevy_ecs::prelude::*;

/// `TypewriterLayoutCache` が自動の導出で `Send + Sync` になることを固定する
/// （`IDWriteTextLayout` は windows-rs が `Send`/`Sync` を付け、時間表はただの値）。
/// 後で `Send` でない欄を足すと、ここが組み立たなくなる。
#[test]
fn test_typewriter_layout_cache_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TypewriterLayoutCache>();
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
    let timeline = TypewriterTimeline {
        full_text: String::new(),
        items: Vec::new(),
        total_cluster_count: 0,
    };
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

// ============================================================
// トークの入れ替えと外し（3 番・実 GPU）
// ============================================================

/// 控えを作る係を 1 回回す世界。控えは `GraphicsCore` の DirectWrite で作る。
fn new_layout_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let mut schedule = Schedule::default();
    schedule.add_systems(init_typewriter_layout);
    (world, schedule)
}

fn text_talk(text: &str) -> TypewriterTalk {
    TypewriterTalk::new(vec![TypewriterToken::Text(text.into())], 0.0)
}

fn cached_text(world: &World, entity: Entity) -> Option<String> {
    world
        .get::<TypewriterLayoutCache>(entity)
        .map(|c| c.timeline().full_text.clone())
}

#[test]
fn replacing_talk_rebuilds_cache_with_new_text_and_removing_clears_list() {
    let (mut world, mut schedule) = new_layout_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            Arrangement {
                size: Size {
                    width: 200.0,
                    height: 100.0,
                },
                ..Default::default()
            },
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert_eq!(cached_text(&world, entity).as_deref(), Some("あいう"));

    // 入れ替えると控えが外れ、作り直した控えは新しい文になる。
    world.entity_mut(entity).insert(text_talk("かきくけこ"));
    world.flush();
    assert!(world.get::<TypewriterLayoutCache>(entity).is_none());
    schedule.run(&mut world);
    assert_eq!(cached_text(&world, entity).as_deref(), Some("かきくけこ"));

    // 外すと命令の列が空になり、控えも外れる。
    let list = {
        let core = world.resource::<GraphicsCore>();
        let dc = core.device_context().expect("device context");
        crate::ecs::widget::record::record_command_list(dc, |_| Ok(())).expect("record")
    };
    world
        .entity_mut(entity)
        .insert(GraphicsCommandList::new(list));
    world.entity_mut(entity).remove::<TypewriterTalk>();
    world.flush();
    assert!(
        world
            .get::<GraphicsCommandList>(entity)
            .expect("list")
            .command_list()
            .is_none()
    );
    assert!(world.get::<TypewriterLayoutCache>(entity).is_none());
}

#[test]
fn despawning_typewriter_with_talk_does_not_panic() {
    let (mut world, mut schedule) = new_layout_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            Arrangement {
                size: Size {
                    width: 200.0,
                    height: 100.0,
                },
                ..Default::default()
            },
            text_talk("あ"),
        ))
        .id();
    schedule.run(&mut world);
    world.despawn(entity);
    world.flush();
    assert!(world.get_entity(entity).is_err());
}

// ============================================================
// 最後の待ちと知らせ（4 番）・消えた宛先（5 番）— CPU だけ
// ============================================================

/// 「おしまい」の 4 文字（0.1 秒おき）→ 1 秒の待ち → 宛先への完了の知らせ。
fn wait_then_fire_timeline(target: Entity) -> TypewriterTimeline {
    let mut timeline = make_glyph_timeline(4, 0.1);
    timeline.full_text = "おしまい".into();
    timeline.items.push(TimelineItem::Wait {
        duration: 1.0,
        start_at: 0.4,
    });
    timeline.items.push(TimelineItem::FireEvent {
        target,
        event: TypewriterEventKind::Complete,
        fire_at: 1.4,
    });
    timeline
}

#[test]
fn trailing_wait_and_event_run_before_completion() {
    let target = Entity::from_raw_u32(3).unwrap();
    let timeline = wait_then_fire_timeline(target);
    let mut talk = TypewriterTalk::new(vec![], 0.0);

    // 全部の文字が出た時刻でも、後ろの待ちが残るので再生中のまま。
    let events = talk.update(0.4, &timeline);
    assert!(events.is_empty());
    assert_eq!(talk.visible_cluster_count(), 4);
    assert_eq!(talk.progress(), 1.0);
    assert_eq!(talk.state(), TypewriterState::Playing);

    // 1 秒後に知らせが返り、完了する。
    let events = talk.update(1.4, &timeline);
    assert_eq!(events, vec![(target, TypewriterEventKind::Complete)]);
    assert_eq!(talk.state(), TypewriterState::Completed);
}

/// 知らせの係を回す世界。控えの配置は DirectWrite の工場だけで作る（GPU は使わない）。
fn spawn_firing_typewriter(world: &mut World, target: Entity) -> Entity {
    use crate::com::dwrite::{DWriteFactoryExt, dwrite_create_factory};
    use windows::Win32::Graphics::DirectWrite::*;
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("dwrite factory");
    let format = factory
        .create_text_format(
            &windows::core::HSTRING::from("メイリオ"),
            None::<&IDWriteFontCollection>,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            16.0,
            &windows::core::HSTRING::from("ja-JP"),
        )
        .expect("text format");
    let layout = factory
        .create_text_layout(
            &windows::core::HSTRING::from("おしまい"),
            &format,
            200.0,
            100.0,
        )
        .expect("text layout");
    // トークを入れたときのフックが控えを外すので、控えはトークの後に入れる。
    let entity = world.spawn(TypewriterTalk::new(vec![], 0.0)).id();
    world.flush();
    world.entity_mut(entity).insert(TypewriterLayoutCache::new(
        layout,
        wait_then_fire_timeline(target),
    ));
    entity
}

fn new_update_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(crate::ecs::graphics::FrameTime(10.0));
    let mut schedule = Schedule::default();
    schedule.add_systems(crate::ecs::widget::text::typewriter_draw::update_typewriters);
    (world, schedule)
}

#[test]
fn event_reaches_live_target() {
    use crate::ecs::widget::text::typewriter_ir::TypewriterEvent;
    let (mut world, mut schedule) = new_update_world();
    let target = world.spawn_empty().id();
    let entity = spawn_firing_typewriter(&mut world, target);
    schedule.run(&mut world);
    assert_eq!(
        world.get::<TypewriterEvent>(target),
        Some(&TypewriterEvent::Complete)
    );
    assert!(world.get::<TypewriterTalk>(entity).unwrap().is_completed());
}

#[test]
fn event_to_despawned_target_is_dropped_without_panic() {
    let (mut world, mut schedule) = new_update_world();
    let target = world.spawn_empty().id();
    let entity = spawn_firing_typewriter(&mut world, target);
    world.despawn(target);
    schedule.run(&mut world);
    assert!(world.get_entity(target).is_err());
    assert!(world.get::<TypewriterTalk>(entity).unwrap().is_completed());
}

// ============================================================
// 時間表と隠す範囲をクラスタと UTF-16 で数える（6 番）— CPU だけ
// ============================================================

use crate::ecs::widget::text::typewriter_draw::hidden_text_range;
use crate::ecs::widget::text::typewriter_layout::timeline_items;

/// 項目を (種類, クラスタ番号) に縮めて並びを比べやすくする。
fn item_kinds(items: &[TimelineItem]) -> Vec<(&'static str, Option<u32>)> {
    items
        .iter()
        .map(|item| match item {
            TimelineItem::Glyph { cluster_index, .. } => ("glyph", Some(*cluster_index)),
            TimelineItem::Wait { .. } => ("wait", None),
            TimelineItem::FireEvent { .. } => ("event", None),
        })
        .collect()
}

#[test]
fn timeline_counts_surrogate_pair_as_one_cluster() {
    // 𠮷 は UTF-16 で 2 つ・クラスタは 1 つ。
    let tokens = vec![TypewriterToken::Text("𠮷野家です".into())];
    let items = timeline_items(&tokens, &[2, 1, 1, 1, 1], 0.1);
    assert_eq!(
        item_kinds(&items),
        (0..5).map(|i| ("glyph", Some(i))).collect::<Vec<_>>()
    );
}

#[test]
fn timeline_puts_glyph_after_wait_when_cluster_spans_two_chars() {
    // ❤️ は 2 文字（U+2764 U+FE0F）で 1 クラスタ。「!」は待ちの後ろに来る。
    let tokens = vec![
        TypewriterToken::Text("\u{2764}\u{FE0F}ok".into()),
        TypewriterToken::Wait(1.0),
        TypewriterToken::Text("!".into()),
    ];
    let items = timeline_items(&tokens, &[2, 1, 1, 1], 0.1);
    assert_eq!(
        item_kinds(&items),
        vec![
            ("glyph", Some(0)),
            ("glyph", Some(1)),
            ("glyph", Some(2)),
            ("wait", None),
            ("glyph", Some(3)),
        ]
    );
    let TimelineItem::Glyph { show_at, .. } = items[4] else {
        panic!("last item must be a glyph");
    };
    assert!((show_at - 1.4).abs() < 1e-9);
}

#[test]
fn hidden_range_is_counted_in_utf16() {
    // 𠮷野家です は UTF-16 で 6。𠮷 を 1 つ見せたら (2, 4) を隠す。
    let lengths = [2, 1, 1, 1, 1];
    assert_eq!(hidden_text_range(&lengths, 1), Some((2, 4)));
    assert_eq!(hidden_text_range(&lengths, 0), Some((0, 6)));
    assert_eq!(hidden_text_range(&lengths, 5), None);
    assert_eq!(hidden_text_range(&lengths, 9), None);
}

// ============================================================
// 背景の描き直しと空にする所（8 番）・止まった表示（12 番）— 実 GPU
// ============================================================

use crate::ecs::graphics::FrameTime;
use crate::ecs::widget::brushes::Brushes;
use crate::ecs::widget::text::typewriter_draw::{
    draw_typewriter_backgrounds, draw_typewriters, update_typewriters,
};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::ID2D1CommandList;

/// 控えを作る → 進める → 文を描く → 背景を描く、を 1 回で回す世界（本番の Update → Draw の順）。
fn new_draw_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    world.insert_resource(FrameTime(100.0));
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            init_typewriter_layout,
            update_typewriters,
            draw_typewriters,
            draw_typewriter_backgrounds,
        )
            .chain(),
    );
    (world, schedule)
}

fn sized(width: f32, height: f32) -> Arrangement {
    Arrangement {
        size: Size { width, height },
        ..Default::default()
    }
}

fn list_of(world: &World, entity: Entity) -> Option<Option<ID2D1CommandList>> {
    world
        .get::<GraphicsCommandList>(entity)
        .map(|l| l.command_list().cloned())
}

const BG: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 0.2,
    g: 0.4,
    b: 0.6,
    a: 1.0,
};

#[test]
fn resizing_empty_talk_background_replaces_command_list() {
    let (mut world, mut schedule) = new_draw_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(100.0, 50.0),
            Brushes::with_background(BG),
        ))
        .id();
    schedule.run(&mut world);
    let first = list_of(&world, entity).flatten().expect("background drawn");

    world.entity_mut(entity).insert(sized(200.0, 80.0));
    schedule.run(&mut world);
    let second = list_of(&world, entity)
        .flatten()
        .expect("background redrawn");
    assert_ne!(first, second, "大きさを変えたら命令の列が入れ替わる");
}

#[test]
fn replacing_text_talk_with_empty_talk_without_background_clears_list() {
    let (mut world, mut schedule) = new_draw_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(200.0, 100.0),
            Brushes::default(),
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert!(list_of(&world, entity).flatten().is_some(), "文が描かれる");

    world
        .entity_mut(entity)
        .insert(TypewriterTalk::new(vec![], 0.0));
    schedule.run(&mut world);
    assert_eq!(
        list_of(&world, entity),
        Some(None),
        "描けない空のトークでは前の文の絵を残さない"
    );
}

#[test]
fn stopped_display_is_not_redrawn_on_second_update() {
    let (mut world, mut schedule) = new_draw_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(200.0, 100.0),
            Brushes::default(),
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert!(world.get::<TypewriterTalk>(entity).unwrap().is_completed());
    let first = list_of(&world, entity).flatten().expect("text drawn");
    let first_tick = world
        .entity(entity)
        .get_ref::<TypewriterTalk>()
        .unwrap()
        .last_changed();

    world.insert_resource(FrameTime(101.0));
    schedule.run(&mut world);
    let second_tick = world
        .entity(entity)
        .get_ref::<TypewriterTalk>()
        .unwrap()
        .last_changed();
    assert_eq!(first_tick, second_tick, "止まった表示に変化の印を立てない");
    assert_eq!(
        list_of(&world, entity).flatten(),
        Some(first),
        "止まった表示の命令の列を入れ替えない"
    );
}

#[test]
fn leftover_transparent_effect_is_cleared_on_next_draw() {
    use crate::com::d2d::D2D1DeviceContextExt;
    use windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_RANGE;
    let (mut world, mut schedule) = new_draw_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(200.0, 100.0),
            Brushes::default(),
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert!(world.get::<TypewriterTalk>(entity).unwrap().is_completed());

    // 描いた後の戻しに失敗した形: 控えの配置に透明の効果が残っている
    let layout = world
        .get::<TypewriterLayoutCache>(entity)
        .unwrap()
        .text_layout()
        .clone();
    let brush = {
        let core = world.resource::<GraphicsCore>();
        let dc = core.device_context().expect("device context");
        dc.create_solid_color_brush(&crate::ecs::widget::record::TRANSPARENT, None)
            .expect("brush")
    };
    unsafe {
        layout
            .SetDrawingEffect(
                &brush,
                DWRITE_TEXT_RANGE {
                    startPosition: 1,
                    length: 2,
                },
            )
            .expect("set effect");
    }

    // 隠す範囲の無い次の描画で、残った効果が消える
    world.entity_mut(entity).insert(sized(200.0, 100.0));
    schedule.run(&mut world);
    let mut effect = None;
    unsafe { layout.GetDrawingEffect(1, &mut effect, None) }.expect("get effect");
    assert!(effect.is_none(), "前の描画の透明の効果を残さない");
}
