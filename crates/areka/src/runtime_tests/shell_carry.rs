//! `areka_runtime::shell_carry` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::shell_carry::*;
use std::collections::BTreeMap;
use areka_emo_compose::BindSet;
use areka_seriko::BindResolver;
use areka_seriko::DefaultBinds;
use bevy_ecs::world::World;
use tracing::warn;
use areka_runtime::frame::RuntimeWiring;

#[cfg(test)]
#[path = "shell_carry_tests.rs"]
mod tests;
