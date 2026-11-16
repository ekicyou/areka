//! `areka_runtime::switch_cue` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::switch_cue::*;
use dola::cue::TalkCue;
use tracing::debug;
use tracing::warn;
use areka_runtime::skin_switch::SkinKind;
use areka_runtime::skin_switch::SkinRequestRaw;

#[cfg(test)]
#[path = "switch_cue_tests.rs"]
mod tests;
