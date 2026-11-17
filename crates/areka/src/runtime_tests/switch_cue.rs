//! `areka_runtime::switch_cue` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::skin_switch::SkinRequestRaw;
use areka_runtime::switch_cue::*;

#[cfg(test)]
#[path = "switch_cue_tests.rs"]
mod tests;
