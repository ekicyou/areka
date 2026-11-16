//! `areka_runtime::frame::window_state` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::frame::window_state::*;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::info;
use wintf::ecs::window::BandOp;
use wintf::ecs::window::StayOnTop;
use wintf::ecs::window::WindowBandQueue;
use areka_runtime::window_state_cue::WindowStateDirective;
use areka_placement::spawn::GhostWindows;

#[cfg(test)]
#[path = "window_state_tests.rs"]
mod tests;
