use super::*;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::Arrangement;
use crate::ecs::layout::metrics::Size;
use crate::ecs::widget::text::typewriter_layout::init_typewriter_layout;
use bevy_ecs::prelude::*;
use dola::TimeSpan;
use dola::time::testing::{at_ms, ms};
use windows::Win32::Graphics::Direct2D::ID2D1CommandList;

mod cache;
mod draw;
mod events;
mod talk;
mod timeline;

/// グリフ N 個を等間隔 `step` で表示するだけの timeline を、DirectWrite を通さずに組む。
fn make_glyph_timeline(glyph_count: u32, step: TimeSpan) -> TypewriterTimeline {
    let mut items = Vec::new();
    let mut t = TimeSpan::ZERO;
    for i in 0..glyph_count {
        t = t + step;
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

/// 「おしまい」の 4 文字（0.1 秒おき）→ 1 秒の待ち → 宛先への完了の知らせ。
fn wait_then_fire_timeline(target: Entity) -> TypewriterTimeline {
    let mut timeline = make_glyph_timeline(4, ms(100));
    timeline.full_text = "おしまい".into();
    timeline.items.push(TimelineItem::Wait {
        duration: ms(1000),
        start_at: ms(400),
    });
    timeline.items.push(TimelineItem::FireEvent {
        target,
        event: TypewriterEventKind::Complete,
        fire_at: ms(1400),
    });
    timeline
}

/// 控えを作る係を 1 回回す世界。控えは `GraphicsCore` の DirectWrite で作る。
fn new_layout_world() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let mut schedule = Schedule::default();
    schedule.add_systems(init_typewriter_layout);
    (world, schedule)
}

fn text_talk(text: &str) -> TypewriterTalk {
    TypewriterTalk::new(vec![TypewriterToken::Text(text.into())], at_ms(0))
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
