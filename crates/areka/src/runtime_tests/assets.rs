//! `areka_runtime::assets` の代わりの親（子のテストの宣言だけ）。

use areka_emo_atlas::WicDecoderArm;
use areka_runtime::assets::*;

#[cfg(test)]
#[path = "assets_shell_tests.rs"]
mod shell_tests;
#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
