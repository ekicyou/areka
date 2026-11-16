//! `areka_runtime::shell_balloon_resolve` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::shell_balloon_resolve::*;
use areka_runtime::skin_switch::SkinKind;
use areka_runtime::skin_switch::SkinSpec;

#[cfg(test)]
#[path = "shell_balloon_resolve_tests.rs"]
mod tests;
