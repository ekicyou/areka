//! 控えの作り直しと、トーク・Typewriter の入れ替えと外し（実 GPU）

use super::*;
use crate::ecs::widget::text::typewriter_layout::invalidate_typewriter_layout_on_arrangement_change;

/// 本番の Draw と同じ順（控えを外す → 控えを作る）で回す世界。
fn new_cache_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            invalidate_typewriter_layout_on_arrangement_change,
            init_typewriter_layout,
        )
            .chain(),
    );
    (world, schedule)
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

/// `Typewriter` を外すと、命令の列が空になり（絵を消す）、控えも外れる。
#[test]
fn removing_typewriter_clears_list_and_cache() {
    let (mut world, mut schedule) = new_layout_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(200.0, 100.0),
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert!(world.get::<TypewriterLayoutCache>(entity).is_some());
    let list = {
        let core = world.resource::<GraphicsCore>();
        let dc = core.device_context().expect("device context");
        crate::ecs::widget::record::record_command_list(dc, |_| Ok(())).expect("record")
    };
    world
        .entity_mut(entity)
        .insert(GraphicsCommandList::new(list));

    world.entity_mut(entity).remove::<Typewriter>();
    world.flush();
    assert_eq!(list_of(&world, entity), Some(None), "絵を残さない");
    assert!(world.get::<TypewriterLayoutCache>(entity).is_none());
}

/// 控えが作れない部品は、次の変化まで作り直さない（毎回の warn! を止める）。変われば作り直す。
#[test]
fn failed_cache_build_is_not_retried_until_typewriter_changes() {
    let (mut world, mut schedule) = new_cache_world();
    let entity = world
        .spawn((
            Typewriter {
                font_size: 0.0,
                ..Default::default()
            },
            sized(200.0, 100.0),
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert!(
        world.get::<TypewriterLayoutCache>(entity).is_none(),
        "大きさ 0 では作れない"
    );

    // 原因を直しても変化の印を立てなければ、作り直さない
    world
        .get_mut::<Typewriter>(entity)
        .unwrap()
        .bypass_change_detection()
        .font_size = 16.0;
    schedule.run(&mut world);
    assert!(
        world.get::<TypewriterLayoutCache>(entity).is_none(),
        "やり直さない"
    );

    world.get_mut::<Typewriter>(entity).unwrap().set_changed();
    schedule.run(&mut world);
    assert!(
        world.get::<TypewriterLayoutCache>(entity).is_some(),
        "変われば作る"
    );
}

/// `Typewriter`（文字の大きさなど）を変えると控えが作り直される。
#[test]
fn changing_typewriter_rebuilds_cache() {
    let (mut world, mut schedule) = new_cache_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(200.0, 100.0),
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert_eq!(font_size_of(&world, entity), 16.0, "既定の大きさの控え");

    world.get_mut::<Typewriter>(entity).unwrap().font_size = 24.0;
    schedule.run(&mut world);
    assert_eq!(
        font_size_of(&world, entity),
        24.0,
        "新しい大きさの控えになる"
    );
}

/// 控えの配置の文字の大きさ。
fn font_size_of(world: &World, entity: Entity) -> f32 {
    let mut size = 0.0;
    unsafe {
        world
            .get::<TypewriterLayoutCache>(entity)
            .expect("cache")
            .text_layout()
            .GetFontSize(0, &mut size, None)
    }
    .expect("font size");
    size
}
