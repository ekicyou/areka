//! `areka_runtime::shell_carry` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::frame::RuntimeWiring;
use areka_runtime::shell_carry::*;
use bevy_ecs::world::World;

#[cfg(test)]
#[path = "shell_carry_tests.rs"]
mod tests;
