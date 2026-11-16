//! `areka_runtime::shell_balloon_resolve` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::shell_balloon_resolve::*;
use std::path::Path;
use std::path::PathBuf;
use areka_ghost::BasewareRoots;
use bevy_ecs::world::World;
use areka_runtime::skin_switch::SkinKind;
use areka_runtime::skin_switch::SkinSpec;

#[cfg(test)]
#[path = "shell_balloon_resolve_tests.rs"]
mod tests;
