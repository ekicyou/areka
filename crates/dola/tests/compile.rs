// tests/compile.rs — dola compile domain test entry point
#[path = "compile/common/mod.rs"]
mod common;
#[path = "compile/boundary_test.rs"]
mod boundary_test;
#[path = "compile/error_test.rs"]
mod error_test;
#[path = "compile/integration_test.rs"]
mod integration_test;
#[path = "compile/metadata_test.rs"]
mod metadata_test;
#[path = "compile/serde_test.rs"]
mod serde_test;
#[path = "compile/time_resolution_test.rs"]
mod time_resolution_test;
#[path = "compile/transition_test.rs"]
mod transition_test;
