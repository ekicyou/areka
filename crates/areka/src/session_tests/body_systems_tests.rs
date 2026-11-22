//! 本体の口の束（`BODY_SYSTEMS`）が据わること・据わらないときの倒れ方と、`after_input` の登録の順を確かめるテスト。

use std::any::TypeId;
use std::sync::mpsc;

use areka_kanade::KanadeNotice;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::system::{IntoSystem, System};
use bevy_ecs::world::World;
use log_capture_kit::capture;
use tracing::Level;
use wintf::AppExit;
use wintf::ecs::Input;

use areka_session::body_systems::BodySystems;
use areka_session::ghost_session::register_systems;

fn registered_world() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(AppExit::new());
    let (_stop_tx, stop_rx) = mpsc::channel::<KanadeNotice>();
    register_systems(&mut world, stop_rx, crate::BODY_SYSTEMS);
    world
}

/// 本体の束で登録すると、束が資源として据わる。
#[test]
fn register_systems_installs_body_systems() {
    let world = registered_world();

    assert!(
        world.get_resource::<BodySystems>().is_some(),
        "register_systems の後に BodySystems の資源が無い"
    );
}

/// 束の無い World では `body_systems_missing` の error を 1 行残し、机の問いは偽で答える。
#[test]
fn missing_body_systems_logs_and_answers_idle() {
    let world = World::new();

    let (body, events) = capture(|| BodySystems::of(&world));

    let missing = super::levels_of(&events, "body_systems_missing");
    assert_eq!(
        (
            missing,
            (body.update_busy)(&world),
            (body.install_busy)(&world),
        ),
        (vec![Level::ERROR], false, false),
        "(body_systems_missing の行, update_busy, install_busy)"
    );
}

/// `Input` の段で、インストール → 更新 → MCP の汲み出しの系がこの順に 1 つずつ並ぶ
/// （系の名は bevy の `debug` の機能が無いと読めないので、型で引く）。
#[test]
fn after_input_registers_install_update_mcp_in_order() {
    let world = registered_world();
    let wanted: [TypeId; 3] = [
        IntoSystem::into_system(crate::install::desk::drain).system_type(),
        IntoSystem::into_system(crate::update::desk::drain).system_type(),
        IntoSystem::into_system(crate::mcp::drain).system_type(),
    ];
    let input = world
        .resource::<Schedules>()
        .get(Input)
        .expect("Input の段が在る");

    let found: Vec<TypeId> = input
        .graph()
        .systems
        .iter()
        .map(|(_, system, _)| system.system_type())
        .filter(|ty| wanted.contains(ty))
        .collect();

    assert_eq!(
        found, wanted,
        "Input の段の (install, update, mcp) の汲み出しの系の並びが登録の順でない"
    );
}
