//! `areka_runtime::assets` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::assets::*;
use std::collections::BTreeMap;
use std::path::Path;
use areka_emo_atlas::AtlasTable;
use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::EmoWorld;
use areka_parsers::charset::DefaultEncoding;
use areka_sakura::ActorKey;
use areka_seriko::AnimationTable;
use tracing::error;
use areka_runtime::BootWiringError;
use areka_runtime::balloon_background;
use areka_placement::DefaultSurfaces;

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "assets_shell_tests.rs"]
mod shell_tests;
