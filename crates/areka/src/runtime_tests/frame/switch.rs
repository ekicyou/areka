//! `areka_runtime::frame::switch` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::frame::switch::*;

#[cfg(test)]
#[path = "switch_finish_tests.rs"]
mod finish_tests;
#[cfg(test)]
#[path = "switch_tests.rs"]
mod tests;
