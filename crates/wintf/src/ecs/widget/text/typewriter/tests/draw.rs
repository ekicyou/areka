//! 文と背景の描き直し・空にする所・止まった表示（実 GPU）

use super::*;
use crate::ecs::graphics::FrameTime;
use crate::ecs::widget::brushes::Brushes;
use crate::ecs::widget::text::typewriter_draw::{
    draw_typewriter_backgrounds, draw_typewriters, update_typewriters,
};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

/// 控えを作る → 進める → 文を描く → 背景を描く、を 1 回で回す世界（本番の Update → Draw の順）。
fn new_draw_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    world.insert_resource(FrameTime(at_ms(100_000)));
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
        .insert(TypewriterTalk::new(vec![], at_ms(0)));
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

    world.insert_resource(FrameTime(at_ms(101_000)));
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

/// 文の無いトーク（待ち＋イベント）も控えができて進み、イベントが届いて完了する。
#[test]
fn textless_wait_and_event_talk_fires_event_and_completes() {
    use crate::ecs::widget::text::typewriter_ir::{TypewriterEvent, TypewriterEventKind};
    let (mut world, mut schedule) = new_draw_world();
    let target = world.spawn_empty().id();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(200.0, 100.0),
            Brushes::with_background(BG),
            TypewriterTalk::new(
                vec![
                    TypewriterToken::Wait(1.0),
                    TypewriterToken::FireEvent {
                        target,
                        event: TypewriterEventKind::Complete,
                    },
                ],
                at_ms(0),
            ),
        ))
        .id();
    schedule.run(&mut world);
    schedule.run(&mut world);
    assert_eq!(
        world.get::<TypewriterEvent>(target),
        Some(&TypewriterEvent::Complete),
        "文の無いトークでもイベントが届く"
    );
    assert!(world.get::<TypewriterTalk>(entity).unwrap().is_completed());
    assert!(
        list_of(&world, entity).flatten().is_some(),
        "背景は文の係が描く"
    );
}

/// `Typewriter` を外して（絵が消える）入れ直すと、空のトークの背景を描き直す。
#[test]
fn reinserting_typewriter_redraws_empty_talk_background() {
    let (mut world, mut schedule) = new_draw_world();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(100.0, 50.0),
            Brushes::with_background(BG),
        ))
        .id();
    schedule.run(&mut world);
    assert!(
        list_of(&world, entity).flatten().is_some(),
        "背景が描かれる"
    );

    world.entity_mut(entity).remove::<Typewriter>();
    world.flush();
    assert_eq!(list_of(&world, entity), Some(None), "外すと絵が消える");

    world.entity_mut(entity).insert(Typewriter::default());
    schedule.run(&mut world);
    assert!(
        list_of(&world, entity).flatten().is_some(),
        "入れ直したら背景を描き直す"
    );
}

/// 文の無いトークで控えが作れない部品（大きさが足りない）は、背景の係が前の文の絵を消す。
#[test]
fn textless_talk_without_cache_clears_previous_text_list() {
    use crate::ecs::widget::text::typewriter_ir::TypewriterEventKind;
    let (mut world, mut schedule) = new_draw_world();
    let target = world.spawn_empty().id();
    let entity = world
        .spawn((
            Typewriter::default(),
            sized(200.0, 100.0),
            Brushes::with_background(BG),
            text_talk("あいう"),
        ))
        .id();
    schedule.run(&mut world);
    assert!(list_of(&world, entity).flatten().is_some(), "文が描かれる");

    world.entity_mut(entity).insert((
        sized(5.0, 5.0),
        TypewriterTalk::new(
            vec![
                TypewriterToken::Wait(1.0),
                TypewriterToken::FireEvent {
                    target,
                    event: TypewriterEventKind::Complete,
                },
            ],
            at_ms(0),
        ),
    ));
    schedule.run(&mut world);
    assert_eq!(
        list_of(&world, entity),
        Some(None),
        "控えの無い文の無いトークでは前の文の絵を残さない"
    );
}
