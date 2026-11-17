//! `areka_runtime::frame::window_state` の代わりの親（子のテストの宣言だけ）。

use areka_placement::spawn::GhostWindows;
use areka_runtime::frame::window_state::*;
use areka_runtime::window_state_cue::WindowStateDirective;
use wintf::ecs::window::BandOp;

#[cfg(test)]
#[path = "window_state_tests.rs"]
mod tests;
