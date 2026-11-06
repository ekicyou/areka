//! 帯の系と帯の指令の列を本番の結線（[`wire_zorder_pair`](super::wire_zorder_pair)）へ
//! 載せたことを、本番の経路そのもので固定するテスト（stay-on-top-directives task 5.2・
//! 要件 2.1・3.1・7.3）。
//!
//! 並び順の字面は兄弟の `spawn_zorder_chain_wiring_tests.rs` が押さえる。ここは⑴列の資源が
//! World に入ること、⑵確定段を回すとその巡のうちに帯の系が列を空にすることを測る。
//!
//! 積む指令の対象は居ない entity なので、帯の系は窓を 1 枚も書かない（実際の
//! `SetWindowPos` は走らない）。測っているのは配線の実在であり、帯の当て方の正しさは
//! wintf 側の兄弟テスト（`window_band_tests.rs`）が受け持つ。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::Schedules;
use wintf::ecs::FrameFinalize;
use wintf::ecs::window::{BandOp, WindowBandQueue};

use super::wire_zorder_pair;

/// 列の中身の写し（読み出しの口が無いので `Debug` で比べる）。
fn queue_dump(world: &World) -> String {
    format!("{:?}", world.resource::<WindowBandQueue>())
}

/// 空の列の写し。
fn empty_dump() -> String {
    format!("{:?}", WindowBandQueue::default())
}

/// 居ない entity（作ってすぐ消す）。
fn gone_entity(world: &mut World) -> Entity {
    let entity = world.spawn_empty().id();
    world.despawn(entity);
    entity
}

/// 結線すると帯の指令の列が World に入る（取り出しの相が積む先・要件 2.1）。
#[test]
fn the_wiring_inserts_the_band_queue() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    assert!(!world.contains_resource::<WindowBandQueue>(), "対照の前提");

    wire_zorder_pair(&mut world);

    assert!(
        world.contains_resource::<WindowBandQueue>(),
        "結線しても帯の指令の列が World に無い"
    );
    assert_eq!(queue_dump(&world), empty_dump(), "入れたての列は空");
}

/// 結線の前に積まれていた指令は、列を入れ直しても消えない（init は上書きしない）。
#[test]
fn the_wiring_keeps_ops_already_queued() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    let target = gone_entity(&mut world);
    let mut queue = WindowBandQueue::default();
    queue.push(BandOp::Enter, vec![target]);
    let before = format!("{queue:?}");
    world.insert_resource(queue);

    wire_zorder_pair(&mut world);

    assert_eq!(queue_dump(&world), before, "結線が積まれていた指令を消した");
}

/// 本番の結線を回すと、帯の系がその巡のうちに列を空にする（要件 3.1・7.3）。
///
/// 対照: 結線しない World では列がそのまま残る（帯の系の他に列を空にするものは無い）。
#[test]
fn the_wired_route_drains_the_band_queue_in_the_same_pass() {
    let mut wired = World::new();
    wired.init_resource::<Schedules>();
    wire_zorder_pair(&mut wired);
    let target = gone_entity(&mut wired);
    wired
        .resource_mut::<WindowBandQueue>()
        .push(BandOp::Enter, vec![target]);
    assert_ne!(
        queue_dump(&wired),
        empty_dump(),
        "対照の前提: 列に指令が在る"
    );

    wired.run_schedule(FrameFinalize);

    assert_eq!(
        queue_dump(&wired),
        empty_dump(),
        "本番の結線を回しても列が空にならない（確定段に帯の系が載っていない）"
    );

    let mut unwired = World::new();
    unwired.add_schedule(Schedule::new(FrameFinalize));
    let target = gone_entity(&mut unwired);
    let mut queue = WindowBandQueue::default();
    queue.push(BandOp::Enter, vec![target]);
    unwired.insert_resource(queue);

    unwired.run_schedule(FrameFinalize);

    assert_ne!(
        queue_dump(&unwired),
        empty_dump(),
        "結線していない World でも列が空になった（上の檻が配線を測っていない）"
    );
}
