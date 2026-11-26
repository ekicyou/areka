//! 宛先へ知らせを届ける係

use super::*;

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
    let entity = world.spawn(TypewriterTalk::new(vec![], at_ms(0))).id();
    world.flush();
    world.entity_mut(entity).insert(TypewriterLayoutCache::new(
        layout,
        wait_then_fire_timeline(target),
    ));
    entity
}

fn new_update_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(crate::ecs::graphics::FrameTime(at_ms(10_000)));
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
