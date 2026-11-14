pub(super) mod builder;
#[cfg(test)]
mod builder_opacity_tests;
#[cfg(test)]
mod builder_tests;
mod clip_geometry;
pub mod command;
pub(crate) mod diff;
#[cfg(test)]
mod diff_tests;
mod emit;
#[cfg(test)]
mod emit_blend_tests;
#[cfg(test)]
mod emit_layer_tests;
pub(crate) use emit::{emit, emit_within};
#[cfg(test)]
pub(crate) mod emit_test_support;
#[cfg(test)]
mod emit_tests;
