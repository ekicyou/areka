pub(super) mod builder;
#[cfg(test)]
mod builder_opacity_tests;
#[cfg(test)]
mod builder_tests;
mod clip_geometry;
pub mod command;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "計画（タスク 3.x）が項の列を使い始めたら外す")
)]
pub(crate) mod diff;
#[cfg(test)]
mod diff_tests;
mod emit;
#[cfg(test)]
mod emit_blend_tests;
#[cfg(test)]
mod emit_layer_tests;
pub(crate) use emit::emit;
#[cfg(test)]
pub(crate) mod emit_test_support;
#[cfg(test)]
mod emit_tests;
